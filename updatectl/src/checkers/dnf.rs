use super::UpdateChecker;

/// DNF package manager checker (Fedora, RHEL 8+, CentOS Stream, etc.)
pub struct DnfChecker;

impl UpdateChecker for DnfChecker {
    fn check_command(&self) -> (&str, Vec<&str>) {
        // dnf check-update returns exit code 100 if updates available
        // Use --cacheonly to avoid refreshing repos (much faster)
        // Cache refresh is handled automatically in the background (see executor)
        // Use full path for SSH compatibility
        ("/usr/bin/dnf", vec!["check-update", "--quiet", "--cacheonly"])
    }

    fn parse_updates(&self, output: &str) -> Vec<String> {
        /*
        Example output:
        docker-ce.x86_64                    3:25.0.0-1.fc39                    docker-ce-stable
        kernel.x86_64                       6.6.8-200.fc39                     updates
        vim-enhanced.x86_64                 2:9.0.2120-1.fc39                  updates
        */

        output
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .filter(|line| {
                // Lines with updates have at least 3 parts (package, version, repo)
                line.split_whitespace().count() >= 3
            })
            .map(|line| {
                // Extract package name (first column, before the dot and arch)
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(first) = parts.first() {
                    // Split on '.' to remove arch suffix (e.g., "docker-ce.x86_64" -> "docker-ce")
                    first.split('.').next().unwrap_or(first).to_string()
                } else {
                    line.to_string()
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dnf_output() {
        let checker = DnfChecker;
        let output = r#"docker-ce.x86_64                    3:25.0.0-1.fc39                    docker-ce-stable
kernel.x86_64                       6.6.8-200.fc39                     updates
vim-enhanced.x86_64                 2:9.0.2120-1.fc39                  updates
"#;

        let updates = checker.parse_updates(output);

        assert_eq!(updates.len(), 3);
        assert_eq!(updates[0], "docker-ce");
        assert_eq!(updates[1], "kernel");
        assert_eq!(updates[2], "vim-enhanced");
    }

    #[test]
    fn test_parse_empty_output() {
        let checker = DnfChecker;
        let output = "";

        let updates = checker.parse_updates(output);
        assert_eq!(updates.len(), 0);
    }
}
