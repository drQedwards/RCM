//! Utility functions for RCM
//! 
//! Provides common functionality shared across the codebase

use anyhow::{anyhow, Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tokio::fs;
use tokio::process::Command as AsyncCommand;
use walkdir::WalkDir;

#[derive(Debug, Serialize, Deserialize)]
pub struct OsInfo {
    pub family: String,
    pub name: String,
    pub version: String,
    pub arch: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandResult {
    pub success: bool,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

/// Check if a command exists in PATH
pub async fn command_exists(command: &str) -> bool {
    Command::new("which")
        .arg(command)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or_else(|_| {
            // Fallback for Windows
            Command::new("where")
                .arg(command)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|status| status.success())
                .unwrap_or(false)
        })
}

/// Execute a command and return result
pub async fn execute_command(cmd: &mut Command) -> Result<CommandResult> {
    let start = std::time::Instant::now();
    
    let output = cmd.output()
        .context("Failed to execute command")?;
    
    let duration_ms = start.elapsed().as_millis() as u64;
    
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    
    let exit_code = output.status.code().unwrap_or(-1);
    let success = output.status.success();
    
    if !success {
        return Err(anyhow!(
            "Command failed with exit code {}\nStdout: {}\nStderr: {}",
            exit_code,
            stdout,
            stderr
        ));
    }
    
    Ok(CommandResult {
        success,
        exit_code,
        stdout,
        stderr,
        duration_ms,
    })
}

/// Execute a command asynchronously
pub async fn execute_command_async(cmd: &mut AsyncCommand) -> Result<CommandResult> {
    let start = std::time::Instant::now();
    
    let output = cmd.output().await
        .context("Failed to execute async command")?;
    
    let duration_ms = start.elapsed().as_millis() as u64;
    
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    
    let exit_code = output.status.code().unwrap_or(-1);
    let success = output.status.success();
    
    if !success {
        return Err(anyhow!(
            "Async command failed with exit code {}\nStdout: {}\nStderr: {}",
            exit_code,
            stdout,
            stderr
        ));
    }
    
    Ok(CommandResult {
        success,
        exit_code,
        stdout,
        stderr,
        duration_ms,
    })
}

/// Get operating system information
pub async fn get_os_info() -> Result<OsInfo> {
    let family = std::env::consts::FAMILY.to_string();
    let arch = std::env::consts::ARCH.to_string();
    
    let (name, version) = match family.as_str() {
        "unix" => {
            if cfg!(target_os = "macos") {
                get_macos_info().await?
            } else if cfg!(target_os = "linux") {
                get_linux_info().await?
            } else {
                ("Unix".to_string(), "Unknown".to_string())
            }
        }
        "windows" => get_windows_info().await?,
        _ => ("Unknown".to_string(), "Unknown".to_string()),
    };
    
    Ok(OsInfo {
        family,
        name,
        version,
        arch,
    })
}

/// Get macOS system information
async fn get_macos_info() -> Result<(String, String)> {
    let output = AsyncCommand::new("sw_vers")
        .arg("-productName")
        .output()
        .await?;
    
    let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
    
    let output = AsyncCommand::new("sw_vers")
        .arg("-productVersion")
        .output()
        .await?;
    
    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    
    Ok((name, version))
}

/// Get Linux system information
async fn get_linux_info() -> Result<(String, String)> {
    // Try to read /etc/os-release
    if let Ok(content) = fs::read_to_string("/etc/os-release").await {
        let mut name = "Linux".to_string();
        let mut version = "Unknown".to_string();
        
        for line in content.lines() {
            if line.starts_with("NAME=") {
                name = line.strip_prefix("NAME=")
                    .unwrap_or("Linux")
                    .trim_matches('"')
                    .to_string();
            } else if line.starts_with("VERSION=") {
                version = line.strip_prefix("VERSION=")
                    .unwrap_or("Unknown")
                    .trim_matches('"')
                    .to_string();
            }
        }
        
        return Ok((name, version));
    }
    
    // Fallback to uname
    let output = AsyncCommand::new("uname")
        .arg("-sr")
        .output()
        .await?;
    
    let info = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let parts: Vec<&str> = info.split_whitespace().collect();
    
    let name = parts.first().unwrap_or(&"Linux").to_string();
    let version = parts.get(1).unwrap_or(&"Unknown").to_string();
    
    Ok((name, version))
}

/// Get Windows system information
async fn get_windows_info() -> Result<(String, String)> {
    let output = AsyncCommand::new("ver")
        .output()
        .await?;
    
    let info = String::from_utf8_lossy(&output.stdout);
    
    // Parse Windows version info
    if let Some(captures) = Regex::new(r"Microsoft Windows \[Version ([^\]]+)\]")
        .unwrap()
        .captures(&info) {
        let version = captures.get(1).unwrap().as_str().to_string();
        Ok(("Microsoft Windows".to_string(), version))
    } else {
        Ok(("Windows".to_string(), "Unknown".to_string()))
    }
}

/// Validate package name using common rules
pub fn validate_package_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(anyhow!("Package name cannot be empty"));
    }
    
    if name.len() > 214 {
        return Err(anyhow!("Package name too long (max 214 characters)"));
    }
    
    // Check for valid characters (alphanumeric, hyphens, underscores, dots, slashes)
    let valid_chars_regex = Regex::new(r"^[a-zA-Z0-9._/-]+$")?;
    if !valid_chars_regex.is_match(name) {
        return Err(anyhow!("Package name contains invalid characters"));
    }
    
    // Check for reserved names
    let reserved = [".", "..", "node_modules", "favicon.ico", "package.json", "Cargo.toml"];
    if reserved.contains(&name) {
        return Err(anyhow!("Reserved package name: {}", name));
    }
    
    Ok(())
}

