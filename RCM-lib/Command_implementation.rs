//! Ensure command implementation
//! 
//! Verifies environment setup and installs missing dependencies

use anyhow::{anyhow, Context, Result};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use tokio::time::{sleep, Duration};
use crate::workspace::Workspace;
use crate::npm::{NpmManager, NpmManagerType};
use crate::ppm::ComposerManager;
use crate::system::SystemManager;
use crate::util;

#[derive(Debug)]
struct ManagerStatus {
    name: String,
    available: bool,
    version: Option<String>,
    issues: Vec<String>,
    dependencies_count: usize,
    missing_dependencies: Vec<String>,
}

/// Ensure all dependencies are installed and environment is properly configured
pub async fn run(workspace: &Workspace, managers: Option<Vec<String>>) -> Result<()> {
    println!("{}", style("🔍 Ensuring workspace dependencies...").cyan().bold());
    
    let target_managers = if let Some(mgrs) = managers {
        mgrs
    } else {
        workspace.enabled_managers()
    };
    
    if target_managers.is_empty() {
        return Err(anyhow!("No package managers enabled. Run 'rcm init' to configure managers."));
    }
    
    // Create progress bar for overall process
    let pb = ProgressBar::new(target_managers.len() as u64 * 3);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );
    
    let mut manager_statuses = Vec::new();
    
    // Phase 1: Check environment for each manager
    pb.set_message("Checking environment...");
    for manager in &target_managers {
        pb.set_message(format!("Checking {}...", manager));
        let status = check_manager_environment(workspace, manager).await?;
        manager_statuses.push(status);
        pb.inc(1);
        sleep(Duration::from_millis(100)).await;
    }
    
    // Phase 2: Validate configurations
    pb.set_message("Validating configurations...");
    for status in &mut manager_statuses {
        pb.set_message(format!("Validating {}...", status.name));
        validate_manager_config(workspace, status).await?;
        pb.inc(1);
        sleep(Duration::from_millis(100)).await;
    }
    
    // Phase 3: Install missing dependencies
    pb.set_message("Installing dependencies...");
    for status in &manager_statuses {
        if !status.missing_dependencies.is_empty() {
            pb.set_message(format!("Installing {} dependencies...", status.name));
            install_missing_dependencies(workspace, status).await?;
        }
        pb.inc(1);
        sleep(Duration::from_millis(100)).await;
    }
    
    pb.finish_with_message("Completed");
    
    // Print summary
    print_summary(&manager_statuses).await?;
    
    // Check for any critical issues
    let has_errors = manager_statuses.iter().any(|s| !s.issues.is_empty() || !s.available);
    
    if has_errors {
        println!();
        println!("{}", style("⚠️  Some issues were found:").yellow().bold());
        for status in &manager_statuses {
            if !status.available {
                println!("  {} {}: Not available", 
                    style("✗").red(), 
                    style(&status.name).bold()
                );
            }
            for issue in &status.issues {
                println!("  {} {}: {}", 
                    style("⚠").yellow(), 
                    style(&status.name).bold(), 
                    issue
                );
            }
        }
        println!();
        println!("Run {} for more detailed information.", style("rcm --help").cyan());
    } else {
        println!();
        println!("{}", style("✅ All dependencies are properly configured!").green().bold());
    }
    
    Ok(())
}

/// Check if a package manager is available and working
async fn check_manager_environment(workspace: &Workspace, manager: &str) -> Result<ManagerStatus> {
    let mut status = ManagerStatus {
        name: manager.to_string(),
        available: false,
        version: None,
        issues: Vec::new(),
        dependencies_count: 0,
        missing_dependencies: Vec::new(),
    };
    
    match manager {
        "cargo" => check_cargo_environment(workspace, &mut status).await?,
        "npm" => check_npm_environment(workspace, &mut status).await?,
        "composer" => check_composer_environment(workspace, &mut status).await?,
        "system" => check_system_environment(workspace, &mut status).await?,
        _ => {
            status.issues.push(format!("Unknown package manager: {}", manager));
        }
    }
    
    Ok(status)
}

/// Check Cargo (Rust) environment
async fn check_cargo_environment(workspace: &Workspace, status: &mut ManagerStatus) -> Result<()> {
    // Check if cargo is available
    if !util::command_exists("cargo").await {
        status.issues.push("Cargo not found. Install Rust from https://rustup.rs/".to_string());
        return Ok(());
    }
    
    status.available = true;
    
    // Get Cargo version
    if let Ok(output) = tokio::process::Command::new("cargo")
        .arg("--version")
        .output()
        .await
    {
        let version_str = String::from_utf8_lossy(&output.stdout);
        if let Some(version) = version_str.split_whitespace().nth(1) {
            status.version = Some(version.to_string());
        }
    }
    
    // Check for Cargo.toml
    let cargo_toml = workspace.root().join("Cargo.toml");
    if !cargo_toml.exists() {
        status.issues.push("No Cargo.toml found".to_string());
        return Ok(());
    }
    
    // Parse Cargo.toml to count dependencies
    if let Ok(content) = tokio::fs::read_to_string(&cargo_toml).await {
        let cargo_toml: toml::Value = toml::from_str(&content)
            .context("Failed to parse Cargo.toml")?;
        
        let mut dep_count = 0;
        if let Some(deps) = cargo_toml.get("dependencies").and_then(|d| d.as_table()) {
            dep_count += deps.len();
        }
        if let Some(dev_deps) = cargo_toml.get("dev-dependencies").and_then(|d| d.as_table()) {
            dep_count += dev_deps.len();
        }
        
        status.dependencies_count = dep_count;
    }
    
    // Check if Cargo.lock exists (indicates dependencies were installed)
    let cargo_lock = workspace.root().join("Cargo.lock");
    if !cargo_lock.exists() && status.dependencies_count > 0 {
        status.missing_dependencies.push("Dependencies not installed (no Cargo.lock)".to_string());
    }
    
    Ok(())
}

