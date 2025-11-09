# updatectl Webhook Server API Documentation

The updatectl webhook server provides HTTP endpoints for triggering updates and cleanup operations remotely. This enables integration with notification systems like ntfy.sh for one-click updates from mobile devices.

## Base URL

```
https://webhook.example.com
```

Replace with your actual `UPDATECTL_WEBHOOK_DOMAIN` from `.env`.

## Authentication

All endpoints (except `/health`) require token-based authentication via query parameter.

**Parameter:** `token`
**Type:** Query string
**Required:** Yes (except `/health`)
**Value:** Must match `UPDATECTL_WEBHOOK_SECRET` from `.env`

**Security:**
- Generate a strong random token: `openssl rand -base64 32`
- Keep token secret and secure
- Rotate token if compromised
- Use HTTPS in production (Traefik handles this)

**Error Response:**
```
HTTP 401 Unauthorized
Invalid token
```

---

## Endpoints

### 1. Update OS Packages

Updates operating system packages (apt/dnf/pacman) on a specified server.

**Endpoint:** `POST /webhook/update/os`

**Parameters:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `server` | string | Yes | Server name from `UPDATE_SERVERS` or "localhost" |
| `token` | string | Yes | Authentication token |

**Example Request:**
```bash
curl -X POST "https://webhook.example.com/webhook/update/os?server=Cloud%20VM1&token=your_secret_token"
```

**Success Response:**
```
HTTP 202 Accepted
OS update started for Cloud VM1
```

**What Happens:**
1. Validates token and server name
2. Spawns background task
3. Executes: `sudo apt-get update && sudo apt-get full-upgrade -y` (or dnf/pacman equivalent)
4. Sends Gotify/ntfy notification when complete

**Notification Example:**
```
Title: Cloud VM1 - OS update complete
Message: ✅ OS: 52 packages upgraded
```

**Error Responses:**
```
HTTP 401 Unauthorized - Invalid token
HTTP 400 Bad Request - Unknown server: <name>
```

---

### 2. Update All Docker Images

Updates all Docker images on a specified server.

**Endpoint:** `POST /webhook/update/docker/all`

**Parameters:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `server` | string | Yes | Server name from `UPDATE_SERVERS` or "localhost" |
| `token` | string | Yes | Authentication token |

**Example Request:**
```bash
curl -X POST "https://webhook.example.com/webhook/update/docker/all?server=localhost&token=your_secret_token"
```

**Success Response:**
```
HTTP 202 Accepted
Docker update started for localhost
```

**What Happens:**
1. Validates token and server name
2. Spawns background task
3. Pulls all Docker images with available updates
4. Restarts containers using updated images (respects `UPDATECTL_RESTART_POLICY`)
5. Sends notification when complete

**Notification Example:**
```
Title: localhost - Docker update complete
Message: ✅ Docker: 10 images updated, 8 containers restarted
```

**Notes:**
- Respects `UPDATECTL_RESTART_EXCLUDE` settings
- Never restarts `updatectl_webhook` container
- Skips containers in exclusion list

---

### 3. Update Specific Docker Image

Updates a single specified Docker image on a server.

**Endpoint:** `POST /webhook/update/docker/image`

**Parameters:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `server` | string | Yes | Server name from `UPDATE_SERVERS` or "localhost" |
| `image` | string | Yes | Docker image name (e.g., `nginx:latest`) |
| `token` | string | Yes | Authentication token |

**Example Request:**
```bash
curl -X POST "https://webhook.example.com/webhook/update/docker/image?server=localhost&image=nginx:latest&token=your_secret_token"
```

**Success Response:**
```
HTTP 202 Accepted
Docker image update started for localhost
```

**What Happens:**
1. Validates token and server name
2. Spawns background task
3. Pulls the specified Docker image
4. Restarts containers using that image
5. Sends notification when complete

**Notification Example:**
```
Title: localhost - Docker update complete
Message: ✅ Docker: nginx:latest updated, 1 container restarted
```

---

