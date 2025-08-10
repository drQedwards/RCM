//! Workspace commands implementation
//! 
//! Commands for workspace management operations

use anyhow::{anyhow, Result};
use console::style;
use tabled::{Table, Tabled};
use serde_json;
use crate::commands::WorkspaceCommands;
use crate::workspace::Workspace;
use crate::npm::{NpmManager, NpmManagerType};
use crate::ppm::ComposerManager;
use crate::system::SystemManager;

#[derive(Tabled)]
struct DependencyRow {
    #[tabled(rename = "Package")]
    name: String,
    #[tabled(rename = "Version")]
    version: String,
    #[tabled(rename = "Manager")]
    manager: String,
    #[tabled(rename = "Type")]
    dep_type: String,
    #[tabled(rename = "Platforms")]
    platforms: String,
}

/// Handle workspace commands
pub async fn handle_command(workspace: &Workspace, cmd: WorkspaceCommands) -> Result<()> {
    match cmd {
        WorkspaceCommands::List { format } => list_packages(workspace, &format).await,
        WorkspaceCommands::Sync => sync_packages(workspace).await,
        WorkspaceCommands::Clean => clean_workspace(workspace).await,
        WorkspaceCommands::Update => update_packages(workspace).await,
        WorkspaceCommands::Check => check_workspace(workspace).await,
    }
}

/// List all packages in the workspace
async fn list_packages(workspace: &Workspace, format: &str) -> Result<()> {
    let dependencies = workspace.list_dependencies();
    
    if dependencies.is_empty() {
        println!("{}", style("📦 No dependencies found in workspace").yellow());
        println!("Run {} to add packages", style("rcm add <package>").cyan());
        return Ok(());
    }
    
    match format {
        "table" => {
            let rows: Vec<DependencyRow> = dependencies
                .into_iter()
                .map(|(name, spec)| DependencyRow {
                    name: name.clone(),
                    version: spec.version.clone(),
                    manager: spec.manager.clone(),
                    dep_type: if spec.dev_only { "dev".to_string() } else { "prod".to_string() },
                    platforms: if spec.platforms.is_empty() { 
                        "all".to_string() 
                    } else { 
                        spec.platforms.join(",") 
                    },
                })
                .collect();
            
            let table = Table::new(rows);
            println!("{}", table);
        }
        "json" => {
            let json = serde_json::to_string_pretty(&dependencies)
                .map_err(|e| anyhow!("Failed to serialize dependencies: {}", e))?;
            println!("{}", json);
        }
        "names" => {
            for (name, _) in dependencies {
                println!("{}", name);
            }
        }
        _ => {
            return Err(anyhow!("Unknown format: {}. Use 'table', 'json', or 'names'", format));
        }
    }
    
    // Show summary
    let mut manager_counts = std::collections::HashMap::new();
    let mut total_count = 0;
    
    for (_, spec) in workspace.list_dependencies() {
        *manager_counts.entry(&spec.manager).or_insert(0) += 1;
        total_count += 1;
    }
    
    println!();
    println!("{}", style(format!("📊 Total: {} packages", total_count)).bold());
    for (manager, count) in manager_counts {
        println!("  • {}: {} packages", style(manager).cyan(), count);
    }
    
    Ok(())
}

/// Synchronize all package managers
async fn sync_packages(workspace: &Workspace) -> Result<()> {
    println!("{}", style("🔄 Synchronizing all package managers...").cyan().bold());
    
    let enabled_managers = workspace.enabled_managers();
    let mut sync_results = Vec::new();
    
    for manager in &enabled_managers {
        println!("{}", style(format!("🔧 Synchronizing {}...", manager)).blue());
        
        let result = match manager.as_str() {
            "cargo" => sync_cargo(workspace).await,
            "npm" => sync_npm(workspace).await,
            "composer" => sync_composer(workspace).await,
            "system" => sync_system(workspace).await,
            _ => Err(anyhow!("Unknown manager: {}", manager)),
        };
        
        match result {
            Ok(_) => {
                println!("{}", style(format!("✅ {} synchronized", manager)).green());
                sync_results.push((manager.clone(), true));
            }
            Err(e) => {
                println!("{}", style(format!("❌ {} failed: {}", manager, e)).red());
                sync_results.push((manager.clone(), false));
            }
        }
    }
    
    // Print summary
    println!();
    let successful = sync_results.iter().filter(|(_, success)| *success).count();
    let total = sync_results.len();
    
    if successful == total {
        println!("{}", style("✅ All package managers synchronized successfully!").green().bold());
    } else {
        println!("{}", style(format!("⚠️ {}/{} package managers synchronized", successful, total)).yellow().bold());
    }
    
    Ok(())
}

