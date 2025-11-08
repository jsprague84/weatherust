# Bash Aliases for weatherust Services

Easy-to-use shell aliases for all weatherust services. Add these to your `~/.bashrc`, `~/.bash_aliases`, or `~/.zshrc`.

## Quick Setup

### 1. Copy the aliases you want

Choose between the full set or individual aliases based on your needs.

### 2. Update the path

Replace `/path/to/weatherust` with your actual docker-compose.yml location:
- Example: `~/docker-compose/weatherust`
- Example: `~/weatherust`
- Example: `/opt/weatherust`

### 3. Reload your shell

```bash
source ~/.bashrc
```

---

## Complete Alias Set

Copy and paste this entire block, then update the `WEATHERUST_COMPOSE_DIR` variable:

```bash
# ==============================================================================
# weatherust Service Aliases
# ==============================================================================

# Set your docker-compose.yml directory here
WEATHERUST_COMPOSE_DIR=~/docker-compose/weatherust

# Health monitoring
alias healthmon='docker compose -f ${WEATHERUST_COMPOSE_DIR}/docker-compose.yml exec healthmon_runner /app/healthmon'

# Update monitoring and control
alias updatemon='docker compose -f ${WEATHERUST_COMPOSE_DIR}/docker-compose.yml exec updatemon_runner /app/updatemon'
alias updatectl='docker compose -f ${WEATHERUST_COMPOSE_DIR}/docker-compose.yml exec updatectl_runner /app/updatectl'

# One-off services (use run --rm)
alias weatherust='docker compose -f ${WEATHERUST_COMPOSE_DIR}/docker-compose.yml run --rm weatherust'
alias speedynotify='docker compose -f ${WEATHERUST_COMPOSE_DIR}/docker-compose.yml run --rm speedynotify'

# Service management helpers
alias weatherust-up='docker compose -f ${WEATHERUST_COMPOSE_DIR}/docker-compose.yml up -d'
alias weatherust-down='docker compose -f ${WEATHERUST_COMPOSE_DIR}/docker-compose.yml down'
alias weatherust-pull='docker compose -f ${WEATHERUST_COMPOSE_DIR}/docker-compose.yml pull'
alias weatherust-logs='docker compose -f ${WEATHERUST_COMPOSE_DIR}/docker-compose.yml logs -f'
alias weatherust-ps='docker compose -f ${WEATHERUST_COMPOSE_DIR}/docker-compose.yml ps'
```

---

## Individual Aliases

If you prefer to add aliases individually:

### Health Monitoring

```bash
# healthmon - Docker container health checks
alias healthmon='docker compose -f ~/docker-compose/weatherust/docker-compose.yml exec healthmon_runner /app/healthmon'
```

**Usage:**
```bash
healthmon health
healthmon health --quiet
healthmon health --cpu-warn-pct 90
```

### Update Monitoring

```bash
# updatemon - Check for OS and Docker updates
alias updatemon='docker compose -f ~/docker-compose/weatherust/docker-compose.yml exec updatemon_runner /app/updatemon'
```

**Usage:**
```bash
updatemon --docker
updatemon --local --docker
updatemon --servers "Cloud VM1"
```

### Update Control

```bash
# updatectl - Apply updates and perform cleanup
alias updatectl='docker compose -f ~/docker-compose/weatherust/docker-compose.yml exec updatectl_runner /app/updatectl'
```

**Usage:**
```bash
updatectl list servers
updatectl os --yes --local
updatectl clean-docker --local
```

### One-Off Services

```bash
# weatherust - Weather reports
alias weatherust='docker compose -f ~/docker-compose/weatherust/docker-compose.yml run --rm weatherust'

# speedynotify - Internet speed tests
alias speedynotify='docker compose -f ~/docker-compose/weatherust/docker-compose.yml run --rm speedynotify'
```

**Usage:**
```bash
weatherust --zip 52726 --units imperial
speedynotify --min-down 300 --min-up 20
```

### Service Management

```bash
# Convenience aliases for docker-compose operations
alias weatherust-up='docker compose -f ~/docker-compose/weatherust/docker-compose.yml up -d'
alias weatherust-down='docker compose -f ~/docker-compose/weatherust/docker-compose.yml down'
alias weatherust-pull='docker compose -f ~/docker-compose/weatherust/docker-compose.yml pull'
alias weatherust-logs='docker compose -f ~/docker-compose/weatherust/docker-compose.yml logs -f'
alias weatherust-ps='docker compose -f ~/docker-compose/weatherust/docker-compose.yml ps'
```

**Usage:**
```bash
weatherust-up              # Start all services
weatherust-down            # Stop all services
weatherust-pull            # Pull latest images
weatherust-logs            # Follow all logs
weatherust-logs healthmon  # Follow specific service
weatherust-ps              # List running services
```

---

## Alternative: Wrapper Scripts

If you prefer scripts over aliases, create executable files in `~/bin/`:

### Setup

```bash
mkdir -p ~/bin
export PATH="$HOME/bin:$PATH"  # Add to ~/.bashrc if not already there
```

### healthmon Script

```bash
cat > ~/bin/healthmon << 'EOF'
#!/bin/bash
cd ~/docker-compose/weatherust
docker compose exec healthmon_runner /app/healthmon "$@"
EOF
chmod +x ~/bin/healthmon
```

