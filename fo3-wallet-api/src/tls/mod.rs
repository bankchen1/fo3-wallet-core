//! TLS configuration and certificate management

use std::fs;
use std::path::Path;
use tonic::transport::{Identity, ServerTlsConfig, Certificate};
use anyhow::{Result, Context};

/// TLS configuration for the gRPC server
pub struct TlsConfig {
    pub server_config: ServerTlsConfig,
    pub cert_path: String,
    pub key_path: String,
    pub ca_cert_path: Option<String>,
}

impl TlsConfig {
    /// Create TLS configuration from certificate files
    pub fn from_files(
        cert_path: &str,
        key_path: &str,
        ca_cert_path: Option<&str>,
    ) -> Result<Self> {
        // Read server certificate and private key
        let cert = fs::read_to_string(cert_path)
            .with_context(|| format!("Failed to read certificate file: {}", cert_path))?;
        
        let key = fs::read_to_string(key_path)
            .with_context(|| format!("Failed to read private key file: {}", key_path))?;

        let identity = Identity::from_pem(cert, key);
        let mut server_config = ServerTlsConfig::new().identity(identity);

        // Add client certificate validation if CA cert is provided
        if let Some(ca_path) = ca_cert_path {
            let ca_cert = fs::read_to_string(ca_path)
                .with_context(|| format!("Failed to read CA certificate file: {}", ca_path))?;
            
            let ca_certificate = Certificate::from_pem(ca_cert);
            server_config = server_config.client_ca_root(ca_certificate);
        }

        Ok(Self {
            server_config,
            cert_path: cert_path.to_string(),
            key_path: key_path.to_string(),
            ca_cert_path: ca_cert_path.map(|s| s.to_string()),
        })
    }

    /// Create TLS configuration with self-signed certificate
    pub fn self_signed() -> Result<Self> {
        let cert_dir = std::env::var("CERT_DIR").unwrap_or_else(|_| "./certs".to_string());
        let cert_path = format!("{}/server.crt", cert_dir);
        let key_path = format!("{}/server.key", cert_dir);

        // Create certificate directory if it doesn't exist
        fs::create_dir_all(&cert_dir)
            .with_context(|| format!("Failed to create certificate directory: {}", cert_dir))?;

        // Generate self-signed certificate if it doesn't exist
        if !Path::new(&cert_path).exists() || !Path::new(&key_path).exists() {
            Self::generate_self_signed_cert(&cert_path, &key_path)?;
        }

        Self::from_files(&cert_path, &key_path, None)
    }

    /// Generate self-signed certificate
    fn generate_self_signed_cert(cert_path: &str, key_path: &str) -> Result<()> {
        use openssl::asn1::Asn1Time;
        use openssl::bn::{BigNum, MsbOption};
        use openssl::hash::MessageDigest;
        use openssl::pkey::{PKey, Private};
        use openssl::rsa::Rsa;
        use openssl::x509::extension::{BasicConstraints, KeyUsage, SubjectAlternativeName};
        use openssl::x509::{X509NameBuilder, X509};

        // Generate RSA key pair
        let rsa = Rsa::generate(2048)?;
        let key_pair = PKey::from_rsa(rsa)?;

        // Create certificate
        let mut cert_builder = X509::builder()?;
        cert_builder.set_version(2)?;

        // Set serial number
        let serial_number = {
            let mut serial = BigNum::new()?;
            serial.rand(159, MsbOption::MAYBE_ZERO, false)?;
            serial.to_asn1_integer()?
        };
        cert_builder.set_serial_number(&serial_number)?;

        // Set subject name
        let mut subject_name = X509NameBuilder::new()?;
        subject_name.append_entry_by_text("C", "US")?;
        subject_name.append_entry_by_text("ST", "CA")?;
        subject_name.append_entry_by_text("O", "FO3 Wallet")?;
        subject_name.append_entry_by_text("CN", "localhost")?;
        let subject_name = subject_name.build();
        cert_builder.set_subject_name(&subject_name)?;
        cert_builder.set_issuer_name(&subject_name)?;

        // Set validity period (1 year)
        let not_before = Asn1Time::days_from_now(0)?;
        let not_after = Asn1Time::days_from_now(365)?;
        cert_builder.set_not_before(&not_before)?;
        cert_builder.set_not_after(&not_after)?;

        // Set public key
        cert_builder.set_pubkey(&key_pair)?;

        // Add extensions
        let context = cert_builder.x509v3_context(None, None);
        let basic_constraints = BasicConstraints::new().critical().ca().build()?;
        cert_builder.append_extension(basic_constraints)?;

        let key_usage = KeyUsage::new()
            .critical()
            .key_encipherment()
            .digital_signature()
            .build()?;
        cert_builder.append_extension(key_usage)?;

        let subject_alt_name = SubjectAlternativeName::new()
            .dns("localhost")
            .dns("fo3-wallet-api")
            .ip("127.0.0.1")
            .ip("::1")
            .build(&context)?;
        cert_builder.append_extension(subject_alt_name)?;

        // Sign the certificate
        cert_builder.sign(&key_pair, MessageDigest::sha256())?;
        let cert = cert_builder.build();

        // Write certificate and private key to files
        let cert_pem = cert.to_pem()?;
        let key_pem = key_pair.private_key_to_pem_pkcs8()?;

        fs::write(cert_path, cert_pem)
            .with_context(|| format!("Failed to write certificate to: {}", cert_path))?;
        
        fs::write(key_path, key_pem)
            .with_context(|| format!("Failed to write private key to: {}", key_path))?;

        tracing::info!("Generated self-signed certificate: {}", cert_path);
        tracing::info!("Generated private key: {}", key_path);

        Ok(())
    }

