//! System package management for RCM
//! 
//! Provides integration with system package managers (apt, yum, dnf, brew, chocolatey, etc.)

use anyhow::{anyhow, Context, Result};
use clap::Subcommand;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::fs;
use crate::workspace::Workspace;
use crate::util::{self, execute_command, get_os_info};

#[derive(Subcommand)]
pub enum SystemCommands {
    /// Install system packages
    Install {
        /// Packages to install
        packages: Vec<String>,
        /// Force installation
        #[arg(long)]
        force: bool,
        /// Skip confirmation prompts
        #[arg(long)]
        yes: bool,
        /// Specific package manager to use
        #[arg(long)]
        manager: Option<String>,
    },
    
    /// Remove system packages
    Remove {
        /// Packages to remove
        packages: Vec<String>,
        /// Remove configuration files
        #[arg(long)]
        purge: bool,
        /// Skip confirmation prompts
        #[arg(long)]
        yes: bool,
        /// Specific package manager to use
        #[arg(long)]
        manager: Option<String>,
    },
    
    /// Update package lists and upgrade packages
    Update {
        /// Only update package lists
        #[arg(long)]
        lists_only: bool,
        /// Skip confirmation prompts
        #[arg(long)]
        yes: bool,
        /// Specific package manager to use
        #[arg(long)]
        manager: Option<String>,
    },
    
    /// Search for packages
    Search {
        /// Search terms
        terms: Vec<String>,
        /// Show detailed information
        #[arg(long)]
        details: bool,
        /// Specific package manager to use
        #[arg(long)]
        manager: Option<String>,
    },
    
    /// Show package information
    Info {
        /// Package name
        package: String,
        /// Specific package manager to use
        #[arg(long)]
        manager: Option<String>,
    },
    
    /// List installed packages
    List {
        /// Show only manually installed packages
        #[arg(long)]
        manual: bool,
        /// Output format (table, json, names)
        #[arg(long, default_value = "table")]
        format: String,
        /// Filter by pattern
        #[arg(long)]
        filter: Option<String>,
    },
    
    /// Clean package cache
    Clean {
        /// Clean everything (cache, orphans, etc.)
        #[arg(long)]
        all: bool,
        /// Specific package manager to use
        #[arg(long)]
        manager: Option<String>,
    },
    
    /// Manage repositories
    Repo {
        #[command(subcommand)]
        cmd: RepoCommands,
    },
    
    /// Install from source (compile-time dependencies)
    Source {
        /// Source URL or package specification
        source: String,
        /// Build directory
        #[arg(long)]
        build_dir: Option<String>,
        /// Install prefix
        #[arg(long, default_value = "/usr/local")]
        prefix: String,
        /// Make jobs (parallel compilation)
        #[arg(long, short)]
        jobs: Option<usize>,
        /// Configure options
        #[arg(long, value_delimiter = ' ')]
        configure_opts: Vec<String>,
    },
}

