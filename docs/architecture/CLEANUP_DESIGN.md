# Docker Cleanup Feature Design

> **Note:** This document describes the Docker cleanup feature that was originally planned for `dockermon` but was later moved to `updatectl` as part of an architectural refactoring. As of the refactoring:
> - `healthmon` (formerly `dockermon`) - Health monitoring only (read-only)
> - `updatectl` - All system modifications including Docker cleanup, OS cleanup, OS updates, Docker updates

## Overview
Docker cleanup capabilities in `updatectl` identify and optionally remove unused Docker resources (images, networks, build cache, old containers). **Volume deletion is explicitly excluded** - volumes will only be reported for informational purposes.

---

## Architecture

### Module Structure (Implemented in updatectl)
```
updatectl/
├── src/
│   ├── main.rs              // CLI entry point with all subcommands
│   ├── cleanup/             // Docker cleanup module
│   │   ├── mod.rs           // Public API and orchestration
│   │   ├── types.rs         // Shared data structures
│   │   ├── images.rs        // Image analysis and pruning
│   │   ├── networks.rs      // Network analysis and pruning
│   │   ├── containers.rs    // Stopped container analysis
│   │   ├── build_cache.rs   // Build cache analysis
│   │   ├── layers.rs        // Image layer sharing analysis
│   │   ├── logs.rs          // Log size analysis (read-only)
│   │   ├── volumes.rs       // Volume reporting (read-only, no deletion)
│   │   └── profiles.rs      // Cleanup profiles (conservative/moderate/aggressive)
│   └── remote_cleanup.rs    // Remote cleanup via SSH
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

## CLI Interface (As Implemented in updatectl)

### Health Monitoring (healthmon)
```bash
# Health check only - no cleanup functionality
healthmon health
healthmon health --quiet
healthmon health --cpu-warn-pct 90
```

### Docker Cleanup Commands (updatectl)
```bash
# Full cleanup report (analysis only, default)
updatectl clean-docker --local
updatectl clean-docker --servers "server1,server2"

# Execute cleanup with profiles
updatectl clean-docker --local --execute --profile conservative  # Safe: dangling images + unused networks
updatectl clean-docker --local --execute --profile moderate      # + build cache
updatectl clean-docker --local --execute --profile aggressive    # + old stopped containers (30 days)

# Output control
updatectl clean-docker --local --quiet           # Send to notifications only
```

### CLI Arguments (As Implemented)
```rust
// healthmon (health monitoring only)
#[derive(Parser, Debug)]
#[command(name = "healthmon")]
enum Commands {
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
}

// updatectl (includes cleanup + updates)
#[derive(Parser, Debug)]
#[command(name = "updatectl")]
enum Commands {
    /// Update OS packages
    Os { /* ... */ },

    /// Update Docker images
    Docker { /* ... */ },

    /// Update both OS and Docker
    All { /* ... */ },

    /// Clean Docker resources (images, networks, containers, build cache)
    CleanDocker {
        /// Cleanup profile: conservative (default), moderate, or aggressive
        #[arg(long, default_value = "conservative")]
        profile: String,

        /// Actually execute cleanup (default is analysis only)
        #[arg(long)]
        execute: bool,

        // ... server targeting flags (--local, --servers)
    },

    /// Clean OS resources (package cache, old kernels, etc.)
    CleanOs {
        /// Clean package manager cache (apt clean, dnf clean)
        #[arg(long)]
        cache: bool,

        /// Remove unused packages (apt autoremove, dnf autoremove)
        #[arg(long)]
        autoremove: bool,

        /// Clean all (cache + autoremove)
        #[arg(long)]
        all: bool,

        /// Actually execute cleanup (default is analysis only)
        #[arg(long)]
        execute: bool,

        // ... server targeting flags
    },
}
```

---

## Notification Format (As Implemented)

Cleanup notifications are sent via the standard updatectl notification system:

### Gotify Notification (cleanup report)
```
Title: 🧹 Docker Cleanup Report - docker-vm

Message:
📊 Analysis Results:

Dangling Images:
  - Count: 3 images
  - Size: 523MB
  - Status: ✅ Safe to remove

Unused Networks:
  - Count: 12 networks
  - Status: ✅ Safe to remove

Build Cache:
  - Size: 1.2GB
  - Status: ⚠️  Moderate profile

Stopped Containers:
  - Count: 5 containers (>30 days old)
  - Status: ⚠️  Aggressive profile only

Total Reclaimable:
  - Conservative: 523MB (dangling images + unused networks)
  - Moderate: 1.7GB (+ build cache)
  - Aggressive: 1.75GB (+ old containers)

To execute cleanup:
  updatectl clean-docker --local --execute --profile conservative
```

### Gotify Notification (cleanup execution results)
```
Title: ✅ Docker Cleanup Complete - docker-vm

