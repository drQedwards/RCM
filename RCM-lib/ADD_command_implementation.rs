//! Add command implementation
//! 
//! Adds packages to the workspace with automatic manager detection

use anyhow::{anyhow, Context, Result};
use console::style;
use dialoguer::{Confirm, Select};
use regex::Regex;
use crate::workspace::Workspace;
use crate::npm::{NpmManager, NpmManagerType};
use crate::ppm::ComposerManager;
use crate::system::SystemManager;
use crate::util::validate_package_name;

/// Add a package to the workspace
pub async fn run(
    workspace: &Workspace,
    spec: &str,
    manager: Option<&str>,
    dev: bool,
) -> Result<()> {
    println!("{}", style(format!("📦 Adding package: {}", spec)).cyan().bold());
    
    // Parse package specification
    let (package_name, version, detected_manager) = parse_package_spec(spec)?;
    
    // Determine which manager to use
    let target_manager = if let Some(mgr) = manager {
        mgr.to_string()
    } else if let Some(mgr) = detected_manager {
        mgr
    } else {
        // Auto-detect based on workspace and package name
        detect_manager(workspace, &package_name).await?
    };
    
    println!("{}", style(format!("🔍 Using manager: {}", target_manager)).blue());
    
    // Validate manager is enabled
    if !workspace.has_manager(&target_manager) {
        return Err(anyhow!(
            "Manager '{}' is not enabled in this workspace. Run 'rcm init' to configure managers.",
            target_manager
        ));
    }
    
    // Install package using appropriate manager
    match target_manager.as_str() {
        "cargo" => install_cargo_package(workspace, &package_name, &version, dev).await?,
        "npm" => install_npm_package(workspace, &package_name, &version, dev).await?,
        "composer" => install_composer_package(workspace, &package_name, &version, dev).await?,
        "system" => install_system_package(workspace, &package_name).await?,
        _ => return Err(anyhow!("Unsupported package manager: {}", target_manager)),
    }
    
    // Update workspace manifest
    let mut workspace_mut = workspace.clone();
    workspace_mut.add_dependency(&package_name, &version, &target_manager, dev).await?;
    
    println!("{}", style(format!("✅ Successfully added {} ({})", package_name, target_manager)).green().bold());
    
    // Suggest related packages
    suggest_related_packages(&target_manager, &package_name).await?;
    
    Ok(())
}

/// Parse package specification (name[@version] or manager:name[@version])
fn parse_package_spec(spec: &str) -> Result<(String, String, Option<String>)> {
    // Check for manager prefix (e.g., npm:package@1.0.0)
    if let Some((manager, rest)) = spec.split_once(':') {
        let (name, version) = if let Some((n, v)) = rest.split_once('@') {
            (n.to_string(), v.to_string())
        } else {
            (rest.to_string(), "latest".to_string())
        };
        
        validate_package_name(&name)?;
        return Ok((name, version, Some(manager.to_string())));
    }
    
    // Parse name@version
    let (name, version) = if let Some((n, v)) = spec.split_once('@') {
        (n.to_string(), v.to_string())
    } else {
        (spec.to_string(), "latest".to_string())
    };
    
    validate_package_name(&name)?;
    Ok((name, version, None))
}

/// Auto-detect appropriate package manager
async fn detect_manager(workspace: &Workspace, package_name: &str) -> Result<String> {
    let enabled_managers = workspace.enabled_managers();
    
    // Heuristics for package manager detection
    let mut candidates = Vec::new();
    
    // Cargo patterns
    if enabled_managers.contains(&"cargo".to_string()) {
        if is_cargo_package(package_name) {
            candidates.push(("cargo", 90));
        }
    }
    
    // NPM patterns
    if enabled_managers.contains(&"npm".to_string()) {
        if is_npm_package(package_name) {
            candidates.push(("npm", 85));
        }
    }
    
    // Composer patterns
    if enabled_managers.contains(&"composer".to_string()) {
        if is_composer_package(package_name) {
            candidates.push(("composer", 85));
        }
    }
    
    // System package patterns
    if enabled_managers.contains(&"system".to_string()) {
        if is_system_package(package_name) {
            candidates.push(("system", 70));
        }
    }
    
    // If no strong candidates, check workspace context
    if candidates.is_empty() {
        candidates = detect_by_workspace_context(workspace);
    }
    
    // If still ambiguous, ask user
    if candidates.len() > 1 || candidates.is_empty() {
        return interactive_manager_selection(&enabled_managers).await;
    }
    
    Ok(candidates[0].0.to_string())
}

/// Check if package name matches Cargo patterns
fn is_cargo_package(name: &str) -> bool {
    // Rust crates often use kebab-case and certain prefixes
    let rust_patterns = [
        "serde", "tokio", "async", "clap", "anyhow", "thiserror", "log",
        "env_logger", "reqwest", "hyper", "axum", "warp", "actix",
    ];
    
    // Check for common Rust package patterns
    if rust_patterns.iter().any(|&pattern| name.contains(pattern)) {
        return true;
    }
    
    // Rust packages often use snake_case or kebab-case
    let rust_regex = Regex::new(r"^[a-z][a-z0-9_-]*$").unwrap();
    rust_regex.is_match(name) && !name.contains('/')
}

