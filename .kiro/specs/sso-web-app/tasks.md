# Implementation Plan

- [x] 1. Set up project structure and dependencies



  - Create new Rust project with Cargo.toml
  - Add all required dependencies: axum, tokio, sqlx, askama, tower-sessions, oauth2, reqwest, thiserror, serde
  - Set up basic project directory structure with src/main.rs, src/lib.rs, and module directories
  - _Requirements: 5.5_



- [x] 2. Implement database models and repository


  - [x] 2.1 Create User model and database schema

    - Define User struct with sqlx::FromRow derive

    - Create CreateUser struct for user creation
    - Write SQL migration for users table with proper indexes
    - _Requirements: 4.1, 4.3_
  
  - [x] 2.2 Implement UserRepository with CRUD operations

    - Create UserRepository struct with SqlitePool
    - Implement find_by_provider_id method for user lookup
    - Implement create_user method with proper error handling
    - Implement update_last_login method
    - Write unit tests for all repository methods using in-memory SQLite
    - _Requirements: 4.1, 4.2_

- [x] 3. Create error handling system




  - Define AppError and AuthError enums with thiserror
  - Implement From traits for error conversion
  - Create error response handlers for Axum
  - Write unit tests for error handling scenarios
  - _Requirements: 1.4, 2.4, 3.4_

- [x] 4. Implement OAuth2 authentication service


  - [x] 4.1 Create OAuth2 client configuration


    - Set up OAuth2Client structs for Microsoft and GitHub
    - Implement configuration loading from environment variables
    - Create OAuth2 authorization URL generation methods
    - _Requirements: 1.2, 1.3_
  
  - [x] 4.2 Implement OAuth2 callback handlers


    - Create token exchange logic for authorization codes
    - Implement user profile fetching from Microsoft Graph API
    - Implement user profile fetching from GitHub API
    - Add proper error handling for OAuth2 failures
    - Write unit tests with mocked HTTP clients
    - _Requirements: 1.4, 1.5, 1.6_

- [x] 5. Create Askama templates


  - [x] 5.1 Create base template layout


    - Design base.html template with common HTML structure
    - Include CSS styling for login and dashboard pages
    - Add navigation and footer sections
    - _Requirements: 5.1, 5.2_
  
  - [x] 5.2 Implement login page template


    - Create login.html template extending base layout
    - Add Microsoft 365 and GitHub login buttons
    - Include error message display section
    - Create LoginTemplate struct with Askama derive
    - _Requirements: 1.1, 5.2_
  
  - [x] 5.3 Implement dashboard page template


    - Create dashboard.html template extending base layout
    - Add user greeting with dynamic username injection
    - Include logout button with proper form handling
    - Create DashboardTemplate struct with username field
    - _Requirements: 2.1, 2.2, 2.3, 5.3_

- [x] 6. Implement session management



  - Set up tower-sessions with cookie store
  - Create session middleware for Axum
  - Implement session creation after successful authentication
  - Implement session validation for protected routes
  - Add session cleanup on logout
  - Write tests for session lifecycle
  - _Requirements: 1.7, 3.1, 3.2_

- [x] 7. Create Axum route handlers


  - [x] 7.1 Implement authentication route handlers


    - Create GET /login handler rendering login template
    - Create GET /auth/microsoft handler for OAuth2 initiation
    - Create GET /auth/github handler for OAuth2 initiation
    - Create GET /auth/callback/microsoft handler for OAuth2 callback
    - Create GET /auth/callback/github handler for OAuth2 callback
    - Add proper error handling and redirects for all auth routes
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.7_
  
  - [x] 7.2 Implement protected route handlers


    - Create GET /dashboard handler with authentication middleware
    - Implement user data injection into dashboard template
    - Add redirect to login for unauthenticated access
    - Create POST /logout handler with session cleanup
    - _Requirements: 2.1, 2.2, 2.4, 3.1, 3.2, 3.3_
  
  - [x] 7.3 Implement root route handler


    - Create GET / handler that redirects based on authentication status
    - Redirect authenticated users to dashboard
    - Redirect unauthenticated users to login
    - _Requirements: 2.4_

- [x] 8. Set up application configuration and startup



  - Create configuration struct for environment variables
  - Implement database connection and migration runner
  - Set up Axum server with all middleware and routes
  - Add graceful shutdown handling
  - Create main.rs with proper error handling
  - _Requirements: 4.3, 5.4_

- [x] 9. Write integration tests


  - [x] 9.1 Create end-to-end authentication flow tests


    - Set up test environment with mock OAuth2 providers using wiremock
    - Test complete Microsoft OAuth2 flow from login to dashboard
    - Test complete GitHub OAuth2 flow from login to dashboard
    - Test session persistence across requests
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7_
  
  - [x] 9.2 Create protected route access tests


    - Test dashboard access with valid session
    - Test dashboard redirect for unauthenticated users
    - Test logout functionality and session cleanup
    - Test root route redirects based on authentication status
    - _Requirements: 2.1, 2.2, 2.4, 3.1, 3.2, 3.3_

- [x] 10. Add configuration and deployment setup



  - Create .env.example file with all required environment variables
  - Add database initialization script
  - Create README.md with setup and running instructions
  - Add proper logging configuration
  - _Requirements: 4.3_