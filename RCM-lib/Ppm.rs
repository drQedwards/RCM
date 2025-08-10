//! PPM (PHP Package Manager) integration for RCM
//! 
//! Provides Composer package management for PHP projects

use anyhow::{anyhow, Context, Result};
use clap::Subcommand;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::fs;
use crate::workspace::Workspace;
use crate::util::{self, execute_command, validate_package_name};

#[derive(Subcommand)]
pub enum PpmCommands {
    /// Install PHP packages via Composer
    Install {
        /// Packages to install (vendor/name[:version])
        packages: Vec<String>,
        /// Install as dev dependencies
        #[arg(long)]
        dev: bool,
        /// Global installation
        #[arg(long)]
        global: bool,
        /// Optimize autoloader
        #[arg(long)]
        optimize: bool,
    },
    
    /// Remove PHP packages
    Remove {
        /// Packages to remove
        packages: Vec<String>,
        /// Remove from dev dependencies
        #[arg(long)]
        dev: bool,
        /// Optimize autoloader after removal
        #[arg(long)]
        optimize: bool,
    },
    
    /// Update PHP packages
    Update {
        /// Specific packages to update (all if empty)
        packages: Vec<String>,
        /// Update with dependencies
        #[arg(long)]
        with_dependencies: bool,
        /// Optimize autoloader
        #[arg(long)]
        optimize: bool,
    },
    
    /// Show installed packages
    Show {
        /// Show specific package info
        package: Option<String>,
        /// Show only installed packages
        #[arg(long)]
        installed: bool,
        /// Show platform packages
        #[arg(long)]
        platform: bool,
        /// Output format (json, table)
        #[arg(long, default_value = "table")]
        format: String,
    },
    
    /// Initialize composer.json
    Init {
        /// Package name (vendor/name)
        #[arg(long)]
        name: Option<String>,
        /// Package description
        #[arg(long)]
        description: Option<String>,
        /// Author name <email>
        #[arg(long)]
        author: Option<String>,
        /// Package type (library, project, etc.)
        #[arg(long, default_value = "project")]
        package_type: String,
        /// Minimum PHP version
        #[arg(long)]
        php_version: Option<String>,
    },
    
    /// Run Composer scripts
    Run {
        /// Script name
        script: String,
        /// Additional arguments
        args: Vec<String>,
    },
    
    /// Validate composer.json
    Validate {
        /// Strict validation
        #[arg(long)]
        strict: bool,
    },
    
    /// Check for security vulnerabilities
    Audit {
        /// Output format (text, json)
        #[arg(long, default_value = "text")]
        format: String,
    },
    
    /// Generate autoloader files
    DumpAutoload {
        /// Optimize autoloader (PSR-4/PSR-0)
        #[arg(long)]
        optimize: bool,
        /// Enable APCu caching
        #[arg(long)]
        apcu: bool,
        /// Authoritative class maps
        #[arg(long)]
        classmap_authoritative: bool,
    },
    
    /// Search for packages
    Search {
        /// Search terms
        terms: Vec<String>,
        /// Only search names
        #[arg(long)]
        only_name: bool,
    },
    
