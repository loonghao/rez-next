//! Self-update command implementation
//!
//! Implements the `rez-next self-update` command for updating the rez-next binary
//! to the latest (or a specified) release from GitHub.

use clap::Args;
use rez_next_common::{error::RezCoreResult, RezCoreError};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const REPO: &str = "loonghao/rez-next";
const BINARY_NAME: &str = "rez-next";

/// Arguments for the self-update command
#[derive(Args, Clone, Debug)]
pub struct SelfUpdateArgs {
    /// Specific version to install (e.g. "0.2.0"). Defaults to latest.
    #[arg(short, long, value_name = "VERSION")]
    pub version: Option<String>,

    /// Only check for updates without installing
    #[arg(long)]
    pub check: bool,

    /// Force reinstall even if already on the latest version
    #[arg(short, long)]
    pub force: bool,

    /// Print verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

// ─── helpers ────────────────────────────────────────────────────────────────

fn info(msg: &str) {
    println!("info: {}", msg);
}

fn success(msg: &str) {
    println!("success: {}", msg);
}

fn warn(msg: &str) {
    eprintln!("warn: {}", msg);
}

/// Detect the target triple for the current platform.
fn detect_target() -> RezCoreResult<String> {
    let os = env::consts::OS;
    let arch = env::consts::ARCH;

    let arch_str = match arch {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        other => {
            return Err(RezCoreError::Python(format!(
                "Unsupported architecture: {}",
                other
            )))
        }
    };

    let target = match os {
        "linux" => {
            // Detect musl vs gnu
            let libc = detect_linux_libc();
            format!("{}-unknown-linux-{}", arch_str, libc)
        }
        "macos" => format!("{}-apple-darwin", arch_str),
        "windows" => format!("{}-pc-windows-msvc", arch_str),
        other => return Err(RezCoreError::Python(format!("Unsupported OS: {}", other))),
    };

    Ok(target)
}

fn detect_linux_libc() -> &'static str {
    // Check for musl via ldd
    if let Ok(output) = Command::new("ldd").arg("--version").output() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stderr.to_lowercase().contains("musl") || stdout.to_lowercase().contains("musl") {
            return "musl";
        }
    }
    // Check /etc/os-release for Alpine / Void
    if let Ok(content) = fs::read_to_string("/etc/os-release") {
        for line in content.lines() {
            if line.starts_with("ID=") {
                let id = line.trim_start_matches("ID=").trim_matches('"');
                if id == "alpine" || id == "void" {
                    return "musl";
                }
            }
        }
    }
    "gnu"
}

/// Detect which downloader is available (curl or wget).
fn detect_downloader() -> RezCoreResult<&'static str> {
    if command_exists("curl") {
        Ok("curl")
    } else if command_exists("wget") {
        Ok("wget")
    } else {
        Err(RezCoreError::Python(
            "Neither curl nor wget found. Please install one of them.".to_string(),
        ))
    }
}

