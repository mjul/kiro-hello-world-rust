use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    PkceCodeChallenge, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use reqwest::Client as HttpClient;
use serde::Deserialize;

use crate::{
    config::Config,
    database::UserRepository,
    error::{AppError, AuthError},
    models::{CreateUser, User},
};

#[derive(Debug, Clone)]
pub struct OAuth2Config {
    pub microsoft_client: BasicClient,
    pub github_client: BasicClient,
    pub http_client: HttpClient,
}

impl OAuth2Config {
    pub fn new(config: &Config) -> Result<Self, AppError> {
        // Microsoft OAuth2 client
        let microsoft_client = BasicClient::new(
            ClientId::new(config.microsoft_client_id.clone()),
            Some(ClientSecret::new(config.microsoft_client_secret.clone())),
            AuthUrl::new("https://login.microsoftonline.com/common/oauth2/v2.0/authorize".to_string())
                .map_err(|e| AppError::Config(std::env::VarError::NotPresent))?,
            Some(
                TokenUrl::new("https://login.microsoftonline.com/common/oauth2/v2.0/token".to_string())
                    .map_err(|e| AppError::Config(std::env::VarError::NotPresent))?,
            ),
        )
        .set_redirect_uri(
            RedirectUrl::new(format!("{}/auth/callback/microsoft", config.base_url))
                .map_err(|e| AppError::Config(std::env::VarError::NotPresent))?,
        );

        // GitHub OAuth2 client
        let github_client = BasicClient::new(
            ClientId::new(config.github_client_id.clone()),
            Some(ClientSecret::new(config.github_client_secret.clone())),
            AuthUrl::new("https://github.com/login/oauth/authorize".to_string())
                .map_err(|e| AppError::Config(std::env::VarError::NotPresent))?,
            Some(
                TokenUrl::new("https://github.com/login/oauth/access_token".to_string())
                    .map_err(|e| AppError::Config(std::env::VarError::NotPresent))?,
            ),
        )
        .set_redirect_uri(
            RedirectUrl::new(format!("{}/auth/callback/github", config.base_url))
                .map_err(|e| AppError::Config(std::env::VarError::NotPresent))?,
        );

        let http_client = HttpClient::new();

        Ok(OAuth2Config {
            microsoft_client,
            github_client,
            http_client,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AuthService {
    oauth2_config: OAuth2Config,
    user_repository: UserRepository,
}

impl AuthService {
    pub fn new(oauth2_config: OAuth2Config, user_repository: UserRepository) -> Self {
        Self {
            oauth2_config,
            user_repository,
        }
    }

    pub fn initiate_microsoft_auth(&self) -> Result<(String, CsrfToken), AuthError> {
        let (pkce_challenge, _pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let (auth_url, csrf_token) = self
            .oauth2_config
            .microsoft_client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("openid".to_string()))
            .add_scope(Scope::new("profile".to_string()))
            .add_scope(Scope::new("email".to_string()))
            .set_pkce_challenge(pkce_challenge)
            .url();

        Ok((auth_url.to_string(), csrf_token))
    }

    pub fn initiate_github_auth(&self) -> Result<(String, CsrfToken), AuthError> {
        let (auth_url, csrf_token) = self
            .oauth2_config
            .github_client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("user:email".to_string()))
            .url();

        Ok((auth_url.to_string(), csrf_token))
    }
}

// Microsoft Graph API user profile response
#[derive(Debug, Deserialize)]
pub struct MicrosoftUserProfile {
    pub id: String,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    #[serde(rename = "userPrincipalName")]
    pub user_principal_name: Option<String>,
    pub mail: Option<String>,
}

// GitHub API user profile response
#[derive(Debug, Deserialize)]
pub struct GitHubUserProfile {
    pub id: u64,
    pub login: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
}

// GitHub API email response
#[derive(Debug, Deserialize)]
pub struct GitHubEmail {
    pub email: String,
    pub primary: bool,
    pub verified: bool,
}

impl AuthService {
    pub async fn handle_microsoft_callback(
        &self,
        code: String,
        state: String,
        expected_csrf_token: CsrfToken,
    ) -> Result<User, AuthError> {
        // Verify CSRF token
        if state != *expected_csrf_token.secret() {
            return Err(AuthError::StateMismatch);
        }

        // Exchange authorization code for access token
        let token_result = self
            .oauth2_config
            .microsoft_client
            .exchange_code(AuthorizationCode::new(code))
            .request_async(oauth2::reqwest::async_http_client)
            .await
            .map_err(|e| AuthError::TokenExchange(e.to_string()))?;

        let access_token = token_result.access_token().secret();

        // Fetch user profile from Microsoft Graph API
        let profile_response = self
            .oauth2_config
            .http_client
            .get("https://graph.microsoft.com/v1.0/me")
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| AuthError::ProfileFetch(e.to_string()))?;

        if !profile_response.status().is_success() {
            return Err(AuthError::ProfileFetch(format!(
                "HTTP {}",
                profile_response.status()
            )));
        }

        let profile: MicrosoftUserProfile = profile_response
            .json()
            .await
            .map_err(|e| AuthError::ProfileFetch(e.to_string()))?;

        // Check if user exists or create new user
        let user = match self
            .user_repository
            .find_by_provider_id("microsoft", &profile.id)
            .await
        {
            Ok(Some(existing_user)) => {
                // Update last login
                self.user_repository
                    .update_last_login(existing_user.id)
                    .await
                    .map_err(|e| AuthError::ProfileFetch(e.to_string()))?;
                existing_user
            }
            Ok(None) => {
                // Create new user
                let create_user = CreateUser {
                    provider: "microsoft".to_string(),
                    provider_id: profile.id,
                    username: profile
                        .display_name
                        .or(profile.user_principal_name)
                        .unwrap_or_else(|| "Microsoft User".to_string()),
                    email: profile.mail,
                    avatar_url: None,
                };

                self.user_repository
                    .create_user(create_user)
                    .await
                    .map_err(|e| AuthError::ProfileFetch(e.to_string()))?
            }
            Err(e) => return Err(AuthError::ProfileFetch(e.to_string())),
        };

        Ok(user)
    }

    pub async fn handle_github_callback(
        &self,
        code: String,
        state: String,
        expected_csrf_token: CsrfToken,
    ) -> Result<User, AuthError> {
        // Verify CSRF token
        if state != *expected_csrf_token.secret() {
            return Err(AuthError::StateMismatch);
        }

        // Exchange authorization code for access token
        let token_result = self
            .oauth2_config
            .github_client
            .exchange_code(AuthorizationCode::new(code))
            .request_async(oauth2::reqwest::async_http_client)
            .await
            .map_err(|e| AuthError::TokenExchange(e.to_string()))?;

        let access_token = token_result.access_token().secret();

        // Fetch user profile from GitHub API
        let profile_response = self
            .oauth2_config
            .http_client
            .get("https://api.github.com/user")
            .bearer_auth(access_token)
            .header("User-Agent", "sso-web-app")
            .send()
            .await
            .map_err(|e| AuthError::ProfileFetch(e.to_string()))?;

        if !profile_response.status().is_success() {
            return Err(AuthError::ProfileFetch(format!(
                "HTTP {}",
                profile_response.status()
            )));
        }

        let mut profile: GitHubUserProfile = profile_response
            .json()
            .await
            .map_err(|e| AuthError::ProfileFetch(e.to_string()))?;

        // If email is not public, fetch it from the emails endpoint
        if profile.email.is_none() {
            let emails_response = self
                .oauth2_config
                .http_client
                .get("https://api.github.com/user/emails")
                .bearer_auth(access_token)
                .header("User-Agent", "sso-web-app")
                .send()
                .await
                .map_err(|e| AuthError::ProfileFetch(e.to_string()))?;

            if emails_response.status().is_success() {
                let emails: Vec<GitHubEmail> = emails_response
                    .json()
                    .await
                    .map_err(|e| AuthError::ProfileFetch(e.to_string()))?;

                // Find primary verified email
                profile.email = emails
                    .into_iter()
                    .find(|email| email.primary && email.verified)
                    .map(|email| email.email);
            }
        }

        // Check if user exists or create new user
        let user = match self
            .user_repository
            .find_by_provider_id("github", &profile.id.to_string())
            .await
        {
            Ok(Some(existing_user)) => {
                // Update last login
                self.user_repository
                    .update_last_login(existing_user.id)
                    .await
                    .map_err(|e| AuthError::ProfileFetch(e.to_string()))?;
                existing_user
            }
            Ok(None) => {
                // Create new user
                let create_user = CreateUser {
                    provider: "github".to_string(),
                    provider_id: profile.id.to_string(),
                    username: profile.name.unwrap_or(profile.login),
                    email: profile.email,
                    avatar_url: profile.avatar_url,
                };

                self.user_repository
                    .create_user(create_user)
                    .await
                    .map_err(|e| AuthError::ProfileFetch(e.to_string()))?
            }
            Err(e) => return Err(AuthError::ProfileFetch(e.to_string())),
        };

        Ok(user)
    }
}#[cfg(test)
]
mod tests {
    use super::*;
    use crate::database::Database;
    use tempfile::NamedTempFile;
    use wiremock::{matchers::{method, path, header}, Mock, MockServer, ResponseTemplate};

