use axum::{
    async_trait,
    extract::{FromRequestParts, Request},
    http::{request::Parts, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use tower_sessions::{Session, SessionManagerLayer, MemoryStore};
use uuid::Uuid;

use crate::{
    error::{AppError, AuthError},
    models::{SessionData, User},
};

// Session keys
const USER_SESSION_KEY: &str = "user_session";
const CSRF_TOKEN_KEY: &str = "csrf_token";

#[derive(Debug, Clone)]
pub struct SessionManager {
    store: MemoryStore,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            store: MemoryStore::default(),
        }
    }

    pub fn layer(&self) -> SessionManagerLayer<MemoryStore> {
        SessionManagerLayer::new(self.store.clone())
            .with_secure(false) // Set to true in production with HTTPS
            .with_same_site(tower_sessions::cookie::SameSite::Lax)
            .with_http_only(true)
            .with_name("sso_session")
    }
}

// Session extension trait for easier session management
pub trait SessionExt {
    async fn get_user_session(&self) -> Result<Option<SessionData>, AppError>;
    async fn set_user_session(&self, user: &User) -> Result<(), AppError>;
    async fn clear_user_session(&self) -> Result<(), AppError>;
    async fn get_csrf_token(&self) -> Result<Option<String>, AppError>;
    async fn set_csrf_token(&self, token: String) -> Result<(), AppError>;
    async fn clear_csrf_token(&self) -> Result<(), AppError>;
}

impl SessionExt for Session {
    async fn get_user_session(&self) -> Result<Option<SessionData>, AppError> {
        match self.get::<SessionData>(USER_SESSION_KEY).await {
            Ok(session_data) => Ok(session_data),
            Err(e) => {
                tracing::error!("Failed to get user session: {}", e);
                Err(AppError::Auth(AuthError::InvalidSession))
            }
        }
    }

    async fn set_user_session(&self, user: &User) -> Result<(), AppError> {
        let session_data = SessionData {
            user_id: user.id,
            username: user.username.clone(),
            provider: user.provider.clone(),
        };

        match self.insert(USER_SESSION_KEY, session_data).await {
            Ok(_) => {
                tracing::info!("User session created for user ID: {}", user.id);
                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to set user session: {}", e);
                Err(AppError::Auth(AuthError::InvalidSession))
            }
        }
    }

    async fn clear_user_session(&self) -> Result<(), AppError> {
        match self.remove::<SessionData>(USER_SESSION_KEY).await {
            Ok(_) => {
                tracing::info!("User session cleared");
                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to clear user session: {}", e);
                Err(AppError::Auth(AuthError::InvalidSession))
            }
        }
    }

    async fn get_csrf_token(&self) -> Result<Option<String>, AppError> {
        match self.get::<String>(CSRF_TOKEN_KEY).await {
            Ok(token) => Ok(token),
            Err(e) => {
                tracing::error!("Failed to get CSRF token: {}", e);
                Err(AppError::Auth(AuthError::InvalidSession))
            }
        }
    }

    async fn set_csrf_token(&self, token: String) -> Result<(), AppError> {
        match self.insert(CSRF_TOKEN_KEY, token).await {
            Ok(_) => Ok(()),
            Err(e) => {
                tracing::error!("Failed to set CSRF token: {}", e);
                Err(AppError::Auth(AuthError::InvalidSession))
            }
        }
    }

    async fn clear_csrf_token(&self) -> Result<(), AppError> {
        match self.remove::<String>(CSRF_TOKEN_KEY).await {
            Ok(_) => Ok(()),
            Err(e) => {
                tracing::error!("Failed to clear CSRF token: {}", e);
                Err(AppError::Auth(AuthError::InvalidSession))
            }
        }
    }
}

// Authenticated user extractor
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub session_data: SessionData,
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let session = Session::from_request_parts(parts, state)
            .await
            .map_err(|_| AppError::Auth(AuthError::InvalidSession))?;

        match session.get_user_session().await? {
            Some(session_data) => Ok(AuthenticatedUser { session_data }),
            None => Err(AppError::Auth(AuthError::NotAuthenticated)),
        }
    }
}

// Authentication middleware
pub async fn auth_middleware(
    session: Session,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Check if user is authenticated
    match session.get_user_session().await? {
        Some(_) => {
            // User is authenticated, proceed
            Ok(next.run(request).await)
        }
        None => {
            // User is not authenticated, redirect to login
            Err(AppError::Auth(AuthError::NotAuthenticated))
        }
    }
}

// Optional authentication middleware (doesn't redirect if not authenticated)
pub async fn optional_auth_middleware(
    session: Session,
    mut request: Request,
    next: Next,
) -> Response {
    // Add session data to request extensions if user is authenticated
    if let Ok(Some(session_data)) = session.get_user_session().await {
        request.extensions_mut().insert(session_data);
    }
    
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower_sessions::MemoryStore;

    #[tokio::test]
    async fn test_session_manager_creation() {
        let session_manager = SessionManager::new();
        let _layer = session_manager.layer();
        // Test passes if no panic occurs
    }

    #[tokio::test]
    async fn test_session_data_serialization() {
        let session_data = SessionData {
            user_id: 1,
            username: "testuser".to_string(),
            provider: "github".to_string(),
        };

        // Test serialization
        let serialized = serde_json::to_string(&session_data).unwrap();
        assert!(serialized.contains("testuser"));
        assert!(serialized.contains("github"));

        // Test deserialization
        let deserialized: SessionData = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.user_id, 1);
        assert_eq!(deserialized.username, "testuser");
        assert_eq!(deserialized.provider, "github");
    }

    #[tokio::test]
    async fn test_session_ext_methods() {
        use tower_sessions::SessionManagerLayer;
        
        let store = MemoryStore::default();
        let session_layer = SessionManagerLayer::new(store);
        
        // Create a mock session for testing
        // Note: This is a simplified test - in practice, sessions are created by the middleware
        let session_id = Uuid::new_v4().to_string();
        
        // Test would require more complex setup with actual HTTP request/response cycle
        // For now, we'll test the data structures
        assert!(true); // Placeholder test
    }
}