/// Synchronize Cargo dependencies
async fn sync_cargo(workspace: &Workspace) -> Result<()> {
    let cargo_toml = workspace.root().join("Cargo.toml");
    if !cargo_toml.exists() {
        return Ok(());
    }
    
    let mut cmd = tokio::process::Command::new("cargo");
    cmd.current_dir(workspace.root());
    cmd.arg("update");
    
    let output = cmd.output().await?;
    if !output.status.success() {
        return Err(anyhow!("Cargo sync failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    
    Ok(())
}

/// Synchronize NPM dependencies
async fn sync_npm(workspace: &Workspace) -> Result<()> {
    let package_json = workspace.root().join("package.json");
    if !package_json.exists() {
        return Ok(());
    }
    
    let npm_manager = NpmManager::new(workspace.root(), NpmManagerType::Npm);
    let mut cmd = tokio::process::Command::new("npm");
    cmd.current_dir(workspace.root());
    cmd.arg("install");
    
    let output = cmd.output().await?;
    if !output.status.success() {
        return Err(anyhow!("NPM sync failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    
    Ok(())
}

/// Synchronize Composer dependencies
async fn sync_composer(workspace: &Workspace) -> Result<()> {
    let composer_json = workspace.root().join("composer.json");
    if !composer_json.exists() {
        return Ok(());
    }
    
    let mut cmd = tokio::process::Command::new("composer");
    cmd.current_dir(workspace.root());
    cmd.arg("install");
    
    let output = cmd.output().await?;
    if !output.status.success() {
        return Err(anyhow!("Composer sync failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    
    Ok(())
}

/// Synchronize system dependencies
async fn sync_system(workspace: &Workspace) -> Result<()> {
    // System dependencies are handled individually
    // Check if any system dependencies are missing and prompt for installation
    let dependencies = workspace.list_dependencies();
    let system_deps: Vec<_> = dependencies
        .iter()
        .filter(|(_, spec)| spec.manager == "system")
        .collect();
    
    if system_deps.is_empty() {
        return Ok(());
    }
    
    println!("{}", style(format!("Found {} system dependencies", system_deps.len())).blue());
    println!("System dependencies should be installed manually or using:");
    for (name, _) in system_deps {
        println!("  rcm system install {}", name);
    }
    
    Ok(())
}

/// Clean workspace build artifacts
async fn clean_workspace(workspace: &Workspace) -> Result<()> {
    println!("{}", style("🧹 Cleaning workspace...").cyan().bold());
    
    let enabled_managers = workspace.enabled_managers();
    let mut cleaned_items = Vec::new();
    
    for manager in &enabled_managers {
        match manager.as_str() {
            "cargo" => {
                if let Err(e) = clean_cargo(workspace).await {
                    println!("{}", style(format!("⚠️ Failed to clean Cargo: {}", e)).yellow());
                } else {
                    cleaned_items.push("Cargo build artifacts");
                }
            }
            "npm" => {
                if let Err(e) = clean_npm(workspace).await {
                    println!("{}", style(format!("⚠️ Failed to clean NPM: {}", e)).yellow());
                } else {
                    cleaned_items.push("NPM cache and node_modules");
                }
            }
            "composer" => {
                if let Err(e) = clean_composer(workspace).await {
                    println!("{}", style(format!("⚠️ Failed to clean Composer: {}", e)).yellow());
                } else {
                    cleaned_items.push("Composer vendor directory");
                }
            }
            _ => {}
        }
    }
    
    // Clean RCM cache
    let cache_dir = workspace.root().join(".rcm").join("cache");
    if cache_dir.exists() {
        tokio::fs::remove_dir_all(&cache_dir).await?;
        cleaned_items.push("RCM cache");
    }
    
    // Clean temporary files
    let temp_dir = workspace.root().join(".rcm").join("temp");
    if temp_dir.exists() {
        tokio::fs::remove_dir_all(&temp_dir).await?;
        cleaned_items.push("Temporary files");
    }
    
    if cleaned_items.is_empty() {
        println!("{}", style("✨ Workspace already clean!").green());
    } else {
        println!("{}", style("✅ Cleaned:").green().bold());
        for item in cleaned_items {
            println!("  • {}", item);
        }
    }
    
    Ok(())
}

/// Clean Cargo artifacts
async fn clean_cargo(workspace: &Workspace) -> Result<()> {
    let cargo_toml = workspace.root().join("Cargo.toml");
    if !cargo_toml.exists() {
        return Ok(());
    }
    
    let mut cmd = tokio::process::Command::new("cargo");
    cmd.current_dir(workspace.root());
    cmd.arg("clean");
    
    let output = cmd.output().await?;
    if !output.status.success() {
        return Err(anyhow!("Cargo clean failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    
    Ok(())
}

/// Clean NPM artifacts
async fn clean_npm(workspace: &Workspace) -> Result<()> {
    let package_json = workspace.root().join("package.json");
    if !package_json.exists() {
        return Ok(());
    }
    
    // Remove node_modules
    let node_modules = workspace.root().join("node_modules");
    if node_modules.exists() {
        tokio::fs::remove_dir_all(&node_modules).await?;
    }
    
    // Remove package-lock.json
    let package_lock = workspace.root().join("package-lock.json");
    if package_lock.exists() {
        tokio::fs::remove_file(&package_lock).await?;
    }
    
    Ok(())
}

/// Clean Composer artifacts
async fn clean_composer(workspace: &Workspace) -> Result<()> {
    let composer_json = workspace.root().join("composer.json");
    if !composer_json.exists() {
        return Ok(());
    }
    
    // Remove vendor directory
    let vendor = workspace.root().join("vendor");
    if vendor.exists() {
        tokio::fs::remove_dir_all(&vendor).await?;
    }
    
    // Remove composer.lock
    let composer_lock = workspace.root().join("composer.lock");
    if composer_lock.exists() {
        tokio::fs::remove_file(&composer_lock).await?;
    }
    
    Ok(())
}

/// Update all packages
async fn update_packages(workspace: &Workspace) -> Result<()> {
    println!("{}", style("📈 Updating all packages...").cyan().bold());
    
    let enabled_managers = workspace.enabled_managers();
    let mut update_results = Vec::new();
    
    for manager in &enabled_managers {
        println!("{}", style(format!("🔄 Updating {} packages...", manager)).blue());
        
        let result = match manager.as_str() {
            "cargo" => update_cargo(workspace).await,
            "npm" => update_npm(workspace).await,
            "composer" => update_composer(workspace).await,
            "system" => update_system(workspace).await,
            _ => Err(anyhow!("Unknown manager: {}", manager)),
        };
        
        match result {
            Ok(_) => {
                println!("{}", style(format!("✅ {} packages updated", manager)).green());
                update_results.push((manager.clone(), true));
            }
            Err(e) => {
                println!("{}", style(format!("❌ {} update failed: {}", manager, e)).red());
                update_results.push((manager.clone(), false));
            }
        }
    }
    
    // Print summary
    println!();
    let successful = update_results.iter().filter(|(_, success)| *success).count();
    let total = update_results.len();
    
    if successful == total {
        println!("{}", style("✅ All packages updated successfully!").green().bold());
    } else {
        println!("{}", style(format!("⚠️ {}/{} package managers updated", successful, total)).yellow().bold());
    }
    
    Ok(())
}

/// Update Cargo packages
async fn update_cargo(workspace: &Workspace) -> Result<()> {
    let cargo_toml = workspace.root().join("Cargo.toml");
    if !cargo_toml.exists() {
        return Ok(());
    }
    
    let mut cmd = tokio::process::Command::new("cargo");
    cmd.current_dir(workspace.root());
    cmd.arg("update");
    
    let output = cmd.output().await?;
    if !output.status.success() {
        return Err(anyhow!("Cargo update failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    
    Ok(())
}

/// Update NPM packages
async fn update_npm(workspace: &Workspace) -> Result<()> {
    let package_json = workspace.root().join("package.json");
    if !package_json.exists() {
        return Ok(());
    }
    
    let mut cmd = tokio::process::Command::new("npm");
    cmd.current_dir(workspace.root());
    cmd.arg("update");
    
    let output = cmd.output().await?;
    if !output.status.success() {
        return Err(anyhow!("NPM update failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    
    Ok(())
}

/// Update Composer packages
async fn update_composer(workspace: &Workspace) -> Result<()> {
    let composer_json = workspace.root().join("composer.json");
    if !composer_json.exists() {
        return Ok(());
    }
    
    let mut cmd = tokio::process::Command::new("composer");
    cmd.current_dir(workspace.root());
    cmd.arg("update");
    
    let output = cmd.output().await?;
    if !output.status.success() {
        return Err(anyhow!("Composer update failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    
    Ok(())
}

/// Update system packages
async fn update_system(workspace: &Workspace) -> Result<()> {
    let system_manager = SystemManager::new(workspace.root()).await?;
    system_manager.update(false, false).await?;
    Ok(())
}

/// Check workspace health
async fn check_workspace(workspace: &Workspace) -> Result<()> {
    println!("{}", style("🏥 Checking workspace health...").cyan().bold());
    
    let summary = workspace.get_summary().await?;
    
    // Print health metrics
    println!();
    println!("{}", style("📊 Workspace Metrics").bold());
    println!("{}", style("─".repeat(40)).dim());
    
    println!("📦 Total dependencies: {}", style(summary.total_dependencies).bold());
    println!("💾 Disk usage: {:.1} MB", summary.disk_usage_mb);
    println!("🎯 Health score: {:.0}%", summary.health_score);
    
    if summary.health_score >= 90.0 {
        println!("{}", style("✅ Excellent health!").green().bold());
    } else if summary.health_score >= 70.0 {
        println!("{}", style("⚠️ Good health with minor issues").yellow().bold());
    } else {
        println!("{}", style("❌ Poor health - needs attention").red().bold());
    }
    
    // Show dependencies by manager
    if !summary.dependencies_by_manager.is_empty() {
        println!();
        println!("{}", style("📦 Dependencies by Manager").bold());
        for (manager, count) in &summary.dependencies_by_manager {
            println!("  • {}: {} packages", style(manager).cyan(), count);
        }
    }
    
    // Show security vulnerabilities if any
    if summary.security_vulnerabilities > 0 {
        println!();
        println!("{}", 
            style(format!("⚠️ {} security vulnerabilities found", summary.security_vulnerabilities))
                .red().bold()
        );
        println!("Run {} to scan for vulnerabilities", style("rcm audit").cyan());
    }
    
    // Show outdated dependencies if any
    if !summary.outdated_dependencies.is_empty() {
        println!();
        println!("{}", style("📈 Outdated Dependencies").yellow().bold());
        for dep in &summary.outdated_dependencies {
            println!("  • {}", dep);
        }
        println!("Run {} to update packages", style("rcm workspace update").cyan());
    }
    
    // Recommendations
    println!();
    println!("{}", style("💡 Recommendations").bold());
    
    if summary.health_score < 90.0 {
        println!("  • Run {} to install missing dependencies", style("rcm ensure").cyan());
    }
    
    if summary.disk_usage_mb > 1000.0 {
        println!("  • Run {} to clean build artifacts", style("rcm workspace clean").cyan());
    }
    
    if summary.total_dependencies > 100 {
        println!("  • Consider reducing dependencies for better maintainability");
    }
    
    println!("  • Run {} to keep packages up to date", style("rcm workspace update").cyan());
    
    Ok(())
}