    /// Create new PHP project from template
    Create {
        /// Package template (vendor/name)
        template: String,
        /// Target directory
        directory: String,
        /// Prefer stable packages
        #[arg(long)]
        stability: Option<String>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComposerJson {
    pub name: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub package_type: Option<String>,
    pub version: Option<String>,
    pub license: Option<serde_json::Value>,
    pub authors: Option<Vec<ComposerAuthor>>,
    pub require: Option<HashMap<String, String>>,
    #[serde(rename = "require-dev")]
    pub require_dev: Option<HashMap<String, String>>,
    pub autoload: Option<ComposerAutoload>,
    #[serde(rename = "autoload-dev")]
    pub autoload_dev: Option<ComposerAutoload>,
    pub scripts: Option<HashMap<String, serde_json::Value>>,
    pub config: Option<HashMap<String, serde_json::Value>>,
    pub repositories: Option<Vec<serde_json::Value>>,
    pub minimum_stability: Option<String>,
    #[serde(rename = "prefer-stable")]
    pub prefer_stable: Option<bool>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComposerAuthor {
    pub name: String,
    pub email: Option<String>,
    pub homepage: Option<String>,
    pub role: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComposerAutoload {
    #[serde(rename = "psr-4")]
    pub psr4: Option<HashMap<String, serde_json::Value>>,
    #[serde(rename = "psr-0")]
    pub psr0: Option<HashMap<String, serde_json::Value>>,
    pub classmap: Option<Vec<String>>,
    pub files: Option<Vec<String>>,
    #[serde(rename = "exclude-from-classmap")]
    pub exclude_from_classmap: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComposerLock {
    #[serde(rename = "_readme")]
    pub readme: Option<Vec<String>>,
    #[serde(rename = "content-hash")]
    pub content_hash: String,
    pub packages: Vec<ComposerPackage>,
    #[serde(rename = "packages-dev")]
    pub packages_dev: Vec<ComposerPackage>,
    pub aliases: Vec<serde_json::Value>,
    #[serde(rename = "minimum-stability")]
    pub minimum_stability: String,
    #[serde(rename = "stability-flags")]
    pub stability_flags: HashMap<String, i32>,
    #[serde(rename = "prefer-stable")]
    pub prefer_stable: bool,
    #[serde(rename = "prefer-lowest")]
    pub prefer_lowest: bool,
    pub platform: HashMap<String, String>,
    #[serde(rename = "platform-dev")]
    pub platform_dev: HashMap<String, String>,
    #[serde(rename = "plugin-api-version")]
    pub plugin_api_version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComposerPackage {
    pub name: String,
    pub version: String,
    pub source: Option<ComposerSource>,
    pub dist: Option<ComposerDist>,
    pub require: Option<HashMap<String, String>>,
    #[serde(rename = "require-dev")]
    pub require_dev: Option<HashMap<String, String>>,
    #[serde(rename = "type")]
    pub package_type: Option<String>,
    pub autoload: Option<ComposerAutoload>,
    pub license: Option<Vec<String>>,
    pub authors: Option<Vec<ComposerAuthor>>,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub time: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComposerSource {
    #[serde(rename = "type")]
    pub source_type: String,
    pub url: String,
    pub reference: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComposerDist {
    #[serde(rename = "type")]
    pub dist_type: String,
    pub url: String,
    pub reference: Option<String>,
    pub shasum: Option<String>,
}

#[derive(Debug)]
pub struct ComposerManager {
    workspace_root: PathBuf,
    composer_json_path: PathBuf,
    composer_lock_path: PathBuf,
    vendor_path: PathBuf,
}

impl ComposerManager {
    pub fn new(workspace_root: &Path) -> Self {
        let composer_json_path = workspace_root.join("composer.json");
        let composer_lock_path = workspace_root.join("composer.lock");
        let vendor_path = workspace_root.join("vendor");
        
        Self {
            workspace_root: workspace_root.to_path_buf(),
            composer_json_path,
            composer_lock_path,
            vendor_path,
        }
    }
    
    /// Check if PHP and Composer are available
    pub async fn check_environment(&self) -> Result<()> {
        // Check PHP
        if !util::command_exists("php").await {
            return Err(anyhow!("PHP is not installed or not in PATH"));
        }
        
        // Check Composer
        if !util::command_exists("composer").await {
            return Err(anyhow!("Composer is not installed or not in PATH"));
        }
        
        // Check PHP version
        let output = Command::new("php")
            .args(&["-v"])
            .output()
            .await
            .context("Failed to check PHP version")?;
        
        if !output.status.success() {
            return Err(anyhow!("Failed to get PHP version"));
        }
        
        let version_output = String::from_utf8_lossy(&output.stdout);
        if !version_output.contains("PHP") {
            return Err(anyhow!("Invalid PHP installation"));
        }
        
        Ok(())
    }
    
    /// Load composer.json
    pub async fn load_composer_json(&self) -> Result<ComposerJson> {
        if !self.composer_json_path.exists() {
            return Ok(ComposerJson {
                name: None,
                description: None,
                package_type: Some("project".to_string()),
                version: None,
                license: None,
                authors: None,
                require: None,
                require_dev: None,
                autoload: None,
                autoload_dev: None,
                scripts: None,
                config: None,
                repositories: None,
                minimum_stability: None,
                prefer_stable: None,
                extra: HashMap::new(),
            });
        }
        
        let content = fs::read_to_string(&self.composer_json_path).await
            .context("Failed to read composer.json")?;
        
        serde_json::from_str(&content)
            .context("Failed to parse composer.json")
    }
    
    /// Save composer.json
    pub async fn save_composer_json(&self, composer_json: &ComposerJson) -> Result<()> {
        let content = serde_json::to_string_pretty(composer_json)
            .context("Failed to serialize composer.json")?;
        
        fs::write(&self.composer_json_path, content).await
            .context("Failed to write composer.json")
    }
    
    /// Install packages
    pub async fn install(&self, packages: &[String], dev: bool, global: bool, optimize: bool) -> Result<()> {
        self.check_environment().await?;
        
        let mut cmd = Command::new("composer");
        cmd.current_dir(&self.workspace_root);
        
        if global {
            cmd.arg("global");
        }
        
        cmd.arg("require");
        
        if dev {
            cmd.arg("--dev");
        }
        
        if optimize {
            cmd.arg("--optimize-autoloader");
        }
        
        cmd.args(packages);
        
        execute_command(&mut cmd).await
            .context("Failed to install composer packages")
    }
    
    /// Remove packages
    pub async fn remove(&self, packages: &[String], dev: bool, optimize: bool) -> Result<()> {
        self.check_environment().await?;
        
        let mut cmd = Command::new("composer");
        cmd.current_dir(&self.workspace_root);
        cmd.arg("remove");
        
        if dev {
            cmd.arg("--dev");
        }
        
        if optimize {
            cmd.arg("--optimize-autoloader");
        }
        
        cmd.args(packages);
        
        execute_command(&mut cmd).await
            .context("Failed to remove composer packages")
    }
    
    /// Update packages
    pub async fn update(&self, packages: &[String], with_dependencies: bool, optimize: bool) -> Result<()> {
        self.check_environment().await?;
        
        let mut cmd = Command::new("composer");
        cmd.current_dir(&self.workspace_root);
        cmd.arg("update");
        
        if with_dependencies {
            cmd.arg("--with-dependencies");
        }
        
        if optimize {
            cmd.arg("--optimize-autoloader");
        }
        
        if !packages.is_empty() {
            cmd.args(packages);
        }
        
        execute_command(&mut cmd).await
            .context("Failed to update composer packages")
    }
    
    /// Run composer script
    pub async fn run_script(&self, script: &str, args: &[String]) -> Result<()> {
        self.check_environment().await?;
        
        let mut cmd = Command::new("composer");
        cmd.current_dir(&self.workspace_root);
        cmd.arg("run-script");
        cmd.arg(script);
        
        if !args.is_empty() {
            cmd.arg("--");
            cmd.args(args);
        }
        
        execute_command(&mut cmd).await
            .context("Failed to run composer script")
    }
    
    /// Validate composer.json
    pub async fn validate(&self, strict: bool) -> Result<()> {
        self.check_environment().await?;
        
        let mut cmd = Command::new("composer");
        cmd.current_dir(&self.workspace_root);
        cmd.arg("validate");
        
        if strict {
            cmd.arg("--strict");
        }
        
        execute_command(&mut cmd).await
            .context("Failed to validate composer.json")
    }
    
    /// Generate autoloader
    pub async fn dump_autoload(&self, optimize: bool, apcu: bool, authoritative: bool) -> Result<()> {
        self.check_environment().await?;
        
        let mut cmd = Command::new("composer");
        cmd.current_dir(&self.workspace_root);
        cmd.arg("dump-autoload");
        
        if optimize {
            cmd.arg("--optimize");
        }
        
        if apcu {
            cmd.arg("--apcu");
        }
        
        if authoritative {
            cmd.arg("--classmap-authoritative");
        }
        
        execute_command(&mut cmd).await
            .context("Failed to generate autoloader")
    }
    
    /// Search for packages
    pub async fn search(&self, terms: &[String], only_name: bool) -> Result<()> {
        self.check_environment().await?;
        
        let mut cmd = Command::new("composer");
        cmd.current_dir(&self.workspace_root);
        cmd.arg("search");
        
        if only_name {
            cmd.arg("--only-name");
        }
        
        cmd.args(terms);
        
        execute_command(&mut cmd).await
            .context("Failed to search composer packages")
    }
    
    /// Create project from template
    pub async fn create_project(&self, template: &str, directory: &str, stability: Option<&str>) -> Result<()> {
        self.check_environment().await?;
        
        let mut cmd = Command::new("composer");
        cmd.arg("create-project");
        cmd.arg(template);
        cmd.arg(directory);
        
        if let Some(stability) = stability {
            cmd.arg("--stability");
            cmd.arg(stability);
        }
        
        execute_command(&mut cmd).await
            .context("Failed to create composer project")
    }
    
    /// Validate PHP package name
    pub fn validate_package_name(name: &str) -> Result<()> {
        // Composer package name validation (vendor/name format)
        let composer_regex = Regex::new(r"^[a-z0-9]([_.-]?[a-z0-9]+)*/[a-z0-9]([_.-]?[a-z0-9]+)*$")?;
        
        if !composer_regex.is_match(name) {
            return Err(anyhow!("Invalid composer package name: {}. Must be in vendor/name format", name));
        }
        
        let parts: Vec<&str> = name.split('/').collect();
        if parts.len() != 2 {
            return Err(anyhow!("Package name must contain exactly one slash"));
        }
        
        let (vendor, package) = (parts[0], parts[1]);
        
        if vendor.len() < 2 || package.len() < 2 {
            return Err(anyhow!("Vendor and package names must be at least 2 characters"));
        }
        
        Ok(())
    }
}

/// Handle PPM commands
pub async fn handle_command(workspace: &Workspace, cmd: PpmCommands) -> Result<()> {
    match cmd {
        PpmCommands::Install { packages, dev, global, optimize } => {
            let composer = ComposerManager::new(workspace.root());
            
            // Validate package names
            for package in &packages {
                let name = package.split(':').next().unwrap_or(package);
                ComposerManager::validate_package_name(name)?;
            }
            
            composer.install(&packages, dev, global, optimize).await
        }
        
        PpmCommands::Remove { packages, dev, optimize } => {
            let composer = ComposerManager::new(workspace.root());
            composer.remove(&packages, dev, optimize).await
        }
        
        PpmCommands::Update { packages, with_dependencies, optimize } => {
            let composer = ComposerManager::new(workspace.root());
            composer.update(&packages, with_dependencies, optimize).await
        }
        
        PpmCommands::Show { package: _, installed: _, platform: _, format: _ } => {
            // Implementation for showing packages
            println!("PPM show functionality not yet implemented");
            Ok(())
        }
        
        PpmCommands::Init { name, description, author, package_type, php_version } => {
            let composer = ComposerManager::new(workspace.root());
            let mut composer_json = ComposerJson {
                name,
                description,
                package_type: Some(package_type),
                version: None,
                license: Some(serde_json::Value::String("proprietary".to_string())),
                authors: author.map(|a| vec![ComposerAuthor {
                    name: a,
                    email: None,
                    homepage: None,
                    role: None,
                }]),
                require: Some(HashMap::from([
                    ("php".to_string(), php_version.unwrap_or("^8.1".to_string())),
                ])),
                require_dev: Some(HashMap::new()),
                autoload: Some(ComposerAutoload {
                    psr4: Some(HashMap::new()),
                    psr0: None,
                    classmap: None,
                    files: None,
                    exclude_from_classmap: None,
                }),
                autoload_dev: None,
                scripts: None,
                config: None,
                repositories: None,
                minimum_stability: Some("stable".to_string()),
                prefer_stable: Some(true),
                extra: HashMap::new(),
            };
            
            composer.save_composer_json(&composer_json).await
        }
        
        PpmCommands::Run { script, args } => {
            let composer = ComposerManager::new(workspace.root());
            composer.run_script(&script, &args).await
        }
        
        PpmCommands::Validate { strict } => {
            let composer = ComposerManager::new(workspace.root());
            composer.validate(strict).await
        }
        
        PpmCommands::Audit { format: _ } => {
            // Implementation for security audit
            println!("PPM audit functionality not yet implemented");
            Ok(())
        }
        
        PpmCommands::DumpAutoload { optimize, apcu, classmap_authoritative } => {
            let composer = ComposerManager::new(workspace.root());
            composer.dump_autoload(optimize, apcu, classmap_authoritative).await
        }
        
        PpmCommands::Search { terms, only_name } => {
            let composer = ComposerManager::new(workspace.root());
            composer.search(&terms, only_name).await
        }
        
        PpmCommands::Create { template, directory, stability } => {
            let composer = ComposerManager::new(workspace.root());
            composer.create_project(&template, &directory, stability.as_deref()).await
        }
    }
}
