use self_update::{cargo::CargoUpdate, UpdateStatus};
use std::env;

/// Checks GitHub releases for a newer version and updates the binary.
/// Returns Ok(()) if up‑to‑date or after a successful update.
/// Errors are returned and can be shown to the user.
pub fn check_self_update() -> Result<(), String> {
    // Locate the running executable.
    let target = env::current_exe()
        .map_err(|e| format!("Could not locate current executable: {e}"))?;

    // Configure the updater to look at the repo "AkumaNomu/Parker".
    let status = CargoUpdate::configure()
        .repo_owner("AkumaNomu")
        .repo_name("Parker")
        .bin_name("parker")
        .target("x86_64-pc-windows-msvc")
        .show_download_progress(true)
        .current_exe(target)
        .run()
        .map_err(|e| format!("Self‑update failed: {e}"))?;

    match status {
        UpdateStatus::UpToDate => Ok(()),
        UpdateStatus::Updated => Ok(()),
    }
}
