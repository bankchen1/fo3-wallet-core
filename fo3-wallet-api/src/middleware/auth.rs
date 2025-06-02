//! Authentication and authorization middleware

use std::sync::Arc;
use std::collections::HashMap;
use tonic::{Request, Status, metadata::MetadataValue};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation, Algorithm};
use serde::{Deserialize, Serialize};
use bcrypt::{hash, verify, DEFAULT_COST};
use uuid::Uuid;
use chrono::{DateTime, Utc, Duration};

use crate::proto::fo3::wallet::v1::{UserRole, Permission};
use crate::state::AppState;

/// JWT claims structure
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,        // Subject (user ID)
    pub username: String,   // Username
    pub role: i32,         // User role
    pub permissions: Vec<i32>, // User permissions
    pub exp: i64,          // Expiration time
    pub iat: i64,          // Issued at
    pub jti: String,       // JWT ID
}

/// API key structure
#[derive(Debug, Clone)]
pub struct ApiKeyData {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub permissions: Vec<Permission>,
    pub rate_limit: RateLimit,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimit {
    pub requests_per_minute: i32,
    pub burst_limit: i32,
    pub daily_limit: i32,
}

/// Authentication context
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: String,
    pub username: String,
    pub role: UserRole,
    pub permissions: Vec<Permission>,
    pub auth_type: AuthType,
}

/// Authentication type
#[derive(Debug, Clone)]
pub enum AuthType {
    JWT(String),
    ApiKey(String),
}

/// Authentication service
pub struct AuthService {
    state: Arc<AppState>,
    jwt_secret: String,
    api_keys: Arc<tokio::sync::RwLock<HashMap<String, ApiKeyData>>>,
}

impl AuthService {
    pub fn new(state: Arc<AppState>) -> Self {
        let jwt_secret = std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| "default_secret_change_in_production".to_string());

        Self {
            state,
            jwt_secret,
            api_keys: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Generate JWT token
    pub fn generate_jwt(&self, user_id: &str, username: &str, role: UserRole, permissions: Vec<Permission>) -> Result<String, Status> {
        let now = Utc::now();
        let exp = now + Duration::hours(24); // 24 hour expiration

        let claims = Claims {
            sub: user_id.to_string(),
            username: username.to_string(),
            role: role as i32,
            permissions: permissions.iter().map(|p| *p as i32).collect(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
            jti: Uuid::new_v4().to_string(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_ref()),
        )
        .map_err(|e| Status::internal(format!("Failed to generate JWT: {}", e)))
    }

    /// Validate JWT token
    pub fn validate_jwt(&self, token: &str) -> Result<Claims, Status> {
        let validation = Validation::new(Algorithm::HS256);
        
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_ref()),
            &validation,
        )
        .map(|data| data.claims)
        .map_err(|e| Status::unauthenticated(format!("Invalid JWT: {}", e)))
    }

