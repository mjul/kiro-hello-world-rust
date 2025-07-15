#!/bin/bash

# Database initialization script for SSO Web App
# This script sets up the database and runs migrations

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}🗄️  SSO Web App - Database Initialization${NC}"
echo "================================================"

# Check if .env file exists
if [ ! -f .env ]; then
    echo -e "${YELLOW}⚠️  .env file not found. Creating from .env.example...${NC}"
    if [ -f .env.example ]; then
        cp .env.example .env
        echo -e "${GREEN}✅ Created .env file from template${NC}"
        echo -e "${YELLOW}⚠️  Please edit .env with your OAuth2 credentials before running the app${NC}"
    else
        echo -e "${RED}❌ .env.example not found. Please create .env manually.${NC}"
        exit 1
    fi
fi

# Load environment variables
source .env

# Set default database URL if not specified
DATABASE_URL=${DATABASE_URL:-"sqlite:sso_app.db"}

echo -e "${GREEN}📍 Database URL: ${DATABASE_URL}${NC}"

# Extract database file path from URL
DB_FILE=$(echo $DATABASE_URL | sed 's/sqlite://')

# Check if database file already exists
if [ -f "$DB_FILE" ]; then
    echo -e "${YELLOW}⚠️  Database file already exists: $DB_FILE${NC}"
    read -p "Do you want to recreate it? This will delete all data. (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        echo -e "${YELLOW}🗑️  Removing existing database...${NC}"
        rm "$DB_FILE"
        echo -e "${GREEN}✅ Database removed${NC}"
    else
        echo -e "${GREEN}✅ Keeping existing database${NC}"
    fi
fi

# Create database directory if it doesn't exist
DB_DIR=$(dirname "$DB_FILE")
if [ "$DB_DIR" != "." ] && [ ! -d "$DB_DIR" ]; then
    echo -e "${GREEN}📁 Creating database directory: $DB_DIR${NC}"
    mkdir -p "$DB_DIR"
fi

# Check if Rust and Cargo are installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}❌ Cargo not found. Please install Rust from https://rustup.rs/${NC}"
    exit 1
fi

echo -e "${GREEN}🔧 Building application...${NC}"
cargo build --release

echo -e "${GREEN}🚀 Running database migrations...${NC}"
# The application will automatically create the database and run migrations
echo "Database will be initialized when the application starts."

echo -e "${GREEN}✅ Database initialization script completed!${NC}"
echo ""
echo -e "${GREEN}Next steps:${NC}"
echo "1. Edit .env with your OAuth2 credentials"
echo "2. Run: cargo run"
echo "3. Visit: http://localhost:3000"
echo ""
echo -e "${YELLOW}💡 Tip: Use 'RUST_LOG=debug cargo run' for detailed logging${NC}"