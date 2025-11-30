<div align="center">
  <h1 align="center">🚀 Cybertemp Authoritative DNS Server</h1>
  <p align="center">
    A high-performance, production-ready authoritative DNS server built with Rust, powering Cybertemp's domain management infrastructure. Handles DNS queries, domain verification, and dynamic record management with PostgreSQL and Supabase integration.
    <br />
    <br />
    <a href="https://discord.cyberious.xyz">💬 Discord</a>
    ·
    <a href="#-changelog">📜 ChangeLog</a>
    ·
    <a href="https://github.com/sexfrance/dns-server/issues">⚠️ Report Bug</a>
    ·
    <a href="https://github.com/sexfrance/dns-server/issues">💡 Request Feature</a>
  </p>
</div>

---

## 📖 Table of Contents

- [About](#-about)
- [Architecture](#️-architecture)
- [Features](#-features)
- [Prerequisites](#-prerequisites)
- [Installation](#️-installation)
- [Configuration](#-configuration)
- [Database Setup](#-database-setup)
- [Running the Server](#-running-the-server)
- [API Endpoints](#-api-endpoints)
- [How It Works](#-how-it-works)
- [Domain Management](#-domain-management)
- [Verification Process](#-verification-process)
- [Supabase Integration](#-supabase-integration)
- [Important Notes](#-important-notes)
- [Troubleshooting](#-troubleshooting)
- [Development Status](#-development-status)
- [ChangeLog](#-changelog)

---

## 📚 About

**THIS README IS GENERATED FOR THE CYBERTEMP DNS SERVER**

**This is the authoritative DNS server currently powering [Cybertemp](https://cybertemp.xyz)** - a domain management service handling DNS resolution for managed domains. The codebase is functional and designed for high concurrency and reliability.

This server is designed as an **authoritative DNS server** that:

- Responds to DNS queries on port 53 (UDP/TCP)
- Manages domain records in PostgreSQL
- Supports A, MX, NS, and SOA records
- Implements domain verification via NS record checking
- Syncs domains from Supabase for centralized management
- Provides HTTP redirects (optional, currently disabled)
- Handles concurrent queries efficiently with Tokio async runtime

---

## 🏗️ Architecture

```
┌─────────────────┐
│   DNS Queries   │
│   Port 53       │
│   (UDP/TCP)     │
└────────┬────────┘
         │
         ▼
┌─────────────────────────────────┐
│   Rust DNS Server (Tokio)       │
│  - Trust-DNS for protocol       │
│  - Domain verification          │
│  - Record generation            │
│  - Concurrent query handling    │
└────────┬────────────────────────┘
         │
         ├─────────────────┬────────────────┬────────────────┬────────────────┐
         ▼                 ▼                ▼                ▼                ▼
┌─────────────────┐ ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌──────────────┐
│   PostgreSQL    │ │   Supabase   │ │   HTTP API   │ │   Redirects  │ │   Verification│
│  (Local Domains)│ │   (Sync)     │ │   (Optional) │ │   (Disabled) │ │   (NS Check)  │
└─────────────────┘ └──────────────┘ └──────────────┘ └──────────────┘ └──────────────┘
```

---

## 🔥 Features

- ✅ **Production-Ready**: Currently handling Cybertemp's DNS traffic
- ✅ **High Performance**: Async Rust with Tokio for concurrent DNS queries
- ✅ **Authoritative DNS**: Responds authoritatively for managed domains
- ✅ **Domain Verification**: Automatic NS record checking and verification
- ✅ **PostgreSQL Storage**: Reliable domain storage with indexing
- ✅ **Supabase Sync**: Real-time domain synchronization
- ✅ **Dynamic Records**: A, MX, NS, SOA record generation
- ✅ **HTTP API**: RESTful API for domain management
- ✅ **HTTP Redirects**: Optional HTTP redirect server (currently disabled)
- ✅ **Auto-Discovery**: Automatic domain detection and addition
- ✅ **Graceful Verification**: 48-hour grace period for NS mismatches
- ✅ **Connection Pooling**: Efficient database connections
- ✅ **Comprehensive Logging**: Detailed tracing with configurable levels
- ✅ **Self-Hosted**: No external dependencies for core functionality

---

## 📋 Prerequisites

- **Rust** 1.70+ (for compilation)
- **PostgreSQL** 12+ (for domain storage)
- **Linux/Windows/macOS** (any platform supporting Rust)
- **Port 53 Access** (requires root/admin on Linux for port 53)
- **Supabase Account** (optional, for domain sync)

---

## ⚙️ Installation

### Using Rust (Recommended)

1. **Install Rust** (if not already installed):

   ```bash
   # Linux/macOS
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

   # Windows
   # Download from https://rustup.rs/
   ```

2. **Clone the repository**:

   ```bash
   git clone https://github.com/sexfrance/dns-server.git
   cd dns-server
   ```

3. **Build the project**:

   ```bash
   # Development build
   cargo build

   # Production build (optimized)
   cargo build --release
   ```

---

## 🔧 Configuration

Create `config/dns.toml` in the project root with the following settings:

```toml
# Server Configuration
bind_address = "0.0.0.0"
port = 53
default_ttl = 300

# Domain Settings
mx_priority = 10
mail_server = "mail.{domain}"
nameservers = ["ns1.cybertemp.xyz", "ns2.cybertemp.xyz"]

# Verification Settings
verification_interval_seconds = 3600
grace_period_hours = 48

# Database Configuration (REQUIRED)
database_url = "postgresql://username:password@localhost:5432/dns"

# Supabase Configuration (OPTIONAL)
supabase_url = "https://your-project.supabase.co"
supabase_key = "your-service-role-key"

# HTTP Redirect Configuration (OPTIONAL - Currently Disabled)
http_redirect_enabled = false
http_redirect_port = 8080
redirect_target = "https://cybertemp.xyz"

# Auto-Discovery
auto_discovery_enabled = true
```

### Configuration Options Explained

| Setting                      | Required | Default | Description |
|------------------------------|----------|---------|-------------|
| `bind_address`               | ❌ No    | 0.0.0.0 | IP address to bind the DNS server |
| `port`                       | ❌ No    | 53      | DNS server port |
| `default_ttl`                | ❌ No    | 300     | Default TTL for DNS records |
| `mx_priority`                | ❌ No    | 10      | MX record priority |
| `mail_server`                | ❌ No    | mail.{domain} | Mail server template |
| `nameservers`                | ❌ No    | []      | Authoritative nameservers |
| `verification_interval_seconds` | ❌ No    | 3600    | Domain verification interval |
| `grace_period_hours`          | ❌ No    | 48      | Grace period before disabling domains |
| `database_url`               | ✅ Yes   | -       | PostgreSQL connection string |
| `supabase_url`               | ❌ No    | -       | Supabase project URL |
| `supabase_key`               | ❌ No    | -       | Supabase service role key |
| `http_redirect_enabled`      | ❌ No    | false   | Enable HTTP redirect server |
| `http_redirect_port`         | ❌ No    | 8080    | HTTP redirect server port |
| `redirect_target`            | ❌ No    | -       | HTTP redirect target URL |
| `auto_discovery_enabled`     | ❌ No    | true    | Enable automatic domain discovery |

---

## 💾 Database Setup

### PostgreSQL Schema

Run the SQL migration located in `migrations/001_initial_schema.sql`:

```bash
psql -U your_user -d dns -f migrations/001_initial_schema.sql
```

This creates:

- `domains` table - stores domain information and records
- Indexes for performance
- Triggers for automatic timestamp updates

### Optional Supabase Tables

For Supabase integration, create a `domains` table in Supabase:

```sql
CREATE TABLE domains (
  id UUID DEFAULT gen_random_uuid() PRIMARY KEY,
  user_id UUID,
  domain TEXT NOT NULL UNIQUE,
  cloudflare_domain BOOLEAN DEFAULT false,
  pending_ns_check BOOLEAN DEFAULT true,
  added_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW(),
  active BOOLEAN DEFAULT true,
  discord BOOLEAN DEFAULT false,
  stripe_payment_id TEXT,
  payment_status TEXT DEFAULT 'pending',
  amount_paid DECIMAL,
  is_one_time_purchase BOOLEAN DEFAULT true
);

-- Indexes
CREATE INDEX idx_domains_domain ON domains(domain);
CREATE INDEX idx_domains_active ON domains(active);
```

---

## 🚀 Running the Server

### Development Mode

```bash
cargo run
```

### Production Mode

```bash
cargo build --release
./target/release/cybertemp_dns
```

### Running on Port 53 (Linux)

Port 53 requires root privileges or capability:

```bash
# Option 1: Run as root
sudo ./target/release/cybertemp_dns

# Option 2: Grant port binding capability (recommended)
sudo setcap CAP_NET_BIND_SERVICE=+eip ./target/release/cybertemp_dns
./target/release/cybertemp_dns
```

### Running as a Service (Systemd)

Create `/etc/systemd/system/cybertemp-dns.service`:

```ini
[Unit]
Description=Cybertemp DNS Server
After=network.target postgresql.service

[Service]
Type=simple
User=your-user
WorkingDirectory=/path/to/dns-server
ExecStart=/path/to/dns-server/target/release/cybertemp_dns
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable cybertemp-dns
sudo systemctl start cybertemp-dns
sudo systemctl status cybertemp-dns
```

---

## 🔌 API Endpoints

The server includes a RESTful API for domain management (runs on port 8080 by default, configurable):

### Health Check
```http
GET /health
```

### Get Statistics
```http
GET /stats
```

### List Domains
```http
GET /domains
```

### Add Domain
```http
POST /domains
Content-Type: application/json

{
  "domain": "example.com",
  "ip": "192.168.1.1",
  "discord": false
}
```

### Remove Domain
```http
DELETE /domains/example.com
```

### Force Verification
```http
POST /verify/example.com
```

---

## 🔍 How It Works

### DNS Query Flow

1. **Query Reception**: DNS query arrives on port 53
2. **Domain Lookup**: Server checks if domain is managed in PostgreSQL
3. **Record Generation**: Generates appropriate DNS records (A, MX, NS, SOA)
4. **Response**: Sends authoritative DNS response
5. **Verification**: Periodically checks NS records for managed domains

### Record Types Supported

- **A Records**: IPv4 address resolution
- **MX Records**: Mail server configuration
- **NS Records**: Nameserver delegation
- **SOA Records**: Start of Authority information

---

## 🏷️ Domain Management

### Adding Domains

Domains can be added through:

1. **Supabase Sync**: Automatic sync from Supabase `domains` table
2. **API**: POST to `/domains` endpoint
3. **Auto-Discovery**: Automatic detection when domains point to your NS

### Domain States

- **Pending Verification**: Domain added but NS not verified
- **Verified**: NS records match configured nameservers
- **Grace Period**: NS mismatch detected, 48-hour grace period
- **Failed**: Grace period expired, domain disabled

---

## ✅ Verification Process

The server verifies domain ownership by:

1. Querying the domain's NS records
2. Checking if they match configured nameservers (`ns1.cybertemp.xyz`, `ns2.cybertemp.xyz`)
3. Updating verification status in database
4. Disabling domains that fail verification after grace period

Verification runs every 3600 seconds (1 hour) by default.

---

## ☁️ Supabase Integration

### Sync Process

- **From Supabase**: Pulls active domains every 5 minutes
- **To Supabase**: Updates verification status and metadata
- **Conflict Resolution**: Uses domain as unique key

### Supabase Tables

- `domains`: Domain whitelist and metadata
- Automatic updates prevent manual database management

---

## ⚠️ Important Notes

### Code Quality

This codebase is functional but could use refactoring. It's a working production system that evolved organically.

### Port Conflicts

- Port 53 conflicts with systemd-resolved on Linux
- Disable systemd-resolved and set `/etc/resolv.conf` to external DNS (e.g., 8.8.8.8)

### Security

- Database credentials in config file
- Supabase keys have full access
- Run as non-root user when possible

### HTTP Redirects

Currently disabled due to port conflicts. Can be re-enabled for HTTP-to-HTTPS redirects.

---

## 🛠️ Troubleshooting

### Common Issues

#### 1. "Address already in use" on port 53

**Solution**: Disable systemd-resolved

```bash
sudo systemctl stop systemd-resolved
sudo systemctl disable systemd-resolved
echo "nameserver 8.8.8.8" > /etc/resolv.conf
```

#### 2. Database connection errors

**Check**:
- PostgreSQL is running
- Connection string is correct
- Database and user exist

#### 3. Supabase sync failures

**Check**:
- Network connectivity
- Supabase URL and key are correct
- Supabase tables exist

#### 4. Domain verification failures

**Check**:
- Domain NS records point to your nameservers
- Nameservers resolve to your server IP
- DNS propagation (can take 24-48 hours)

#### 5. Permission denied binding to port 53

**Solution**:

```bash
# Run as root
sudo ./target/release/cybertemp_dns

# Or grant capability
sudo setcap CAP_NET_BIND_SERVICE=+eip ./target/release/cybertemp_dns
```

---

## 📊 Development Status

| Component                     | Status               | Notes |
|-------------------------------|----------------------|-------|
| **Rust Implementation**       | ✅ **Active**        | Production-ready |
| **PostgreSQL Storage**        | ✅ Production        | Stable |
| **Supabase Integration**      | ✅ Production        | Working |
| **DNS Protocol Handling**     | ✅ Production        | Trust-DNS |
| **Domain Verification**       | ✅ Production        | Robust |
| **HTTP API**                  | ✅ Production        | RESTful |
| **HTTP Redirects**            | ⚠️ Disabled         | Port conflicts |
| **Auto-Discovery**            | ✅ Production        | Working |
| **Code Quality**              | ⚠️ Needs Refactoring | Functional but messy |
| **Documentation**             | ✅ Complete          | Comprehensive |

---

## 📜 ChangeLog

```diff
v1.0.0 ⋮ 11/30/2025
+ Comprehensive README documentation
+ Supabase integration for domain sync
+ Domain verification with grace periods
+ HTTP API for domain management
+ Auto-discovery functionality
+ PostgreSQL storage with migrations

v0.5.0 ⋮ 11/25/2025
+ HTTP redirect server implementation
+ Config file support (TOML)
+ Improved error handling
+ Database connection pooling

v0.1.0 ⋮ 11/01/2025
! Initial implementation
+ Basic DNS server with Trust-DNS
+ PostgreSQL domain storage
+ A, MX, NS record support
+ Tokio async runtime
```

---

## 📞 Support

- **Discord**: [discord.cyberious.xyz](https://discord.cyberious.xyz)
- **Email**: support@cybertemp.xyz
- **Issues**: [GitHub Issues](https://github.com/sexfrance/dns-server/issues)

---

## 📄 License

MIT License - See LICENSE file for details

---

## 🙏 Acknowledgments

- Built with ❤️ for the Cybertemp community
- Powered by Rust 🦀, Tokio, Trust-DNS, and PostgreSQL
- Supabase integration for centralized management

---

<p align="center">
  <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white"/>
  <img src="https://img.shields.io/badge/PostgreSQL-316192?style=for-the-badge&logo=postgresql&logoColor=white"/>
  <img src="https://img.shields.io/badge/Supabase-181818?style=for-the-badge&logo=supabase&logoColor=white"/>
  <img src="https://img.shields.io/badge/DNS-Ready-success?style=for-the-badge"/>
</p>

<div align="center">
  <strong>⭐ Star this repo if you found it helpful!</strong>
  <br />
  <sub>Currently powering Cybertemp's DNS infrastructure 🚀</sub>
</div>