    /// Generate API key
    pub async fn generate_api_key(
        &self,
        user_id: &str,
        name: &str,
        permissions: Vec<Permission>,
        rate_limit: RateLimit,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<(String, String), Status> {
        let key_id = Uuid::new_v4().to_string();
        let secret = Uuid::new_v4().to_string();
        let key_hash = hash(&secret, DEFAULT_COST)
            .map_err(|e| Status::internal(format!("Failed to hash API key: {}", e)))?;

        let api_key = ApiKeyData {
            id: key_id.clone(),
            user_id: user_id.to_string(),
            name: name.to_string(),
            permissions,
            rate_limit,
            is_active: true,
            created_at: Utc::now(),
            last_used: None,
            expires_at,
        };

        // Store in memory (in production, store in database)
        let mut keys = self.api_keys.write().await;
        keys.insert(key_hash, api_key);

        // Return key prefix for identification and full secret
        let key_prefix = format!("fo3_{}", &secret[..8]);
        let full_key = format!("fo3_{}", secret);

        Ok((key_prefix, full_key))
    }

    /// Validate API key
    pub async fn validate_api_key(&self, key: &str) -> Result<ApiKeyData, Status> {
        if !key.starts_with("fo3_") {
            return Err(Status::unauthenticated("Invalid API key format"));
        }

        let secret = &key[4..]; // Remove "fo3_" prefix
        let keys = self.api_keys.read().await;

        // In production, this would be a database lookup with proper indexing
        for (hash, api_key) in keys.iter() {
            if verify(secret, hash).unwrap_or(false) {
                if !api_key.is_active {
                    return Err(Status::unauthenticated("API key is inactive"));
                }

                if let Some(expires_at) = api_key.expires_at {
                    if Utc::now() > expires_at {
                        return Err(Status::unauthenticated("API key has expired"));
                    }
                }

                return Ok(api_key.clone());
            }
        }

        Err(Status::unauthenticated("Invalid API key"))
    }

    /// Extract authentication from request metadata
    pub async fn extract_auth(&self, request: &Request<()>) -> Result<AuthContext, Status> {
        let metadata = request.metadata();

        // Try JWT token first
        if let Some(auth_header) = metadata.get("authorization") {
            let auth_str = auth_header.to_str()
                .map_err(|_| Status::unauthenticated("Invalid authorization header"))?;

            if auth_str.starts_with("Bearer ") {
                let token = &auth_str[7..];
                let claims = self.validate_jwt(token)?;

                return Ok(AuthContext {
                    user_id: claims.sub,
                    username: claims.username,
                    role: UserRole::try_from(claims.role).unwrap_or(UserRole::UserRoleUser),
                    permissions: claims.permissions.into_iter()
                        .filter_map(|p| Permission::try_from(p).ok())
                        .collect(),
                    auth_type: AuthType::JWT(token.to_string()),
                });
            }
        }

        // Try API key
        if let Some(api_key_header) = metadata.get("x-api-key") {
            let api_key = api_key_header.to_str()
                .map_err(|_| Status::unauthenticated("Invalid API key header"))?;

            let api_key_data = self.validate_api_key(api_key).await?;

            return Ok(AuthContext {
                user_id: api_key_data.user_id,
                username: format!("api_key_{}", api_key_data.name),
                role: UserRole::UserRoleUser, // API keys default to user role
                permissions: api_key_data.permissions,
                auth_type: AuthType::ApiKey(api_key.to_string()),
            });
        }

        Err(Status::unauthenticated("No valid authentication provided"))
    }

    /// Check if user has required permission
    pub fn check_permission(&self, auth: &AuthContext, required_permission: Permission) -> Result<(), Status> {
        // Super admin has all permissions
        if auth.role == UserRole::UserRoleSuperAdmin {
            return Ok(());
        }

        // Admin has most permissions except super admin actions
        if auth.role == UserRole::UserRoleAdmin && required_permission != Permission::PermissionAdmin {
            return Ok(());
        }

        // Check specific permissions
        if auth.permissions.contains(&required_permission) {
            return Ok(());
        }

        Err(Status::permission_denied(format!(
            "Insufficient permissions. Required: {:?}",
            required_permission
        )))
    }
}

/// Authentication interceptor for gRPC services
pub struct AuthInterceptor {
    auth_service: Arc<AuthService>,
    required_permission: Permission,
}

impl AuthInterceptor {
    pub fn new(auth_service: Arc<AuthService>, required_permission: Permission) -> Self {
        Self {
            auth_service,
            required_permission,
        }
    }

    pub async fn intercept<T>(&self, mut request: Request<T>) -> Result<Request<T>, Status> {
        // Create a dummy request for auth extraction
        let auth_request = Request::from_parts(request.metadata().clone(), ());
        let auth_context = self.auth_service.extract_auth(&auth_request).await?;

        // Check permissions
        self.auth_service.check_permission(&auth_context, self.required_permission)?;

        // Add auth context to request extensions
        request.extensions_mut().insert(auth_context);

        Ok(request)
    }
}

/// Helper macro to create authenticated gRPC service
#[macro_export]
macro_rules! authenticated_service {
    ($service:expr, $auth_service:expr, $permission:expr) => {{
        let interceptor = AuthInterceptor::new($auth_service, $permission);
        tonic::service::interceptor($service, move |req| {
            let interceptor = interceptor.clone();
            async move { interceptor.intercept(req).await }
        })
    }};
}

/// Password hashing utilities
pub struct PasswordUtils;

impl PasswordUtils {
    pub fn hash_password(password: &str) -> Result<String, Status> {
        hash(password, DEFAULT_COST)
            .map_err(|e| Status::internal(format!("Failed to hash password: {}", e)))
    }

    pub fn verify_password(password: &str, hash: &str) -> Result<bool, Status> {
        verify(password, hash)
            .map_err(|e| Status::internal(format!("Failed to verify password: {}", e)))
    }
}