### updatemon Script

```bash
cat > ~/bin/updatemon << 'EOF'
#!/bin/bash
cd ~/docker-compose/weatherust
docker compose exec updatemon_runner /app/updatemon "$@"
EOF
chmod +x ~/bin/updatemon
```

### updatectl Script

```bash
cat > ~/bin/updatectl << 'EOF'
#!/bin/bash
cd ~/docker-compose/weatherust
docker compose exec updatectl_runner /app/updatectl "$@"
EOF
chmod +x ~/bin/updatectl
```

### weatherust Script

```bash
cat > ~/bin/weatherust << 'EOF'
#!/bin/bash
cd ~/docker-compose/weatherust
docker compose run --rm weatherust "$@"
EOF
chmod +x ~/bin/weatherust
```

### speedynotify Script

```bash
cat > ~/bin/speedynotify << 'EOF'
#!/bin/bash
cd ~/docker-compose/weatherust
docker compose run --rm speedynotify "$@"
EOF
chmod +x ~/bin/speedynotify
```

**Advantages of wrapper scripts:**
- Work from any directory
- Can add custom logic (logging, validation, etc.)
- Easier to version control
- Can be shared across users

---

## Usage Examples

After setting up aliases or scripts:

### Quick Health Check

```bash
healthmon health
```

### Check for Updates

```bash
updatemon --docker
```

### List Configured Servers

```bash
updatectl list servers
```

### Safe Update Preview

```bash
updatectl all --dry-run --local
```

### Execute Updates

```bash
updatectl os --yes --local
```

### Docker Cleanup

```bash
updatectl clean-docker --local
updatectl clean-docker --local --execute --profile conservative
```

### Weather Report

```bash
weatherust --zip 52726 --units imperial
```

### Speed Test

```bash
speedynotify --min-down 300 --min-up 20
```

### Service Management

```bash
weatherust-ps              # Check what's running
weatherust-pull            # Update images
weatherust-up              # Start services
weatherust-logs ofelia     # Check scheduler logs
```

---

## Advanced Aliases

### Combine with Common Flags

```bash
# Pre-configured commands with common flags

# Health check in quiet mode
alias healthmon-check='healthmon health --quiet'

# Update check for localhost + Docker
alias updatemon-check='updatemon --local --docker --quiet'

# Safe dry-run for all updates
alias updatectl-preview='updatectl all --dry-run --local'

# Docker cleanup analysis
alias updatectl-analyze='updatectl clean-docker --local'

# Safe Docker cleanup execution
alias updatectl-clean='updatectl clean-docker --local --execute --profile conservative'
```

### Scheduled Task Helpers

```bash
# Test Ofelia scheduled commands locally

# Test weatherust daily job
alias test-weather='weatherust --zip ${DEFAULT_ZIP:-52726} --units ${DEFAULT_UNITS:-imperial} --quiet'

# Test speedtest job
alias test-speed='speedynotify --quiet --min-down ${SPEEDTEST_MIN_DOWN:-300} --min-up ${SPEEDTEST_MIN_UP:-20}'

# Test health monitor job
alias test-health='healthmon health --quiet'

# Test update monitor job
alias test-updates='updatemon --docker --quiet'

# Test cleanup report job
alias test-cleanup='updatectl clean-docker --local --quiet'
```

### Multi-Server Shortcuts

If you frequently target specific servers:

```bash
# Update specific servers
alias update-cloud='updatectl os --yes --servers "Cloud VM1,Cloud VM2"'
alias update-local='updatectl os --yes --local'
alias update-office='updatectl os --yes --servers "Office-HP-WS"'

# Check updates for specific servers
alias check-cloud='updatemon --servers "Cloud VM1,Cloud VM2" --docker'
alias check-local='updatemon --local --docker'
```

---

## Troubleshooting

### "command not found"

1. Verify alias exists:
   ```bash
   alias | grep healthmon
   ```

2. Reload shell:
   ```bash
   source ~/.bashrc
   ```

3. Check path in alias matches your actual docker-compose.yml location

### "No such file or directory"

Update the path in your aliases:
```bash
# Wrong
alias healthmon='docker compose -f /wrong/path/docker-compose.yml ...'

# Right
alias healthmon='docker compose -f ~/docker-compose/weatherust/docker-compose.yml ...'
```

### Aliases not persisting

Make sure you added them to your shell rc file:
- Bash: `~/.bashrc` or `~/.bash_aliases`
- Zsh: `~/.zshrc`
- Fish: `~/.config/fish/config.fish`

### Testing aliases

```bash
# Show what the alias expands to
type healthmon

# Should output:
# healthmon is aliased to `docker compose -f ~/docker-compose/weatherust/docker-compose.yml exec healthmon_runner /app/healthmon'
```

---

## See Also

- [CLI-COMMANDS.md](CLI-COMMANDS.md) - Complete CLI reference for all services
- [README.md](README.md) - Main project documentation
- [updatectl/README.md](updatectl/README.md) - updatectl documentation
- [updatemon/README.md](updatemon/README.md) - updatemon documentation
- [healthmon/README.md](healthmon/README.md) - healthmon documentation