#[derive(Subcommand)]
pub enum RepoCommands {
    /// Add repository
    Add {
        /// Repository URL or identifier
        repo: String,
        /// Repository key/signature
        #[arg(long)]
        key: Option<String>,
    },
    /// Remove repository
    Remove {
        /// Repository identifier
        repo: String,
    },
    /// List repositories
    List,
    /// Update repository information
    Update,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemPackageManager {
    Apt,      // Debian/Ubuntu
    Yum,      // RHEL/CentOS (legacy)
    Dnf,      // Fedora/RHEL 8+
    Pacman,   // Arch Linux
    Brew,     // macOS
    Chocolatey, // Windows
    Winget,   // Windows
    Zypper,   // openSUSE
    Portage,  // Gentoo
    Apk,      // Alpine
    Pkg,      // FreeBSD
    PkgNg,    // FreeBSD (new)
}

impl SystemPackageManager {
    /// Detect system package manager
    pub async fn detect() -> Result<Self> {
        let os_info = get_os_info().await?;
        
        match os_info.family.to_lowercase().as_str() {
            "debian" | "ubuntu" => {
                if util::command_exists("apt").await {
                    Ok(Self::Apt)
                } else {
                    Err(anyhow!("No supported package manager found for Debian/Ubuntu"))
                }
            }
            "rhel" | "centos" | "fedora" => {
                if util::command_exists("dnf").await {
                    Ok(Self::Dnf)
                } else if util::command_exists("yum").await {
                    Ok(Self::Yum)
                } else {
                    Err(anyhow!("No supported package manager found for RHEL/Fedora"))
                }
            }
            "arch" => {
                if util::command_exists("pacman").await {
                    Ok(Self::Pacman)
                } else {
                    Err(anyhow!("Pacman not found on Arch Linux"))
                }
            }
            "macos" | "darwin" => {
                if util::command_exists("brew").await {
                    Ok(Self::Brew)
                } else {
                    Err(anyhow!("Homebrew not installed. Install from https://brew.sh/"))
                }
            }
            "windows" => {
                if util::command_exists("winget").await {
                    Ok(Self::Winget)
                } else if util::command_exists("choco").await {
                    Ok(Self::Chocolatey)
                } else {
                    Err(anyhow!("No supported package manager found for Windows. Install winget or chocolatey."))
                }
            }
            "opensuse" | "suse" => {
                if util::command_exists("zypper").await {
                    Ok(Self::Zypper)
                } else {
                    Err(anyhow!("Zypper not found on openSUSE"))
                }
            }
            "gentoo" => {
                if util::command_exists("emerge").await {
                    Ok(Self::Portage)
                } else {
                    Err(anyhow!("Portage not found on Gentoo"))
                }
            }
            "alpine" => {
                if util::command_exists("apk").await {
                    Ok(Self::Apk)
                } else {
                    Err(anyhow!("APK not found on Alpine Linux"))
                }
            }
            "freebsd" => {
                if util::command_exists("pkg").await {
                    Ok(Self::PkgNg)
                } else {
                    Ok(Self::Pkg)
                }
            }
            _ => Err(anyhow!("Unsupported operating system: {}", os_info.family))
        }
    }
    
    /// Get package manager command
    pub fn command(&self) -> &'static str {
        match self {
            Self::Apt => "apt",
            Self::Yum => "yum",
            Self::Dnf => "dnf",
            Self::Pacman => "pacman",
            Self::Brew => "brew",
            Self::Chocolatey => "choco",
            Self::Winget => "winget",
            Self::Zypper => "zypper",
            Self::Portage => "emerge",
            Self::Apk => "apk",
            Self::Pkg => "pkg_add",
            Self::PkgNg => "pkg",
        }
    }
    
    /// Get sudo requirement
    pub fn requires_sudo(&self) -> bool {
        match self {
            Self::Brew | Self::Chocolatey | Self::Winget => false,
            _ => true,
        }
    }
    
    /// Build install command
    pub fn install_cmd(&self, packages: &[String], force: bool, yes: bool) -> Command {
        let mut cmd = if self.requires_sudo() {
            let mut c = Command::new("sudo");
            c.arg(self.command());
            c
        } else {
            Command::new(self.command())
        };
        
        match self {
            Self::Apt => {
                cmd.arg("install");
                if yes {
                    cmd.arg("-y");
                }
                if force {
                    cmd.arg("--force-yes");
                }
                cmd.args(packages);
            }
            Self::Yum | Self::Dnf => {
                cmd.arg("install");
                if yes {
                    cmd.arg("-y");
                }
                cmd.args(packages);
            }
            Self::Pacman => {
                cmd.arg("-S");
                if force {
                    cmd.arg("--force");
                }
                if yes {
                    cmd.arg("--noconfirm");
                }
                cmd.args(packages);
            }
            Self::Brew => {
                cmd.arg("install");
                if force {
                    cmd.arg("--force");
                }
                cmd.args(packages);
            }
            Self::Chocolatey => {
                cmd.arg("install");
                if yes {
                    cmd.arg("-y");
                }
                if force {
                    cmd.arg("--force");
                }
                cmd.args(packages);
            }
            Self::Winget => {
                cmd.arg("install");
                if yes {
                    cmd.arg("--accept-package-agreements");
                    cmd.arg("--accept-source-agreements");
                }
                cmd.args(packages);
            }
            Self::Zypper => {
                cmd.arg("install");
                if yes {
                    cmd.arg("-y");
                }
                if force {
                    cmd.arg("--force");
                }
                cmd.args(packages);
            }
            Self::Portage => {
                cmd.args(packages);
            }
            Self::Apk => {
                cmd.arg("add");
                if force {
                    cmd.arg("--force");
                }
                cmd.args(packages);
            }
            Self::Pkg => {
                cmd.args(packages);
            }
            Self::PkgNg => {
                cmd.arg("install");
                if yes {
                    cmd.arg("-y");
                }
                cmd.args(packages);
            }
        }
        
        cmd
    }
    