fn command_exists(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Download a URL to a file.
fn download_to_file(downloader: &str, url: &str, dest: &Path) -> RezCoreResult<()> {
    let status = match downloader {
        "curl" => Command::new("curl")
            .args(["-fsSL", url, "-o", dest.to_str().unwrap()])
            .status(),
        "wget" => Command::new("wget")
            .args(["-qO", dest.to_str().unwrap(), url])
            .status(),
        _ => unreachable!(),
    }
    .map_err(RezCoreError::Io)?;

    if !status.success() {
        return Err(RezCoreError::Python(format!(
            "Download failed for URL: {}",
            url
        )));
    }
    Ok(())
}

/// Fetch text from a URL.
fn download_text(downloader: &str, url: &str) -> RezCoreResult<String> {
    let output = match downloader {
        "curl" => Command::new("curl").args(["-fsSL", url]).output(),
        "wget" => Command::new("wget").args(["-qO-", url]).output(),
        _ => unreachable!(),
    }
    .map_err(RezCoreError::Io)?;

    if !output.status.success() {
        return Err(RezCoreError::Python(format!("Failed to fetch: {}", url)));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Parse the tag_name from a minimal GitHub releases/latest JSON response.
fn parse_tag_name(json: &str) -> Option<String> {
    // Simple extraction without a JSON parser dependency
    json.lines()
        .find(|l| l.contains("\"tag_name\""))
        .and_then(|l| {
            let _start = l.find('"')? + 1; // first quote in value
                                           // tag_name": "v0.2.0"  →  find the value quotes
            let after_colon = l[l.find(':')? + 1..].trim();
            let inner = after_colon.trim_matches(|c| c == '"' || c == ',' || c == ' ');
            Some(inner.trim_start_matches('v').to_string())
        })
}

/// Resolve the latest version string from GitHub.
fn fetch_latest_version(downloader: &str) -> RezCoreResult<String> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", REPO);
    let body = download_text(downloader, &url)?;
    parse_tag_name(&body).ok_or_else(|| {
        RezCoreError::Python("Failed to parse latest version from GitHub API".to_string())
    })
}

/// Return the path of the currently running binary.
fn current_binary_path() -> RezCoreResult<PathBuf> {
    env::current_exe().map_err(RezCoreError::Io)
}

/// Extract an archive into a directory.
fn extract_archive(archive: &Path, dest_dir: &Path) -> RezCoreResult<()> {
    let ext = archive.extension().and_then(|e| e.to_str()).unwrap_or("");

    let status = if ext == "gz" {
        Command::new("tar")
            .args([
                "xzf",
                archive.to_str().unwrap(),
                "-C",
                dest_dir.to_str().unwrap(),
            ])
            .status()
    } else if ext == "zip" {
        Command::new("unzip")
            .args([
                "-qo",
                archive.to_str().unwrap(),
                "-d",
                dest_dir.to_str().unwrap(),
            ])
            .status()
    } else {
        return Err(RezCoreError::Python(format!(
            "Unknown archive extension: {}",
            ext
        )));
    }
    .map_err(RezCoreError::Io)?;

    if !status.success() {
        return Err(RezCoreError::Python("Extraction failed".to_string()));
    }
    Ok(())
}

/// Find the rez-next binary inside the extracted temp directory.
fn find_binary_in_dir(dir: &Path) -> Option<PathBuf> {
    let candidates = [BINARY_NAME, &format!("{}.exe", BINARY_NAME)];
    for entry in walkdir(dir) {
        let name = entry
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if candidates.iter().any(|c| name == *c) {
            return Some(entry.to_path_buf());
        }
    }
    None
}

/// Minimal recursive directory walker (avoids adding walkdir dependency).
fn walkdir(root: &Path) -> Vec<PathBuf> {
    let mut results = Vec::new();
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                results.extend(walkdir(&path));
            } else {
                results.push(path);
            }
        }
    }
    results
}

/// Perform an atomic binary replacement: write to a temp file then rename.
#[cfg(not(target_os = "windows"))]
fn replace_binary(new_binary: &Path, target: &Path) -> RezCoreResult<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(new_binary)
        .map_err(RezCoreError::Io)?
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(new_binary, perms).map_err(RezCoreError::Io)?;
    fs::rename(new_binary, target).map_err(RezCoreError::Io)
}

#[cfg(target_os = "windows")]
fn replace_binary(new_binary: &Path, target: &Path) -> RezCoreResult<()> {
    // On Windows we cannot rename over a running executable; instead we write the
    // new binary next to it and use a helper .cmd wrapper, or we copy and ask the
    // user to restart. For simplicity we copy to <target>.new and instruct the user.
    let pending = target.with_extension("new.exe");
    fs::copy(new_binary, &pending).map_err(RezCoreError::Io)?;
    println!();
    warn("Windows: cannot replace a running executable directly.");
    println!("  New binary written to: {}", pending.display());
    println!("  To finish the update, run:");
    println!(
        "    move /Y \"{}\" \"{}\"",
        pending.display(),
        target.display()
    );
    Ok(())
}

// ─── main execute function ───────────────────────────────────────────────────

