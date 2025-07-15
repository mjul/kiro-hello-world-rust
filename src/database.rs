use sqlx::{sqlite::SqlitePool, migrate::MigrateDatabase, Sqlite};
use crate::{models::{User, CreateUser}, error::AppError};

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self, AppError> {
        // Create database if it doesn't exist
        if !Sqlite::database_exists(database_url).await.unwrap_or(false) {
            tracing::info!("Creating database {}", database_url);
            Sqlite::create_database(database_url).await?;
        }

        // Connect to database
        let pool = SqlitePool::connect(database_url).await?;
        
        // Run migrations
        tracing::info!("Running database migrations");
        sqlx::migrate!("./migrations").run(&pool).await?;
        
        Ok(Database { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

#[derive(Debug, Clone)]
pub struct UserRepository {
    pool: SqlitePool,
}

impl UserRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn find_by_provider_id(
        &self,
        provider: &str,
        provider_id: &str,
    ) -> Result<Option<User>, AppError> {
        let user = sqlx::query_as::<_, User>(
            "SELECT id, provider, provider_id, username, email, avatar_url, created_at, last_login 
             FROM users 
             WHERE provider = ? AND provider_id = ?"
        )
        .bind(provider)
        .bind(provider_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    pub async fn create_user(&self, user: CreateUser) -> Result<User, AppError> {
        let now = chrono::Utc::now();
        
        let result = sqlx::query(
            "INSERT INTO users (provider, provider_id, username, email, avatar_url, created_at, last_login)
             VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&user.provider)
        .bind(&user.provider_id)
        .bind(&user.username)
        .bind(&user.email)
        .bind(&user.avatar_url)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        let user_id = result.last_insert_rowid();
        
        // Fetch the created user
        let created_user = sqlx::query_as::<_, User>(
            "SELECT id, provider, provider_id, username, email, avatar_url, created_at, last_login 
             FROM users 
             WHERE id = ?"
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(created_user)
    }

    pub async fn update_last_login(&self, user_id: i64) -> Result<(), AppError> {
        let now = chrono::Utc::now();
        
        sqlx::query(
            "UPDATE users SET last_login = ? WHERE id = ?"
        )
        .bind(now.to_rfc3339())
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    async fn setup_test_db() -> Database {
        let temp_file = NamedTempFile::new().unwrap();
        let database_url = format!("sqlite:{}", temp_file.path().to_str().unwrap());
        Database::new(&database_url).await.unwrap()
    }

    #[tokio::test]
    async fn test_create_and_find_user() {
        let db = setup_test_db().await;
        let repo = UserRepository::new(db.pool().clone());

        let create_user = CreateUser {
            provider: "github".to_string(),
            provider_id: "12345".to_string(),
            username: "testuser".to_string(),
            email: Some("test@example.com".to_string()),
            avatar_url: Some("https://example.com/avatar.jpg".to_string()),
        };

        // Test user creation
        let created_user = repo.create_user(create_user).await.unwrap();
        assert_eq!(created_user.provider, "github");
        assert_eq!(created_user.provider_id, "12345");
        assert_eq!(created_user.username, "testuser");
        assert_eq!(created_user.email, Some("test@example.com".to_string()));
        assert!(created_user.id > 0);

        // Test finding user by provider ID
        let found_user = repo
            .find_by_provider_id("github", "12345")
            .await
            .unwrap()
            .unwrap();
        
        assert_eq!(found_user.id, created_user.id);
        assert_eq!(found_user.username, "testuser");
        assert_eq!(found_user.email, Some("test@example.com".to_string()));
    }

    #[tokio::test]
    async fn test_find_nonexistent_user() {
        let db = setup_test_db().await;
        let repo = UserRepository::new(db.pool().clone());

        let result = repo
            .find_by_provider_id("github", "nonexistent")
            .await
            .unwrap();
        
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_update_last_login() {
        let db = setup_test_db().await;
        let repo = UserRepository::new(db.pool().clone());

        let create_user = CreateUser {
            provider: "microsoft".to_string(),
            provider_id: "67890".to_string(),
            username: "msuser".to_string(),
            email: None,
            avatar_url: None,
        };

        let created_user = repo.create_user(create_user).await.unwrap();
        let original_login = created_user.last_login;

        // Wait a bit to ensure timestamp difference
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Update last login
        repo.update_last_login(created_user.id).await.unwrap();

        // Verify the update
        let updated_user = repo
            .find_by_provider_id("microsoft", "67890")
            .await
            .unwrap()
            .unwrap();
        
        assert!(updated_user.last_login > original_login);
    }

    #[tokio::test]
    async fn test_unique_constraint() {
        let db = setup_test_db().await;
        let repo = UserRepository::new(db.pool().clone());

        let create_user1 = CreateUser {
            provider: "github".to_string(),
            provider_id: "duplicate".to_string(),
            username: "user1".to_string(),
            email: None,
            avatar_url: None,
        };

        let create_user2 = CreateUser {
            provider: "github".to_string(),
            provider_id: "duplicate".to_string(),
            username: "user2".to_string(),
            email: None,
            avatar_url: None,
        };

        // First user should succeed
        repo.create_user(create_user1).await.unwrap();

        // Second user with same provider + provider_id should fail
        let result = repo.create_user(create_user2).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_different_providers_same_id() {
        let db = setup_test_db().await;
        let repo = UserRepository::new(db.pool().clone());

        let github_user = CreateUser {
            provider: "github".to_string(),
            provider_id: "sameid".to_string(),
            username: "githubuser".to_string(),
            email: None,
            avatar_url: None,
        };

        let microsoft_user = CreateUser {
            provider: "microsoft".to_string(),
            provider_id: "sameid".to_string(),
            username: "msuser".to_string(),
            email: None,
            avatar_url: None,
        };

        // Both should succeed since they have different providers
        let github_created = repo.create_user(github_user).await.unwrap();
        let microsoft_created = repo.create_user(microsoft_user).await.unwrap();

        assert_ne!(github_created.id, microsoft_created.id);
        assert_eq!(github_created.provider, "github");
        assert_eq!(microsoft_created.provider, "microsoft");
    }
}