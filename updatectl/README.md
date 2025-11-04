# updatectl - Update Controller for Multi-Server Infrastructure

Apply OS package updates and Docker image updates across multiple servers from a single command.

## What It Does

updatectl is the companion tool to [updatemon](../updatemon) - where updatemon **checks** for updates, updatectl **applies** them.

- **Update OS packages** (apt, dnf, pacman) across remote servers via SSH
- **Pull updated Docker images** automatically
- **Parallel execution** across multiple servers
- **Dry-run mode** to preview changes before applying
- **Gotify notifications** with update results
- **Server name resolution** - use simple names instead of connection strings

## Quick Start

### 1. Configure Servers

Add servers to your `.env` file (shared with updatemon):

```bash
# Same servers used by updatemon
UPDATE_SERVERS=Office-HP-WS:jsprague@192.168.1.189,Cloud VM1:ubuntu@cloud-vm1.js-node.com,Cloud VM2:ubuntu@cloud-vm2.js-node.com
UPDATE_SSH_KEY=/home/ubuntu/.ssh/id_ed25519
UPDATECTL_GOTIFY_KEY=your_updatectl_gotify_token
```

### 2. Start the Service

```bash
docker compose up -d updatectl_runner
```

### 3. Set Up Shell Alias (Recommended!)

Instead of typing the full docker compose command every time, add this to your `~/.bashrc` or `~/.bash_aliases`:

```bash
# updatectl alias - makes CLI usage much easier
alias updatectl='docker compose -f ~/docker-compose/weatherust/docker-compose.yml exec updatectl_runner /app/updatectl'
```

Then reload your shell:

```bash
source ~/.bashrc
```

**Now you can use simple commands:**

```bash
updatectl list servers
updatectl list examples
updatectl os --dry-run --servers "Cloud VM1"
updatectl all --yes --local
```

### Alternative: Wrapper Script

If you prefer a script over an alias:

```bash
# Create ~/bin/updatectl
mkdir -p ~/bin
cat > ~/bin/updatectl << 'EOF'
#!/bin/bash
cd ~/docker-compose/weatherust
docker compose exec updatectl_runner /app/updatectl "$@"
EOF

chmod +x ~/bin/updatectl

# Add ~/bin to PATH in ~/.bashrc if not already there
export PATH="$HOME/bin:$PATH"
```

## Discovery Commands

### List Configured Servers

```bash
updatectl list servers
```

Output:
```
Configured servers (4):

  Cloud VM1 → ubuntu@cloud-vm1.js-node.com
  Cloud VM2 → ubuntu@cloud-vm2.js-node.com
  Office-HP-WS → jsprague@192.168.1.189
  adminjs-n8n-server → jsprague@192.168.3.200

Usage:
  updatectl os --servers "Cloud VM1"
  updatectl all --servers "Cloud VM1,Cloud VM2"
```

### Show Usage Examples

```bash
updatectl list examples
```

Shows common command patterns with real syntax.

## Usage Examples

### Safe Testing with Dry-Run

Always test first to see what would be updated:

```bash
# Preview updates on all servers
updatectl all --dry-run

# Preview updates on specific servers
updatectl os --dry-run --servers "Cloud VM1,Cloud VM2"

# Preview local system
updatectl all --dry-run --local
```

### Update OS Packages

```bash
# Update localhost only
updatectl os --yes --local

# Update specific server
updatectl os --yes --servers "Cloud VM1"

# Update multiple servers
updatectl os --yes --servers "Cloud VM1,Cloud VM2"

# Update all configured servers
updatectl os --yes
```

### Update Docker Images

```bash
# Update all Docker images on localhost
updatectl docker --all --yes --local

# Update specific images
updatectl docker --images nginx:latest,redis:latest --yes --local

# Update all Docker images on specific server
updatectl docker --all --yes --servers "Cloud VM1"
```

### Update Everything

```bash
# Update OS + Docker on localhost
updatectl all --yes --local

# Update OS + Docker on specific server
updatectl all --yes --servers "Cloud VM1"

# Update all servers (OS + Docker)
updatectl all --yes
```

## Server Targeting

| Flag | Behavior |
|------|----------|
| `--local` | Update **localhost only** |
| `--servers "name1,name2"` | Update specific servers by name |
| `--servers "name:user@host"` | Update ad-hoc server (not in UPDATE_SERVERS) |
| *(no flags)* | Update **all servers** from UPDATE_SERVERS |
| `--local --servers "name"` | Update **both** localhost AND named servers |

## Confirmation Prompts

By default, updatectl asks for confirmation before making changes:

```bash
updatectl os --servers "Cloud VM1"
```

Output:
```
This will update the following servers:
  - Cloud VM1 (ubuntu@cloud-vm1.js-node.com)

Operation: OS package updates

Continue? [y/N]
```

### Skip Confirmation (for Automation)

Use `--yes` or `-y` to auto-confirm:

```bash
updatectl os --yes --local
```

