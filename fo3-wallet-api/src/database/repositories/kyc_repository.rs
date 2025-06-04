//! SQLx-based KYC repository implementation
//! 
//! Replaces the in-memory HashMap storage with persistent database operations

use async_trait::async_trait;
use sqlx::{Row, FromRow};
use uuid::Uuid;
use chrono::{DateTime, Utc, NaiveDate};
use tracing::{info, error, warn};

use crate::database::connection::DatabasePool;
use crate::models::kyc::{KycRepository, KycSubmission, KycStatus, PersonalInfo, Address, Document, DocumentType};
use crate::error::ServiceError;

/// SQLx-based KYC repository implementation
pub struct SqlxKycRepository {
    pool: DatabasePool,
}

impl SqlxKycRepository {
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl KycRepository for SqlxKycRepository {
    type Error = ServiceError;

    async fn create_submission(&self, submission: &KycSubmission) -> Result<(), Self::Error> {
        info!("Creating KYC submission for wallet: {}", submission.wallet_id);

        let query = r#"
            INSERT INTO kyc_submissions (
                id, wallet_id, status, first_name, last_name, date_of_birth,
                nationality, country_of_residence, street_address, city,
                state_province, postal_code, address_country, submitted_at,
                reviewed_at, reviewer_id, reviewer_notes, rejection_reason, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)
        "#;

        match &self.pool {
            DatabasePool::Postgres(pool) => {
                sqlx::query(query)
                    .bind(submission.id)
                    .bind(submission.wallet_id)
                    .bind(submission.status.to_string())
                    .bind(&submission.personal_info.first_name)
                    .bind(&submission.personal_info.last_name)
                    .bind(submission.personal_info.date_of_birth)
                    .bind(&submission.personal_info.nationality)
                    .bind(&submission.personal_info.country_of_residence)
                    .bind(&submission.personal_info.address.street_address)
                    .bind(&submission.personal_info.address.city)
                    .bind(&submission.personal_info.address.state_province)
                    .bind(&submission.personal_info.address.postal_code)
                    .bind(&submission.personal_info.address.country)
                    .bind(submission.submitted_at)
                    .bind(submission.reviewed_at)
                    .bind(&submission.reviewer_id)
                    .bind(&submission.reviewer_notes)
                    .bind(&submission.rejection_reason)
                    .bind(submission.updated_at)
                    .execute(pool)
                    .await
                    .map_err(|e| ServiceError::DatabaseError(format!("Failed to create KYC submission: {}", e)))?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(query)
                    .bind(submission.id.to_string())
                    .bind(submission.wallet_id.to_string())
                    .bind(submission.status.to_string())
                    .bind(&submission.personal_info.first_name)
                    .bind(&submission.personal_info.last_name)
                    .bind(submission.personal_info.date_of_birth.format("%Y-%m-%d").to_string())
                    .bind(&submission.personal_info.nationality)
                    .bind(&submission.personal_info.country_of_residence)
                    .bind(&submission.personal_info.address.street_address)
                    .bind(&submission.personal_info.address.city)
                    .bind(&submission.personal_info.address.state_province)
                    .bind(&submission.personal_info.address.postal_code)
                    .bind(&submission.personal_info.address.country)
                    .bind(submission.submitted_at.to_rfc3339())
                    .bind(submission.reviewed_at.map(|dt| dt.to_rfc3339()))
                    .bind(&submission.reviewer_id)
                    .bind(&submission.reviewer_notes)
                    .bind(&submission.rejection_reason)
                    .bind(submission.updated_at.to_rfc3339())
                    .execute(pool)
                    .await
                    .map_err(|e| ServiceError::DatabaseError(format!("Failed to create KYC submission: {}", e)))?;
            }
        }

        info!("KYC submission created successfully: {}", submission.id);
        Ok(())
    }

    async fn get_submission_by_id(&self, id: Uuid) -> Result<Option<KycSubmission>, Self::Error> {
        info!("Fetching KYC submission by ID: {}", id);

        let query = r#"
            SELECT id, wallet_id, status, first_name, last_name, date_of_birth,
                   nationality, country_of_residence, street_address, city,
                   state_province, postal_code, address_country, submitted_at,
                   reviewed_at, reviewer_id, reviewer_notes, rejection_reason, updated_at
            FROM kyc_submissions WHERE id = $1
        "#;

        match &self.pool {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(query)
                    .bind(id)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| ServiceError::DatabaseError(format!("Failed to fetch KYC submission: {}", e)))?;

                if let Some(row) = row {
                    let submission = self.row_to_kyc_submission_postgres(&row)?;
                    Ok(Some(submission))
                } else {
                    Ok(None)
                }
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(query)
                    .bind(id.to_string())
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| ServiceError::DatabaseError(format!("Failed to fetch KYC submission: {}", e)))?;

                if let Some(row) = row {
                    let submission = self.row_to_kyc_submission_sqlite(&row)?;
                    Ok(Some(submission))
                } else {
                    Ok(None)
                }
            }
        }
    }

    async fn get_submission_by_wallet_id(&self, wallet_id: Uuid) -> Result<Option<KycSubmission>, Self::Error> {
        info!("Fetching KYC submission by wallet ID: {}", wallet_id);

        let query = r#"
            SELECT id, wallet_id, status, first_name, last_name, date_of_birth,
                   nationality, country_of_residence, street_address, city,
                   state_province, postal_code, address_country, submitted_at,
                   reviewed_at, reviewer_id, reviewer_notes, rejection_reason, updated_at
            FROM kyc_submissions WHERE wallet_id = $1
        "#;

        match &self.pool {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(query)
                    .bind(wallet_id)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| ServiceError::DatabaseError(format!("Failed to fetch KYC submission: {}", e)))?;

                if let Some(row) = row {
                    let submission = self.row_to_kyc_submission_postgres(&row)?;
                    Ok(Some(submission))
                } else {
                    Ok(None)
                }
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(query)
                    .bind(wallet_id.to_string())
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| ServiceError::DatabaseError(format!("Failed to fetch KYC submission: {}", e)))?;

                if let Some(row) = row {
                    let submission = self.row_to_kyc_submission_sqlite(&row)?;
                    Ok(Some(submission))
                } else {
                    Ok(None)
                }
            }
        }
    }

    async fn update_submission(&self, submission: &KycSubmission) -> Result<(), Self::Error> {
        info!("Updating KYC submission: {}", submission.id);

        let query = r#"
            UPDATE kyc_submissions SET
                status = $2, first_name = $3, last_name = $4, date_of_birth = $5,
                nationality = $6, country_of_residence = $7, street_address = $8,
                city = $9, state_province = $10, postal_code = $11, address_country = $12,
                reviewed_at = $13, reviewer_id = $14, reviewer_notes = $15,
                rejection_reason = $16, updated_at = $17
            WHERE id = $1
        "#;

        match &self.pool {
            DatabasePool::Postgres(pool) => {
                sqlx::query(query)
                    .bind(submission.id)
                    .bind(submission.status.to_string())
                    .bind(&submission.personal_info.first_name)
                    .bind(&submission.personal_info.last_name)
                    .bind(submission.personal_info.date_of_birth)
                    .bind(&submission.personal_info.nationality)
                    .bind(&submission.personal_info.country_of_residence)
                    .bind(&submission.personal_info.address.street_address)
                    .bind(&submission.personal_info.address.city)
                    .bind(&submission.personal_info.address.state_province)
                    .bind(&submission.personal_info.address.postal_code)
                    .bind(&submission.personal_info.address.country)
                    .bind(submission.reviewed_at)
                    .bind(&submission.reviewer_id)
                    .bind(&submission.reviewer_notes)
                    .bind(&submission.rejection_reason)
                    .bind(submission.updated_at)
                    .execute(pool)
                    .await
                    .map_err(|e| ServiceError::DatabaseError(format!("Failed to update KYC submission: {}", e)))?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(query)
                    .bind(submission.id.to_string())
                    .bind(submission.status.to_string())
                    .bind(&submission.personal_info.first_name)
                    .bind(&submission.personal_info.last_name)
                    .bind(submission.personal_info.date_of_birth.format("%Y-%m-%d").to_string())
                    .bind(&submission.personal_info.nationality)
                    .bind(&submission.personal_info.country_of_residence)
                    .bind(&submission.personal_info.address.street_address)
                    .bind(&submission.personal_info.address.city)
                    .bind(&submission.personal_info.address.state_province)
                    .bind(&submission.personal_info.address.postal_code)
                    .bind(&submission.personal_info.address.country)
                    .bind(submission.reviewed_at.map(|dt| dt.to_rfc3339()))
                    .bind(&submission.reviewer_id)
                    .bind(&submission.reviewer_notes)
                    .bind(&submission.rejection_reason)
                    .bind(submission.updated_at.to_rfc3339())
                    .execute(pool)
                    .await
                    .map_err(|e| ServiceError::DatabaseError(format!("Failed to update KYC submission: {}", e)))?;
            }
        }

        info!("KYC submission updated successfully: {}", submission.id);
        Ok(())
    }

    async fn list_submissions(&self, limit: Option<i32>, offset: Option<i32>) -> Result<Vec<KycSubmission>, Self::Error> {
        let limit = limit.unwrap_or(50);
        let offset = offset.unwrap_or(0);

        info!("Listing KYC submissions with limit: {}, offset: {}", limit, offset);

        let query = r#"
            SELECT id, wallet_id, status, first_name, last_name, date_of_birth,
                   nationality, country_of_residence, street_address, city,
                   state_province, postal_code, address_country, submitted_at,
                   reviewed_at, reviewer_id, reviewer_notes, rejection_reason, updated_at
            FROM kyc_submissions
            ORDER BY submitted_at DESC
            LIMIT $1 OFFSET $2
        "#;

        match &self.pool {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query(query)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(pool)
                    .await
                    .map_err(|e| ServiceError::DatabaseError(format!("Failed to list KYC submissions: {}", e)))?;

                let submissions = rows.iter()
                    .map(|row| self.row_to_kyc_submission_postgres(row))
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(submissions)
            }
            DatabasePool::Sqlite(pool) => {
                let rows = sqlx::query(query)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(pool)
                    .await
                    .map_err(|e| ServiceError::DatabaseError(format!("Failed to list KYC submissions: {}", e)))?;

                let submissions = rows.iter()
                    .map(|row| self.row_to_kyc_submission_sqlite(row))
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(submissions)
            }
        }
    }

    async fn delete_submission(&self, id: Uuid) -> Result<(), Self::Error> {
        info!("Deleting KYC submission: {}", id);

        let query = "DELETE FROM kyc_submissions WHERE id = $1";

        match &self.pool {
            DatabasePool::Postgres(pool) => {
                sqlx::query(query)
                    .bind(id)
                    .execute(pool)
                    .await
                    .map_err(|e| ServiceError::DatabaseError(format!("Failed to delete KYC submission: {}", e)))?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(query)
                    .bind(id.to_string())
                    .execute(pool)
                    .await
                    .map_err(|e| ServiceError::DatabaseError(format!("Failed to delete KYC submission: {}", e)))?;
            }
        }

        info!("KYC submission deleted successfully: {}", id);
        Ok(())
    }
}

