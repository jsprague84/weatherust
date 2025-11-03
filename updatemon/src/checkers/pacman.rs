use super::UpdateChecker;

/// Pacman package manager checker (Arch Linux, Manjaro, etc.)
pub struct PacmanChecker;

impl UpdateChecker for PacmanChecker {
    fn check_command(&self) -> (&str, Vec<&str>) {
        // checkupdates is a script that comes with pacman
        // It's safer than 'pacman -Qu' which requires sync
        // Use full path for SSH compatibility
        ("/usr/bin/checkupdates", vec![])
    }

    fn parse_updates(&self, output: &str) -> Vec<String> {
        /*
        Example output:
        docker 1:25.0.0-1 -> 1:25.0.1-1
        linux 6.6.8.arch1-1 -> 6.6.9.arch1-1
        vim 9.0.2120-1 -> 9.0.2121-1
        */

        output
            .lines()
            .filter(|line| !line.is_empty())
            .filter_map(|line| {
                // Split by whitespace and get first column (package name)
                line.split_whitespace().next().map(|s| s.to_string())
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pacman_output() {
        let checker = PacmanChecker;
        let output = r#"docker 1:25.0.0-1 -> 1:25.0.1-1
linux 6.6.8.arch1-1 -> 6.6.9.arch1-1
vim 9.0.2120-1 -> 9.0.2121-1
"#;

        let updates = checker.parse_updates(output);

        assert_eq!(updates.len(), 3);
        assert_eq!(updates[0], "docker");
        assert_eq!(updates[1], "linux");
        assert_eq!(updates[2], "vim");
    }

    #[test]
    fn test_parse_empty_output() {
        let checker = PacmanChecker;
        let output = "";

        let updates = checker.parse_updates(output);
        assert_eq!(updates.len(), 0);
    }
}