/// Check NPM environment
async fn check_npm_environment(workspace: &Workspace, status: &mut ManagerStatus) -> Result<()> {
    // Check if Node.js and npm are available
    if !util::command_exists("node").await {
        status.issues.push("Node.js not found. Install from https://nodejs.org/".to_string());
        return Ok(());
    }
    
    if !util::command_exists("npm").await {
        status.issues.push("NPM not found. Install Node.js or npm separately".to_string());
        return Ok(());
    }
    
    status.available = true;
    
    // Get Node.js version
    if let Ok(output) = tokio::process::Command::new("node")
        .arg("--version")
        .output()
        .await
    {
        let version_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        status.version = Some(version_str);
    }
    
    // Check for package.json
    let package_json = workspace.root().join("package.json");
    if !package_json.exists() {
        status.issues.push("No package.json found".to_string());
        return Ok(());
    }
    
    // Parse package.json to count dependencies
    if let Ok(content) = tokio::fs::read_to_string(&package_json).await {
        let package_json: serde_json::Value = serde_json::from_str(&content)
            .context("Failed to parse package.json")?;
        
        let mut dep_count = 0;
        if let Some(deps) = package_json.get("dependencies").and_then(|d| d.as_object()) {
            dep_count += deps.len();
        }
        if let Some(dev_deps) = package_json.get("devDependencies").and_then(|d| d.as_object()) {
            dep_count += dev_deps.len();
        }
        
        status.dependencies_count = dep_count;
    }
    
    // Check if node_modules exists
    let node_modules = workspace.root().join("node_modules");
    if !node_modules.exists() && status.dependencies_count > 0 {
        status.missing_dependencies.push("Dependencies not installed (no node_modules)".to_string());
    }
    
    Ok(())
}

/// Check Composer (PHP) environment
async fn check_composer_environment(workspace: &Workspace, status: &mut ManagerStatus) -> Result<()> {
    // Check if PHP is available
    if !util::command_exists("php").await {
        status.issues.push("PHP not found. Install PHP from your system package manager".to_string());
        return Ok(());
    }
    
    // Check if Composer is available
    if !util::command_exists("composer").await {
        status.issues.push("Composer not found. Install from https://getcomposer.org/".to_string());
        return Ok(());
    }
    
    status.available = true;
    
    // Get PHP version
    if let Ok(output) = tokio::process::Command::new("php")
        .arg("--version")
        .output()
        .await
    {
        let version_str = String::from_utf8_lossy(&output.stdout);
        if let Some(line) = version_str.lines().next() {
            if let Some(version) = line.split_whitespace().nth(1) {
                status.version = Some(version.to_string());
            }
        }
    }
    
    // Check for composer.json
    let composer_json = workspace.root().join("composer.json");
    if !composer_json.exists() {
        status.issues.push("No composer.json found".to_string());
        return Ok(());
    }
    
    // Parse composer.json to count dependencies
    if let Ok(content) = tokio::fs::read_to_string(&composer_json).await {
        let composer_json: serde_json::Value = serde_json::from_str(&content)
            .context("Failed to parse composer.json")?;
        
        let mut dep_count = 0;
        if let Some(deps) = composer_json.get("require").and_then(|d| d.as_object()) {
            dep_count += deps.len();
        }
        if let Some(dev_deps) = composer_json.get("require-dev").and_then(|d| d.as_object()) {
            dep_count += dev_deps.len();
        }
        
        status.dependencies_count = dep_count;
    }
    
    // Check if vendor directory exists
    let vendor = workspace.root().join("vendor");
    if !vendor.exists() && status.dependencies_count > 0 {
        status.missing_dependencies.push("Dependencies not installed (no vendor directory)".to_string());
    }
    
    Ok(())
}

/// Check system package manager environment
async fn check_system_environment(workspace: &Workspace, status: &mut ManagerStatus) -> Result<()> {
    let system_manager = SystemManager::new(workspace.root()).await;
    
    match system_manager {
        Ok(_) => {
            status.available = true;
            status.version = Some("Available".to_string());
        }
        Err(e) => {
            status.issues.push(format!("System package manager not available: {}", e));
        }
    }
    
    // Count system dependencies from workspace manifest
    let dependencies = workspace.list_dependencies();
    status.dependencies_count = dependencies
        .iter()
        .filter(|(_, dep)| dep.manager == "system")
        .count();
    
    Ok(())
}

