use anyhow::{anyhow, Result};

use crate::checkers::UpdateChecker;
use crate::types::PackageManager;
use common::{RemoteExecutor, Server};

/// Extension trait for updatemon-specific executor methods
pub trait UpdatemonExecutor {
    async fn detect_package_manager(&self) -> Result<PackageManager>;
    async fn check_updates(&self, checker: &Box<dyn UpdateChecker>) -> Result<Vec<String>>;
}

impl UpdatemonExecutor for RemoteExecutor {
    /// Detect which package manager is available on this server
    async fn detect_package_manager(&self) -> Result<PackageManager> {
        for pm in PackageManager::all() {
            let binary = pm.binary();

            // Check if the binary exists using full path
            // Use 'test -x' which works in minimal SSH environments without full PATH
            let check_cmd = format!("test -x /usr/bin/{} && echo found", binary);
            match self.execute_command("sh", &["-c", &check_cmd]).await {
                Ok(output) if output.trim() == "found" => {
                    log::info!("Detected package manager: {:?}", pm);
                    return Ok(pm);
                }
                _ => continue,
            }
        }

        Err(anyhow!("No supported package manager found on {}", self.server().name))
    }

    /// Check for updates using the given checker
    async fn check_updates(&self, checker: &Box<dyn UpdateChecker>) -> Result<Vec<String>> {
        let (cmd, args) = checker.check_command();

        // If this is DNF with --cacheonly, refresh the cache in the background for next run
        // This keeps the current check fast while ensuring the cache stays fresh
        if cmd == "/usr/bin/dnf" && args.contains(&"--cacheonly") {
            let server = self.server().clone();
            // Note: We don't have access to ssh_key here, but the spawned task will use default keys
            tokio::spawn(async move {
                if let Ok(executor) = RemoteExecutor::new(server.clone(), None) {
                    log::debug!("Refreshing DNF cache in background on {}", server.name);
                    let _ = executor.execute_command("/usr/bin/dnf", &["makecache", "--quiet"]).await;
                }
            });
        }

        let output = self.execute_command(cmd, &args).await?;
        let updates = checker.parse_updates(&output);

        log::info!("Found {} updates on {}", updates.len(), self.server().name);

        Ok(updates)
    }
}
