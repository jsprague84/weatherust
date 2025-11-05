# updatemon - Update Monitor for Multi-Server Infrastructure

Monitor OS package updates and Docker image updates across multiple servers from a single command.

## What It Does

updatemon checks for available updates across your infrastructure and sends Gotify notifications with the results.

- **Monitor OS packages** (apt, dnf, pacman) across remote servers via SSH
- **Check Docker images** for newer versions in registries
- **Parallel execution** across multiple servers
- **Gotify notifications** with update summaries
- **Safe read-only** operations - never modifies anything

## Quick Start

### 1. Configure Servers

Add servers to your `.env` file:

```bash
# Comma-separated server list
UPDATE_SERVERS=Office-HP-WS:jsprague@192.168.1.189,Cloud VM1:ubuntu@cloud-vm1.js-node.com,Cloud VM2:ubuntu@cloud-vm2.js-node.com

# SSH key for passwordless authentication
UPDATE_SSH_KEY=/home/ubuntu/.ssh/id_ed25519

# Gotify notification token
UPDATEMON_GOTIFY_KEY=your_updatemon_gotify_token
```

### 2. Start the Service

```bash
docker compose up -d updatemon_runner
```

### 3. Run Manually

```bash
# Check all configured servers
docker compose exec updatemon_runner /app/updatemon --docker --quiet

# Check localhost only
docker compose exec updatemon_runner /app/updatemon --local --docker --quiet

# Check specific servers (by name from UPDATE_SERVERS)
docker compose exec updatemon_runner /app/updatemon --servers "Cloud VM1,Cloud VM2" --docker --quiet
```

### 4. Set Up Shell Alias (Optional)

Add to your `~/.bashrc` or `~/.bash_aliases`:

```bash
# updatemon alias
alias updatemon='docker compose -f ~/docker-compose/weatherust/docker-compose.yml exec updatemon_runner /app/updatemon'
```

Then reload:

```bash
source ~/.bashrc
```

**Now you can use:**

```bash
updatemon --local --docker
updatemon --servers "Cloud VM1" --docker
```

## Usage

### Check All Configured Servers

```bash
updatemon --docker
```

Checks all servers from `UPDATE_SERVERS` environment variable.

### Check Localhost Only

```bash
updatemon --local --docker
```

### Check Specific Servers

```bash
updatemon --servers "Cloud VM1,Cloud VM2" --docker
```

### Check Local + Remote

```bash
updatemon --local --servers "Cloud VM1" --docker
```

### OS Updates Only (Skip Docker)

```bash
updatemon --local
```

By default, `--docker` is enabled. To check only OS packages, omit the flag.

## Command-Line Options

| Flag | Description |
|------|-------------|
| `--local` | Include localhost in the check |
| `--servers <list>` | Comma-separated server list (overrides UPDATE_SERVERS) |
| `--docker` | Check Docker images for updates (default: true) |
| `--ssh-key <path>` | SSH key path (overrides UPDATE_SSH_KEY) |
| `--quiet` | Suppress stdout output (Gotify only) |

## Example Output

```bash
$ updatemon --local --docker
Checking localhost...

🖥️  localhost (local)
   Package Manager: APT (Debian/Ubuntu)
   OS: ✅ Up to date
   Docker: 🐳 7 of 12 images with updates
      - ghcr.io/jsprague84/updatemon:latest (update available)
      - nginx:latest (update available)
      - redis:latest (update available)
      - portainer/portainer-ce:latest (update available)
      - postgres:16 (update available)
      ... and 2 more with updates
```

Gotify notification sent with summary: **"📦 Updates available (1 server)"**

## Automated Monitoring via Ofelia

updatemon is configured to run daily at 3:00 AM in `docker-compose.yml`:

```yaml
- "ofelia.job-exec.updatemon.schedule=0 0 3 * * *"
- "ofelia.job-exec.updatemon.container=updatemon_runner"
- "ofelia.job-exec.updatemon.command=/app/updatemon --local --docker --quiet"
```

This sends daily Gotify notifications about available updates.

## Configuration

### Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `UPDATE_SERVERS` | No* | Comma-separated server list (format: `name:user@host`) |
| `UPDATE_SSH_KEY` | No | Path to SSH private key for passwordless auth |
| `UPDATEMON_GOTIFY_KEY` | Yes | Gotify API token for notifications |
| `GOTIFY_URL` | Yes | Gotify server URL |

\* Required if not using `--local` or `--servers` flag

### Server Format

Three formats supported:

1. **With name:** `servername:user@host`
   - Example: `Cloud VM1:ubuntu@cloud-vm1.js-node.com`

2. **Without name:** `user@host`
   - Example: `ubuntu@192.168.1.10`
   - Name derived from hostname

3. **Localhost:** `name:local` or `name:localhost` **(NEW)**
   - Example: `docker-vm:local`
   - Creates a localhost entry with custom name
   - Eliminates need for `--local` flag
   - Automatically included in scheduled runs

### Localhost Customization

You can customize how localhost appears in reports using environment variables:

```bash
# In .env
UPDATE_LOCAL_NAME=docker-vm           # Custom name instead of "localhost"
UPDATE_LOCAL_DISPLAY=192.168.1.100    # IP/hostname instead of "local"
```

**Three ways to include localhost:**

1. **Use --local flag:**
   ```bash
   updatemon --local --docker
   ```
   Shows: `🖥️  localhost (local)`

2. **Add to UPDATE_SERVERS:**
   ```bash
   UPDATE_SERVERS=docker-vm:local,Cloud VM1:ubuntu@cloud
   ```
   Shows: `🖥️  docker-vm (local)`