    /// Build remove command
    pub fn remove_cmd(&self, packages: &[String], purge: bool, yes: bool) -> Command {
        let mut cmd = if self.requires_sudo() {
            let mut c = Command::new("sudo");
            c.arg(self.command());
            c
        } else {
            Command::new(self.command())
        };
        
        match self {
            Self::Apt => {
                if purge {
                    cmd.arg("purge");
                } else {
                    cmd.arg("remove");
                }
                if yes {
                    cmd.arg("-y");
                }
                cmd.args(packages);
            }
            Self::Yum | Self::Dnf => {
                cmd.arg("remove");
                if yes {
                    cmd.arg("-y");
                }
                cmd.args(packages);
            }
            Self::Pacman => {
                cmd.arg("-R");
                if yes {
                    cmd.arg("--noconfirm");
                }
                cmd.args(packages);
            }
            Self::Brew => {
                cmd.arg("uninstall");
                cmd.args(packages);
            }
            Self::Chocolatey => {
                cmd.arg("uninstall");
                if yes {
                    cmd.arg("-y");
                }
                cmd.args(packages);
            }
            Self::Winget => {
                cmd.arg("uninstall");
                cmd.args(packages);
            }
            Self::Zypper => {
                cmd.arg("remove");
                if yes {
                    cmd.arg("-y");
                }
                cmd.args(packages);
            }
            Self::Portage => {
                cmd.arg("--unmerge");
                cmd.args(packages);
            }
            Self::Apk => {
                cmd.arg("del");
                cmd.args(packages);
            }
            Self::Pkg => {
                cmd.arg("delete");
                cmd.args(packages);
            }
            Self::PkgNg => {
                cmd.arg("delete");
                if yes {
                    cmd.arg("-y");
                }
                cmd.args(packages);
            }
        }
        
        cmd
    }
    
    /// Build update command
    pub fn update_cmd(&self, lists_only: bool, yes: bool) -> Command {
        let mut cmd = if self.requires_sudo() {
            let mut c = Command::new("sudo");
            c.arg(self.command());
            c
        } else {
            Command::new(self.command())
        };
        
        match self {
            Self::Apt => {
                if lists_only {
                    cmd.arg("update");
                } else {
                    cmd.arg("update");
                    cmd.arg("&&");
                    cmd.arg("apt");
                    cmd.arg("upgrade");
                    if yes {
                        cmd.arg("-y");
                    }
                }
            }
            Self::Yum | Self::Dnf => {
                if lists_only {
                    cmd.arg("check-update");
                } else {
                    cmd.arg("update");
                    if yes {
                        cmd.arg("-y");
                    }
                }
            }
            Self::Pacman => {
                if lists_only {
                    cmd.arg("-Sy");
                } else {
                    cmd.arg("-Syu");
                    if yes {
                        cmd.arg("--noconfirm");
                    }
                }
            }
            Self::Brew => {
                if lists_only {
                    cmd.arg("update");
                } else {
                    cmd.arg("upgrade");
                }
            }
            Self::Chocolatey => {
                cmd.arg("upgrade");
                cmd.arg("all");
                if yes {
                    cmd.arg("-y");
                }
            }
            Self::Winget => {
                cmd.arg("upgrade");
                cmd.arg("--all");
            }
            Self::Zypper => {
                if lists_only {
                    cmd.arg("refresh");
                } else {
                    cmd.arg("update");
                    if yes {
                        cmd.arg("-y");
                    }
                }
            }
            Self::Portage => {
                cmd.arg("--sync");
                if !lists_only {
                    cmd.arg("--update");
                    cmd.arg("--deep");
                    cmd.arg("--newuse");
                    cmd.arg("@world");
                }
            }
            Self::Apk => {
                if lists_only {
                    cmd.arg("update");
                } else {
                    cmd.arg("upgrade");
                }
            }
            Self::Pkg => {
                cmd.arg("update");
            }
            Self::PkgNg => {
                if lists_only {
                    cmd.arg("update");
                } else {
                    cmd.arg("upgrade");
                    if yes {
                        cmd.arg("-y");
                    }
                }
            }
        }
        
        cmd
    }
    