impl SqlxKycRepository {
    /// Convert PostgreSQL row to KycSubmission
    fn row_to_kyc_submission_postgres(&self, row: &sqlx::postgres::PgRow) -> Result<KycSubmission, ServiceError> {
        let id: Uuid = row.try_get("id")
            .map_err(|e| ServiceError::DatabaseError(format!("Failed to get id: {}", e)))?;
        let wallet_id: Uuid = row.try_get("wallet_id")
            .map_err(|e| ServiceError::DatabaseError(format!("Failed to get wallet_id: {}", e)))?;
        let status_str: String = row.try_get("status")
            .map_err(|e| ServiceError::DatabaseError(format!("Failed to get status: {}", e)))?;
        let status = KycStatus::from_string(&status_str);

        let personal_info = PersonalInfo {
            first_name: row.try_get("first_name")
                .map_err(|e| ServiceError::DatabaseError(format!("Failed to get first_name: {}", e)))?,
            last_name: row.try_get("last_name")
                .map_err(|e| ServiceError::DatabaseError(format!("Failed to get last_name: {}", e)))?,
            date_of_birth: row.try_get("date_of_birth")
                .map_err(|e| ServiceError::DatabaseError(format!("Failed to get date_of_birth: {}", e)))?,
            nationality: row.try_get("nationality")
                .map_err(|e| ServiceError::DatabaseError(format!("Failed to get nationality: {}", e)))?,
            country_of_residence: row.try_get("country_of_residence")
                .map_err(|e| ServiceError::DatabaseError(format!("Failed to get country_of_residence: {}", e)))?,
            address: Address {
                street_address: row.try_get("street_address")
                    .map_err(|e| ServiceError::DatabaseError(format!("Failed to get street_address: {}", e)))?,
                city: row.try_get("city")
                    .map_err(|e| ServiceError::DatabaseError(format!("Failed to get city: {}", e)))?,
                state_province: row.try_get("state_province")
                    .map_err(|e| ServiceError::DatabaseError(format!("Failed to get state_province: {}", e)))?,
                postal_code: row.try_get("postal_code")
                    .map_err(|e| ServiceError::DatabaseError(format!("Failed to get postal_code: {}", e)))?,
                country: row.try_get("address_country")
                    .map_err(|e| ServiceError::DatabaseError(format!("Failed to get address_country: {}", e)))?,
            },
        };

        Ok(KycSubmission {
            id,
            wallet_id,
            status,
            personal_info,
            documents: Vec::new(), // Documents are loaded separately
            submitted_at: row.try_get("submitted_at")
                .map_err(|e| ServiceError::DatabaseError(format!("Failed to get submitted_at: {}", e)))?,
            reviewed_at: row.try_get("reviewed_at")
                .map_err(|e| ServiceError::DatabaseError(format!("Failed to get reviewed_at: {}", e)))?,
            reviewer_id: row.try_get("reviewer_id")
                .map_err(|e| ServiceError::DatabaseError(format!("Failed to get reviewer_id: {}", e)))?,
            reviewer_notes: row.try_get("reviewer_notes")
                .map_err(|e| ServiceError::DatabaseError(format!("Failed to get reviewer_notes: {}", e)))?,
            rejection_reason: row.try_get("rejection_reason")
                .map_err(|e| ServiceError::DatabaseError(format!("Failed to get rejection_reason: {}", e)))?,
            updated_at: row.try_get("updated_at")
                .map_err(|e| ServiceError::DatabaseError(format!("Failed to get updated_at: {}", e)))?,
        })
    }