### 4. Safe Docker Cleanup

Executes conservative Docker cleanup (dangling images + unused networks).

**Endpoint:** `POST /webhook/cleanup/safe`

**Parameters:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `server` | string | Yes | Server name from `UPDATE_SERVERS` or "localhost" |
| `token` | string | Yes | Authentication token |

**Example Request:**
```bash
curl -X POST "https://webhook.example.com/webhook/cleanup/safe?server=localhost&token=your_secret_token"
```

**Success Response:**
```
HTTP 202 Accepted
Safe cleanup started for localhost
```

**What Happens:**
1. Validates token and server name
2. Spawns background task
3. Executes conservative cleanup profile:
   - ✅ Removes dangling images (`<none>:<none>`)
   - ✅ Removes unused networks
   - ❌ Does NOT remove build cache
   - ❌ Does NOT remove old stopped containers
4. Sends notification with results

**Notification Example:**
```
Title: localhost - Docker Cleanup: Complete
Message: ✅ Removed 3 dangling images + 5 networks | Reclaimed 523MB
```

**Safety:**
- Conservative profile - safest automated cleanup
- Never removes volumes
- Never removes images in use
- Never removes running containers

---

### 5. Prune Unused Docker Images

Removes unused Docker images (more aggressive cleanup).

**Endpoint:** `POST /webhook/cleanup/images/prune-unused`

**Parameters:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `server` | string | Yes | Server name from `UPDATE_SERVERS` or "localhost" |
| `token` | string | Yes | Authentication token |

**Example Request:**
```bash
curl -X POST "https://webhook.example.com/webhook/cleanup/images/prune-unused?server=localhost&token=your_secret_token"
```

**Success Response:**
```
HTTP 202 Accepted
Unused image cleanup started for localhost
```

**What Happens:**
1. Validates token and server name
2. Spawns background task
3. Executes moderate cleanup profile:
   - ✅ Removes dangling images
   - ✅ Removes unused networks
   - ✅ Removes unused images (no containers using them)
   - ✅ Clears build cache
   - ❌ Does NOT remove old stopped containers
4. Sends notification with results

**Notification Example:**
```
Title: localhost - Docker Cleanup: Unused images pruned
Message: ✅ Removed 8 unused images + 5 networks + build cache | Reclaimed 2.1GB
```

**Warning:**
- More aggressive than safe cleanup
- May remove images you plan to use later
- Removes build cache (slower subsequent builds)
- Cannot be undone

---

### 6. Health Check

Simple health check endpoint for monitoring and Traefik.

**Endpoint:** `GET /health`

**Parameters:** None (no authentication required)

**Example Request:**
```bash
curl http://localhost:8080/health
```

**Success Response:**
```
HTTP 200 OK
OK
```

**Use Cases:**
- Traefik health checks
- Monitoring systems (Uptime Kuma, etc.)
- Verify webhook server is running
- Load balancer health checks

---

## Server Names

The `server` parameter accepts:

### Configured Servers
From `UPDATE_SERVERS` environment variable:
```bash
UPDATE_SERVERS=docker-vm:local,Cloud VM1:ubuntu@cloud-vm1.js-node.com
```

**Valid server names:**
- `localhost` or `docker-vm` (for local server)
- `Cloud VM1` (exact match, case-sensitive)

### Ad-Hoc Servers
You can also use full connection strings:
- Not supported via webhooks currently
- Use `updatectl` CLI for ad-hoc servers

---

## Async Execution Model

All webhook endpoints (except `/health`) use asynchronous execution:

1. **Immediate Response:** HTTP 202 Accepted
2. **Background Task:** Operation runs in separate thread
3. **Completion Notification:** Gotify/ntfy when done

**Why Async?**
- Updates can take several minutes
- Prevents HTTP timeout errors
- Allows monitoring progress via notifications
- Non-blocking for subsequent requests

**Monitoring:**
- Watch for completion notification in Gotify/ntfy
- Check logs: `docker compose logs -f updatectl_webhook`

---

