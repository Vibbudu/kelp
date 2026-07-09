use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ReleaseInfo {
    pub version: String,
    pub release_notes: String,
    pub download_url: String,
    pub publish_date: String,
}

#[allow(async_fn_in_trait)]
pub trait UpdateChecker: Send + Sync {
    /// Checks if a newer version of Kelp is available on GitHub Releases
    async fn check_for_updates(&self, current_version: &str) -> Result<Option<ReleaseInfo>, String>;

    /// Downloads the installer to a temporary location
    async fn download_installer(&self, release: &ReleaseInfo) -> Result<PathBuf, String>;

    /// Launches the installer and exits the current process
    fn install_and_exit(&self, installer_path: &Path) -> Result<(), String>;
}
