//! NPM package management for RCM
//! 
//! Provides integration with Node.js ecosystem via npm, yarn, and pnpm

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
pub enum NpmCommands {
    /// Install NPM packages
    Install {
        /// Packages to install (name[@version])
        packages: Vec<String>,
        /// Install as dev dependencies
        #[arg(long)]
        dev: bool,
        /// Use specific package manager (npm, yarn, pnpm)
        #[arg(long, default_value = "npm")]
        manager: String,
        /// Global installation
        #[arg(long)]
        global: bool,
    },
    
    /// Uninstall NPM packages
    Uninstall {
        /// Packages to uninstall
        packages: Vec<String>,
        /// Package manager to use
        #[arg(long, default_value = "npm")]
        manager: String,
        /// Global uninstallation
        #[arg(long)]
        global: bool,
    },
    
    /// Update NPM packages
    Update {
        /// Specific packages to update (all if empty)
        packages: Vec<String>,
        /// Package manager to use
        #[arg(long, default_value = "npm")]
        manager: String,
    },
    
    /// List installed packages
    List {
        /// Show only top-level dependencies
        #[arg(long)]
        depth: Option<u32>,
        /// Output format (json, tree, table)
        #[arg(long, default_value = "tree")]
        format: String,
        /// Package manager to use
        #[arg(long, default_value = "npm")]
        manager: String,
    },
    
    /// Initialize package.json
    Init {
        /// Package name
        #[arg(long)]
        name: Option<String>,
        /// Package version
        #[arg(long, default_value = "1.0.0")]
        version: String,
        /// Use defaults without prompts
        #[arg(long)]
        yes: bool,
    },
    
    /// Run npm scripts
    Run {
        /// Script name
        script: String,
        /// Additional arguments
        args: Vec<String>,
        /// Package manager to use
        #[arg(long, default_value = "npm")]
        manager: String,
    },
    
    /// Audit packages for vulnerabilities
    Audit {
        /// Auto-fix vulnerabilities
        #[arg(long)]
        fix: bool,
        /// Package manager to use
        #[arg(long, default_value = "npm")]
        manager: String,
    },
    
    /// Show package information
    Info {
        /// Package name
        package: String,
        /// Show specific field
        #[arg(long)]
        field: Option<String>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PackageJson {
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub main: Option<String>,
    pub scripts: Option<HashMap<String, String>>,
    pub dependencies: Option<HashMap<String, String>>,
    #[serde(rename = "devDependencies")]
    pub dev_dependencies: Option<HashMap<String, String>>,
    #[serde(rename = "peerDependencies")]
    pub peer_dependencies: Option<HashMap<String, String>>,
    pub keywords: Option<Vec<String>>,
    pub author: Option<String>,
    pub license: Option<String>,
    pub repository: Option<serde_json::Value>,
    pub bugs: Option<serde_json::Value>,
    pub homepage: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NpmPackageInfo {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub dependencies: HashMap<String, String>,
    pub resolved: String,
    pub integrity: Option<String>,
}

#[derive(Debug)]
pub struct NpmManager {
    workspace_root: PathBuf,
    package_json_path: PathBuf,
    lock_file_path: PathBuf,
    manager_type: NpmManagerType,
}

#[derive(Debug, Clone)]
pub enum NpmManagerType {
    Npm,
    Yarn,
    Pnpm,
}

impl NpmManagerType {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "npm" => Ok(Self::Npm),
            "yarn" => Ok(Self::Yarn),
            "pnpm" => Ok(Self::Pnpm),
            _ => Err(anyhow!("Unsupported npm manager: {}", s)),
        }
    }
    
    pub fn command(&self) -> &'static str {
        match self {
            Self::Npm => "npm",
            Self::Yarn => "yarn",
            Self::Pnpm => "pnpm",
        }
    }
    
    pub fn lock_file(&self) -> &'static str {
        match self {
            Self::Npm => "package-lock.json",
            Self::Yarn => "yarn.lock",
            Self::Pnpm => "pnpm-lock.yaml",
        }
    }
}

impl NpmManager {
    pub fn new(workspace_root: &Path, manager_type: NpmManagerType) -> Self {
        let package_json_path = workspace_root.join("package.json");
        let lock_file_path = workspace_root.join(manager_type.lock_file());
        
        Self {
            workspace_root: workspace_root.to_path_buf(),
            package_json_path,
            lock_file_path,
            manager_type,
        }
    }
    