/// Validate version string (semantic versioning)
pub fn validate_version(version: &str) -> Result<()> {
    if version.is_empty() {
        return Err(anyhow!("Version cannot be empty"));
    }
    
    // Basic semver pattern
    let semver_regex = Regex::new(r"^(?:>=|<=|>|<|\^|~|=)?(\d+)(?:\.(\d+))?(?:\.(\d+))?(?:-([a-zA-Z0-9.-]+))?(?:\+([a-zA-Z0-9.-]+))?$")?;
    
    if !semver_regex.is_match(version) {
        return Err(anyhow!("Invalid version format: {}", version));
    }
    
    Ok(())
}

/// Parse key=value arguments
pub fn parse_key_value_args(args: &[String]) -> Result<HashMap<String, String>> {
    let mut parsed = HashMap::new();
    
    for arg in args {
        if let Some((key, value)) = arg.split_once('=') {
            parsed.insert(key.to_string(), value.to_string());
        } else {
            return Err(anyhow!("Invalid key=value argument: {}", arg));
        }
    }
    
    Ok(parsed)
}

/// Calculate directory size recursively
pub async fn calculate_directory_size(path: &Path) -> Result<u64> {
    let mut total_size = 0u64;
    
    if !path.exists() {
        return Ok(0);
    }
    
    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            if let Ok(metadata) = entry.metadata() {
                total_size += metadata.len();
            }
        }
    }
    
    Ok(total_size)
}

/// Create a backup of a file
pub async fn backup_file(path: &Path) -> Result<PathBuf> {
    if !path.exists() {
        return Err(anyhow!("File does not exist: {}", path.display()));
    }
    
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let backup_path = path.with_extension(format!("{}.backup.{}", 
        path.extension().and_then(|s| s.to_str()).unwrap_or(""), 
        timestamp));
    
    fs::copy(path, &backup_path).await
        .context("Failed to create backup")?;
    
    Ok(backup_path)
}

