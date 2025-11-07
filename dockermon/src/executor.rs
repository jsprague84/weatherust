use anyhow::{anyhow, Result};
use common::Server;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

/// Execute commands on remote servers via SSH
pub struct RemoteExecutor {
    server: Server,
    ssh_key: Option<String>,
}

impl RemoteExecutor {
    /// Create a new remote executor for the given server
    pub fn new(server: &Server, ssh_key_path: Option<&str>) -> Result<Self> {
        if server.is_local() {
            return Err(anyhow!("Cannot create SSH executor for local server"));
        }

        Ok(RemoteExecutor {
            server: server.clone(),
            ssh_key: ssh_key_path.map(|s| s.to_string()),
        })
    }

    /// Execute a command on the remote server
    pub async fn execute(&self, command: &str) -> Result<String> {
        let ssh_host = self.server.ssh_host.as_ref()
            .ok_or_else(|| anyhow!("No SSH host configured"))?;

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

        ssh_cmd.arg(ssh_host).arg(command);

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
            return Err(anyhow!(
                "Command failed with exit code {}: {}\nStderr: {}",
                output.status.code().unwrap_or(-1),
                command,
                stderr
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(stdout)
    }
}