    /// Check if Node.js and the package manager are available
    pub async fn check_environment(&self) -> Result<()> {
        // Check Node.js
        if !util::command_exists("node").await {
            return Err(anyhow!("Node.js is not installed or not in PATH"));
        }
        
        // Check package manager
        let cmd = self.manager_type.command();
        if !util::command_exists(cmd).await {
            return Err(anyhow!("{} is not installed or not in PATH", cmd));
        }
        
        Ok(())
    }
    
    /// Load package.json
    pub async fn load_package_json(&self) -> Result<PackageJson> {
        if !self.package_json_path.exists() {
            return Ok(PackageJson {
                name: None,
                version: None,
                description: None,
                main: None,
                scripts: None,
                dependencies: None,
                dev_dependencies: None,
                peer_dependencies: None,
                keywords: None,
                author: None,
                license: None,
                repository: None,
                bugs: None,
                homepage: None,
                extra: HashMap::new(),
            });
        }
        
        let content = fs::read_to_string(&self.package_json_path).await
            .context("Failed to read package.json")?;
        
        serde_json::from_str(&content)
            .context("Failed to parse package.json")
    }
    
    /// Save package.json
    pub async fn save_package_json(&self, package_json: &PackageJson) -> Result<()> {
        let content = serde_json::to_string_pretty(package_json)
            .context("Failed to serialize package.json")?;
        
        fs::write(&self.package_json_path, content).await
            .context("Failed to write package.json")
    }
    
    /// Install packages
    pub async fn install(&self, packages: &[String], dev: bool, global: bool) -> Result<()> {
        self.check_environment().await?;
        
        let mut cmd = Command::new(self.manager_type.command());
        cmd.current_dir(&self.workspace_root);
        
        match self.manager_type {
            NpmManagerType::Npm => {
                cmd.arg("install");
                if global {
                    cmd.arg("--global");
                }
                if dev {
                    cmd.arg("--save-dev");
                }
                cmd.args(packages);
            }
            NpmManagerType::Yarn => {
                cmd.arg("add");
                if global {
                    cmd.arg("global");
                }
                if dev {
                    cmd.arg("--dev");
                }
                cmd.args(packages);
            }
            NpmManagerType::Pnpm => {
                cmd.arg("add");
                if global {
                    cmd.arg("--global");
                }
                if dev {
                    cmd.arg("--save-dev");
                }
                cmd.args(packages);
            }
        }
        
        execute_command(&mut cmd).await
            .context("Failed to install npm packages")
    }
    
    /// Uninstall packages
    pub async fn uninstall(&self, packages: &[String], global: bool) -> Result<()> {
        self.check_environment().await?;
        
        let mut cmd = Command::new(self.manager_type.command());
        cmd.current_dir(&self.workspace_root);
        
        match self.manager_type {
            NpmManagerType::Npm => {
                cmd.arg("uninstall");
                if global {
                    cmd.arg("--global");
                }
                cmd.args(packages);
            }
            NpmManagerType::Yarn => {
                cmd.arg("remove");
                if global {
                    cmd.arg("global");
                }
                cmd.args(packages);
            }
            NpmManagerType::Pnpm => {
                cmd.arg("remove");
                if global {
                    cmd.arg("--global");
                }
                cmd.args(packages);
            }
        }
        
        execute_command(&mut cmd).await
            .context("Failed to uninstall npm packages")
    }
    
    /// Update packages
    pub async fn update(&self, packages: &[String]) -> Result<()> {
        self.check_environment().await?;
        
        let mut cmd = Command::new(self.manager_type.command());
        cmd.current_dir(&self.workspace_root);
        
        match self.manager_type {
            NpmManagerType::Npm => {
                cmd.arg("update");
                cmd.args(packages);
            }
            NpmManagerType::Yarn => {
                if packages.is_empty() {
                    cmd.arg("upgrade");
                } else {
                    cmd.arg("upgrade");
                    cmd.args(packages);
                }
            }
            NpmManagerType::Pnpm => {
                cmd.arg("update");
                cmd.args(packages);
            }
        }
        
        execute_command(&mut cmd).await
            .context("Failed to update npm packages")
    }
    
    /// Run npm script
    pub async fn run_script(&self, script: &str, args: &[String]) -> Result<()> {
        self.check_environment().await?;
        
        let mut cmd = Command::new(self.manager_type.command());
        cmd.current_dir(&self.workspace_root);
        
        match self.manager_type {
            NpmManagerType::Npm => {
                cmd.arg("run");
                cmd.arg(script);
                if !args.is_empty() {
                    cmd.arg("--");
                    cmd.args(args);
                }
            }
            NpmManagerType::Yarn => {
                cmd.arg("run");
                cmd.arg(script);
                cmd.args(args);
            }
            NpmManagerType::Pnpm => {
                cmd.arg("run");
                cmd.arg(script);
                cmd.args(args);
            }
        }
        
        execute_command(&mut cmd).await
            .context("Failed to run npm script")
    }
    