## Error Responses

### 401 Unauthorized
**Cause:** Invalid or missing `token` parameter

**Response:**
```
HTTP 401 Unauthorized
Invalid token
```

**Solution:** Check `UPDATECTL_WEBHOOK_SECRET` in `.env`

---

### 400 Bad Request
**Cause:** Unknown server name

**Response:**
```
HTTP 400 Bad Request
Unknown server: MyServer
```

**Solution:**
- Check server name matches `UPDATE_SERVERS` exactly
- Server names are case-sensitive
- Use `updatectl list servers` to see valid names

---

### 500 Internal Server Error
**Cause:** Unexpected error during execution

**Response:**
```
HTTP 500 Internal Server Error
Error message here
```

**Solution:**
- Check webhook server logs
- Verify SSH connectivity to remote servers
- Check Docker socket permissions

---

## Configuration

### Environment Variables

**Required:**
```bash
UPDATECTL_WEBHOOK_SECRET=your_secure_random_token
UPDATECTL_WEBHOOK_DOMAIN=webhook.example.com
UPDATECTL_WEBHOOK_URL=https://webhook.example.com
```

**Optional:**
```bash
UPDATE_SERVERS=server1:user@host,server2:user@host
UPDATE_SSH_KEY=/path/to/ssh/key
UPDATECTL_GOTIFY_KEY=gotify_token
UPDATECTL_NTFY_TOPIC=update-actions
```

### Docker Compose

**Service Definition:**
```yaml
updatectl_webhook:
  image: ghcr.io/jsprague84/updatectl:latest
  container_name: updatectl_webhook
  env_file: .env
  volumes:
    - /var/run/docker.sock:/var/run/docker.sock:rw
    - ${HOME}/.ssh:/root/.ssh:ro
  entrypoint: ["/app/updatectl"]
  command: ["serve", "--port", "8080"]
  restart: unless-stopped
  networks:
    - proxy
  labels:
    - "traefik.enable=true"
    - "traefik.http.routers.updatectl-webhook.rule=Host(`${UPDATECTL_WEBHOOK_DOMAIN}`)"
    - "traefik.http.routers.updatectl-webhook-secure.tls=true"
```

### Traefik Integration

Webhooks are exposed via Traefik reverse proxy with HTTPS:

1. DNS: Point `webhook.example.com` to your server
2. Traefik: Automatic HTTPS with Let's Encrypt
3. Internal: Webhook server listens on `0.0.0.0:8080`
4. External: Access via `https://webhook.example.com`

---

## Integration Examples

### ntfy.sh Action Buttons

When `updatemon` detects updates, it creates ntfy notifications with action buttons:

**Notification JSON:**
```json
{
  "topic": "update-actions",
  "title": "📦 Updates available - Cloud VM1",
  "message": "OS: 52 packages\nDocker: 10 images",
  "actions": [
    {
      "action": "http",
      "label": "Update OS",
      "url": "https://webhook.example.com/webhook/update/os?server=Cloud%20VM1&token=secret123",
      "method": "POST"
    },
    {
      "action": "http",
      "label": "Update Docker",
      "url": "https://webhook.example.com/webhook/update/docker/all?server=Cloud%20VM1&token=secret123",
      "method": "POST"
    }
  ]
}
```

**User Experience:**
1. Receive ntfy notification on phone
2. Tap "Update OS" button
3. ntfy sends POST to webhook
4. Update runs in background
5. Receive completion notification

---

### Automation with curl

**Update all servers via script:**
```bash
#!/bin/bash
TOKEN="your_secret_token"
WEBHOOK_URL="https://webhook.example.com"

# Update OS on all servers
for server in "localhost" "Cloud VM1" "Cloud VM2"; do
    echo "Updating OS on $server..."
    curl -X POST "$WEBHOOK_URL/webhook/update/os?server=$server&token=$TOKEN"
done
```

---

### Monitoring Integration