    /// Check if certificates need renewal (for Let's Encrypt integration)
    pub fn needs_renewal(&self) -> Result<bool> {
        use openssl::x509::X509;
        use chrono::{DateTime, Utc};

        let cert_pem = fs::read_to_string(&self.cert_path)?;
        let cert = X509::from_pem(cert_pem.as_bytes())?;
        
        let not_after = cert.not_after();
        let expiry_time = DateTime::parse_from_rfc3339(&not_after.to_string())?;
        let expiry_utc = expiry_time.with_timezone(&Utc);
        
        // Renew if certificate expires within 30 days
        let renewal_threshold = Utc::now() + chrono::Duration::days(30);
        
        Ok(expiry_utc < renewal_threshold)
    }
}

/// Certificate manager for automatic renewal
pub struct CertificateManager {
    config: TlsConfig,
    auto_renew: bool,
}

impl CertificateManager {
    pub fn new(config: TlsConfig, auto_renew: bool) -> Self {
        Self { config, auto_renew }
    }

    /// Start certificate renewal background task
    pub async fn start_renewal_task(&self) {
        if !self.auto_renew {
            return;
        }

        let cert_path = self.config.cert_path.clone();
        let key_path = self.config.key_path.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(86400)); // Check daily
            
            loop {
                interval.tick().await;
                
                // Check if renewal is needed
                if let Ok(config) = TlsConfig::from_files(&cert_path, &key_path, None) {
                    if let Ok(needs_renewal) = config.needs_renewal() {
                        if needs_renewal {
                            tracing::warn!("Certificate needs renewal: {}", cert_path);
                            
                            // In production, integrate with Let's Encrypt or certificate authority
                            // For now, generate new self-signed certificate
                            if let Err(e) = TlsConfig::generate_self_signed_cert(&cert_path, &key_path) {
                                tracing::error!("Failed to renew certificate: {}", e);
                            } else {
                                tracing::info!("Certificate renewed successfully");
                            }
                        }
                    }
                }
            }
        });
    }
}

/// Helper function to get TLS configuration from environment
pub fn get_tls_config() -> Result<Option<TlsConfig>> {
    let enable_tls = std::env::var("ENABLE_TLS")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);

    if !enable_tls {
        return Ok(None);
    }

    let cert_path = std::env::var("TLS_CERT_PATH");
    let key_path = std::env::var("TLS_KEY_PATH");
    let ca_cert_path = std::env::var("TLS_CA_CERT_PATH").ok();

    match (cert_path, key_path) {
        (Ok(cert), Ok(key)) => {
            // Use provided certificate files
            let config = TlsConfig::from_files(&cert, &key, ca_cert_path.as_deref())?;
            Ok(Some(config))
        }
        _ => {
            // Generate self-signed certificate
            let config = TlsConfig::self_signed()?;
            Ok(Some(config))
        }
    }
}
