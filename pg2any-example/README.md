# PostgreSQL CDC Example

A Rust-based Change Data Capture (CDC) application that streams real-time data changes from PostgreSQL to other databases (MySQL, SQL Server, etc.) using logical replication.

## Overview

This project demonstrates how to build a production-ready CDC pipeline using the `pg2any_lib` crate. It captures changes from PostgreSQL using logical replication and streams them to destination databases in real-time.

### Features

- **Real-time streaming** from PostgreSQL to multiple database types
- **Logical replication** with configurable replication slots and publications
- **Comprehensive monitoring** with Prometheus metrics and health checks
- **Docker containerization** for easy deployment
- **Structured logging** with configurable log levels
- **Graceful error handling** and automatic recovery mechanisms

## Quick Start

### Prerequisites

- Docker and Docker Compose
- Rust 1.88+ (for local development)
- PostgreSQL 10+ with logical replication enabled
- Destination database (MySQL 8.0+, SQL Server, etc.)

### 1. Clone and Setup

```bash
git clone <your-repo>
cd pg2any-example
```

### 2. Environment Configuration

Copy and modify the environment file:

```bash
cp .env .env.local
# Edit .env.local with your specific database configurations
```

Key environment variables:

```bash
# Source PostgreSQL Database
CDC_SOURCE_CONNECTION_STRING=postgresql://user:password@host:port/database?replication=database

# Destination Database 
CDC_DEST_TYPE=MySQL  # or SqlServer
CDC_DEST_URI=mysql://user:password@host:port/database

# CDC Configuration
CDC_REPLICATION_SLOT=cdc_slot
CDC_PUBLICATION=cdc_pub
CDC_PROTOCOL_VERSION=2
```

### 3. Run with Docker Compose

```bash
# Start all services (PostgreSQL, MySQL, CDC app, Prometheus)
docker-compose up -d

# View logs
docker-compose logs -f cdc_app

# Stop all services
docker-compose down
```

### 4. Local Development

```bash
# Install dependencies
cargo build

# Run locally (ensure databases are accessible)
cargo run

# Run with custom environment file
env $(cat .env.local | xargs) cargo run
```

## Configuration

### Environment Variables

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `CDC_SOURCE_CONNECTION_STRING` | PostgreSQL connection with replication | - | Yes |
| `CDC_DEST_TYPE` | Destination database type (MySQL/SqlServer) | - | Yes |
| `CDC_DEST_URI` | Destination database connection string | - | Yes |
| `CDC_REPLICATION_SLOT` | PostgreSQL replication slot name | `cdc_slot` | No |
| `CDC_PUBLICATION` | PostgreSQL publication name | `cdc_pub` | No |
| `CDC_PROTOCOL_VERSION` | Logical replication protocol version | `2` | No |
| `CDC_BINARY_FORMAT` | Use binary format for data | `false` | No |
| `CDC_STREAMING` | Enable streaming mode | `true` | No |
| `CDC_CONNECTION_TIMEOUT` | Connection timeout (seconds) | `30` | No |
| `CDC_QUERY_TIMEOUT` | Query timeout (seconds) | `10` | No |
| `CDC_HEARTBEAT_INTERVAL` | Heartbeat interval (seconds) | `10` | No |
| `METRICS_PORT` | Prometheus metrics port | `8080` | No |
| `RUST_LOG` | Logging level | `info` | No |

### PostgreSQL Setup

Your PostgreSQL instance must have logical replication enabled:

```sql
-- Check current settings
SHOW wal_level;
SHOW max_replication_slots;
SHOW max_wal_senders;

-- If needed, update postgresql.conf:
-- wal_level = logical
-- max_replication_slots = 10
-- max_wal_senders = 10

-- Create publication for tables you want to replicate
CREATE PUBLICATION cdc_pub FOR ALL TABLES;
-- Or for specific tables:
-- CREATE PUBLICATION cdc_pub FOR TABLE table1, table2;

-- Grant necessary permissions
GRANT REPLICATION ON DATABASE your_db TO your_user;
GRANT SELECT ON ALL TABLES IN SCHEMA public TO your_user;
```

## Architecture

```
┌─────────────────┐    ┌───────────────────┐    ┌─────────────────┐
│   PostgreSQL    │───▶│   CDC Application │───▶│ MySQL/SqlServer │
│ (Logical Replic)│    │  (pg2any-example) │    │  (Destination)  │
└─────────────────┘    └───────────────────┘    └─────────────────┘
                                │
                                ▼
                       ┌─────────────────┐
                       │   Prometheus    │
                       │   (Metrics)     │
                       └─────────────────┘
```

### Components

1. **CDC Application**: Main Rust application that handles replication
2. **Source Database**: PostgreSQL with logical replication enabled
3. **Destination Database**: Target database (MySQL, SQL Server, etc.)
4. **Monitoring**: Prometheus metrics and health checks

## Monitoring

### Health Checks

The application exposes health endpoints:

- **Health**: `GET /health` - Application health status
- **Metrics**: `GET /metrics` - Prometheus metrics

### Prometheus Metrics

Available at `http://localhost:9090` when using Docker Compose:

- Connection status
- Replication lag
- Message processing rates
- Error counts

### Logs

Structured logging with configurable levels:

```bash
# Set log level
export RUST_LOG=debug

# View logs in Docker
docker-compose logs -f cdc_app
```

## Troubleshooting

### Common Issues

1. **Connection Refused**
   ```bash
   # Check if PostgreSQL allows replication connections
   # Ensure pg_hba.conf has replication entries
   ```

2. **Replication Slot Already Exists**
   ```sql
   -- Drop existing slot if needed
   SELECT pg_drop_replication_slot('cdc_slot');
   ```

3. **Permission Denied**
   ```sql
   -- Grant necessary permissions
   GRANT REPLICATION ON DATABASE your_db TO your_user;
   ```

4. **High Replication Lag**
   - Check network connectivity
   - Monitor destination database performance
   - Review CDC application logs

## Performance Tuning

### PostgreSQL Optimization

```sql
-- Increase WAL settings for high throughput
ALTER SYSTEM SET max_wal_senders = 20;
ALTER SYSTEM SET max_replication_slots = 20;
ALTER SYSTEM SET wal_keep_segments = 100; -- or wal_keep_size for PG 13+
```

### Application Tuning

```bash
# Adjust timeouts for your environment
CDC_CONNECTION_TIMEOUT=60
CDC_QUERY_TIMEOUT=30
CDC_HEARTBEAT_INTERVAL=5
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make changes with proper tests
4. Run `cargo fmt` and `cargo clippy`
5. Submit a pull request

## Dependencies

- [pg2any_lib](https://crates.io/crates/pg2any_lib) - Core CDC functionality
- [tokio](https://crates.io/crates/tokio) - Async runtime
- [tracing](https://crates.io/crates/tracing) - Structured logging

## Support

- Check the [pg2any_lib documentation](https://docs.rs/pg2any_lib)
- Open an issue for bugs or feature requests
- Review logs for detailed error information
