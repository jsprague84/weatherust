# Docker Cleanup Feature Design

## Overview
Add cleanup reporting and execution capabilities to `dockermon` to identify and optionally remove unused Docker resources (images, networks, logs). **Volume deletion is explicitly excluded** - volumes will only be reported for informational purposes.

---

## Architecture

### Module Structure
```
dockermon/
├── src/
│   ├── main.rs              // CLI entry point with subcommands
│   ├── health.rs            // Existing health check logic (extracted)
│   └── cleanup/
│       ├── mod.rs           // Public API and orchestration
│       ├── types.rs         // Shared data structures
│       ├── images.rs        // Image analysis and pruning
│       ├── networks.rs      // Network analysis and pruning
│       ├── logs.rs          // Log size analysis and recommendations
│       └── volumes.rs       // Volume reporting (read-only, no deletion)
```

### Separation of Concerns
- **Analysis** - Read-only, gather information
- **Reporting** - Format results for notifications
- **Execution** - Actually delete resources (with safety checks)

---

## Data Structures

### CleanupReport (per-server summary)
```rust
pub struct CleanupReport {
    pub server: String,              // "docker-vm" or "local"
    pub dangling_images: ImageStats,
    pub unused_images: ImageStats,
    pub unused_networks: NetworkStats,
    pub large_logs: LogStats,
    pub volumes: VolumeStats,
    pub total_reclaimable_bytes: u64,
}

pub struct ImageStats {
    pub count: usize,
    pub total_size_bytes: u64,
    pub items: Vec<ImageInfo>,
}

pub struct ImageInfo {
    pub repository: String,
    pub tag: String,
    pub image_id: String,
    pub size_bytes: u64,
    pub created: DateTime<Utc>,
}

pub struct NetworkStats {
    pub count: usize,
    pub items: Vec<NetworkInfo>,
}

pub struct NetworkInfo {
    pub id: String,
    pub name: String,
    pub driver: String,
    pub created: DateTime<Utc>,
}

pub struct LogStats {
    pub total_size_bytes: u64,
    pub containers_over_threshold: usize,
    pub items: Vec<LogInfo>,
}

pub struct LogInfo {
    pub container_name: String,
    pub container_id: String,
    pub log_size_bytes: u64,
    pub has_rotation: bool,
}

pub struct VolumeStats {
    pub count: usize,
    pub total_size_bytes: u64,
    pub items: Vec<VolumeInfo>,
}

pub struct VolumeInfo {
    pub name: String,
    pub driver: String,
    pub mount_point: String,
    pub size_bytes: u64,
    pub created: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
    pub containers_using: Vec<String>,
}
```

---

## CLI Interface

### Backward Compatible
```bash
# Existing health check (no changes)
dockermon
dockermon --quiet
dockermon --cpu-warn-pct 90
```

### New Cleanup Commands
```bash
# Full cleanup report (all categories)
dockermon cleanup

# Category-specific reports
dockermon cleanup --images
dockermon cleanup --networks
dockermon cleanup --logs
dockermon cleanup --volumes

# Execution modes
dockermon cleanup --report          # Report only (default, read-only)
dockermon cleanup --safe            # Execute safe operations (dangling images, unused networks)
dockermon cleanup --interactive     # Prompt before each action
dockermon cleanup --dry-run         # Show what would be deleted

# Output control
dockermon cleanup --quiet           # Send to notifications only
dockermon cleanup --json            # JSON output for scripting
```

### CLI Arguments
```rust
#[derive(Parser, Debug)]
#[command(name = "dockermon")]
enum Commands {
    /// Check container health and resource usage (default)
    Health {
        #[arg(long)]
        quiet: bool,
        #[arg(long)]
        cpu_warn_pct: Option<f64>,
        #[arg(long)]
        mem_warn_pct: Option<f64>,
        #[arg(long)]
        notify_always: bool,
        #[arg(long, value_delimiter = ',')]
        ignore: Vec<String>,
    },

    /// Analyze and clean up Docker resources
    Cleanup {
        /// Show report without taking action
        #[arg(long, default_value_t = true)]
        report: bool,

        /// Execute safe cleanup operations automatically
        #[arg(long)]
        safe: bool,

        /// Prompt before each action
        #[arg(long)]
        interactive: bool,

        /// Show what would be deleted without doing it
        #[arg(long)]
        dry_run: bool,

        /// Only analyze images
        #[arg(long)]
        images: bool,

        /// Only analyze networks
        #[arg(long)]
        networks: bool,

        /// Only analyze logs
        #[arg(long)]
        logs: bool,

        /// Only analyze volumes
        #[arg(long)]
        volumes: bool,

        /// Suppress stdout output
        #[arg(long)]
        quiet: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}
```

---

## Notification Format