/// Restore file from backup
pub async fn restore_from_backup(original_path: &Path, backup_path: &Path) -> Result<()> {
    if !backup_path.exists() {
        return Err(anyhow!("Backup file does not exist: {}", backup_path.display()));
    }
    
    fs::copy(backup_path, original_path).await
        .context("Failed to restore from backup")?;
    
    Ok(())
}

/// Check if path is inside another path
pub fn is_subpath(path: &Path, parent: &Path) -> bool {
    path.canonicalize()
        .and_then(|p| parent.canonicalize().map(|parent| p.starts_with(parent)))
        .unwrap_or(false)
}

/// Create a temporary directory
pub async fn create_temp_dir(prefix: &str) -> Result<PathBuf> {
    let temp_dir = std::env::temp_dir().join(format!("rcm-{}-{}", prefix, uuid::Uuid::new_v4()));
    fs::create_dir_all(&temp_dir).await
        .context("Failed to create temporary directory")?;
    Ok(temp_dir)
}

/// Remove directory recursively
pub async fn remove_dir_all(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_dir_all(path).await
            .context("Failed to remove directory")?;
    }
    Ok(())
}

/// Copy directory recursively
pub async fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    if !src.exists() {
        return Err(anyhow!("Source directory does not exist: {}", src.display()));
    }
    
    fs::create_dir_all(dst).await
        .context("Failed to create destination directory")?;
    
    let mut entries = fs::read_dir(src).await
        .context("Failed to read source directory")?;
    
    while let Some(entry) = entries.next_entry().await? {
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        
        if src_path.is_dir() {
            copy_dir_all(&src_path, &dst_path).await?;
        } else {
            fs::copy(&src_path, &dst_path).await
                .context("Failed to copy file")?;
        }
    }
    
    Ok(())
}

/// Get file hash (SHA-256)
pub async fn get_file_hash(path: &Path) -> Result<String> {
    use sha2::{Sha256, Digest};
    
    let content = fs::read(path).await
        .context("Failed to read file for hashing")?;
    
    let mut hasher = Sha256::new();
    hasher.update(&content);
    let result = hasher.finalize();
    
    Ok(format!("{:x}", result))
}

/// Verify file hash
pub async fn verify_file_hash(path: &Path, expected_hash: &str) -> Result<bool> {
    let actual_hash = get_file_hash(path).await?;
    Ok(actual_hash.eq_ignore_ascii_case(expected_hash))
}

/// Download file with progress
pub async fn download_file(url: &str, destination: &Path) -> Result<()> {
    let response = reqwest::get(url).await
        .context("Failed to start download")?;
    
    if !response.status().is_success() {
        return Err(anyhow!("Download failed with status: {}", response.status()));
    }
    
    let content = response.bytes().await
        .context("Failed to download content")?;
    
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).await
            .context("Failed to create destination directory")?;
    }
    
    fs::write(destination, content).await
        .context("Failed to write downloaded file")?;
    
    Ok(())
}

/// Extract archive (tar.gz, zip)
pub async fn extract_archive(archive_path: &Path, destination: &Path) -> Result<()> {
    let extension = archive_path.extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    
    match extension {
        "gz" | "tgz" => extract_tar_gz(archive_path, destination).await,
        "zip" => extract_zip(archive_path, destination).await,
        _ => Err(anyhow!("Unsupported archive format: {}", extension)),
    }
}

/// Extract tar.gz archive
async fn extract_tar_gz(archive_path: &Path, destination: &Path) -> Result<()> {
    use flate2::read::GzDecoder;
    use tar::Archive;
    
    let file = std::fs::File::open(archive_path)
        .context("Failed to open archive")?;
    
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);
    
    fs::create_dir_all(destination).await
        .context("Failed to create destination directory")?;
    
    archive.unpack(destination)
        .context("Failed to extract tar.gz archive")?;
    
    Ok(())
}