This is useful for cron jobs or Ofelia schedules.

## Automated Updates via Ofelia

The docker-compose.yml includes **commented-out** Ofelia schedules for automated updates. These are disabled by default for safety.

To enable, uncomment the desired schedules in `docker-compose.yml`:

```yaml
# Weekly OS updates (Sundays at 04:00)
- "ofelia.job-exec.updatectl-os.schedule=0 0 4 * * 0"
- "ofelia.job-exec.updatectl-os.container=updatectl_runner"
- "ofelia.job-exec.updatectl-os.command=/app/updatectl os --yes --local --quiet"

# Weekly Docker updates (Sundays at 04:30)
- "ofelia.job-exec.updatectl-docker.schedule=0 30 4 * * 0"
- "ofelia.job-exec.updatectl-docker.container=updatectl_runner"
- "ofelia.job-exec.updatectl-docker.command=/app/updatectl docker --all --yes --local --quiet"
```

Then restart Ofelia:

```bash
docker compose restart ofelia
```

## Configuration

### Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `UPDATE_SERVERS` | No* | Comma-separated server list (format: `name:user@host`) |
| `UPDATE_SSH_KEY` | No | Path to SSH private key for passwordless auth |
| `UPDATECTL_GOTIFY_KEY` | Yes | Gotify API token for notifications |
| `GOTIFY_URL` | Yes | Gotify server URL |

\* Required if not using `--local` or `--servers` with connection strings

### SSH Setup

Ensure passwordless SSH authentication is configured:

```bash
# Generate SSH key if needed
ssh-keygen -t ed25519 -f ~/.ssh/id_ed25519

# Copy to remote servers
ssh-copy-id -i ~/.ssh/id_ed25519.pub user@remote-server
```

Set `UPDATE_SSH_KEY` to the private key path in `.env`.

## Safety Features

### Dry-Run Mode

Always preview changes first:

```bash
updatectl all --dry-run --local
```

Shows what would be updated without making any changes.

### Confirmation Prompts

Interactive confirmation required unless `--yes` is used.

### Error Isolation

If one server fails, others continue updating. Failed servers are reported in the final summary.

### Gotify Notifications

All update results (success or failure) are sent to Gotify for audit trail.

## Package Managers Supported

- **APT** (Debian, Ubuntu) - `apt-get update && apt-get upgrade`
- **DNF** (Fedora, RHEL 8+) - `dnf upgrade`
- **Pacman** (Arch Linux) - `pacman -Syu`

Package manager is auto-detected on each server.

## Docker Requirements

updatectl needs read-write access to Docker socket:

```yaml
volumes:
  - /var/run/docker.sock:/var/run/docker.sock:rw
```

This allows it to:
- List current images
- Pull updated images
- Detect available updates

## Comparison with updatemon

| Feature | updatemon | updatectl |
|---------|-----------|-----------|
| Purpose | **Check** for updates | **Apply** updates |
| Docker socket | Read-only | Read-write |
| Safe to automate | ✅ Yes | ⚠️  Use caution |
| Gotify key | `UPDATEMON_GOTIFY_KEY` | `UPDATECTL_GOTIFY_KEY` |
| Typical schedule | Daily (3am) | Weekly/manual |

**Recommended workflow:**
1. updatemon runs daily to check for updates (automated)
2. You review updatemon notifications
3. Run updatectl manually or on a conservative schedule (weekly/monthly)

## Troubleshooting

### "Unknown server" error

```bash
updatectl list servers
```

Verify the server name matches UPDATE_SERVERS exactly (case-sensitive).

### "No supported package manager found"

- Ensure server names resolve correctly
- Check SSH connectivity: `ssh user@host which apt`
- Verify UPDATE_SSH_KEY is correct

### "Permission denied" on Docker operations

- Ensure user is in `docker` group on remote servers
- Or use sudo in commands (updatectl runs docker with sudo when needed)

### Command not found (without alias)

Full command:
```bash
docker compose -f ~/docker-compose/weatherust/docker-compose.yml exec updatectl_runner /app/updatectl list servers
```

Or set up the shell alias (see Quick Start section).

## Examples Output

### Dry-Run Example

```bash
$ updatectl all --dry-run --local
DRY-RUN MODE - No changes will be made

Updating localhost...

[DRY-RUN] 🖥️  localhost (local)
   OS Updates: No updates available
   Docker Updates: 16 images would be updated
```

### Actual Update Example

```bash
$ updatectl os --yes --servers "Cloud VM1"
Updating Cloud VM1...

🖥️  Cloud VM1 (ubuntu@cloud-vm1.js-node.com)
   OS Updates: ✅ 52 packages upgraded
```

Gotify notification sent with full details.

## See Also

- [updatemon](../updatemon) - Multi-server update monitoring (companion tool)
- [dockermon](../dockermon) - Docker container health monitoring
- [weatherust](../) - Weather notifications
- [speedynotify](../speedynotify) - Internet speed test monitoring