### Per-Server ntfy Notification
```
Title: docker-vm - Cleanup Report
Tags: house, warning (if issues), or checkmark (if clean)

Message:
🧹 Docker Cleanup Report

📦 Dangling Images
   Count: 3 images
   Size: 523MB
   Status: Safe to remove

🖼️  Unused Images
   Count: 5 images (nginx:1.20, redis:6.0, postgres:13, ...)
   Size: 561MB
   Last Used: 3+ months ago
   Status: Review recommended

🔗 Unused Networks
   Count: 12 networks
   Status: Safe to remove

📝 Large Container Logs
   Total Size: 3.2GB
   Containers Over 1GB: 2
   - postgres_main: 1.8GB
   - app_backend: 1.5GB

   Recommendation: Add to docker-compose.yml:
   logging:
     driver: json-file
     options:
       max-size: "50m"
       max-file: "3"

💾 Volumes (informational)
   Count: 15 volumes
   Total Size: 45GB
   Largest:
   - postgres_data: 12GB (in use)
   - old_backup_vol: 8GB (unused 180+ days) ⚠️
   - redis_data: 5GB (in use)

✅ Total Reclaimable Space: ~1.08GB
   (Dangling images + unused networks)

Action Buttons:
[Prune Safe Items] [View Details]
```

### Summary Gotify Notification (All Servers)
```
Title: 🧹 Docker Cleanup Summary (3 servers)

Message:
Overview:
  docker-vm: 1.08GB reclaimable, 3.2GB logs need rotation
  Cloud VM1: 450MB reclaimable, 890MB logs need rotation
  Cloud VM2: 2.3GB reclaimable, 5.1GB logs need rotation

Totals Across All Servers:
  Dangling Images: 1.2GB (8 images)
  Unused Images: 2.58GB (14 images)
  Unused Networks: 31 networks
  Large Logs: 9.19GB (7 containers without rotation)
  Volumes: 47 volumes (142GB) - manual review needed

Total Reclaimable: 3.78GB

Next Steps:
  - Review per-server details in ntfy
  - Run cleanup with --safe flag
  - Configure log rotation for large containers
  - Audit unused volumes manually
```

---

## Webhook Actions

### Safe Cleanup Endpoint
```
POST /webhook/cleanup/safe?server=<name>&token=<secret>

Actions:
  - Prune dangling images
  - Prune unused networks
  - Do NOT touch volumes
  - Do NOT touch unused images (require explicit confirmation)

Response: 202 Accepted
Background task sends completion notification with results
```

### ~~Configure Log Rotation Endpoint~~ (REMOVED)
Log rotation configuration is manual only. The cleanup report provides
docker-compose.yml snippets as recommendations, but no automatic configuration
or webhooks are provided for log rotation.

### Prune Unused Images Endpoint (Dangerous)
```
POST /webhook/cleanup/images/prune-unused?server=<name>&token=<secret>

Actions:
  - Prune images with no containers using them
  - Exclude images from last 7 days
  - Requires explicit webhook call (not in "safe" category)

Response: 202 Accepted
```

---

## Safety Considerations

### Always Safe (Auto-executable with --safe flag)
✅ **Dangling images** - `docker image prune`
✅ **Unused networks** - `docker network prune`

### Requires Confirmation (Interactive or explicit webhook)
⚠️ **Unused images** - Might be needed later, could be rollback targets
⚠️ **Log rotation** - Requires container restart

### Never Auto-Execute (Report Only)
🚫 **Volumes** - Too dangerous, manual deletion only
🚫 **Running containers** - Even if using old images

### Size Thresholds
```bash
# Environment variables
DOCKERMON_CLEANUP_IMAGE_SIZE_WARN=500M     # Warn if unused images exceed this
DOCKERMON_CLEANUP_LOG_SIZE_WARN=1G         # Warn if container log exceeds this
DOCKERMON_CLEANUP_LOG_SIZE_CONTAINER=100M  # Per-container log threshold
```

---

## Configuration

### Environment Variables
```bash
# Cleanup thresholds
DOCKERMON_CLEANUP_IMAGE_SIZE_WARN=500M
DOCKERMON_CLEANUP_LOG_SIZE_WARN=1G
DOCKERMON_CLEANUP_LOG_SIZE_CONTAINER=100M
DOCKERMON_CLEANUP_IMAGE_AGE_DAYS=90        # Unused images older than this flagged

# Auto-cleanup schedule (via Ofelia)
# Run cleanup report weekly (Sundays at 2 AM)
ofelia.job-exec.dockermon-cleanup.schedule=0 0 2 * * 0
ofelia.job-exec.dockermon-cleanup.container=dockermon_runner
ofelia.job-exec.dockermon-cleanup.command=/app/dockermon cleanup --quiet

# Safe auto-cleanup monthly (first of month at 3 AM)
# ofelia.job-exec.dockermon-cleanup-safe.schedule=0 0 3 1 * *
# ofelia.job-exec.dockermon-cleanup-safe.container=dockermon_runner
# ofelia.job-exec.dockermon-cleanup-safe.command=/app/dockermon cleanup --safe --quiet
```

---

## Implementation Phases