Message:
Profile: Conservative
Removed: 28 items
Reclaimed: 541MB

Details:
  - Dangling images pruned: 3 (523MB)
  - Unused networks removed: 12
  - Build cache cleared: 0MB (not included in conservative profile)
```

---

## Webhook Actions (Planned - Not Yet Implemented)

> **Note:** Webhook integration for cleanup actions is planned but not yet implemented. Currently cleanup must be triggered manually via CLI or Ofelia schedules.

### Future: Safe Cleanup Endpoint
```
POST /webhook/cleanup/safe?server=<name>&token=<secret>

Actions (would use conservative profile):
  - Prune dangling images
  - Prune unused networks
  - Do NOT touch volumes
  - Do NOT touch build cache
  - Do NOT touch stopped containers

Response: 202 Accepted
Background task sends completion notification with results
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

## Configuration (As Implemented)

### Environment Variables
Cleanup functionality uses the same server configuration as other updatectl commands:
```bash
# Server list for remote cleanup
UPDATE_SERVERS=docker-vm:local,Cloud VM1:ubuntu@cloud-vm1.js-node.com
UPDATE_SSH_KEY=/home/ubuntu/.ssh/id_ed25519

# Notification keys
UPDATECTL_GOTIFY_KEY=your_updatectl_token
UPDATECTL_NTFY_TOPIC=update-actions
```

### Ofelia Schedules (docker-compose.yml)
```yaml
# Weekly cleanup report (Sundays at 2:00 AM) - analysis only
- "ofelia.job-exec.docker-cleanup-report.schedule=0 0 2 * * 0"
- "ofelia.job-exec.docker-cleanup-report.container=updatectl_runner"
- "ofelia.job-exec.docker-cleanup-report.command=/app/updatectl clean-docker --local --quiet"

# OPTIONAL: Monthly safe auto-cleanup (DISABLED by default)
# Uncomment to enable automated cleanup with conservative profile
# - "ofelia.job-exec.docker-cleanup-safe.schedule=0 30 2 1 * *"
# - "ofelia.job-exec.docker-cleanup-safe.container=updatectl_runner"
# - "ofelia.job-exec.docker-cleanup-safe.command=/app/updatectl clean-docker --local --quiet --execute --profile conservative"
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

## Example Workflows (As Implemented)

### Weekly Automated Report
```yaml
# Ofelia schedule in docker-compose.yml (enabled by default)
ofelia.job-exec.docker-cleanup-report.schedule=0 0 2 * * 0
ofelia.job-exec.docker-cleanup-report.container=updatectl_runner
ofelia.job-exec.docker-cleanup-report.command=/app/updatectl clean-docker --local --quiet
```

User receives Gotify notification with analysis results and instructions for manual execution.

### Monthly Automated Safe Cleanup (Optional - Disabled by Default)
```yaml
# Uncomment in docker-compose.yml to enable
# ofelia.job-exec.docker-cleanup-safe.schedule=0 30 2 1 * *
# ofelia.job-exec.docker-cleanup-safe.container=updatectl_runner
# ofelia.job-exec.docker-cleanup-safe.command=/app/updatectl clean-docker --local --quiet --execute --profile conservative
```

User receives:
1. Completion notification: "✅ Docker Cleanup Complete - docker-vm"
2. Summary of items removed and space reclaimed

### Manual Cleanup Execution
```bash
# Analysis only (safe, default)
updatectl clean-docker --local

# Execute with conservative profile (safest)
updatectl clean-docker --local --execute --profile conservative

# Execute with moderate profile
updatectl clean-docker --local --execute --profile moderate

# Execute with aggressive profile
updatectl clean-docker --local --execute --profile aggressive

# Remote server cleanup
updatectl clean-docker --servers "Cloud VM1" --execute --profile conservative
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

## Migration Path (Completed)

### Architectural Refactoring
The cleanup feature was moved from `dockermon` to `updatectl` during a major refactoring:

**Before:**
- `dockermon` - Health monitoring + cleanup

**After:**
- `healthmon` (renamed) - Health monitoring only (read-only)
- `updatectl` - All system modifications (OS updates, Docker updates, Docker cleanup, OS cleanup)

### Migration Steps Completed
1. ✅ Renamed `dockermon` to `healthmon`
2. ✅ Moved all cleanup code from healthmon to updatectl
3. ✅ Added OS maintenance commands to updatectl
4. ✅ Updated docker-compose.yml Ofelia jobs
5. ✅ Updated notification functions in common library
6. ✅ Updated environment variable names (`DOCKERMON_*` → `HEALTHMON_*`)
7. ✅ Rebuilt and published all Docker images

### Deployment
1. Update to new images (healthmon, updatectl)
2. Update .env file with new variable names
3. Cleanup schedules now use updatectl instead of healthmon

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