/// Check if package name matches NPM patterns
fn is_npm_package(name: &str) -> bool {
    // NPM packages with scopes
    if name.starts_with('@') {
        return true;
    }
    
    // Common NPM package patterns
    let npm_patterns = [
        "react", "vue", "angular", "express", "webpack", "babel", "eslint",
        "prettier", "jest", "mocha", "lodash", "axios", "moment",
    ];
    
    if npm_patterns.iter().any(|&pattern| name.contains(pattern)) {
        return true;
    }
    
    // NPM packages often use kebab-case
    let npm_regex = Regex::new(r"^[a-z][a-z0-9-]*$").unwrap();
    npm_regex.is_match(name)
}

/// Check if package name matches Composer patterns
fn is_composer_package(name: &str) -> bool {
    // Composer packages always use vendor/package format
    if name.contains('/') {
        let composer_regex = Regex::new(r"^[a-z0-9]([_.-]?[a-z0-9]+)*/[a-z0-9]([_.-]?[a-z0-9]+)*$").unwrap();
        return composer_regex.is_match(name);
    }
    
    // Common PHP framework/library names
    let php_patterns = [
        "symfony", "laravel", "doctrine", "phpunit", "monolog", "guzzle",
        "twig", "composer", "psr", "php",
    ];
    
    php_patterns.iter().any(|&pattern| name.contains(pattern))
}

/// Check if package name matches system package patterns
fn is_system_package(name: &str) -> bool {
    let system_packages = [
        "ffmpeg", "git", "curl", "wget", "nginx", "apache", "mysql", "postgresql",
        "redis", "docker", "kubernetes", "python", "node", "php", "java",
        "golang", "ruby", "perl", "make", "gcc", "cmake", "vim", "emacs",
        "htop", "tree", "jq", "rsync", "ssh", "gpg",
    ];
    
    system_packages.contains(&name)
}

/// Detect manager by workspace context
fn detect_by_workspace_context(workspace: &Workspace) -> Vec<(&str, i32)> {
    let mut candidates = Vec::new();
    
    // Check for project files
    if workspace.root().join("Cargo.toml").exists() {
        candidates.push(("cargo", 80));
    }
    
    if workspace.root().join("package.json").exists() {
        candidates.push(("npm", 80));
    }
    
    if workspace.root().join("composer.json").exists() {
        candidates.push(("composer", 80));
    }
    
    // Always consider system as fallback
    candidates.push(("system", 50));
    
    candidates
}

/// Interactive manager selection
async fn interactive_manager_selection(enabled_managers: &[String]) -> Result<String> {
    println!("{}", style("🤔 Multiple package managers could handle this package.").yellow());
    
    let options: Vec<String> = enabled_managers.iter().map(|m| {
        match m.as_str() {
            "cargo" => "🦀 Cargo (Rust)".to_string(),
            "npm" => "📦 NPM (Node.js)".to_string(),
            "composer" => "🐘 Composer (PHP)".to_string(),
            "system" => "🔧 System (OS packages)".to_string(),
            _ => format!("📋 {}", m),
        }
    }).collect();
    
    let selection = Select::new()
        .with_prompt("Select package manager")
        .items(&options)
        .default(0)
        .interact()?;
    
    Ok(enabled_managers[selection].clone())
}

