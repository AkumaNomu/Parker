use self_update::{backends::github::Update, Status};

/// Checks GitHub releases for a newer version and updates the binary.
/// Returns Ok(()) if up‑to‑date or after a successful update.
/// Errors are returned and can be shown to the user.
pub fn check_self_update() -> Result<(), String> {
    // Configure the updater to look at the repo "AkumaNomu/Parker".
    let status = Update::configure()
        .repo_owner("AkumaNomu")
        .repo_name("Parker")
        .bin_name("parker")
        .target("windows-x64")
        .show_download_progress(true)
        .current_version(env!("CARGO_PKG_VERSION"))
        .no_confirm(true)
        .build()
        .and_then(|update| update.update())
        .map_err(|e| format!("Self‑update failed: {e}"))?;

    match status {
        Status::UpToDate(_) | Status::Updated(_) => Ok(()),
    }
}
