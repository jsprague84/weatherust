# Bash Aliases for weatherust Services

Simple shell aliases to use weatherust commands without typing the full docker compose paths.

## Quick Setup

Add these aliases to your `~/.bashrc` or `~/.bash_aliases`:

```bash
# weatherust Service Aliases
# Update the path to match your docker-compose.yml location
WEATHERUST_DIR=~/docker-compose/weatherust

alias healthmon='docker compose -f $WEATHERUST_DIR/docker-compose.yml exec healthmon_runner /app/healthmon'
alias updatemon='docker compose -f $WEATHERUST_DIR/docker-compose.yml exec updatemon_runner /app/updatemon'
alias updatectl='docker compose -f $WEATHERUST_DIR/docker-compose.yml exec updatectl_runner /app/updatectl'
alias weatherust='docker compose -f $WEATHERUST_DIR/docker-compose.yml run --rm weatherust'
alias speedynotify='docker compose -f $WEATHERUST_DIR/docker-compose.yml run --rm speedynotify'
```

Then reload your shell:

```bash
source ~/.bashrc
```

---

## Usage Examples

After setting up aliases, you can use the commands directly:

### healthmon
```bash
healthmon health
healthmon health --quiet
healthmon health --cpu-warn-pct 90
healthmon health --ignore "ofelia,traefik"
```

### updatemon
```bash
updatemon --docker
updatemon --local --docker
updatemon --servers "Cloud VM1"
updatemon --servers "Cloud VM1,Cloud VM2" --docker
```

### updatectl
```bash
# Discovery
updatectl list servers
updatectl list examples

# Updates
updatectl os --dry-run --local
updatectl os --yes --local
updatectl docker --all --yes --local
updatectl all --yes --servers "Cloud VM1"

# Docker cleanup
updatectl clean-docker --local
updatectl clean-docker --local --execute --profile conservative
updatectl clean-docker --servers "Cloud VM1" --execute --profile moderate

# OS cleanup
updatectl clean-os --local --execute --all -y
updatectl clean-os --servers "Cloud VM1" --execute --cache -y
```

### weatherust
```bash
weatherust --zip 52726 --units imperial
weatherust --location "London,UK" --units metric --quiet
```

### speedynotify
```bash
speedynotify --min-down 300 --min-up 20
speedynotify --min-down 500 --min-up 50 --quiet
```

---

## Customization

### Change the Directory Path

Edit the `WEATHERUST_DIR` variable to match your setup:

```bash
# If installed in /opt
WEATHERUST_DIR=/opt/weatherust

# If installed in home directory
WEATHERUST_DIR=~/weatherust

# If installed elsewhere
WEATHERUST_DIR=/path/to/your/weatherust
```

### Per-Service Customization

You can also define aliases without the variable:

```bash
alias healthmon='docker compose -f ~/docker-compose/weatherust/docker-compose.yml exec healthmon_runner /app/healthmon'
alias updatemon='docker compose -f ~/docker-compose/weatherust/docker-compose.yml exec updatemon_runner /app/updatemon'
alias updatectl='docker compose -f ~/docker-compose/weatherust/docker-compose.yml exec updatectl_runner /app/updatectl'
alias weatherust='docker compose -f ~/docker-compose/weatherust/docker-compose.yml run --rm weatherust'
alias speedynotify='docker compose -f ~/docker-compose/weatherust/docker-compose.yml run --rm speedynotify'
```

---

## Verifying Aliases

Test that aliases work:

```bash
# Should show the help message
healthmon --help
updatemon --help
updatectl --help

# Should show version/build info
weatherust --help
speedynotify --help
```

Check what an alias expands to:

```bash
type healthmon
# Output: healthmon is aliased to `docker compose -f ~/docker-compose/weatherust/docker-compose.yml exec healthmon_runner /app/healthmon'
```

---

## Troubleshooting

### "command not found"

1. Make sure you added the aliases to `~/.bashrc` or `~/.bash_aliases`
2. Reload your shell: `source ~/.bashrc`
3. Check the alias exists: `alias | grep healthmon`

### "No such file or directory"

The path in `WEATHERUST_DIR` is wrong. Update it to match where your `docker-compose.yml` is located.

### "container not found"

The runner containers aren't running. Start them:

```bash
cd ~/docker-compose/weatherust  # or your actual path
docker compose up -d
```

---

## See Also

- [CLI-COMMANDS.md](CLI-COMMANDS.md) - Complete command reference
- [README.md](README.md) - Main documentation