    /// Convert SQLite row to KycSubmission
    fn row_to_kyc_submission_sqlite(&self, row: &sqlx::sqlite::SqliteRow) -> Result<KycSubmission, ServiceError> {
        let id_str: String = row.try_get("id")
            .map_err(|e| ServiceError::DatabaseError(format!("Failed to get id: {}", e)))?;
        let id = Uuid::parse_str(&id_str)
            .map_err(|e| ServiceError::DatabaseError(format!("Failed to parse id UUID: {}", e)))?;

        let wallet_id_str: String = row.try_get("wallet_id")
            .map_err(|e| ServiceError::DatabaseError(format!("Failed to get wallet_id: {}", e)))?;
        let wallet_id = Uuid::parse_str(&wallet_id_str)
            .map_err(|e| ServiceError::DatabaseError(format!("Failed to parse wallet_id UUID: {}", e)))?;

        let status_str: String = row.try_get("status")
            .map_err(|e| ServiceError::DatabaseError(format!("Failed to get status: {}", e)))?;
        let status = KycStatus::from_string(&status_str);

        let date_of_birth_str: String = row.try_get("date_of_birth")
            .map_err(|e| ServiceError::DatabaseError(format!("Failed to get date_of_birth: {}", e)))?;
        let date_of_birth = NaiveDate::parse_from_str(&date_of_birth_str, "%Y-%m-%d")
            .map_err(|e| ServiceError::DatabaseError(format!("Failed to parse date_of_birth: {}", e)))?;

        let personal_info = PersonalInfo {
            first_name: row.try_get("first_name")
                .map_err(|e| ServiceError::DatabaseError(format!("Failed to get first_name: {}", e)))?,
            last_name: row.try_get("last_name")
                .map_err(|e| ServiceError::DatabaseError(format!("Failed to get last_name: {}", e)))?,
            date_of_birth,
            nationality: row.try_get("nationality")
                .map_err(|e| ServiceError::DatabaseError(format!("Failed to get nationality: {}", e)))?,
            country_of_residence: row.try_get("country_of_residence")
                .map_err(|e| ServiceError::DatabaseError(format!("Failed to get country_of_residence: {}", e)))?,
            address: Address {
                street_address: row.try_get("street_address")
                    .map_err(|e| ServiceError::DatabaseError(format!("Failed to get street_address: {}", e)))?,
                city: row.try_get("city")
                    .map_err(|e| ServiceError::DatabaseError(format!("Failed to get city: {}", e)))?,
                state_province: row.try_get("state_province")
                    .map_err(|e| ServiceError::DatabaseError(format!("Failed to get state_province: {}", e)))?,
                postal_code: row.try_get("postal_code")
                    .map_err(|e| ServiceError::DatabaseError(format!("Failed to get postal_code: {}", e)))?,
                country: row.try_get("address_country")
                    .map_err(|e| ServiceError::DatabaseError(format!("Failed to get address_country: {}", e)))?,
            },
        };

        let submitted_at_str: String = row.try_get("submitted_at")
            .map_err(|e| ServiceError::DatabaseError(format!("Failed to get submitted_at: {}", e)))?;
        let submitted_at = DateTime::parse_from_rfc3339(&submitted_at_str)
            .map_err(|e| ServiceError::DatabaseError(format!("Failed to parse submitted_at: {}", e)))?
            .with_timezone(&Utc);

        let reviewed_at = if let Ok(reviewed_at_str) = row.try_get::<Option<String>, _>("reviewed_at") {
            if let Some(reviewed_at_str) = reviewed_at_str {
                Some(DateTime::parse_from_rfc3339(&reviewed_at_str)
                    .map_err(|e| ServiceError::DatabaseError(format!("Failed to parse reviewed_at: {}", e)))?
                    .with_timezone(&Utc))
            } else {
                None
            }
        } else {
            None
        };

        let updated_at_str: String = row.try_get("updated_at")
            .map_err(|e| ServiceError::DatabaseError(format!("Failed to get updated_at: {}", e)))?;
        let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
            .map_err(|e| ServiceError::DatabaseError(format!("Failed to parse updated_at: {}", e)))?
            .with_timezone(&Utc);

        Ok(KycSubmission {
            id,
            wallet_id,
            status,
            personal_info,
            documents: Vec::new(), // Documents are loaded separately
            submitted_at,
            reviewed_at,
            reviewer_id: row.try_get("reviewer_id")
                .map_err(|e| ServiceError::DatabaseError(format!("Failed to get reviewer_id: {}", e)))?,
            reviewer_notes: row.try_get("reviewer_notes")
                .map_err(|e| ServiceError::DatabaseError(format!("Failed to get reviewer_notes: {}", e)))?,
            rejection_reason: row.try_get("rejection_reason")
                .map_err(|e| ServiceError::DatabaseError(format!("Failed to get rejection_reason: {}", e)))?,
            updated_at,
        })
    }
}
