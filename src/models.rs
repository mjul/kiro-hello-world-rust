use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub provider: String,
    pub provider_id: String,
    pub username: String,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_login: DateTime<Utc>,
}

impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for User {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        
        let created_at_str: String = row.try_get("created_at")?;
        let last_login_str: String = row.try_get("last_login")?;
        
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map_err(|e| sqlx::Error::ColumnDecode {
                index: "created_at".to_string(),
                source: Box::new(e),
            })?
            .with_timezone(&Utc);
            
        let last_login = DateTime::parse_from_rfc3339(&last_login_str)
            .map_err(|e| sqlx::Error::ColumnDecode {
                index: "last_login".to_string(),
                source: Box::new(e),
            })?
            .with_timezone(&Utc);
        
        Ok(User {
            id: row.try_get("id")?,
            provider: row.try_get("provider")?,
            provider_id: row.try_get("provider_id")?,
            username: row.try_get("username")?,
            email: row.try_get("email")?,
            avatar_url: row.try_get("avatar_url")?,
            created_at,
            last_login,
        })
    }
}

#[derive(Debug, Clone)]
pub struct CreateUser {
    pub provider: String,
    pub provider_id: String,
    pub username: String,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub user_id: i64,
    pub username: String,
    pub provider: String,
}