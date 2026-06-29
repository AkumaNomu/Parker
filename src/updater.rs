use self_update::backends::github::{GitHubUpdateStatus, Update};
use std::env;

/// Checks GitHub releases for a newer version and updates the binary.
/// Returns Ok(()) if up-to-date or after a successful update.
/// Errors are returned and can be shown to the user.
pub fn check_self_update() -> Result<(), String> {
    let target =
        env::current_exe().map_err(|e| format!("Could not locate current executable: {e}"))?;

    let mut builder = Update::configure();
    builder
        .repo_owner("AkumaNomu")
        .repo_name("Parker")
        .bin_name("parker")
        .target("x86_64-pc-windows-msvc")
        .bin_install_path(&target)
        .show_download_progress(true)
        .current_version(env!("CARGO_PKG_VERSION"));

    let status = builder
        .build()
        .map_err(|e| format!("Self-update configuration failed: {e}"))?
        .update_extended()
        .map_err(|e| format!("Self-update failed: {e}"))?;

    match status {
        GitHubUpdateStatus::UpToDate => Ok(()),
        GitHubUpdateStatus::Updated(_) => Ok(()),
    }
}