### Phase 1: Analysis & Reporting (Read-Only)
- ✅ Module structure
- ✅ Image analysis (dangling + unused)
- ✅ Network analysis
- ✅ Log size analysis
- ✅ Volume reporting (no deletion)
- ✅ Per-server ntfy notifications
- ✅ Summary Gotify notification
- ✅ CLI with --report mode

### Phase 2: Safe Execution
- ✅ Prune dangling images
- ✅ Prune unused networks
- ✅ --safe and --dry-run modes
- ✅ Webhook endpoint for safe cleanup
- ✅ Completion notifications

### Phase 3: Advanced Features
- ✅ Interactive mode (prompt before actions)
- ✅ Prune unused images (with confirmation)
- ✅ Webhook for pruning unused images
- ✅ JSON output for scripting

### Phase 4: Polish
- ✅ Error handling for all edge cases
- ✅ Comprehensive logging
- ✅ Unit tests
- ✅ Documentation in README
- ✅ Update .env.example with new settings

---

## Example Workflows

### Weekly Automated Report
```yaml
# Ofelia schedule in docker-compose.yml
ofelia.job-exec.dockermon-cleanup.schedule=0 0 2 * * 0
ofelia.job-exec.dockermon-cleanup.container=dockermon_runner
ofelia.job-exec.dockermon-cleanup.command=/app/dockermon cleanup --quiet
```

User receives:
1. Per-server ntfy notifications with details
2. Summary Gotify with all servers
3. Action buttons to execute safe cleanup

### Monthly Automated Safe Cleanup
```yaml
# Run safe cleanup on first of month
ofelia.job-exec.dockermon-safe-cleanup.schedule=0 0 3 1 * *
ofelia.job-exec.dockermon-safe-cleanup.container=dockermon_runner
ofelia.job-exec.dockermon-safe-cleanup.command=/app/dockermon cleanup --safe --quiet
```

User receives:
1. Completion notification: "Cleaned 1.2GB on docker-vm"
2. Summary of what was removed

### Manual Interactive Cleanup
```bash
# SSH into server
docker compose exec dockermon_runner /app/dockermon cleanup --interactive

# Prompts for each action:
# "Remove 3 dangling images (523MB)? [y/N]"
# "Remove 12 unused networks? [y/N]"
# "Remove unused image nginx:1.20 (142MB)? [y/N]"
```

---

## Testing Plan

### Unit Tests
- Image categorization (dangling vs unused vs in-use)
- Network detection (unused vs in-use)
- Log size calculation
- Volume info gathering
- Size formatting (bytes → human readable)

### Integration Tests
- Connect to test Docker daemon
- Create test containers/images/networks
- Run cleanup analysis
- Verify correct categorization
- Test pruning operations

### Safety Tests
- Verify volumes are NEVER deleted
- Verify in-use images are NEVER deleted
- Verify running containers are NEVER affected
- Test dry-run mode doesn't modify anything

---

## Migration Path

### Backward Compatibility
Existing `dockermon` usage remains unchanged:
```bash
dockermon              # Still runs health check
dockermon --quiet      # Still works as before
```

New cleanup feature is opt-in via subcommand:
```bash
dockermon cleanup      # New cleanup functionality
```

### Deployment
1. Update `dockermon` binary (backward compatible)
2. Optionally add cleanup schedule to docker-compose.yml
3. Optionally add cleanup webhook endpoints
4. No config changes required for existing health checks

---

## Decisions Made

1. **Log Rotation Implementation**: ✅ **Report only + docker-compose.yml example**
   - Show containers with large logs
   - Provide basic docker-compose.yml snippet
   - User applies manually if needed
   - No automation, no webhooks for logs

2. **Image Age Threshold**: ✅ **Configurable via env var, default 90 days**
   - `DOCKERMON_CLEANUP_IMAGE_AGE_DAYS=90`

3. **Webhook Integration**: ✅ **Part of updatectl webhook server**
   - Consistent with update buttons
   - Single webhook server for all actions
   - New endpoints: `/webhook/cleanup/safe`, `/webhook/cleanup/images/prune-unused`

4. **Volume Reporting Detail Level**: ✅ **Top 10 largest + count**
   - Balanced approach - useful without overwhelming
   - Shows most important volumes
   - Total count for awareness

5. **Notification Frequency**: ✅ **Only if reclaimable space > threshold**
   - Default threshold: 100MB
   - Configurable: `DOCKERMON_CLEANUP_NOTIFY_THRESHOLD=100M`
   - Reduces noise for servers with nothing to clean

---

## Summary

This design adds comprehensive Docker cleanup capabilities to `dockermon` while maintaining strict safety guardrails:

**Safe to automate:**
- Dangling image pruning
- Unused network pruning

**Requires confirmation:**
- Unused image removal
- Log rotation configuration

**Never automated:**
- Volume deletion (report only)
- Any operation affecting running containers

The per-server notification strategy matches `updatemon` patterns and provides actionable information with webhook buttons for safe operations.
