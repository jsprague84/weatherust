use anyhow::{anyhow, Result};
use tokio::process::Command;

use crate::checkers::UpdateChecker;
use crate::types::{PackageManager, Server};

/// Handles executing commands either locally or via SSH
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
    /// Public so other modules (like docker) can use it
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
        log::debug!("Executing locally: {} {}", cmd, args.join(" "));

        let output = Command::new(cmd)
            .args(args)
            .output()
            .await
            .map_err(|e| anyhow!("Failed to execute {}: {}", cmd, e))?;

        // Note: Some commands use non-zero exit codes to indicate updates available
        // (e.g., dnf check-update returns 100 if updates exist)
        // So we don't fail on non-zero exit here

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !stderr.is_empty() {
            log::warn!("stderr from {}: {}", cmd, stderr);
        }

        Ok(stdout)
    }

    /// Execute command via SSH
    async fn execute_ssh(&self, cmd: &str, args: &[&str]) -> Result<String> {
        let ssh_host = self.server.ssh_host.as_ref()
            .ok_or_else(|| anyhow!("No SSH host configured"))?;

        // Build the remote command string
        let remote_cmd = if args.is_empty() {
            cmd.to_string()
        } else {
            format!("{} {}", cmd, args.join(" "))
        };

        log::debug!("Executing via SSH on {}: {}", ssh_host, remote_cmd);

        // Build SSH command
        let mut ssh_cmd = Command::new("ssh");
        ssh_cmd.arg("-o")
            .arg("BatchMode=yes") // No interactive prompts
            .arg("-o")
            .arg("StrictHostKeyChecking=accept-new"); // Accept new host keys

        // Add SSH key if specified
        if let Some(key_path) = &self.ssh_key {
            ssh_cmd.arg("-i").arg(key_path);
        }

        ssh_cmd.arg(ssh_host).arg(remote_cmd);

        let output = ssh_cmd
            .output()
            .await
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

    /// Detect which package manager is available on this server
    pub async fn detect_package_manager(&self) -> Result<PackageManager> {
        for pm in PackageManager::all() {
            let binary = pm.binary();

            // Check if the command exists
            let check_cmd = format!("command -v {}", binary);
            match self.execute("sh", &["-c", &check_cmd]).await {
                Ok(output) if !output.trim().is_empty() => {
                    log::info!("Detected package manager: {:?}", pm);
                    return Ok(pm);
                }
                _ => continue,
            }
        }

        Err(anyhow!("No supported package manager found on {}", self.server.name))
    }

    /// Check for updates using the given checker
    pub async fn check_updates(&self, checker: &Box<dyn UpdateChecker>) -> Result<Vec<String>> {
        let (cmd, args) = checker.check_command();

        let output = self.execute(cmd, &args).await?;
        let updates = checker.parse_updates(&output);

        log::info!("Found {} updates on {}", updates.len(), self.server.name);

        Ok(updates)
    }
}
