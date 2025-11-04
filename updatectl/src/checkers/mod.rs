mod apt;
mod dnf;
mod pacman;

pub use apt::AptChecker;
pub use dnf::DnfChecker;
pub use pacman::PacmanChecker;

use crate::types::PackageManager;

/// Trait for checking updates with different package managers
///
/// This is Rust's way of defining an interface - any type that implements
/// this trait can be used polymorphically via trait objects (Box<dyn UpdateChecker>)
pub trait UpdateChecker: Send + Sync {
    /// Get the command to check for available updates
    /// Returns: (command, args)
    fn check_command(&self) -> (&str, Vec<&str>);

    /// Parse the output from the check command into a list of package names
    fn parse_updates(&self, output: &str) -> Vec<String>;
}

/// Factory function to get the appropriate checker for a package manager
///
/// Returns a Box<dyn UpdateChecker> - this is a "trait object"
/// It allows us to return different concrete types (AptChecker, DnfChecker, etc.)
/// through a single interface
pub fn get_checker(pm: &PackageManager) -> Box<dyn UpdateChecker> {
    match pm {
        PackageManager::Apt => Box::new(AptChecker),
        PackageManager::Dnf => Box::new(DnfChecker),
        PackageManager::Pacman => Box::new(PacmanChecker),
    }
}
