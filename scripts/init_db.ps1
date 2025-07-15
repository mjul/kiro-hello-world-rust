# Database initialization script for SSO Web App (PowerShell)
# This script sets up the database and runs migrations

param(
    [switch]$Force
)

# Colors for output
$Green = "Green"
$Yellow = "Yellow"
$Red = "Red"

Write-Host "🗄️  SSO Web App - Database Initialization" -ForegroundColor $Green
Write-Host "================================================" -ForegroundColor $Green

# Check if .env file exists
if (-not (Test-Path ".env")) {
    Write-Host "⚠️  .env file not found. Creating from .env.example..." -ForegroundColor $Yellow
    if (Test-Path ".env.example") {
        Copy-Item ".env.example" ".env"
        Write-Host "✅ Created .env file from template" -ForegroundColor $Green
        Write-Host "⚠️  Please edit .env with your OAuth2 credentials before running the app" -ForegroundColor $Yellow
    } else {
        Write-Host "❌ .env.example not found. Please create .env manually." -ForegroundColor $Red
        exit 1
    }
}

# Load environment variables from .env file
if (Test-Path ".env") {
    Get-Content ".env" | ForEach-Object {
        $line = $_.Trim()
        if ($line -and -not $line.StartsWith("#") -and $line.Contains("=")) {
            $parts = $line.Split("=", 2)
            if ($parts.Length -eq 2) {
                $name = $parts[0].Trim()
                $value = $parts[1].Trim()
                [Environment]::SetEnvironmentVariable($name, $value, "Process")
            }
        }
    }
}

# Set default database URL if not specified
$DATABASE_URL = $env:DATABASE_URL
if (-not $DATABASE_URL) {
    $DATABASE_URL = "sqlite:sso_app.db"
}

Write-Host "📍 Database URL: $DATABASE_URL" -ForegroundColor $Green

# Extract database file path from URL
$DB_FILE = $DATABASE_URL -replace "sqlite:", ""

# Check if database file already exists
if (Test-Path $DB_FILE) {
    Write-Host "⚠️  Database file already exists: $DB_FILE" -ForegroundColor $Yellow
    if ($Force) {
        $recreate = $true
    } else {
        $response = Read-Host "Do you want to recreate it? This will delete all data. (y/N)"
        $recreate = $response -match "^[Yy]$"
    }
    
    if ($recreate) {
        Write-Host "🗑️  Removing existing database..." -ForegroundColor $Yellow
        Remove-Item $DB_FILE -Force
        Write-Host "✅ Database removed" -ForegroundColor $Green
    } else {
        Write-Host "✅ Keeping existing database" -ForegroundColor $Green
    }
}

# Create database directory if it doesn't exist
$DB_DIR = Split-Path $DB_FILE -Parent
if ($DB_DIR -and ($DB_DIR -ne ".") -and (-not (Test-Path $DB_DIR))) {
    Write-Host "📁 Creating database directory: $DB_DIR" -ForegroundColor $Green
    New-Item -ItemType Directory -Path $DB_DIR -Force | Out-Null
}

# Check if Rust and Cargo are installed
try {
    $null = Get-Command cargo -ErrorAction Stop
} catch {
    Write-Host "❌ Cargo not found. Please install Rust from https://rustup.rs/" -ForegroundColor $Red
    exit 1
}

Write-Host "🔧 Building application..." -ForegroundColor $Green
cargo build --release

if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Build failed" -ForegroundColor $Red
    exit 1
}

Write-Host "🚀 Database migrations will run automatically when the application starts" -ForegroundColor $Green

Write-Host "✅ Database initialization script completed!" -ForegroundColor $Green
Write-Host ""
Write-Host "Next steps:" -ForegroundColor $Green
Write-Host "1. Edit .env with your OAuth2 credentials"
Write-Host "2. Run: cargo run"
Write-Host "3. Visit: http://localhost:3000"
Write-Host ""
Write-Host "💡 Tip: Use cargo run with RUST_LOG=debug for detailed logging" -ForegroundColor $Yellow
Write-Host "   Example: " -NoNewline -ForegroundColor $Yellow
Write-Host "`$env:RUST_LOG=`"debug`"; cargo run" -ForegroundColor $Yellow