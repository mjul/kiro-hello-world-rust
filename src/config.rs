use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub microsoft_client_id: String,
    pub microsoft_client_secret: String,
    pub github_client_id: String,
    pub github_client_secret: String,
    pub session_secret: String,
    pub base_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self, env::VarError> {
        Ok(Config {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:sso_app.db".to_string()),
            microsoft_client_id: env::var("MICROSOFT_CLIENT_ID")?,
            microsoft_client_secret: env::var("MICROSOFT_CLIENT_SECRET")?,
            github_client_id: env::var("GITHUB_CLIENT_ID")?,
            github_client_secret: env::var("GITHUB_CLIENT_SECRET")?,
            session_secret: env::var("SESSION_SECRET")?,
            base_url: env::var("BASE_URL")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
        })
    }
}