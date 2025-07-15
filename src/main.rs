use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use sso_web_app::{
    AppState, AuthService, Config, Database, OAuth2Config, SessionManager, UserRepository,
    dashboard_handler, github_auth_handler, github_callback_handler,
    login_handler, logout_handler, microsoft_auth_handler, microsoft_callback_handler,
    root_handler,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sso_web_app=debug,tower_http=debug,tower_sessions=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    dotenvy::dotenv().ok();
    let config = Config::from_env()?;
    tracing::info!("Configuration loaded successfully");

    // Initialize database connection and run migrations
    let database = Database::new(&config.database_url).await?;
    tracing::info!("Database initialized and migrations completed");

    // Create user repository
    let user_repository = UserRepository::new(database.pool().clone());

    // Initialize OAuth2 clients
    let oauth2_config = OAuth2Config::new(&config)?;
    tracing::info!("OAuth2 clients configured");

    // Create authentication service
    let auth_service = AuthService::new(oauth2_config, user_repository);

    // Set up session management
    let session_manager = SessionManager::new();
    let session_layer = session_manager.layer();
    tracing::info!("Session management configured");

    // Create application state
    let app_state = AppState { auth_service };

    // Build our application with routes
    let app = Router::new()
        // Root route
        .route("/", get(root_handler))
        
        // Authentication routes (public)
        .route("/login", get(login_handler))
        .route("/auth/microsoft", get(microsoft_auth_handler))
        .route("/auth/github", get(github_auth_handler))
        .route("/auth/callback/microsoft", get(microsoft_callback_handler))
        .route("/auth/callback/github", get(github_callback_handler))
        
        // Protected routes (require authentication)
        .route("/dashboard", get(dashboard_handler))
        .route("/logout", post(logout_handler))
        
        // Add application state and middleware
        .with_state(app_state)
        .layer(session_layer)
        .layer(TraceLayer::new_for_http());

    // Run the server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("🚀 SSO Web App server starting on http://{}", addr);
    tracing::info!("📝 Visit http://{} to get started", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    
    // Graceful shutdown handling
    let app_with_graceful_shutdown = app.into_make_service();
    
    tracing::info!("✅ Server is ready to accept connections");
    axum::serve(listener, app_with_graceful_shutdown)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("🛑 Server shutdown complete");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C, shutting down gracefully...");
        },
        _ = terminate => {
            tracing::info!("Received terminate signal, shutting down gracefully...");
        },
    }
}