    async fn setup_test_auth_service() -> (AuthService, MockServer) {
        // Set up test database
        let temp_file = NamedTempFile::new().unwrap();
        let database_url = format!("sqlite:{}", temp_file.path().to_str().unwrap());
        let db = Database::new(&database_url).await.unwrap();
        let user_repo = UserRepository::new(db.pool().clone());

        // Set up mock server
        let mock_server = MockServer::start().await;

        // Create test config with mock server URLs
        let config = Config {
            database_url,
            microsoft_client_id: "test_ms_client_id".to_string(),
            microsoft_client_secret: "test_ms_client_secret".to_string(),
            github_client_id: "test_gh_client_id".to_string(),
            github_client_secret: "test_gh_client_secret".to_string(),
            session_secret: "test_session_secret".to_string(),
            base_url: "http://localhost:3000".to_string(),
        };

        let oauth2_config = OAuth2Config::new(&config).unwrap();
        let auth_service = AuthService::new(oauth2_config, user_repo);

        (auth_service, mock_server)
    }

    #[tokio::test]
    async fn test_initiate_microsoft_auth() {
        let (auth_service, _mock_server) = setup_test_auth_service().await;

        let result = auth_service.initiate_microsoft_auth();
        assert!(result.is_ok());

        let (auth_url, csrf_token) = result.unwrap();
        assert!(auth_url.contains("login.microsoftonline.com"));
        assert!(auth_url.contains("client_id=test_ms_client_id"));
        assert!(auth_url.contains("scope=openid") && auth_url.contains("profile") && auth_url.contains("email"));
        assert!(!csrf_token.secret().is_empty());
    }

