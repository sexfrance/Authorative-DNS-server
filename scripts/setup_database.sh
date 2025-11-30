#!/bin/bash

# Database setup script for DNS server
set -e

# Load environment variables from .env file
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_ROOT"
if [ -f ".env" ]; then
    set -a
    source ".env"
    set +a
fi

DB_NAME=${DB_NAME:-"dns_server"}
DB_USER=${DB_USER:-"dns_user"}
DB_PASS=${DB_PASS:-"dns_password"}
DB_HOST=${DB_HOST:-"localhost"}
DB_PORT=${DB_PORT:-"5432"}

echo "Setting up DNS server database..."

# Check if PostgreSQL is installed
if ! command -v psql &> /dev/null; then
    echo "PostgreSQL is not installed. Please install it first."
    exit 1
fi

# Create database and user
sudo -u postgres psql -c "CREATE DATABASE $DB_NAME;" || true
sudo -u postgres psql -c "CREATE USER $DB_USER WITH PASSWORD '$DB_PASS';" || true
sudo -u postgres psql -c "GRANT ALL PRIVILEGES ON DATABASE $DB_NAME TO $DB_USER;" || true

# Run migrations
export PGPASSWORD="$DB_PASS"
psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME -f migrations/001_initial_schema.sql

echo "Database setup complete!"
echo "Database: $DB_NAME"
echo "User: $DB_USER"
echo "Update your config file with these credentials"