/// Validate manager configuration
async fn validate_manager_config(workspace: &Workspace, status: &mut ManagerStatus) -> Result<()> {
    if !status.available {
        return Ok(());
    }
    
    match status.name.as_str() {
        "cargo" => validate_cargo_config(workspace, status).await?,
        "npm" => validate_npm_config(workspace, status).await?,
        "composer" => validate_composer_config(workspace, status).await?,
        "system" => validate_system_config(workspace, status).await?,
        _ => {}
    }
    
    Ok(())
}

/// Validate Cargo configuration
async fn validate_cargo_config(workspace: &Workspace, status: &mut ManagerStatus) -> Result<()> {
    let cargo_toml = workspace.root().join("Cargo.toml");
    if cargo_toml.exists() {
        // Check if Cargo.toml is valid
        if let Err(e) = tokio::fs::read_to_string(&cargo_toml)
            .await
            .and_then(|content| toml::from_str::<toml::Value>(&content).map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, e)
            }))
        {
            status.issues.push(format!("Invalid Cargo.toml: {}", e));
        }
    }
    
    Ok(())
}

/// Validate NPM configuration
async fn validate_npm_config(workspace: &Workspace, status: &mut ManagerStatus) -> Result<()> {
    let package_json = workspace.root().join("package.json");
    if package_json.exists() {
        // Check if package.json is valid
        if let Err(e) = tokio::fs::read_to_string(&package_json)
            .await
            .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, e)
            }))
        {
            status.issues.push(format!("Invalid package.json: {}", e));
        }
    }
    
    Ok(())
}

/// Validate Composer configuration
async fn validate_composer_config(workspace: &Workspace, status: &mut ManagerStatus) -> Result<()> {
    let composer_json = workspace.root().join("composer.json");
    if composer_json.exists() {
        // Use Composer to validate
        let output = tokio::process::Command::new("composer")
            .current_dir(workspace.root())
            .arg("validate")
            .arg("--no-check-publish")
            .output()
            .await;
        
        if let Ok(output) = output {
            if !output.status.success() {
                let error = String::from_utf8_lossy(&output.stderr);
                status.issues.push(format!("Composer validation failed: {}", error));
            }
        }
    }
    
    Ok(())
}

/// Validate system configuration
async fn validate_system_config(_workspace: &Workspace, _status: &mut ManagerStatus) -> Result<()> {
    // System packages don't have a specific config file to validate
    Ok(())
}

/// Install missing dependencies for a manager
async fn install_missing_dependencies(workspace: &Workspace, status: &ManagerStatus) -> Result<()> {
    if status.missing_dependencies.is_empty() {
        return Ok(());
    }
    
    println!("{}", style(format!("🔧 Installing {} dependencies...", status.name)).blue());
    
    match status.name.as_str() {
        "cargo" => {
            let mut cmd = tokio::process::Command::new("cargo");
            cmd.current_dir(workspace.root());
            cmd.arg("fetch");
            
            let output = cmd.output().await?;
            if !output.status.success() {
                return Err(anyhow!("Failed to install Cargo dependencies"));
            }
        }
        "npm" => {
            let npm_manager = NpmManager::new(workspace.root(), NpmManagerType::Npm);
            let mut cmd = tokio::process::Command::new("npm");
            cmd.current_dir(workspace.root());
            cmd.arg("install");
            
            let output = cmd.output().await?;
            if !output.status.success() {
                return Err(anyhow!("Failed to install NPM dependencies"));
            }
        }
        "composer" => {
            let mut cmd = tokio::process::Command::new("composer");
            cmd.current_dir(workspace.root());
            cmd.arg("install");
            
            let output = cmd.output().await?;
            if !output.status.success() {
                return Err(anyhow!("Failed to install Composer dependencies"));
            }
        }
        "system" => {
            // System dependencies need to be installed individually
            // This is handled by the specific add commands
        }
        _ => {}
    }
    
    Ok(())
}

/// Print summary of environment check
async fn print_summary(statuses: &[ManagerStatus]) -> Result<()> {
    println!();
    println!("{}", style("📊 Environment Summary").bold());
    println!("{}", style("─".repeat(50)).dim());
    
    for status in statuses {
        let status_icon = if status.available {
            style("✅").green()
        } else {
            style("❌").red()
        };
        
        let version_info = status.version
            .as_ref()
            .map(|v| format!(" ({})", v))
            .unwrap_or_default();
        
        println!(
            "{} {} {}{} - {} dependencies",
            status_icon,
            style(&status.name).bold(),
            if status.available { "Available" } else { "Not Available" },
            version_info,
            status.dependencies_count
        );
        
        if !status.missing_dependencies.is_empty() {
            for missing in &status.missing_dependencies {
                println!("    {} {}", style("⚠").yellow(), missing);
            }
        }
    }
    
    Ok(())
}
