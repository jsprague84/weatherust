# Weatherust

> Infrastructure monitoring and automation platform built with Rust

[![Rust](https://img.shields.io/badge/rust-1.90.0-orange.svg)](https://www.rust-lang.org/)
[![Docker](https://img.shields.io/badge/docker-compose-blue.svg)](https://docs.docker.com/compose/)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

Weatherust is a comprehensive Rust-based monitoring and automation platform for infrastructure management, featuring weather monitoring, speed testing, health checks, and system updates across multiple servers.

## 📋 Table of Contents

- [Features](#features)
- [Quick Start](#quick-start)
- [Services](#services)
- [Documentation](#documentation)
- [Development](#development)
- [Configuration](#configuration)
- [Deployment](#deployment)

## ✨ Features

- **🌤️ Weather Monitoring** - OpenWeatherMap integration with 7-day forecasts
- **⚡ Speed Testing** - Ookla Speedtest CLI with threshold alerts
- **🏥 Health Monitoring** - Docker container health and resource tracking
- **📦 Update Monitoring** - Multi-server OS and Docker image update detection
- **🔄 Update Automation** - Remote update application and cleanup operations
- **🔔 Dual Notifications** - Gotify and ntfy.sh support with action buttons
- **🐳 Docker-First** - Containerized services with Ofelia scheduling
- **🔒 Security-Focused** - Constant-time comparisons, minimal key exposure
- **📊 Observable** - Structured logging with tracing and metrics

## 🚀 Quick Start

### Prerequisites

- Docker and Docker Compose
- OpenWeatherMap API key (for weather service)
- Notification backend: Gotify and/or ntfy.sh
- SSH key for remote server access (for update services)

### Installation

1. **Clone and configure**:
   ```bash
   git clone https://github.com/jsprague84/weatherust.git
   cd weatherust
   cp .env.example .env
   ```

2. **Edit `.env`** and configure:
   ```bash
   # OpenWeatherMap
   OWM_API_KEY=your_api_key

   # Notifications (choose one or both)
   GOTIFY_URL=https://gotify.example.com/message
   WEATHERUST_GOTIFY_KEY=your_token

   # OR/AND
   NTFY_URL=https://ntfy.sh
   WEATHERUST_NTFY_TOPIC=weather

   # SSH for remote operations
   UPDATE_SSH_KEY=/home/user/.ssh/id_ed25519
   UPDATE_SERVERS=server1:user@host1,server2:user@host2
   ```

3. **Start the stack**:
   ```bash
   docker compose pull
   docker compose up -d
   ```

4. **Verify**:
   ```bash
   docker compose logs -f ofelia
   docker compose ps
   ```

### Test Individual Services

```bash
# Weather check
docker compose run --rm weatherust --zip 52726 --units imperial

# Speed test
docker compose run --rm speedynotify

# Health check
docker compose exec healthmon_runner /app/healthmon health

# Update monitor
docker compose exec updatemon_runner /app/updatemon --docker --local

# Update controller (list servers)
docker compose exec updatectl_runner /app/updatectl list servers
```

## 🛠️ Services

### Monitoring Services (Read-Only)

| Service | Purpose | Schedule | Details |
|---------|---------|----------|---------|
| **weatherust** | Weather monitoring | Daily 05:30 | [README](./README.md#weatherust) |
| **speedynotify** | Internet speed tests | Daily 02:10 | Threshold alerts |
| **healthmon** | Container health | Every 5 min | CPU/MEM monitoring |
| **updatemon** | Update detection | Daily 03:00 | [README](./updatemon/README.md) |

### Action Services (Write Operations)

| Service | Purpose | Access | Details |
|---------|---------|--------|---------|
| **updatectl** | Update controller | CLI + Webhook | [README](./updatectl/README.md) |
| **updatectl_webhook** | HTTP API | Port 8080 | [API Docs](./docs/reference/WEBHOOK_API.md) |

## 📚 Documentation

### For Users

- **[README.md](./README.md)** (full) - Complete user documentation
- **[updatectl/README.md](./updatectl/README.md)** - Update controller guide
- **[updatemon/README.md](./updatemon/README.md)** - Update monitor guide
- **[Webhook API](./docs/reference/WEBHOOK_API.md)** - Webhook API reference
- **[CLI Commands](./docs/reference/CLI-COMMANDS.md)** - Command reference

### For Developers

- **[CLAUDE.md](./CLAUDE.md)** - Quick reference for AI assistants
- **[Claude Code Guide](./docs/development/CLAUDE_CODE_GUIDE.md)** - **START HERE** - Comprehensive development guide
- **[Architecture](./docs/architecture/ARCHITECTURE.md)** - System architecture and design
- **[Contributing](./docs/development/CONTRIBUTING.md)** - Development workflow and standards
- **[Modernization Summary](./docs/development/MODERNIZATION_SUMMARY.md)** - Recent improvements
- **[Code Examples](./docs/development/MODERNIZATION_EXAMPLES.md)** - Code examples
- **[Documentation Index](./docs/README.md)** - Complete documentation navigation

## 💻 Development

### Local Development

```bash
# Build
cargo build --workspace

# Test
cargo test --workspace

# Run specific service
cargo run --bin updatectl -- list servers

# Format and lint
cargo fmt --all
cargo clippy --workspace -- -D warnings
```

### Docker Build

```bash
# Build all services
docker build -t weatherust:local .
docker build -f Dockerfile.updatectl -t updatectl:local .
docker build -f Dockerfile.healthmon -t healthmon:local .
```

### Documentation

```bash
# Generate API docs
cargo doc --open
```

For detailed development instructions, see [CONTRIBUTING.md](./docs/development/CONTRIBUTING.md).

## ⚙️ Configuration

### Notification Backends

#### Gotify (Self-Hosted)

```bash
GOTIFY_URL=https://gotify.example.com/message

# Service-specific tokens
WEATHERUST_GOTIFY_KEY=token1
UPDATEMON_GOTIFY_KEY=token2
UPDATECTL_GOTIFY_KEY=token3
HEALTHMON_GOTIFY_KEY=token4
SPEEDY_GOTIFY_KEY=token5
```

#### ntfy.sh (Public or Self-Hosted)

```bash
NTFY_URL=https://ntfy.sh  # or your server
NTFY_AUTH=token  # optional for self-hosted

# Service-specific topics
WEATHERUST_NTFY_TOPIC=weather
UPDATEMON_NTFY_TOPIC=updates
UPDATECTL_NTFY_TOPIC=update-actions
HEALTHMON_NTFY_TOPIC=docker-health
SPEEDY_NTFY_TOPIC=speedtest
```

### Server Configuration

```bash
# Format: name:user@host or user@host
UPDATE_SERVERS=Office-WS:user@192.168.1.10,Cloud-VM:user@remote.com

# SSH key (only this key is mounted to containers)
UPDATE_SSH_KEY=/home/user/.ssh/id_ed25519

# Local server name (optional)
UPDATE_LOCAL_NAME=docker-vm
```

### Schedule Customization

Edit `docker-compose.yml` Ofelia labels:

```yaml
# Example: Run weatherust at 06:00 instead of 05:30
ofelia.job-exec.weatherust.schedule: "0 0 6 * * *"
```

Cron format: `second minute hour day month weekday`

## 🚢 Deployment

### Production Deployment

1. **Pin versions** in `.env`:
   ```bash
   WEATHERUST_TAG=v1.0.0
   UPDATECTL_TAG=v1.0.0
   HEALTHMON_TAG=v1.0.0
   UPDATEMON_TAG=v1.0.0
   SPEEDYNOTIFY_TAG=v1.0.0
   ```

2. **Configure secrets** properly (see [Security](#security))

3. **Deploy**:
   ```bash
   docker compose pull
   docker compose up -d
   ```

### Security Best Practices

✅ **DO**:
- Mount only specific SSH key (`UPDATE_SSH_KEY`)
- Use service-specific notification tokens
- Pin Docker image versions for production
- Rotate secrets periodically
- Enable `--dry-run` for testing

❌ **DON'T**:
- Mount entire `.ssh` directory
- Commit `.env` file
- Use `latest` tag in production
- Share secrets between environments

### Monitoring

```bash
# View logs
docker compose logs -f

# Check service status
docker compose ps

# View Ofelia schedule
docker compose logs ofelia

# Check specific service
docker compose logs -f updatectl_webhook
```

## 🏗️ Architecture

```
┌─────────────────────────────────────────┐
│          Ofelia Scheduler               │
│         (Cron-like for Docker)          │
└────┬────┬────┬────┬────────────────────┘
     │    │    │    │
     ▼    ▼    ▼    ▼
┌────────────────────────────────────────┐
│  weatherust  speedynotify  healthmon   │
│  updatemon                             │
└────┬───────────────────────────────────┘
     │
     ▼
┌────────────────────────────────────────┐
│   Notifications (Gotify / ntfy.sh)     │
└────────────────────────────────────────┘

     ┌──────────────┐    ┌──────────────┐
     │  updatectl   │    │  updatectl   │
     │   (CLI)      │    │  (Webhook)   │
     └──────┬───────┘    └──────┬───────┘
            │                   │
            └─────────┬─────────┘
                      ▼
              ┌───────────────┐
              │ Remote Servers│
              │    via SSH    │
              └───────────────┘
```

For detailed architecture, see [ARCHITECTURE.md](./docs/architecture/ARCHITECTURE.md).

## 🔧 Workspace Structure

```
weatherust/
├── common/             # Shared library
│   ├── error.rs       # Structured error types
│   ├── constants.rs   # Configuration constants
│   ├── security.rs    # Security utilities
│   ├── retry.rs       # Retry logic
│   └── ...
├── weatherust/        # Weather monitoring
├── speedynotify/      # Speed testing
├── healthmon/         # Health monitoring
├── updatemon/         # Update monitoring
└── updatectl/         # Update controller
```

## 🤝 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](./docs/development/CONTRIBUTING.md) for guidelines.

### Quick Contribution Guide

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes following our [code standards](./docs/development/CONTRIBUTING.md#code-standards)
4. Test thoroughly (`cargo test --workspace`)
5. Commit using [conventional commits](./docs/development/CONTRIBUTING.md#conventional-commits)
6. Push and create a Pull Request

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [OpenWeatherMap](https://openweathermap.org/) - Weather data API
- [Ookla Speedtest](https://www.speedtest.net/apps/cli) - Speed test CLI
- [Ofelia](https://github.com/mcuadros/ofelia) - Docker job scheduler
- [Gotify](https://gotify.net/) - Self-hosted notification server
- [ntfy.sh](https://ntfy.sh/) - Open-source notification service

## 🆘 Support

- **Documentation**: Check the [docs](#documentation) section
- **Issues**: [GitHub Issues](https://github.com/jsprague84/weatherust/issues)
- **Discussions**: [GitHub Discussions](https://github.com/jsprague84/weatherust/discussions)

---

**Note**: For detailed user documentation including all configuration options, troubleshooting, and examples, see the full [README.md](./README.md).
