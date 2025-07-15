pub mod auth;
pub mod config;
pub mod database;
pub mod error;
pub mod handlers;
pub mod models;
pub mod session;
pub mod templates;

pub use config::Config;
pub use error::{AppError, AuthError};
pub use database::{Database, UserRepository};
pub use auth::{OAuth2Config, AuthService};
pub use templates::{LoginTemplate, DashboardTemplate};
pub use session::{SessionManager, SessionExt, AuthenticatedUser, auth_middleware, optional_auth_middleware};
pub use handlers::{
    AppState, dashboard_handler, github_auth_handler, github_callback_handler,
    login_handler, logout_handler, microsoft_auth_handler, microsoft_callback_handler,
    root_handler,
};