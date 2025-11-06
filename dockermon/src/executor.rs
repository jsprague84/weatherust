use anyhow::{Context, Result};
use common::Server;
use std::io::Read;
use std::net::TcpStream;
use std::path::Path;

/// Execute commands on remote servers via SSH
pub struct RemoteExecutor {
    session: ssh2::Session,
}

impl RemoteExecutor {
    /// Create a new remote executor for the given server
    pub fn new(server: &Server, ssh_key_path: Option<&str>) -> Result<Self> {
        if server.is_local() {
            return Err(anyhow::anyhow!("Cannot create SSH executor for local server"));
        }

        let ssh_host = server.ssh_host.as_ref().unwrap();

        // Parse user@host
        let parts: Vec<&str> = ssh_host.split('@').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid SSH host format. Expected user@host"));
        }
        let (user, host) = (parts[0], parts[1]);

        // Connect via TCP
        let tcp = TcpStream::connect(format!("{}:22", host))
            .context(format!("Failed to connect to {}", host))?;

        let mut session = ssh2::Session::new()
            .context("Failed to create SSH session")?;
        session.set_tcp_stream(tcp);
        session.handshake()
            .context("SSH handshake failed")?;

        // Authenticate
        if let Some(key_path) = ssh_key_path {
            session.userauth_pubkey_file(user, None, Path::new(key_path), None)
                .context(format!("SSH key authentication failed with key: {}", key_path))?;
        } else {
            // Try default SSH key locations
            let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
            let default_keys = [
                format!("{}/.ssh/id_rsa", home),
                format!("{}/.ssh/id_ed25519", home),
            ];

            let mut authenticated = false;
            for key_path in &default_keys {
                if Path::new(key_path).exists() {
                    if session.userauth_pubkey_file(user, None, Path::new(key_path), None).is_ok() {
                        authenticated = true;
                        break;
                    }
                }
            }

            if !authenticated {
                return Err(anyhow::anyhow!("SSH authentication failed. No valid key found."));
            }
        }

        if !session.authenticated() {
            return Err(anyhow::anyhow!("SSH authentication failed"));
        }

        Ok(RemoteExecutor { session })
    }

    /// Execute a command on the remote server
    pub fn execute(&self, command: &str) -> Result<String> {
        let mut channel = self.session.channel_session()
            .context("Failed to open SSH channel")?;

        channel.exec(command)
            .context(format!("Failed to execute command: {}", command))?;

        let mut output = String::new();
        channel.read_to_string(&mut output)
            .context("Failed to read command output")?;

        channel.wait_close()
            .context("Failed to close channel")?;

        let exit_status = channel.exit_status()
            .context("Failed to get exit status")?;

        if exit_status != 0 {
            let mut stderr = String::new();
            channel.stderr().read_to_string(&mut stderr).ok();
            return Err(anyhow::anyhow!(
                "Command failed with exit code {}: {}\nStderr: {}",
                exit_status,
                command,
                stderr
            ));
        }

        Ok(output)
    }
}
