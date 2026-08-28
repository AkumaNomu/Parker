use serde_json::Value;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

const REPO_OWNER: &str = "AkumaNomu";
const REPO_NAME: &str = "Parker";

#[cfg(target_os = "windows")]
pub const TARGET: &str = "windows-x64";
#[cfg(not(target_os = "windows"))]
pub const TARGET: &str = "linux-x64";

pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

struct Release {
    version: String,
    asset_url: String,
    checksum_url: String,
}

fn curl_command() -> Command {
    #[cfg(target_os = "windows")]
    {
        use crate::win::CREATE_NO_WINDOW;
        use std::os::windows::process::CommandExt;
        let mut command = Command::new("curl");
        command.creation_flags(CREATE_NO_WINDOW);
        command
    }
    #[cfg(not(target_os = "windows"))]
    {
        Command::new("curl")
    }
}

fn curl_output(args: &[&str]) -> Result<String, String> {
    let output = curl_command()
        .args(["-sSfL", "--user-agent"])
        .arg(format!("parker/{}", current_version()))
        .args(args)
        .output()
        .map_err(|error| {
            format!("Could not run curl: {error}. Install curl to use self-update.")
        })?;
    if !output.status.success() {
        return Err(format!(
            "Download failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn latest_release() -> Result<Release, String> {
    let body = curl_output(&[
        "-H",
        "Accept: application/vnd.github+json",
        &format!("https://api.github.com/repos/{REPO_OWNER}/{REPO_NAME}/releases/latest"),
    ])
    .map_err(|error| format!("Could not reach GitHub Releases: {error}"))?;
    let value: Value = serde_json::from_str(body.trim())
        .map_err(|error| format!("Bad release response: {error}"))?;
    let tag = value
        .get("tag_name")
        .and_then(Value::as_str)
        .ok_or("Release response had no tag_name.")?
        .trim_start_matches('v')
        .to_string();
    let wanted = format!("parker-{tag}-{TARGET}");
    let assets = value
        .get("assets")
        .and_then(Value::as_array)
        .ok_or("Release response had no assets.")?;
    let url = assets
        .iter()
        .filter_map(|asset| asset.get("browser_download_url").and_then(Value::as_str))
        .find(|url| {
            let name = Path::new(url.split('?').next().unwrap_or(url))
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_default();
            let archive = if cfg!(target_os = "windows") {
                name.ends_with(".zip")
            } else {
                name.ends_with(".tar.gz")
            };
            let expected = if cfg!(target_os = "windows") {
                format!("{wanted}.zip")
            } else {
                format!("{wanted}.tar.gz")
            };
            archive && name == expected
        })
        .map(str::to_string)
        .ok_or_else(|| format!("No release asset found for {TARGET} in v{tag}."))?;
    let checksum_name = format!(
        "{}.sha256",
        Path::new(url.split('?').next().unwrap_or(&url))
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
    );
    let checksum_url = assets
        .iter()
        .filter_map(|asset| asset.get("browser_download_url").and_then(Value::as_str))
        .find(|candidate| {
            Path::new(candidate.split('?').next().unwrap_or(candidate))
                .file_name()
                .and_then(|name| name.to_str())
                == Some(checksum_name.as_str())
        })
        .map(str::to_string)
        .ok_or_else(|| format!("Release is missing checksum asset {checksum_name}."))?;
    Ok(Release {
        version: tag,
        asset_url: url,
        checksum_url,
    })
}

fn version_tuple(version: &str) -> Vec<u64> {
    version
        .trim_start_matches('v')
        .split('.')
        .map(|part| part.parse::<u64>().unwrap_or(0))
        .collect()
}

pub(crate) fn newer(candidate: &str, installed: &str) -> bool {
    let candidate = version_tuple(candidate);
    let installed = version_tuple(installed);
    for index in 0..3 {
        let left = candidate.get(index).copied().unwrap_or(0);
        let right = installed.get(index).copied().unwrap_or(0);
        if left != right {
            return left > right;
        }
    }
    false
}

fn temp_dir() -> Result<PathBuf, String> {
    let dir = env::temp_dir().join(format!("parker-update-{}", std::process::id()));
    fs::create_dir_all(&dir)
        .map_err(|error| format!("Could not create temp directory: {error}"))?;
    Ok(dir)
}

fn download(url: &str, destination: &Path) -> Result<(), String> {
    let status = curl_command()
        .args(["-SfL", "--progress-bar", "--user-agent"])
        .arg(format!("parker/{}", current_version()))
        .args(["-o"])
        .arg(destination)
        .arg(url)
        .status()
        .map_err(|error| format!("Could not start download: {error}"))?;
    if !status.success() {
        return Err("Download failed.".into());
    }
    Ok(())
}

fn verify_checksum(archive: &Path, checksum: &Path) -> Result<(), String> {
    let expected = fs::read_to_string(checksum)
        .map_err(|error| format!("Could not read checksum: {error}"))?
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    if expected.len() != 64 || !expected.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("Downloaded checksum has an invalid format.".into());
    }
    let output = if cfg!(target_os = "windows") {
        Command::new("certutil")
            .args(["-hashfile"])
            .arg(archive)
            .arg("SHA256")
            .output()
            .map_err(|error| format!("Could not calculate archive checksum: {error}"))?
    } else {
        Command::new("sha256sum")
            .arg(archive)
            .output()
            .map_err(|error| format!("Could not calculate archive checksum: {error}"))?
    };
    if !output.status.success() {
        return Err("Could not calculate archive checksum.".into());
    }
    let actual = String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .find(|token| token.len() == 64 && token.chars().all(|c| c.is_ascii_hexdigit()))
        .unwrap_or("")
        .to_ascii_lowercase();
    if actual != expected {
        return Err("Downloaded archive checksum does not match the release checksum.".into());
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn safe_archive_member(member: &str) -> bool {
    let normalized = member.replace('\\', "/");
    let path = Path::new(&normalized);
    !path.is_absolute()
        && !path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
}

fn extract_binary(archive: &Path, destination_dir: &Path) -> Result<PathBuf, String> {
    let binary = destination_dir.join(binary_name());
    let _ = fs::remove_file(&binary);
    #[cfg(target_os = "windows")]
    {
        let quote = |path: &Path| path.display().to_string().replace('\'', "''");
        let script = format!(
            "Expand-Archive -LiteralPath '{}' -DestinationPath '{}' -Force",
            quote(archive),
            quote(destination_dir)
        );
        let status = Command::new("powershell")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command"])
            .arg(&script)
            .status()
            .map_err(|error| format!("Could not run PowerShell: {error}"))?;
        if !status.success() || !binary.exists() {
            return Err("Could not extract the downloaded ZIP.".into());
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let listing = Command::new("tar")
            .args(["-tzf"])
            .arg(archive)
            .output()
            .map_err(|error| format!("Could not read archive contents: {error}"))?;
        if !listing.status.success() {
            return Err("Could not read the downloaded archive.".into());
        }
        let member = String::from_utf8_lossy(&listing.stdout)
            .lines()
            .map(str::trim)
            .find(|entry| {
                (*entry == "parker" || entry.ends_with("/parker"))
                    && !entry.contains("install-linux.sh")
                    && safe_archive_member(entry)
            })
            .ok_or("Archive did not contain the Parker binary.")?
            .to_string();
        let status = Command::new("tar")
            .args(["-xzf"])
            .arg(archive)
            .arg("-C")
            .arg(destination_dir)
            .arg(&member)
            .status()
            .map_err(|error| format!("Could not extract archive: {error}"))?;
        let extracted = destination_dir.join(&member);
        if !status.success() || !extracted.exists() {
            return Err("Could not extract the downloaded archive.".into());
        }
        let extracted = extracted
            .canonicalize()
            .map_err(|error| format!("Could not stage update: {error}"))?;
        let root = destination_dir
            .canonicalize()
            .map_err(|error| format!("Could not stage update: {error}"))?;
        if !extracted.starts_with(&root) {
            return Err("Archive member escaped extraction directory.".into());
        }
        fs::copy(&extracted, &binary)
            .map_err(|error| format!("Could not stage update: {error}"))?;
    }
    set_executable(&binary)?;
    Ok(binary)
}

#[cfg(target_os = "windows")]
fn set_executable(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn set_executable(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path)
        .map_err(|error| format!("Could not stat update: {error}"))?
        .permissions();
    permissions.set_mode(permissions.mode() | 0o755);
    fs::set_permissions(path, permissions)
        .map_err(|error| format!("Could not mark executable: {error}"))
}

fn copy_fallback(from: &Path, to: &Path) -> io::Result<()> {
    fs::copy(from, to)?;
    fs::remove_file(from)?;
    Ok(())
}

fn install_binary(staged: &Path) -> Result<PathBuf, String> {
    let running =
        env::current_exe().map_err(|error| format!("Could not locate Parker: {error}"))?;
    let running = running.canonicalize().unwrap_or(running);
    let backup = running.with_extension(if cfg!(target_os = "windows") {
        "exe.old"
    } else {
        "old"
    });
    let _ = fs::remove_file(&backup);
    fs::rename(&running, &backup).map_err(|error| {
        format!(
            "Could not stage the current executable ({}): {error}",
            running.display()
        )
    })?;
    let replacement = match fs::rename(staged, &running) {
        Ok(()) => Ok(()),
        Err(_) => copy_fallback(staged, &running),
    };
    if let Err(error) = replacement {
        let _ = fs::remove_file(&running);
        let _ = fs::rename(&backup, &running);
        return Err(format!("Could not replace the Parker binary: {error}"));
    }
    Ok(running)
}

fn cleanup_partial(directory: &Path) {
    let _ = fs::remove_dir_all(directory);
}

pub fn check_self_update() -> Result<String, String> {
    let release = latest_release()?;
    if !newer(&release.version, current_version()) {
        return Ok(format!("Parker {} is up to date.", current_version()));
    }
    let work = temp_dir()?;
    let extension = if cfg!(target_os = "windows") {
        "zip"
    } else {
        "tar.gz"
    };
    let archive = work.join(format!("parker-{}.{}", release.version, extension));
    let result = (|| -> Result<String, String> {
        download(&release.asset_url, &archive)?;
        let checksum = work.join(format!(
            "{}.sha256",
            archive
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("archive")
        ));
        download(&release.checksum_url, &checksum)?;
        verify_checksum(&archive, &checksum)?;
        let staged = extract_binary(&archive, &work)?;
        let installed = install_binary(&staged)?;
        Ok(format!(
            "Updated Parker to {} at {}.",
            release.version,
            installed.display()
        ))
    })();
    cleanup_partial(&work);
    result
}

#[cfg(target_os = "windows")]
fn binary_name() -> &'static str {
    "parker.exe"
}

#[cfg(not(target_os = "windows"))]
fn binary_name() -> &'static str {
    "parker"
}

#[cfg(test)]
mod tests {
    use super::newer;

    #[test]
    fn compares_semver_components() {
        assert!(newer("1.2.1", "1.2.0"));
        assert!(!newer("1.2.0", "1.2"));
        assert!(!newer("v1.2.0", "1.2.0"));
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn rejects_unsafe_archive_members() {
        assert!(super::safe_archive_member("parker-1.0/parker"));
        assert!(!super::safe_archive_member("../parker"));
        assert!(!super::safe_archive_member("/tmp/parker"));
        assert!(!super::safe_archive_member(r"foo\\..\\parker"));
    }
}