/// Extract zip archive
async fn extract_zip(archive_path: &Path, destination: &Path) -> Result<()> {
    use zip::ZipArchive;
    
    let file = std::fs::File::open(archive_path)
        .context("Failed to open zip archive")?;
    
    let mut archive = ZipArchive::new(file)
        .context("Failed to read zip archive")?;
    
    fs::create_dir_all(destination).await
        .context("Failed to create destination directory")?;
    
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)
            .context("Failed to read zip entry")?;
        
        let outpath = destination.join(file.sanitized_name());
        
        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath).await
                .context("Failed to create directory from zip")?;
        } else {
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent).await
                    .context("Failed to create parent directory")?;
            }
            
            let mut outfile = fs::File::create(&outpath).await
                .context("Failed to create output file")?;
            
            tokio::io::copy(&mut file, &mut outfile).await
                .context("Failed to extract zip file")?;
        }
        
        // Set permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = file.unix_mode() {
                let permissions = std::fs::Permissions::from_mode(mode);
                std::fs::set_permissions(&outpath, permissions)
                    .context("Failed to set file permissions")?;
            }
        }
    }
    
    Ok(())
}

/// Format bytes as human readable string
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    const THRESHOLD: f64 = 1024.0;
    
    if bytes == 0 {
        return "0 B".to_string();
    }
    
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= THRESHOLD && unit_index < UNITS.len() - 1 {
        size /= THRESHOLD;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

/// Format duration as human readable string
pub fn format_duration(duration_ms: u64) -> String {
    if duration_ms < 1000 {
        format!("{}ms", duration_ms)
    } else if duration_ms < 60_000 {
        format!("{:.1}s", duration_ms as f64 / 1000.0)
    } else if duration_ms < 3_600_000 {
        let minutes = duration_ms / 60_000;
        let seconds = (duration_ms % 60_000) as f64 / 1000.0;
        format!("{}m {:.1}s", minutes, seconds)
    } else {
        let hours = duration_ms / 3_600_000;
        let minutes = (duration_ms % 3_600_000) / 60_000;
        format!("{}h {}m", hours, minutes)
    }
}

/// Check if string is a valid URL
pub fn is_valid_url(url: &str) -> bool {
    url::Url::parse(url).is_ok()
}

/// Sanitize filename for filesystem
pub fn sanitize_filename(name: &str) -> String {
    let invalid_chars = ['<', '>', ':', '"', '|', '?', '*', '/', '\\'];
    name.chars()
        .map(|c| if invalid_chars.contains(&c) { '_' } else { c })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_validate_package_name() {
        assert!(validate_package_name("valid-package").is_ok());
        assert!(validate_package_name("valid_package").is_ok());
        assert!(validate_package_name("@scope/package").is_ok());
        assert!(validate_package_name("").is_err());
        assert!(validate_package_name("invalid package").is_err());
    }
    
    #[tokio::test]
    async fn test_validate_version() {
        assert!(validate_version("1.0.0").is_ok());
        assert!(validate_version("^1.0.0").is_ok());
        assert!(validate_version("~1.0.0").is_ok());
        assert!(validate_version(">=1.0.0").is_ok());
        assert!(validate_version("").is_err());
        assert!(validate_version("invalid").is_err());
    }
    
    #[tokio::test]
    async fn test_parse_key_value_args() {
        let args = vec!["key1=value1".to_string(), "key2=value2".to_string()];
        let parsed = parse_key_value_args(&args).unwrap();
        
        assert_eq!(parsed.get("key1"), Some(&"value1".to_string()));
        assert_eq!(parsed.get("key2"), Some(&"value2".to_string()));
    }
    
    #[tokio::test]
    async fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1023), "1023 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1048576), "1.0 MB");
    }
    
    #[tokio::test]
    async fn test_format_duration() {
        assert_eq!(format_duration(500), "500ms");
        assert_eq!(format_duration(1500), "1.5s");
        assert_eq!(format_duration(65000), "1m 5.0s");
        assert_eq!(format_duration(3665000), "1h 1m");
    }
}