**Health check for Uptime Kuma:**
```
Monitor Type: HTTP(s)
URL: https://webhook.example.com/health
Expected Status: 200
Expected Response: OK
Interval: 60 seconds
```

---

## Security Best Practices

### Token Management
- ✅ Generate strong random tokens (32+ characters)
- ✅ Rotate tokens periodically
- ✅ Never commit tokens to git
- ✅ Use different tokens for dev/staging/prod

**Generate secure token:**
```bash
openssl rand -base64 32
```

### Network Security
- ✅ Use HTTPS in production (Traefik handles this)
- ✅ Webhook server binds to all interfaces (Traefik proxy)
- ✅ No direct exposure - always through Traefik
- ⚠️  Consider IP allowlisting in Traefik if possible

### Permissions
- ✅ Webhook server runs as root in container (required for docker socket)
- ✅ Read-write access to Docker socket (required for updates/cleanup)
- ✅ SSH keys mounted read-only
- ✅ Passwordless sudo required on remote servers

---

## Troubleshooting

### Webhook server not responding

**Check if running:**
```bash
docker compose ps updatectl_webhook
```

**Check health endpoint:**
```bash
curl http://localhost:8080/health
```

**Check logs:**
```bash
docker compose logs -f updatectl_webhook
```

---

### Token authentication failing

**Verify token:**
```bash
# Check what's in .env
grep UPDATECTL_WEBHOOK_SECRET .env

# Test with correct token
curl -X POST "http://localhost:8080/webhook/update/os?server=localhost&token=YOUR_TOKEN_HERE"
```

**Check URL encoding:**
- Tokens with special characters need URL encoding
- Use `%20` for spaces in server names
- Example: `Cloud%20VM1` instead of `Cloud VM1`

---

### Server name not recognized

**List valid servers:**
```bash
updatectl list servers
```

**Common issues:**
- Server names are case-sensitive
- Must match `UPDATE_SERVERS` exactly
- Include spaces if server name has them (URL encoded)

---

### Operation not completing

**Check background task logs:**
```bash
docker compose logs -f updatectl_webhook | grep "Cloud VM1"
```

**Common causes:**
- SSH key not accessible
- Passwordless sudo not configured
- Docker socket permission denied
- Network connectivity issues

---

## Rate Limiting

**Current Implementation:**
- ❌ No rate limiting implemented
- ⚠️  Webhooks can be triggered unlimited times
- ⚠️  Token is only protection

**Recommendations:**
- Implement rate limiting in Traefik if needed
- Use strong random tokens
- Monitor webhook logs for abuse
- Consider IP allowlisting

---

## API Changelog

### v2.0.0 (Current)
- Initial webhook server implementation
- 5 update/cleanup endpoints + health check
- Token-based authentication
- Async execution model
- Gotify + ntfy notifications

### Future Enhancements
- OAuth2 authentication
- Rate limiting
- Webhook signatures (HMAC)
- Scheduled updates via API
- Status query endpoints
- OS cleanup webhooks

---

## Support

For issues, feature requests, or questions:

- **Documentation:** [README.md](README.md)
- **CLI Reference:** [CLI-COMMANDS.md](CLI-COMMANDS.md)
- **GitHub Issues:** https://github.com/jsprague84/weatherust/issues

---

## Quick Reference

| Endpoint | Method | Purpose | Safety |
|----------|--------|---------|--------|
| `/webhook/update/os` | POST | Update OS packages | ✅ Safe |
| `/webhook/update/docker/all` | POST | Update all Docker images | ✅ Safe |
| `/webhook/update/docker/image` | POST | Update specific image | ✅ Safe |
| `/webhook/cleanup/safe` | POST | Conservative cleanup | ✅ Safe |
| `/webhook/cleanup/images/prune-unused` | POST | Aggressive cleanup | ⚠️  Moderate |
| `/health` | GET | Health check | ✅ Safe |

**All endpoints:**
- Require `token` parameter (except `/health`)
- Require `server` parameter
- Return HTTP 202 (accepted immediately)
- Send notifications when complete
- Work with both local and remote servers