3. **Add to UPDATE_SERVERS with custom display:**
   ```bash
   UPDATE_SERVERS=docker-vm:local,Cloud VM1:ubuntu@cloud
   UPDATE_LOCAL_NAME=docker-vm
   UPDATE_LOCAL_DISPLAY=192.168.1.100
   ```
   Shows: `🖥️  docker-vm (192.168.1.100)`

### SSH Setup

Ensure passwordless SSH authentication:

```bash
# Generate SSH key if needed
ssh-keygen -t ed25519 -f ~/.ssh/id_ed25519

# Copy to remote servers
ssh-copy-id -i ~/.ssh/id_ed25519.pub user@remote-server

# Test connection
ssh -i ~/.ssh/id_ed25519 user@remote-server whoami
```

Set `UPDATE_SSH_KEY` in `.env` to the private key path.

## How Docker Update Detection Works

updatemon compares local image digests with remote registry digests:

1. **Local digest:** Extracted from `docker inspect` (RepoDigests field)
2. **Remote digest:** Fetched via `docker manifest inspect` (queries registry)
3. **Comparison:** If digests differ, update is available

This method:
- ✅ Works without pulling images (fast)
- ✅ Detects any changes to image layers
- ✅ Handles multi-arch images correctly
- ⚠️  Requires registry access (may hit rate limits on Docker Hub)

### Rate Limiting

Docker Hub limits manifest queries. If you hit limits, updatemon will:
- Log warnings for affected images
- Assume "no update" to avoid false positives
- Continue checking other images

Private registries without authentication will also show "no update" safely.

## Package Managers Supported

- **APT** (Debian, Ubuntu) - `apt-get update && apt list --upgradable`
- **DNF** (Fedora, RHEL 8+) - `dnf check-update`
- **Pacman** (Arch Linux) - `pacman -Qu`

Package manager is auto-detected on each server.

## Gotify Notifications

### When No Updates Available

```
✅ All systems up to date (4 servers)
```

### When Updates Available

```
📦 Updates available (4 servers)

🖥️  Cloud VM1 (ubuntu@cloud-vm1.js-node.com)
   Package Manager: APT (Debian/Ubuntu)
   OS: 📦 52 updates available
      - base-files
      - coreutils
      - containerd.io
      - docker-ce
      - nginx
      ... and 47 more
   Docker: 🐳 10 of 10 images with updates
      - nginx:latest (update available)
      - redis:latest (update available)
      - postgres:16 (update available)
      - portainer/portainer-ce:latest (update available)
      - mariadb:latest (update available)
      ... and 5 more with updates
```

## Comparison with updatectl

| Feature | updatemon | updatectl |
|---------|-----------|-----------|
| Purpose | **Check** for updates | **Apply** updates |
| Docker socket | Read-only | Read-write |
| Modifies system | ❌ Never | ✅ Yes |
| Safe to automate | ✅ Yes | ⚠️  Use caution |
| Gotify key | `UPDATEMON_GOTIFY_KEY` | `UPDATECTL_GOTIFY_KEY` |
| Typical schedule | Daily | Weekly/manual |

**Recommended workflow:**
1. updatemon runs daily to check for updates (automated)
2. You review updatemon Gotify notifications
3. Run [updatectl](../updatectl) manually or on a conservative schedule to apply updates

## Troubleshooting

### "No supported package manager found"

- Check SSH connectivity: `ssh user@host which apt`
- Verify UPDATE_SSH_KEY is correct
- Ensure server format is correct in UPDATE_SERVERS

### Docker image checks showing "No RepoDigest found"

- Locally built images don't have RepoDigests (expected)
- Images tagged as `<none>` are automatically skipped

### "Could not fetch remote manifest"

- Registry rate limiting (Docker Hub) - expected for some images
- Private registry without auth - expected, returns "no update" safely
- Network issues - temporary, will work on next run

### SSH connection failures

```bash
# Test SSH manually
ssh -i ~/.ssh/id_ed25519 user@host whoami

# Check key permissions
chmod 600 ~/.ssh/id_ed25519

# Verify key is mounted in container
docker compose exec updatemon_runner ls -la /root/.ssh/
```

## Debug Mode

Enable debug logging to see detailed digest comparisons:

```bash
docker compose exec updatemon_runner sh -c "RUST_LOG=debug /app/updatemon --local --docker"
```

Shows:
```
[DEBUG] Local digest for nginx:latest is sha256:abc123...
[DEBUG] Remote digest for nginx:latest is sha256:def456...
[DEBUG] Comparing: local='sha256:abc123...' vs remote='sha256:def456...'
```

## Security Considerations

### Read-Only Operations

updatemon never modifies your systems:
- Docker socket mounted as `:ro` (read-only)
- SSH commands only read package manager state
- No update installation commands are run

### SSH Key Security

- Private key mounted read-only in container
- Key should have restricted permissions (600)
- Use dedicated monitoring key with limited access if possible

### Registry Authentication

updatemon doesn't authenticate to registries by default. This means:
- ✅ No credentials needed
- ⚠️  Rate limiting on Docker Hub may occur
- ⚠️  Private images won't be checked (returns "no update" safely)

## See Also

- [updatectl](../updatectl) - Multi-server update controller (companion tool to apply updates)
- [dockermon](../dockermon) - Docker container health monitoring
- [weatherust](../) - Weather notifications
- [speedynotify](../speedynotify) - Internet speed test monitoring