    /// Build search command
    pub fn search_cmd(&self, terms: &[String]) -> Command {
        let mut cmd = Command::new(self.command());
        
        match self {
            Self::Apt => {
                cmd.arg("search");
                cmd.args(terms);
            }
            Self::Yum | Self::Dnf => {
                cmd.arg("search");
                cmd.args(terms);
            }
            Self::Pacman => {
                cmd.arg("-Ss");
                cmd.args(terms);
            }
            Self::Brew => {
                cmd.arg("search");
                cmd.args(terms);
            }
            Self::Chocolatey => {
                cmd.arg("search");
                cmd.args(terms);
            }
            Self::Winget => {
                cmd.arg("search");
                cmd.args(terms);
            }
            Self::Zypper => {
                cmd.arg("search");
                cmd.args(terms);
            }
            Self::Portage => {
                cmd.arg("--search");
                cmd.args(terms);
            }
            Self::Apk => {
                cmd.arg("search");
                cmd.args(terms);
            }
            Self::Pkg => {
                cmd.arg("search");
                cmd.args(terms);
            }
            Self::PkgNg => {
                cmd.arg("search");
                cmd.args(terms);
            }
        }
        
        cmd
    }
}

#[derive(Debug)]
pub struct SystemManager {
    workspace_root: PathBuf,
    package_manager: SystemPackageManager,
    config_path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemConfig {
    pub default_manager: Option<String>,
    pub package_mappings: HashMap<String, HashMap<String, String>>, // package -> manager -> actual_name
    pub common_packages: HashMap<String, Vec<String>>, // alias -> [actual_packages]
}

impl SystemManager {
    pub async fn new(workspace_root: &Path) -> Result<Self> {
        let package_manager = SystemPackageManager::detect().await?;
        let config_path = workspace_root.join(".rcm").join("system.json");
        
        Ok(Self {
            workspace_root: workspace_root.to_path_buf(),
            package_manager,
            config_path,
        })
    }
    
    /// Load system configuration
    pub async fn load_config(&self) -> Result<SystemConfig> {
        if !self.config_path.exists() {
            return Ok(SystemConfig {
                default_manager: None,
                package_mappings: Self::default_package_mappings(),
                common_packages: Self::default_common_packages(),
            });
        }
        
        let content = fs::read_to_string(&self.config_path).await
            .context("Failed to read system config")?;
        
        serde_json::from_str(&content)
            .context("Failed to parse system config")
    }
    
    /// Default package mappings for common packages
    fn default_package_mappings() -> HashMap<String, HashMap<String, String>> {
        let mut mappings = HashMap::new();
        
        // FFmpeg mappings
        let mut ffmpeg = HashMap::new();
        ffmpeg.insert("apt".to_string(), "ffmpeg".to_string());
        ffmpeg.insert("yum".to_string(), "ffmpeg".to_string());
        ffmpeg.insert("dnf".to_string(), "ffmpeg".to_string());
        ffmpeg.insert("pacman".to_string(), "ffmpeg".to_string());
        ffmpeg.insert("brew".to_string(), "ffmpeg".to_string());
        ffmpeg.insert("chocolatey".to_string(), "ffmpeg".to_string());
        ffmpeg.insert("winget".to_string(), "FFmpeg".to_string());
        mappings.insert("ffmpeg".to_string(), ffmpeg);
        
        // Node.js mappings
        let mut nodejs = HashMap::new();
        nodejs.insert("apt".to_string(), "nodejs npm".to_string());
        nodejs.insert("yum".to_string(), "nodejs npm".to_string());
        nodejs.insert("dnf".to_string(), "nodejs npm".to_string());
        nodejs.insert("pacman".to_string(), "nodejs npm".to_string());
        nodejs.insert("brew".to_string(), "node".to_string());
        nodejs.insert("chocolatey".to_string(), "nodejs".to_string());
        nodejs.insert("winget".to_string(), "OpenJS.NodeJS".to_string());
        mappings.insert("node".to_string(), nodejs);
        
        // Git mappings
        let mut git = HashMap::new();
        git.insert("apt".to_string(), "git".to_string());
        git.insert("yum".to_string(), "git".to_string());
        git.insert("dnf".to_string(), "git".to_string());
        git.insert("pacman".to_string(), "git".to_string());
        git.insert("brew".to_string(), "git".to_string());
        git.insert("chocolatey".to_string(), "git".to_string());
        git.insert("winget".to_string(), "Git.Git".to_string());
        mappings.insert("git".to_string(), git);
        
        mappings
    }
    