    #[tokio::test]
    async fn test_initiate_github_auth() {
        let (auth_service, _mock_server) = setup_test_auth_service().await;

        let result = auth_service.initiate_github_auth();
        assert!(result.is_ok());

        let (auth_url, csrf_token) = result.unwrap();
        assert!(auth_url.contains("github.com/login/oauth/authorize"));
        assert!(auth_url.contains("client_id=test_gh_client_id"));
        assert!(auth_url.contains("scope=user%3Aemail"));
        assert!(!csrf_token.secret().is_empty());
    }

    #[tokio::test]
    async fn test_microsoft_callback_csrf_mismatch() {
        let (auth_service, _mock_server) = setup_test_auth_service().await;

        let csrf_token = CsrfToken::new("expected_token".to_string());
        let result = auth_service
            .handle_microsoft_callback(
                "test_code".to_string(),
                "wrong_token".to_string(),
                csrf_token,
            )
            .await;

        assert!(matches!(result, Err(AuthError::StateMismatch)));
    }

    #[tokio::test]
    async fn test_github_callback_csrf_mismatch() {
        let (auth_service, _mock_server) = setup_test_auth_service().await;

        let csrf_token = CsrfToken::new("expected_token".to_string());
        let result = auth_service
            .handle_github_callback(
                "test_code".to_string(),
                "wrong_token".to_string(),
                csrf_token,
            )
            .await;

        assert!(matches!(result, Err(AuthError::StateMismatch)));
    }

    #[test]
    fn test_oauth2_config_creation() {
        let config = Config {
            database_url: "sqlite::memory:".to_string(),
            microsoft_client_id: "ms_client_id".to_string(),
            microsoft_client_secret: "ms_client_secret".to_string(),
            github_client_id: "gh_client_id".to_string(),
            github_client_secret: "gh_client_secret".to_string(),
            session_secret: "session_secret".to_string(),
            base_url: "http://localhost:3000".to_string(),
        };

        let oauth2_config = OAuth2Config::new(&config);
        assert!(oauth2_config.is_ok());
    }

    #[test]
    fn test_microsoft_user_profile_deserialization() {
        let json = r#"{
            "id": "12345",
            "displayName": "Test User",
            "userPrincipalName": "test@example.com",
            "mail": "test@example.com"
        }"#;

        let profile: Result<MicrosoftUserProfile, _> = serde_json::from_str(json);
        assert!(profile.is_ok());

        let profile = profile.unwrap();
        assert_eq!(profile.id, "12345");
        assert_eq!(profile.display_name, Some("Test User".to_string()));
        assert_eq!(profile.user_principal_name, Some("test@example.com".to_string()));
        assert_eq!(profile.mail, Some("test@example.com".to_string()));
    }

    #[test]
    fn test_github_user_profile_deserialization() {
        let json = r#"{
            "id": 12345,
            "login": "testuser",
            "name": "Test User",
            "email": "test@example.com",
            "avatar_url": "https://example.com/avatar.jpg"
        }"#;

        let profile: Result<GitHubUserProfile, _> = serde_json::from_str(json);
        assert!(profile.is_ok());

        let profile = profile.unwrap();
        assert_eq!(profile.id, 12345);
        assert_eq!(profile.login, "testuser");
        assert_eq!(profile.name, Some("Test User".to_string()));
        assert_eq!(profile.email, Some("test@example.com".to_string()));
        assert_eq!(profile.avatar_url, Some("https://example.com/avatar.jpg".to_string()));
    }

    #[test]
    fn test_github_email_deserialization() {
        let json = r#"[{
            "email": "test@example.com",
            "primary": true,
            "verified": true
        }]"#;

        let emails: Result<Vec<GitHubEmail>, _> = serde_json::from_str(json);
        assert!(emails.is_ok());

        let emails = emails.unwrap();
        assert_eq!(emails.len(), 1);
        assert_eq!(emails[0].email, "test@example.com");
        assert!(emails[0].primary);
        assert!(emails[0].verified);
    }
}