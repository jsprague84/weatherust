use super::UpdateChecker;

/// APT package manager checker (Debian, Ubuntu, etc.)
pub struct AptChecker;

impl UpdateChecker for AptChecker {
    fn check_command(&self) -> (&str, Vec<&str>) {
        // apt list --upgradable
        // Note: We skip "apt-get update" to avoid modifying system state
        // Users should run this manually or via cron before running updatemon
        ("apt", vec!["list", "--upgradable"])
    }

    fn parse_updates(&self, output: &str) -> Vec<String> {
        /*
        Example output:
        Listing...
        docker-ce/jammy 5:25.0.0-1~ubuntu.22.04~jammy amd64 [upgradable from: 5:24.0.7-1~ubuntu.22.04~jammy]
        linux-image-generic/jammy-security 5.15.0.91.89 amd64 [upgradable from: 5.15.0.89.87]
        */

        output
            .lines()
            .skip(1) // Skip "Listing..." header
            .filter(|line| line.contains("[upgradable from:"))
            .map(|line| {
                // Extract package name (everything before the first '/')
                let package_name = line.split('/').next().unwrap_or(line);

                // Check if this is a security update
                let is_security = line.contains("-security");

                if is_security {
                    format!("{} (security)", package_name)
                } else {
                    package_name.to_string()
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_apt_output() {
        let checker = AptChecker;
        let output = r#"Listing...
docker-ce/jammy 5:25.0.0-1~ubuntu.22.04~jammy amd64 [upgradable from: 5:24.0.7-1~ubuntu.22.04~jammy]
linux-image-generic/jammy-security 5.15.0.91.89 amd64 [upgradable from: 5.15.0.89.87]
vim/jammy 2:8.2.3995-1ubuntu2.15 amd64 [upgradable from: 2:8.2.3995-1ubuntu2.14]
"#;

        let updates = checker.parse_updates(output);

        assert_eq!(updates.len(), 3);
        assert_eq!(updates[0], "docker-ce");
        assert_eq!(updates[1], "linux-image-generic (security)");
        assert_eq!(updates[2], "vim");
    }

    #[test]
    fn test_parse_empty_output() {
        let checker = AptChecker;
        let output = "Listing...\n";

        let updates = checker.parse_updates(output);
        assert_eq!(updates.len(), 0);
    }
}
