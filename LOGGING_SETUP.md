# Centralized Logging with Loki & Grafana

This guide explains how to deploy and use the Loki logging stack for centralized log aggregation and analysis.

## Quick Start

### 1. Configure Your Domain

Add to your `.env` file:

```bash
# Grafana domain (adjust for your setup)
GRAFANA_DOMAIN=grafana.js-node.app

# Grafana admin password (CHANGE THIS!)
GRAFANA_PASSWORD=your-secure-password-here
```

### 2. Deploy the Logging Stack

```bash
# Deploy with your existing services
docker compose -f docker-compose.yml -f docker-compose.logging.yml up -d
```

This deploys:
- **Loki** - Log aggregation engine
- **Promtail** - Log collector (automatically scrapes Docker container logs)
- **Grafana** - Visualization and query interface

### 3. Access Grafana

Visit: `https://grafana.js-node.app` (or your configured domain)

**Default credentials:**
- Username: `admin`
- Password: Value from `GRAFANA_PASSWORD` env var

### 4. Query Logs

Grafana comes pre-configured with Loki as the default datasource. Go to **Explore** in the left menu.

## Common Log Queries

### View All Logs from a Service

```logql
{service="weatherust"}
```

### Filter by Log Level

```logql
{service="healthmon"} |= "ERROR"
{service="updatectl"} |= "WARN"
```

### Search Specific Text

```logql
{service="updatemon"} |~ "server1.*updates available"
```

### Multiple Services

```logql
{service=~"weatherust|healthmon|speedynotify"}
```

### Errors Across All Services

```logql
{compose_project="weatherust"} |= "ERROR"
```

### Time Range Queries

Use the time picker in Grafana's UI to select:
- Last 5 minutes
- Last 1 hour
- Last 24 hours
- Custom range

## Architecture

```
┌─────────────────┐
│ Docker          │
│ Containers      │
│ (weatherust,    │
│  healthmon,     │
│  updatectl...)  │
└────────┬────────┘
         │ Container logs
         ▼
    ┌────────────┐
    │  Promtail  │
    │ (collector)│
    └─────┬──────┘
          │ Ship logs
          ▼
     ┌─────────┐
     │  Loki   │
     │ (store) │
     └────┬────┘
          │ Query logs
          ▼
     ┌──────────┐
     │ Grafana  │
     │   (UI)   │
     └──────────┘
```

## Log Retention

Logs are retained for **7 days** by default. To change this, edit `loki-config.yml`:

```yaml
limits_config:
  retention_period: 168h  # Change to desired hours (168h = 7 days)
```

Then restart Loki:
```bash
docker compose -f docker-compose.yml -f docker-compose.logging.yml restart loki
```

## Controlling Log Levels

Set the `RUST_LOG` environment variable in your `.env`:

```bash
# Global log level
RUST_LOG=info

# Per-service (more verbose for specific service)
RUST_LOG=weatherust=debug,healthmon=info,updatectl=trace
```

Then restart services:
```bash
docker compose -f docker-compose.yml -f docker-compose.logging.yml restart
```

## Viewing Logs

### Via Grafana (Recommended)
- Rich UI with filtering, time range selection
- Log context (see logs before/after a specific line)
- Live tailing
- Export to CSV/JSON

### Via Docker (Quick Check)
```bash
# Still works as before
docker compose logs -f healthmon

# View all services
docker compose logs -f
```

## Troubleshooting

### No Logs Appearing in Grafana

1. **Check Promtail is running:**
   ```bash
   docker compose -f docker-compose.logging.yml ps promtail
   docker compose -f docker-compose.logging.yml logs promtail
   ```

2. **Verify Loki is receiving logs:**
   ```bash
   curl http://localhost:3100/ready
   ```

3. **Check Promtail can access Docker socket:**
   ```bash
   docker compose -f docker-compose.logging.yml exec promtail ls -la /var/run/docker.sock
   ```

### Grafana Can't Connect to Loki

1. **Check networks:**
   ```bash
   docker network inspect weatherust_logging
   ```

2. **Verify Loki is healthy:**
   ```bash
   curl http://localhost:3100/metrics
   ```

### High Disk Usage

Loki stores logs in Docker volumes. Check usage:

```bash
docker system df -v | grep loki
```

To clean old logs (beyond retention period):
```bash
docker compose -f docker-compose.logging.yml restart loki
```

## Advanced: Creating Dashboards

### 1. Create a Dashboard in Grafana

1. Click **+** → **Dashboard**
2. Click **Add visualization**
3. Select **Loki** as datasource
4. Enter your LogQL query
5. Save the dashboard

### 2. Example: Error Rate Dashboard

Panel query:
```logql
sum(rate({compose_project="weatherust"} |= "ERROR" [5m])) by (service)
```

This shows error rate per service over 5-minute windows.

### 3. Example: Notification Success Rate

```logql
sum(rate({compose_project="weatherust"} |~ "notification.*(success|failure)" [5m])) by (service)
```

## Log Formats

All weatherust services use structured logging with these fields:

```
timestamp LEVEL service: message key1=value1 key2=value2
```

Example:
```
2025-01-09T10:30:15.123Z INFO common::executor: Executing command locally cmd="docker" args=["ps"]
```

You can filter on any field in Grafana.

## Stopping the Logging Stack

```bash
# Stop logging services only
docker compose -f docker-compose.logging.yml down

# Stop everything
docker compose -f docker-compose.yml -f docker-compose.logging.yml down

# Stop and remove volumes (deletes all logs!)
docker compose -f docker-compose.logging.yml down -v
```

## Cost & Performance

- **Storage:** ~100-500MB per day (depends on log volume and verbosity)
- **CPU:** Negligible (< 1% per container)
- **Memory:**
  - Loki: ~100-200MB
  - Promtail: ~50-100MB
  - Grafana: ~100-150MB

Total: ~250-450MB RAM overhead

## Integration with Existing Monitoring

If you already have Prometheus + Grafana:

1. Skip the Grafana service in `docker-compose.logging.yml`
2. Manually add Loki as a datasource in your existing Grafana:
   - URL: `http://loki:3100`
   - Type: Loki

3. Deploy only Loki + Promtail:
   ```bash
   docker compose -f docker-compose.logging.yml up -d loki promtail
   ```

## Next Steps

- Create custom dashboards for your specific use cases
- Set up alerts in Grafana for error patterns
- Explore LogQL query language: https://grafana.com/docs/loki/latest/logql/

---

**Questions?** Check the [Grafana Loki documentation](https://grafana.com/docs/loki/latest/)
