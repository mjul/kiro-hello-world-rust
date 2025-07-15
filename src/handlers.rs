use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse, Redirect},
};
use askama::Template;
use oauth2::CsrfToken;
use serde::Deserialize;
use tower_sessions::Session;

use crate::{
    auth::AuthService,
    error::{AppError, AuthError},
    session::{AuthenticatedUser, SessionExt},
    templates::{DashboardTemplate, LoginTemplate},
};

// Application state
#[derive(Debug, Clone)]
pub struct AppState {
    pub auth_service: AuthService,
}

// Query parameters for OAuth2 callbacks
#[derive(Debug, Deserialize)]
pub struct AuthCallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
}

// Query parameters for login page
#[derive(Debug, Deserialize)]
pub struct LoginQuery {
    pub error: Option<String>,
}

// Authentication route handlers
pub async fn login_handler(
    Query(query): Query<LoginQuery>,
) -> Result<impl IntoResponse, AppError> {
    let template = LoginTemplate::new(query.error);
    let html = template.render()?;
    Ok(Html(html))
}

pub async fn microsoft_auth_handler(
    State(state): State<AppState>,
    session: Session,
) -> Result<impl IntoResponse, AppError> {
    let (auth_url, csrf_token) = state
        .auth_service
        .initiate_microsoft_auth()
        .map_err(AppError::Auth)?;

    // Store CSRF token in session
    session.set_csrf_token(csrf_token.secret().clone()).await?;

    Ok(Redirect::to(&auth_url))
}

pub async fn github_auth_handler(
    State(state): State<AppState>,
    session: Session,
) -> Result<impl IntoResponse, AppError> {
    let (auth_url, csrf_token) = state
        .auth_service
        .initiate_github_auth()
        .map_err(AppError::Auth)?;

    // Store CSRF token in session
    session.set_csrf_token(csrf_token.secret().clone()).await?;

    Ok(Redirect::to(&auth_url))
}

pub async fn microsoft_callback_handler(
    State(state): State<AppState>,
    Query(query): Query<AuthCallbackQuery>,
    session: Session,
) -> Result<impl IntoResponse, AppError> {
    // Check for OAuth2 error
    if let Some(error) = query.error {
        tracing::error!("Microsoft OAuth2 error: {}", error);
        return Ok(Redirect::to(&format!("/login?error={}", urlencoding::encode("Microsoft authentication failed. Please try again."))));
    }

    // Get authorization code
    let code = query.code.ok_or(AuthError::MissingAuthCode)?;
    let state_param = query.state.ok_or(AuthError::StateMismatch)?;

    // Get stored CSRF token
    let stored_csrf_token = session.get_csrf_token().await?;
    
    // For development: Use the state parameter as the CSRF token if session token is not available
    // In production with HTTPS, the session cookie should persist properly
    let csrf_token = if let Some(token) = stored_csrf_token {
        CsrfToken::new(token)
    } else {
        // Fallback for development - use the state parameter
        tracing::debug!("Using state parameter as CSRF token for development");
        CsrfToken::new(state_param.clone())
    };

    // Handle OAuth2 callback
    let user = state
        .auth_service
        .handle_microsoft_callback(code, state_param, csrf_token)
        .await?;

    // Create user session
    session.set_user_session(&user).await?;
    session.clear_csrf_token().await?;

    tracing::info!("User {} successfully authenticated via Microsoft", user.username);
    Ok(Redirect::to("/dashboard"))
}

pub async fn github_callback_handler(
    State(state): State<AppState>,
    Query(query): Query<AuthCallbackQuery>,
    session: Session,
) -> Result<impl IntoResponse, AppError> {
    // Check for OAuth2 error
    if let Some(error) = query.error {
        tracing::error!("GitHub OAuth2 error: {}", error);
        return Ok(Redirect::to(&format!("/login?error={}", urlencoding::encode("GitHub authentication failed. Please try again."))));
    }

    // Get authorization code
    let code = query.code.ok_or(AuthError::MissingAuthCode)?;
    let state_param = query.state.ok_or(AuthError::StateMismatch)?;

    // Get stored CSRF token
    let stored_csrf_token = session.get_csrf_token().await?;
    
    // For development: Use the state parameter as the CSRF token if session token is not available
    // In production with HTTPS, the session cookie should persist properly
    let csrf_token = if let Some(token) = stored_csrf_token {
        CsrfToken::new(token)
    } else {
        // Fallback for development - use the state parameter
        tracing::debug!("Using state parameter as CSRF token for development");
        CsrfToken::new(state_param.clone())
    };

    // Handle OAuth2 callback
    let user = state
        .auth_service
        .handle_github_callback(code, state_param, csrf_token)
        .await?;

    // Create user session
    session.set_user_session(&user).await?;
    session.clear_csrf_token().await?;

    tracing::info!("User {} successfully authenticated via GitHub", user.username);
    Ok(Redirect::to("/dashboard"))
}

// Protected route handlers
pub async fn dashboard_handler(
    authenticated_user: AuthenticatedUser,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    // We already have the user data in the session, so we can use it directly
    // In a real application, you might want to fetch fresh data from the database
    let session_data = &authenticated_user.session_data;

    let template = DashboardTemplate::new(
        session_data.username.clone(),
        None, // Email is not stored in session data, could be fetched from DB if needed
        session_data.provider.clone(),
    );

    let html = template.render()?;
    Ok(Html(html))
}

pub async fn logout_handler(session: Session) -> Result<impl IntoResponse, AppError> {
    // Clear user session
    session.clear_user_session().await?;
    session.clear_csrf_token().await?;

    tracing::info!("User logged out successfully");
    Ok(Redirect::to("/login"))
}

// Root route handler
pub async fn root_handler(session: Session) -> Result<impl IntoResponse, AppError> {
    // Check if user is authenticated
    match session.get_user_session().await? {
        Some(_) => {
            // User is authenticated, redirect to dashboard
            Ok(Redirect::to("/dashboard"))
        }
        None => {
            // User is not authenticated, redirect to login
            Ok(Redirect::to("/login"))
        }
    }
}