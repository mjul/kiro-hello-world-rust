use axum::{
    http::StatusCode,
    Router,
};
use axum_test::TestServer;
use tempfile::NamedTempFile;

use sso_web_app::{
    AppState, AuthService, Config, Database, OAuth2Config, SessionManager, UserRepository,
    dashboard_handler, github_auth_handler, github_callback_handler,
    login_handler, logout_handler, microsoft_auth_handler, microsoft_callback_handler,
    root_handler,
};

async fn setup_test_app() -> TestServer {
    // Create test database
    let temp_file = NamedTempFile::new().unwrap();
    let database_url = format!("sqlite:{}", temp_file.path().to_str().unwrap());
    
    // Create test configuration
    let config = Config {
        database_url: database_url.clone(),
        microsoft_client_id: "test_ms_client_id".to_string(),
        microsoft_client_secret: "test_ms_client_secret".to_string(),
        github_client_id: "test_gh_client_id".to_string(),
        github_client_secret: "test_gh_client_secret".to_string(),
        session_secret: "test_session_secret_key_for_testing_purposes".to_string(),
        base_url: "http://localhost:3000".to_string(),
    };

    // Initialize database
    let database = Database::new(&database_url).await.unwrap();
    let user_repository = UserRepository::new(database.pool().clone());

    // Initialize OAuth2 clients
    let oauth2_config = OAuth2Config::new(&config).unwrap();
    let auth_service = AuthService::new(oauth2_config, user_repository);

    // Set up session management
    let session_manager = SessionManager::new();
    let session_layer = session_manager.layer();

    // Create application state
    let app_state = AppState { auth_service };

    // Build test application
    let app = Router::new()
        .route("/", axum::routing::get(root_handler))
        .route("/login", axum::routing::get(login_handler))
        .route("/auth/microsoft", axum::routing::get(microsoft_auth_handler))
        .route("/auth/github", axum::routing::get(github_auth_handler))
        .route("/auth/callback/microsoft", axum::routing::get(microsoft_callback_handler))
        .route("/auth/callback/github", axum::routing::get(github_callback_handler))
        .route("/dashboard", axum::routing::get(dashboard_handler))
        .route("/logout", axum::routing::post(logout_handler))
        .with_state(app_state)
        .layer(session_layer);

    TestServer::new(app).unwrap()
}

#[tokio::test]
async fn test_root_redirects_to_login_when_not_authenticated() {
    let server = setup_test_app().await;

    let response = server.get("/").await;
    
    assert_eq!(response.status_code(), StatusCode::SEE_OTHER);
    let location = response.headers().get("location").unwrap().to_str().unwrap();
    assert_eq!(location, "/login");
}

#[tokio::test]
async fn test_login_page_renders_successfully() {
    let server = setup_test_app().await;

    let response = server.get("/login").await;
    
    assert_eq!(response.status_code(), StatusCode::OK);
    let body = response.text();
    assert!(body.contains("Welcome"));
    assert!(body.contains("Sign in with Microsoft 365"));
    assert!(body.contains("Sign in with GitHub"));
}

#[tokio::test]
async fn test_login_page_displays_error_message() {
    let server = setup_test_app().await;

    let response = server.get("/login?error=Test%20error%20message").await;
    
    assert_eq!(response.status_code(), StatusCode::OK);
    let body = response.text();
    assert!(body.contains("Test error message"));
}

#[tokio::test]
async fn test_microsoft_auth_initiation() {
    let server = setup_test_app().await;

    let response = server.get("/auth/microsoft").await;
    
    assert_eq!(response.status_code(), StatusCode::SEE_OTHER);
    
    let location = response.headers().get("location").unwrap().to_str().unwrap();
    assert!(location.contains("login.microsoftonline.com"));
    assert!(location.contains("client_id=test_ms_client_id"));
    assert!(location.contains("scope=openid"));
}

#[tokio::test]
async fn test_github_auth_initiation() {
    let server = setup_test_app().await;

    let response = server.get("/auth/github").await;
    
    assert_eq!(response.status_code(), StatusCode::SEE_OTHER);
    
    let location = response.headers().get("location").unwrap().to_str().unwrap();
    assert!(location.contains("github.com/login/oauth/authorize"));
    assert!(location.contains("client_id=test_gh_client_id"));
    assert!(location.contains("scope=user%3Aemail"));
}

#[tokio::test]
async fn test_microsoft_callback_with_missing_code() {
    let server = setup_test_app().await;

    let response = server.get("/auth/callback/microsoft?state=test_state").await;
    
    assert_eq!(response.status_code(), StatusCode::SEE_OTHER);
    let location = response.headers().get("location").unwrap().to_str().unwrap();
    assert!(location.contains("/login?error="));
}