/// Install Cargo package
async fn install_cargo_package(
    workspace: &Workspace,
    name: &str,
    version: &str,
    dev: bool,
) -> Result<()> {
    let cargo_toml = workspace.root().join("Cargo.toml");
    if !cargo_toml.exists() {
        return Err(anyhow!("No Cargo.toml found. Run 'rcm init --managers cargo' first."));
    }
    
    println!("{}", style("🔧 Installing Rust crate...").blue());
    
    let mut cmd = tokio::process::Command::new("cargo");
    cmd.current_dir(workspace.root());
    cmd.arg("add");
    cmd.arg(if version == "latest" {
        name.to_string()
    } else {
        format!("{}@{}", name, version)
    });
    
    if dev {
        cmd.arg("--dev");
    }
    
    let output = cmd.output().await
        .context("Failed to execute cargo add")?;
    
    if !output.status.success() {
        return Err(anyhow!(
            "Cargo add failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    
    println!("{}", style("✅ Cargo package installed").green());
    Ok(())
}

/// Install NPM package
async fn install_npm_package(
    workspace: &Workspace,
    name: &str,
    version: &str,
    dev: bool,
) -> Result<()> {
    let package_json = workspace.root().join("package.json");
    if !package_json.exists() {
        return Err(anyhow!("No package.json found. Run 'rcm init --managers npm' first."));
    }
    
    println!("{}", style("🔧 Installing NPM package...").blue());
    
    let npm_manager = NpmManager::new(workspace.root(), NpmManagerType::Npm);
    let packages = vec![if version == "latest" {
        name.to_string()
    } else {
        format!("{}@{}", name, version)
    }];
    
    npm_manager.install(&packages, dev, false).await?;
    
    println!("{}", style("✅ NPM package installed").green());
    Ok(())
}

/// Install Composer package
async fn install_composer_package(
    workspace: &Workspace,
    name: &str,
    version: &str,
    dev: bool,
) -> Result<()> {
    let composer_json = workspace.root().join("composer.json");
    if !composer_json.exists() {
        return Err(anyhow!("No composer.json found. Run 'rcm init --managers composer' first."));
    }
    
    println!("{}", style("🔧 Installing Composer package...").blue());
    
    let composer = ComposerManager::new(workspace.root());
    let packages = vec![if version == "latest" {
        name.to_string()
    } else {
        format!("{}:{}", name, version)
    }];
    
    composer.install(&packages, dev, false, true).await?;
    
    println!("{}", style("✅ Composer package installed").green());
    Ok(())
}

/// Install system package
async fn install_system_package(workspace: &Workspace, name: &str) -> Result<()> {
    println!("{}", style("🔧 Installing system package...").blue());
    
    let system = SystemManager::new(workspace.root()).await?;
    
    // Ask for confirmation for system package installation
    let confirm = Confirm::new()
        .with_prompt(format!("Install system package '{}'? This may require admin privileges.", name))
        .default(true)
        .interact()?;
    
    if !confirm {
        return Err(anyhow!("System package installation cancelled"));
    }
    
    system.install(&[name.to_string()], false, false).await?;
    
    println!("{}", style("✅ System package installed").green());
    Ok(())
}

/// Suggest related packages that might be useful
async fn suggest_related_packages(manager: &str, package_name: &str) -> Result<()> {
    let suggestions = match manager {
        "cargo" => get_cargo_suggestions(package_name),
        "npm" => get_npm_suggestions(package_name),
        "composer" => get_composer_suggestions(package_name),
        "system" => get_system_suggestions(package_name),
        _ => Vec::new(),
    };
    
    if !suggestions.is_empty() {
        println!();
        println!("{}", style("💡 You might also want to add:").yellow().bold());
        for suggestion in suggestions {
            println!("  • {}", style(suggestion).cyan());
        }
        println!("  Run {} to add them", style("rcm add <package>").cyan());
    }
    
    Ok(())
}

/// Get Cargo package suggestions
fn get_cargo_suggestions(package_name: &str) -> Vec<String> {
    let mut suggestions = Vec::new();
    
    match package_name {
        "tokio" => {
            suggestions.extend([
                "serde".to_string(),
                "anyhow".to_string(),
                "tracing".to_string(),
            ]);
        }
        "serde" => {
            suggestions.extend([
                "serde_json".to_string(),
                "serde_yaml".to_string(),
            ]);
        }
        "clap" => {
            suggestions.extend([
                "anyhow".to_string(),
                "env_logger".to_string(),
            ]);
        }
        "reqwest" => {
            suggestions.extend([
                "serde".to_string(),
                "serde_json".to_string(),
                "tokio".to_string(),
            ]);
        }
        _ => {}
    }
    
    suggestions
}

/// Get NPM package suggestions
fn get_npm_suggestions(package_name: &str) -> Vec<String> {
    let mut suggestions = Vec::new();
    
    match package_name {
        "react" => {
            suggestions.extend([
                "react-dom".to_string(),
                "@types/react".to_string(),
                "typescript".to_string(),
            ]);
        }
        "express" => {
            suggestions.extend([
                "cors".to_string(),
                "helmet".to_string(),
                "morgan".to_string(),
            ]);
        }
        "typescript" => {
            suggestions.extend([
                "@types/node".to_string(),
                "ts-node".to_string(),
            ]);
        }
        _ => {}
    }
    
    suggestions
}

/// Get Composer package suggestions
fn get_composer_suggestions(package_name: &str) -> Vec<String> {
    let mut suggestions = Vec::new();
    
    if package_name.contains("symfony") {
        suggestions.extend([
            "symfony/console".to_string(),
            "symfony/http-foundation".to_string(),
            "doctrine/orm".to_string(),
        ]);
    } else if package_name.contains("laravel") {
        suggestions.extend([
            "laravel/tinker".to_string(),
            "laravel/sanctum".to_string(),
        ]);
    }
    
    suggestions
}

/// Get system package suggestions
fn get_system_suggestions(package_name: &str) -> Vec<String> {
    let mut suggestions = Vec::new();
    
    match package_name {
        "git" => {
            suggestions.extend([
                "curl".to_string(),
                "wget".to_string(),
                "ssh".to_string(),
            ]);
        }
        "docker" => {
            suggestions.extend([
                "docker-compose".to_string(),
            ]);
        }
        "nginx" => {
            suggestions.extend([
                "ssl-cert".to_string(),
                "certbot".to_string(),
            ]);
        }
        "ffmpeg" => {
            suggestions.extend([
                "imagemagick".to_string(),
                "libavcodec-extra".to_string(),
            ]);
        }
        _ => {}
    }
    
    suggestions
}
