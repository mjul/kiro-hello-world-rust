-- Create users table for SSO authentication
CREATE TABLE users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    provider TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    username TEXT NOT NULL,
    email TEXT,
    avatar_url TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_login DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(provider, provider_id)
);

-- Create index for efficient lookups by provider and provider_id
CREATE INDEX idx_users_provider_id ON users(provider, provider_id);

-- Create index for username lookups
CREATE INDEX idx_users_username ON users(username);