#[tokio::test]
async fn test_github_callback_with_oauth_error() {
    let server = setup_test_app().await;

    let response = server.get("/auth/callback/github?error=access_denied").await;
    
    assert_eq!(response.status_code(), StatusCode::SEE_OTHER);
    let location = response.headers().get("location").unwrap().to_str().unwrap();
    assert!(location.contains("/login?error="));
    assert!(location.contains("GitHub%20authentication%20failed"));
}

#[tokio::test]
async fn test_dashboard_requires_authentication() {
    let server = setup_test_app().await;

    let response = server.get("/dashboard").await;
    
    assert_eq!(response.status_code(), StatusCode::SEE_OTHER);
    let location = response.headers().get("location").unwrap().to_str().unwrap();
    assert_eq!(location, "/login");
}

#[tokio::test]
async fn test_logout_clears_session() {
    let server = setup_test_app().await;

    let response = server.post("/logout").await;
    
    assert_eq!(response.status_code(), StatusCode::SEE_OTHER);
    let location = response.headers().get("location").unwrap().to_str().unwrap();
    assert_eq!(location, "/login");
}

#[tokio::test]
async fn test_oauth_callback_csrf_protection() {
    let server = setup_test_app().await;

    // Test Microsoft callback without proper CSRF state
    let ms_response = server
        .get("/auth/callback/microsoft?code=test_code&state=invalid_state")
        .await;
    
    assert_eq!(ms_response.status_code(), StatusCode::SEE_OTHER);
    let location = ms_response.headers().get("location").unwrap().to_str().unwrap();
    assert!(location.contains("/login?error="));

    // Test GitHub callback without proper CSRF state
    let gh_response = server
        .get("/auth/callback/github?code=test_code&state=invalid_state")
        .await;
    
    assert_eq!(gh_response.status_code(), StatusCode::SEE_OTHER);
    let location = gh_response.headers().get("location").unwrap().to_str().unwrap();
    assert!(location.contains("/login?error="));
}

#[tokio::test]
async fn test_oauth_error_handling() {
    let server = setup_test_app().await;

    // Test Microsoft OAuth error
    let ms_error_response = server
        .get("/auth/callback/microsoft?error=access_denied&error_description=User%20denied%20access")
        .await;
    
    assert_eq!(ms_error_response.status_code(), StatusCode::SEE_OTHER);
    let location = ms_error_response.headers().get("location").unwrap().to_str().unwrap();
    assert!(location.contains("/login?error="));
    assert!(location.contains("Microsoft%20authentication%20failed"));

    // Test GitHub OAuth error
    let gh_error_response = server
        .get("/auth/callback/github?error=access_denied")
        .await;
    
    assert_eq!(gh_error_response.status_code(), StatusCode::SEE_OTHER);
    let location = gh_error_response.headers().get("location").unwrap().to_str().unwrap();
    assert!(location.contains("/login?error="));
    assert!(location.contains("GitHub%20authentication%20failed"));
}

#[tokio::test]
async fn test_invalid_routes_return_404() {
    let server = setup_test_app().await;

    let response = server.get("/nonexistent-route").await;
    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_method_not_allowed() {
    let server = setup_test_app().await;

    // Test POST to GET-only route
    let response = server.post("/login").await;
    assert_eq!(response.status_code(), StatusCode::METHOD_NOT_ALLOWED);

    // Test GET to POST-only route
    let response = server.get("/logout").await;
    assert_eq!(response.status_code(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn test_session_cookie_security() {
    let server = setup_test_app().await;

    let response = server.get("/login").await;
    
    if let Some(cookie_header) = response.headers().get("set-cookie") {
        let cookie_str = cookie_header.to_str().unwrap();
        // Check for security attributes
        assert!(cookie_str.contains("HttpOnly"));
        assert!(cookie_str.contains("SameSite=Lax"));
        // Note: Secure flag would be set in production with HTTPS
    }
}

#[tokio::test]
async fn test_html_content_type() {
    let server = setup_test_app().await;

    let response = server.get("/login").await;
    
    assert_eq!(response.status_code(), StatusCode::OK);
    
    // Check content type is HTML
    if let Some(content_type) = response.headers().get("content-type") {
        let content_type_str = content_type.to_str().unwrap();
        assert!(content_type_str.contains("text/html"));
    }
}

#[tokio::test]
async fn test_template_rendering_with_special_characters() {
    let server = setup_test_app().await;

    // Test with URL-encoded special characters in error message
    let response = server
        .get("/login?error=Special%20characters%3A%20%3C%3E%26%22%27")
        .await;
    
    assert_eq!(response.status_code(), StatusCode::OK);
    let body = response.text();
    
    // Should properly decode and escape HTML
    assert!(body.contains("Special characters"));
    // Should not contain raw HTML entities that could cause XSS
    assert!(!body.contains("<script"));
}