    /// Audit packages for vulnerabilities
    pub async fn audit(&self, fix: bool) -> Result<()> {
        self.check_environment().await?;
        
        let mut cmd = Command::new(self.manager_type.command());
        cmd.current_dir(&self.workspace_root);
        
        match self.manager_type {
            NpmManagerType::Npm => {
                cmd.arg("audit");
                if fix {
                    cmd.arg("--fix");
                }
            }
            NpmManagerType::Yarn => {
                cmd.arg("audit");
                if fix {
                    // Yarn doesn't have auto-fix, but we can suggest manual fixes
                    eprintln!("Note: Yarn audit doesn't support auto-fix. Run 'yarn audit' and update packages manually.");
                }
            }
            NpmManagerType::Pnpm => {
                cmd.arg("audit");
                if fix {
                    cmd.arg("--fix");
                }
            }
        }
        
        execute_command(&mut cmd).await
            .context("Failed to audit npm packages")
    }
    
    /// Validate package name
    pub fn validate_package_name(name: &str) -> Result<()> {
        // NPM package name validation
        let npm_regex = Regex::new(r"^(?:@[a-z0-9-_]+/)?[a-z0-9-_]+$")?;
        
        if name.len() > 214 {
            return Err(anyhow!("Package name too long (max 214 characters)"));
        }
        
        if !npm_regex.is_match(name) {
            return Err(anyhow!("Invalid npm package name: {}", name));
        }
        
        // Reserved names
        let reserved = ["node_modules", "favicon.ico"];
        if reserved.contains(&name) {
            return Err(anyhow!("Reserved package name: {}", name));
        }
        
        Ok(())
    }
}

/// Handle NPM commands
pub async fn handle_command(workspace: &Workspace, cmd: NpmCommands) -> Result<()> {
    match cmd {
        NpmCommands::Install { packages, dev, manager, global } => {
            let manager_type = NpmManagerType::from_str(&manager)?;
            let npm_manager = NpmManager::new(workspace.root(), manager_type);
            
            // Validate package names
            for package in &packages {
                let name = package.split('@').next().unwrap_or(package);
                NpmManager::validate_package_name(name)?;
            }
            
            npm_manager.install(&packages, dev, global).await
        }
        
        NpmCommands::Uninstall { packages, manager, global } => {
            let manager_type = NpmManagerType::from_str(&manager)?;
            let npm_manager = NpmManager::new(workspace.root(), manager_type);
            npm_manager.uninstall(&packages, global).await
        }
        
        NpmCommands::Update { packages, manager } => {
            let manager_type = NpmManagerType::from_str(&manager)?;
            let npm_manager = NpmManager::new(workspace.root(), manager_type);
            npm_manager.update(&packages).await
        }
        
        NpmCommands::List { depth: _, format: _, manager: _ } => {
            // Implementation for listing packages
            println!("NPM list functionality not yet implemented");
            Ok(())
        }
        
        NpmCommands::Init { name, version, yes: _ } => {
            let npm_manager = NpmManager::new(workspace.root(), NpmManagerType::Npm);
            let mut package_json = PackageJson {
                name,
                version: Some(version),
                description: Some("Generated by RCM".to_string()),
                main: Some("index.js".to_string()),
                scripts: Some(HashMap::from([
                    ("test".to_string(), "echo \"Error: no test specified\" && exit 1".to_string()),
                    ("start".to_string(), "node index.js".to_string()),
                ])),
                dependencies: Some(HashMap::new()),
                dev_dependencies: Some(HashMap::new()),
                peer_dependencies: None,
                keywords: Some(vec![]),
                author: None,
                license: Some("ISC".to_string()),
                repository: None,
                bugs: None,
                homepage: None,
                extra: HashMap::new(),
            };
            
            npm_manager.save_package_json(&package_json).await
        }
        
        NpmCommands::Run { script, args, manager } => {
            let manager_type = NpmManagerType::from_str(&manager)?;
            let npm_manager = NpmManager::new(workspace.root(), manager_type);
            npm_manager.run_script(&script, &args).await
        }
        
        NpmCommands::Audit { fix, manager } => {
            let manager_type = NpmManagerType::from_str(&manager)?;
            let npm_manager = NpmManager::new(workspace.root(), manager_type);
            npm_manager.audit(fix).await
        }
        
        NpmCommands::Info { package: _, field: _ } => {
            // Implementation for package info
            println!("NPM info functionality not yet implemented");
            Ok(())
        }
    }
}
