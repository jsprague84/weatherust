use anyhow::{anyhow, Result};
use tokio::process::Command;
use tokio::time::{timeout, Duration};

use crate::Server;

/// Handles executing commands either locally or via SSH
/// Shared executor used by updatemon, dockermon, and updatectl
pub struct RemoteExecutor {
    server: Server,
    ssh_key: Option<String>,
}

impl RemoteExecutor {
    pub fn new(server: Server, ssh_key: Option<&str>) -> Result<Self> {
        Ok(RemoteExecutor {
            server,
            ssh_key: ssh_key.map(|s| s.to_string()),
        })
    }

    /// Execute a command (locally or via SSH)
    /// Public so other modules can use it
    pub async fn execute_command(&self, cmd: &str, args: &[&str]) -> Result<String> {
        self.execute(cmd, args).await
    }

    /// Execute a command (locally or via SSH) - internal helper
    async fn execute(&self, cmd: &str, args: &[&str]) -> Result<String> {
        if self.server.is_local() {
            // Execute locally
            self.execute_local(cmd, args).await
        } else {
            // Execute via SSH
            self.execute_ssh(cmd, args).await
        }
    }

    /// Execute command locally
    async fn execute_local(&self, cmd: &str, args: &[&str]) -> Result<String> {
        eprintln!("Executing locally: {} {}", cmd, args.join(" "));

        // Add timeout to prevent hanging (2 minutes max)
        let output = timeout(
            Duration::from_secs(120),
            Command::new(cmd).args(args).output()
        )
        .await
        .map_err(|_| anyhow!("Command timed out after 120s: {} {}", cmd, args.join(" ")))?
        .map_err(|e| anyhow!("Failed to execute {}: {}", cmd, e))?;

        // Note: Some commands use non-zero exit codes to indicate updates available
        // (e.g., dnf check-update returns 100 if updates exist)
        // So we don't fail on non-zero exit here

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !stderr.is_empty() {
            eprintln!("stderr from {}: {}", cmd, stderr);
        }

        Ok(stdout)
    }

    /// Execute command via SSH
    async fn execute_ssh(&self, cmd: &str, args: &[&str]) -> Result<String> {
        let ssh_host = self.server.ssh_host.as_ref()
            .ok_or_else(|| anyhow!("No SSH host configured"))?;

        // Build the remote command string with proper shell escaping
        // We need to quote arguments properly for the remote shell
        let remote_cmd = if args.is_empty() {
            cmd.to_string()
        } else {
            // Quote each argument that might contain spaces or special chars
            let quoted_args: Vec<String> = args.iter()
                .map(|arg| {
                    // If arg contains spaces or special chars, quote it
                    if arg.contains(' ') || arg.contains('*') || arg.contains('$') {
                        format!("'{}'", arg.replace('\'', "'\\''"))
                    } else {
                        arg.to_string()
                    }
                })
                .collect();
            format!("{} {}", cmd, quoted_args.join(" "))
        };

        eprintln!("Executing via SSH on {}: {}", ssh_host, remote_cmd);

        // Build SSH command
        let mut ssh_cmd = Command::new("ssh");
        ssh_cmd.arg("-o")
            .arg("BatchMode=yes") // No interactive prompts
            .arg("-o")
            .arg("StrictHostKeyChecking=no") // Don't check host keys
            .arg("-o")
            .arg("UserKnownHostsFile=/dev/null"); // Don't save host keys (read-only .ssh mount)

        // Add SSH key if specified
        if let Some(key_path) = &self.ssh_key {
            ssh_cmd.arg("-i").arg(key_path);
        }

        ssh_cmd.arg(ssh_host).arg(remote_cmd);

        // Add timeout to prevent SSH from hanging (2 minutes max)
        let output = timeout(
            Duration::from_secs(120),
            ssh_cmd.output()
        )
        .await
        .map_err(|_| anyhow!("SSH command timed out after 120s to {}", ssh_host))?
        .map_err(|e| anyhow!("Failed to SSH to {}: {}", ssh_host, e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Only fail on actual SSH errors, not command exit codes
            if stderr.contains("Permission denied") || stderr.contains("Connection refused") {
                return Err(anyhow!("SSH failed: {}", stderr));
            }
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(stdout)
    }

    /// Get reference to the server
    pub fn server(&self) -> &Server {
        &self.server
    }
}