    /// Default common package groups
    fn default_common_packages() -> HashMap<String, Vec<String>> {
        let mut common = HashMap::new();
        
        common.insert("media".to_string(), vec![
            "ffmpeg".to_string(),
            "imagemagick".to_string(),
            "vlc".to_string(),
        ]);
        
        common.insert("dev".to_string(), vec![
            "git".to_string(),
            "curl".to_string(),
            "wget".to_string(),
            "node".to_string(),
        ]);
        
        common.insert("build".to_string(), vec![
            "gcc".to_string(),
            "make".to_string(),
            "cmake".to_string(),
            "pkg-config".to_string(),
        ]);
        
        common
    }
    
    /// Resolve package names using mappings
    pub async fn resolve_packages(&self, packages: &[String]) -> Result<Vec<String>> {
        let config = self.load_config().await?;
        let manager_key = self.package_manager.command();
        let mut resolved = Vec::new();
        
        for package in packages {
            // Check if it's a common package group
            if let Some(group_packages) = config.common_packages.get(package) {
                for group_package in group_packages {
                    resolved.extend(self.resolve_packages(&[group_package.clone()]).await?);
                }
                continue;
            }
            
            // Check package mappings
            if let Some(mapping) = config.package_mappings.get(package) {
                if let Some(actual_name) = mapping.get(manager_key) {
                    resolved.extend(actual_name.split_whitespace().map(|s| s.to_string()));
                } else {
                    resolved.push(package.clone());
                }
            } else {
                resolved.push(package.clone());
            }
        }
        
        Ok(resolved)
    }
    
    /// Install packages
    pub async fn install(&self, packages: &[String], force: bool, yes: bool) -> Result<()> {
        let resolved = self.resolve_packages(packages).await?;
        let mut cmd = self.package_manager.install_cmd(&resolved, force, yes);
        
        execute_command(&mut cmd).await
            .context("Failed to install system packages")
    }
    
    /// Remove packages
    pub async fn remove(&self, packages: &[String], purge: bool, yes: bool) -> Result<()> {
        let resolved = self.resolve_packages(packages).await?;
        let mut cmd = self.package_manager.remove_cmd(&resolved, purge, yes);
        
        execute_command(&mut cmd).await
            .context("Failed to remove system packages")
    }
    
    /// Update packages
    pub async fn update(&self, lists_only: bool, yes: bool) -> Result<()> {
        let mut cmd = self.package_manager.update_cmd(lists_only, yes);
        
        execute_command(&mut cmd).await
            .context("Failed to update system packages")
    }
    
    /// Search packages
    pub async fn search(&self, terms: &[String]) -> Result<()> {
        let mut cmd = self.package_manager.search_cmd(terms);
        
        execute_command(&mut cmd).await
            .context("Failed to search system packages")
    }
}

/// Handle system commands
pub async fn handle_command(workspace: &Workspace, cmd: SystemCommands) -> Result<()> {
    match cmd {
        SystemCommands::Install { packages, force, yes, manager } => {
            let system = SystemManager::new(workspace.root()).await?;
            system.install(&packages, force, yes).await
        }
        
        SystemCommands::Remove { packages, purge, yes, manager } => {
            let system = SystemManager::new(workspace.root()).await?;
            system.remove(&packages, purge, yes).await
        }
        
        SystemCommands::Update { lists_only, yes, manager } => {
            let system = SystemManager::new(workspace.root()).await?;
            system.update(lists_only, yes).await
        }
        
        SystemCommands::Search { terms, details: _, manager: _ } => {
            let system = SystemManager::new(workspace.root()).await?;
            system.search(&terms).await
        }
        
        SystemCommands::Info { package: _, manager: _ } => {
            println!("System package info not yet implemented");
            Ok(())
        }
        
        SystemCommands::List { manual: _, format: _, filter: _ } => {
            println!("System package list not yet implemented");
            Ok(())
        }
        
        SystemCommands::Clean { all: _, manager: _ } => {
            println!("System package clean not yet implemented");
            Ok(())
        }
        
        SystemCommands::Repo { cmd: _ } => {
            println!("System repository management not yet implemented");
            Ok(())
        }
        
        SystemCommands::Source { source: _, build_dir: _, prefix: _, jobs: _, configure_opts: _ } => {
            println!("Source installation not yet implemented");
            Ok(())
        }
    }
}
