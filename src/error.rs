use axum::{
    http::StatusCode,
    response::{IntoResponse, Response, Redirect},
    Json,
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Authentication error: {0}")]
    Auth(#[from] AuthError),
    
    #[error("Template error: {0}")]
    Template(#[from] askama::Error),
    
    #[error("HTTP client error: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("Configuration error: {0}")]
    Config(#[from] std::env::VarError),
    
    #[error("Migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("OAuth2 state mismatch")]
    StateMismatch,
    
    #[error("Failed to exchange code for token: {0}")]
    TokenExchange(String),
    
    #[error("Failed to fetch user profile: {0}")]
    ProfileFetch(String),
    
    #[error("User not authenticated")]
    NotAuthenticated,
    
    #[error("Invalid OAuth2 provider: {0}")]
    InvalidProvider(String),
    
    #[error("Missing OAuth2 authorization code")]
    MissingAuthCode,
    
    #[error("Session expired")]
    SessionExpired,
    
    #[error("Invalid session")]
    InvalidSession,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            // Authentication errors that should redirect to login
            AppError::Auth(AuthError::NotAuthenticated) 
            | AppError::Auth(AuthError::SessionExpired) 
            | AppError::Auth(AuthError::InvalidSession) => {
                tracing::warn!("Authentication required, redirecting to login: {}", self);
                Redirect::to("/login").into_response()
            }
            
            // OAuth2 errors that should redirect to login with error message
            AppError::Auth(auth_error) => {
                tracing::error!("Authentication error: {}", auth_error);
                let error_msg = match auth_error {
                    AuthError::StateMismatch => "Security error during login. Please try again.",
                    AuthError::TokenExchange(_) => "Failed to complete login. Please try again.",
                    AuthError::ProfileFetch(_) => "Failed to retrieve your profile. Please try again.",
                    AuthError::InvalidProvider(_) => "Invalid login provider selected.",
                    AuthError::MissingAuthCode => "Login was incomplete. Please try again.",
                    _ => "Authentication failed. Please try again.",
                };
                
                let redirect_url = format!("/login?error={}", urlencoding::encode(error_msg));
                Redirect::to(&redirect_url).into_response()
            }
            
            // Server errors
            AppError::Database(ref db_error) => {
                tracing::error!("Database error: {}", db_error);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Database error",
                        "message": "A database error occurred. Please try again later."
                    }))
                ).into_response()
            }
            
            AppError::Template(ref template_error) => {
                tracing::error!("Template error: {}", template_error);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Template error",
                        "message": "A page rendering error occurred."
                    }))
                ).into_response()
            }
            
            AppError::Http(ref http_error) => {
                tracing::error!("HTTP client error: {}", http_error);
                (
                    StatusCode::BAD_GATEWAY,
                    Json(json!({
                        "error": "External service error",
                        "message": "Failed to communicate with external service. Please try again later."
                    }))
                ).into_response()
            }
            
            AppError::Config(ref config_error) => {
                tracing::error!("Configuration error: {}", config_error);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Configuration error",
                        "message": "Server configuration error."
                    }))
                ).into_response()
            }
            
            AppError::Migration(ref migration_error) => {
                tracing::error!("Migration error: {}", migration_error);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Database migration error",
                        "message": "Database initialization failed."
                    }))
                ).into_response()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[tokio::test]
    async fn test_auth_error_not_authenticated_redirects() {
        let error = AppError::Auth(AuthError::NotAuthenticated);
        let response = error.into_response();
        
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        
        let location = response.headers().get("location").unwrap();
        assert_eq!(location, "/login");
    }

    #[tokio::test]
    async fn test_auth_error_session_expired_redirects() {
        let error = AppError::Auth(AuthError::SessionExpired);
        let response = error.into_response();
        
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        
        let location = response.headers().get("location").unwrap();
        assert_eq!(location, "/login");
    }

    #[tokio::test]
    async fn test_auth_error_state_mismatch_redirects_with_error() {
        let error = AppError::Auth(AuthError::StateMismatch);
        let response = error.into_response();
        
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        
        let location = response.headers().get("location").unwrap().to_str().unwrap();
        assert!(location.starts_with("/login?error="));
        assert!(location.contains("Security%20error"));
    }

    #[tokio::test]
    async fn test_auth_error_token_exchange_redirects_with_error() {
        let error = AppError::Auth(AuthError::TokenExchange("OAuth2 server error".to_string()));
        let response = error.into_response();
        
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        
        let location = response.headers().get("location").unwrap().to_str().unwrap();
        assert!(location.starts_with("/login?error="));
        assert!(location.contains("Failed%20to%20complete"));
    }

    #[tokio::test]
    async fn test_auth_error_invalid_provider() {
        let error = AppError::Auth(AuthError::InvalidProvider("unknown".to_string()));
        let response = error.into_response();
        
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        
        let location = response.headers().get("location").unwrap().to_str().unwrap();
        assert!(location.starts_with("/login?error="));
        assert!(location.contains("Invalid%20login%20provider"));
    }

    #[tokio::test]
    async fn test_database_error_returns_500() {
        let db_error = sqlx::Error::RowNotFound;
        let error = AppError::Database(db_error);
        let response = error.into_response();
        
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_template_error_returns_500() {
        let template_error = askama::Error::Fmt(std::fmt::Error);
        let error = AppError::Template(template_error);
        let response = error.into_response();
        
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_http_error_returns_502() {
        // Create a mock HTTP error by making a request to an invalid URL
        let client = reqwest::Client::new();
        let http_error = client.get("http://invalid-url-that-does-not-exist.local")
            .send()
            .await
            .unwrap_err();
        
        let error = AppError::Http(http_error);
        let response = error.into_response();
        
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    }

    #[tokio::test]
    async fn test_config_error_returns_500() {
        let config_error = std::env::VarError::NotPresent;
        let error = AppError::Config(config_error);
        let response = error.into_response();
        
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_auth_error_display() {
        let error = AuthError::StateMismatch;
        assert_eq!(error.to_string(), "OAuth2 state mismatch");
        
        let error = AuthError::TokenExchange("server error".to_string());
        assert_eq!(error.to_string(), "Failed to exchange code for token: server error");
        
        let error = AuthError::ProfileFetch("API error".to_string());
        assert_eq!(error.to_string(), "Failed to fetch user profile: API error");
        
        let error = AuthError::InvalidProvider("unknown".to_string());
        assert_eq!(error.to_string(), "Invalid OAuth2 provider: unknown");
    }

    #[test]
    fn test_app_error_from_conversions() {
        // Test that various error types can be converted to AppError
        let db_error = sqlx::Error::RowNotFound;
        let app_error: AppError = db_error.into();
        assert!(matches!(app_error, AppError::Database(_)));
        
        let auth_error = AuthError::NotAuthenticated;
        let app_error: AppError = auth_error.into();
        assert!(matches!(app_error, AppError::Auth(_)));
        
        let config_error = std::env::VarError::NotPresent;
        let app_error: AppError = config_error.into();
        assert!(matches!(app_error, AppError::Config(_)));
    }
}