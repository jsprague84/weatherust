use anyhow::{anyhow, Result};

/// Package manager types we support
#[derive(Debug, Clone, PartialEq)]
pub enum PackageManager {
    Apt,
    Dnf,
    Pacman,
}

impl PackageManager {
    /// Get the binary name for this package manager
    pub fn binary(&self) -> &str {
        match self {
            PackageManager::Apt => "apt",
            PackageManager::Dnf => "dnf",
            PackageManager::Pacman => "pacman",
        }
    }

    /// Get display name
    pub fn display_name(&self) -> &str {
        match self {
            PackageManager::Apt => "APT (Debian/Ubuntu)",
            PackageManager::Dnf => "DNF (Fedora/RHEL)",
            PackageManager::Pacman => "Pacman (Arch)",
        }
    }

    /// All supported package managers (for detection)
    pub fn all() -> Vec<PackageManager> {
        vec![
            PackageManager::Apt,
            PackageManager::Dnf,
            PackageManager::Pacman,
        ]
    }
}

/// Represents a server to check
#[derive(Debug, Clone)]
pub struct Server {
    pub name: String,
    pub ssh_host: Option<String>, // None = local, Some = user@host
}

impl Server {
    /// Create a local server instance
    pub fn local() -> Self {
        Server {
            name: "localhost".to_string(),
            ssh_host: None,
        }
    }

    /// Parse server from string
    /// Format: "name:user@host" or "user@host" (name derived from host)
    pub fn parse(input: &str) -> Result<Self> {
        let parts: Vec<&str> = input.split(':').collect();

        match parts.len() {
            1 => {
                // Just "user@host"
                let ssh_host = parts[0].to_string();
                let name = ssh_host.split('@').last().unwrap_or("unknown").to_string();
                Ok(Server {
                    name,
                    ssh_host: Some(ssh_host),
                })
            }
            2 => {
                // "name:user@host"
                Ok(Server {
                    name: parts[0].to_string(),
                    ssh_host: Some(parts[1].to_string()),
                })
            }
            _ => Err(anyhow!("Invalid server format: {}. Expected 'name:user@host' or 'user@host'", input)),
        }
    }

    /// Is this the local system?
    pub fn is_local(&self) -> bool {
        self.ssh_host.is_none()
    }

    /// Get display host string
    pub fn display_host(&self) -> String {
        self.ssh_host.clone().unwrap_or_else(|| "local".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_server_with_name() {
        let server = Server::parse("myserver:ubuntu@192.168.1.10").unwrap();
        assert_eq!(server.name, "myserver");
        assert_eq!(server.ssh_host, Some("ubuntu@192.168.1.10".to_string()));
    }

    #[test]
    fn test_parse_server_without_name() {
        let server = Server::parse("admin@192.168.1.20").unwrap();
        assert_eq!(server.name, "192.168.1.20");
        assert_eq!(server.ssh_host, Some("admin@192.168.1.20".to_string()));
    }

    #[test]
    fn test_local_server() {
        let server = Server::local();
        assert!(server.is_local());
        assert_eq!(server.display_host(), "local");
    }
}