/// Execute the self-update command.
pub fn execute(args: SelfUpdateArgs) -> RezCoreResult<()> {
    let current_version = env!("CARGO_PKG_VERSION");

    // On Windows without curl/wget we fall back to PowerShell install.ps1
    let downloader = detect_downloader()?;

    // Determine target version
    let target_version = if let Some(ref v) = args.version {
        v.trim_start_matches('v').to_string()
    } else {
        info("Checking for latest version...");
        fetch_latest_version(downloader)?
    };

    if args.check {
        if target_version == current_version {
            println!("rez-next is up to date (v{})", current_version);
        } else {
            println!(
                "Update available: v{} → v{}",
                current_version, target_version
            );
            println!(
                "  Run `rez-next self-update` to install v{}",
                target_version
            );
        }
        return Ok(());
    }

    if target_version == current_version && !args.force {
        println!(
            "rez-next v{} is already the latest version.",
            current_version
        );
        println!("  Use --force to reinstall.");
        return Ok(());
    }

    let target_triple = detect_target()?;
    if args.verbose {
        info(&format!("Target platform: {}", target_triple));
    }

    info(&format!(
        "Updating rez-next: v{} → v{}",
        current_version, target_version
    ));

    // Build archive URL
    let (_ext, archive_name) = if cfg!(target_os = "windows") {
        ("zip", format!("{}-{}.zip", BINARY_NAME, target_triple))
    } else {
        (
            "tar.gz",
            format!("{}-{}.tar.gz", BINARY_NAME, target_triple),
        )
    };
    let download_url = format!(
        "https://github.com/{}/releases/download/v{}/{}",
        REPO, target_version, archive_name
    );

    // Create temp directory
    let tmp_dir = env::temp_dir().join(format!("rez-next-update-{}", target_version));
    fs::create_dir_all(&tmp_dir).map_err(RezCoreError::Io)?;
    let archive_path = tmp_dir.join(&archive_name);

    info(&format!("Downloading {}...", download_url));
    download_to_file(downloader, &download_url, &archive_path).map_err(|e| {
        let _ = fs::remove_dir_all(&tmp_dir);
        RezCoreError::Python(format!(
            "Download failed. Check if v{} has pre-built binaries for {}.\n  {}",
            target_version, target_triple, e
        ))
    })?;

    // Optionally verify checksum
    let checksums_url = format!(
        "https://github.com/{}/releases/download/v{}/checksums-sha256.txt",
        REPO, target_version
    );
    let checksums_path = tmp_dir.join("checksums-sha256.txt");
    if download_to_file(downloader, &checksums_url, &checksums_path).is_ok() {
        if let Ok(checksums) = fs::read_to_string(&checksums_path) {
            if let Some(expected) = checksums
                .lines()
                .find(|l| l.contains(&archive_name))
                .and_then(|l| l.split_whitespace().next())
            {
                let actual = compute_sha256(&archive_path)?;
                if actual != expected {
                    let _ = fs::remove_dir_all(&tmp_dir);
                    return Err(RezCoreError::Python(format!(
                        "Checksum mismatch! Expected: {}, Got: {}",
                        expected, actual
                    )));
                }
                success("Checksum verified ✓");
            }
        }
    } else {
        warn("Checksums file not available, skipping verification");
    }

    // Extract archive
    info("Extracting...");
    extract_archive(&archive_path, &tmp_dir).map_err(|e| {
        let _ = fs::remove_dir_all(&tmp_dir);
        e
    })?;

    // Locate new binary
    let new_binary = find_binary_in_dir(&tmp_dir).ok_or_else(|| {
        let _ = fs::remove_dir_all(&tmp_dir);
        RezCoreError::Python(format!(
            "Could not find {} binary in the downloaded archive",
            BINARY_NAME
        ))
    })?;

    // Replace current binary
    let current_exe = current_binary_path()?;
    replace_binary(&new_binary, &current_exe)?;

    // Cleanup
    let _ = fs::remove_dir_all(&tmp_dir);

    success(&format!("rez-next updated to v{} ✓", target_version));
    Ok(())
}

/// Compute SHA-256 of a file using the system sha256sum / shasum tool.
fn compute_sha256(path: &Path) -> RezCoreResult<String> {
    // Try sha256sum (Linux / Git Bash on Windows)
    if let Ok(output) = Command::new("sha256sum").arg(path).output() {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout);
            if let Some(hash) = s.split_whitespace().next() {
                return Ok(hash.to_string());
            }
        }
    }
    // Try shasum -a 256 (macOS)
    if let Ok(output) = Command::new("shasum")
        .args(["-a", "256"])
        .arg(path)
        .output()
    {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout);
            if let Some(hash) = s.split_whitespace().next() {
                return Ok(hash.to_string());
            }
        }
    }
    // Try certutil (native Windows)
    #[cfg(target_os = "windows")]
    if let Ok(output) = Command::new("certutil")
        .args(["-hashfile", path.to_str().unwrap_or(""), "SHA256"])
        .output()
    {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout);
            // certutil output has the hash on the second non-empty line
            let hash = s
                .lines()
                .map(str::trim)
                .find(|l| !l.is_empty() && !l.starts_with("CertUtil") && !l.contains("hash"))
                .unwrap_or("")
                .replace(' ', "");
            if hash.len() == 64 {
                return Ok(hash);
            }
        }
    }
    warn("No SHA256 tool found, skipping checksum verification");
    Ok(String::new())
}
