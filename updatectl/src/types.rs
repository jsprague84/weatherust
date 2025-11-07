use anyhow::{anyhow, Result};

// Re-export Server from common
pub use common::Server;

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
