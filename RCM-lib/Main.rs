//! RCM - Polyglot Package Manager
//! 
//! Supports Rust (Cargo), Node.js (NPM), PHP (Composer), and system packages
//! with imperative LET commands for complex workflows.

mod lockfile;
mod sbom;
mod provenance;
mod registry;
mod util;
mod commands;
mod npm;
mod ppm;
mod system;
mod config;
mod workspace;

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use anyhow::Result;
use clap::{Parser, Subcommand};
use log::{debug, info, warn};

#[derive(Parser)]
#[command(name = "rcm", version, about = "RCM – Polyglot Package Manager")]
#[command(long_about = "A unified package manager for Rust, Node.js, PHP, and system packages with imperative workflow support")]
struct Cli {
    #[command(subcommand)]
    cmd: Commands,
    
    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
    
    /// Workspace root directory
    #[arg(short, long, global = true)]
    workspace: Option<String>,
    
    /// Configuration file path
    #[arg(short, long, global = true)]
    config: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize RCM workspace in the current directory
    Init {
        /// Initialize with specific package managers
        #[arg(long, value_delimiter = ',')]
        managers: Option<Vec<String>>,
        /// Template to use (rust, node, php, polyglot)
        #[arg(long, default_value = "polyglot")]
        template: String,
    },
    
    /// Add a package requirement with auto-detection of package manager
    Add { 
        /// Package specification (name[@version] or manager:name[@version])
        spec: String,
        /// Force specific package manager (cargo, npm, composer, system)
        #[arg(long)]
        manager: Option<String>,
        /// Development/optional dependency
        #[arg(long)]
        dev: bool,
    },
    
    /// Remove a package
    Remove {
        /// Package name or manager:name
        spec: String,
        /// Force specific package manager
        #[arg(long)]
        manager: Option<String>,
    },
    
    /// Check environment, ensure lockfiles exist, validate metadata
    Ensure {
        /// Check only specific managers
        #[arg(long, value_delimiter = ',')]
        managers: Option<Vec<String>>,
    },
    
    /// Show what would change (dry-run)
    Plan {
        /// Show plan for specific managers only
        #[arg(long, value_delimiter = ',')]
        managers: Option<Vec<String>>,
        /// Output format (text, json, yaml)
        #[arg(long, default_value = "text")]
        format: String,
    },
    
    /// Apply the planned changes
    Apply {
        /// Apply for specific managers only
        #[arg(long, value_delimiter = ',')]
        managers: Option<Vec<String>>,
        /// Force apply without confirmation
        #[arg(long)]
        force: bool,
    },
    
    /// Create a workspace snapshot
    Snapshot { 
        #[arg(long)] 
        name: String,
        /// Include lockfiles in snapshot
        #[arg(long)]
        include_locks: bool,
        /// Snapshot format (tar, zip, json)
        #[arg(long, default_value = "tar")]
        format: String,
    },
    
    /// Generate SBOM (Software Bill of Materials)
    Sbom { 
        #[arg(long)] 
        out: String,
        /// SBOM format (cyclonedx, spdx, json)
        #[arg(long, default_value = "cyclonedx")]
        format: String,
        /// Include specific managers only
        #[arg(long, value_delimiter = ',')]
        managers: Option<Vec<String>>,
    },
    
    /// Generate provenance information
    Provenance { 
        #[arg(long)] 
        out: String,
        /// Provenance format (slsa, json)
        #[arg(long, default_value = "slsa")]
        format: String,
    },

    /// NPM-specific commands
    #[cfg(feature = "npm")]
    Npm {
        #[command(subcommand)]
        cmd: npm::NpmCommands,
    },

    /// PHP Composer-specific commands  
    #[cfg(feature = "ppm")]
    Ppm {
        #[command(subcommand)]
        cmd: ppm::PpmCommands,
    },

    /// System package commands (apt, yum, brew, etc.)
    #[cfg(feature = "system")]
    System {
        #[command(subcommand)]
        cmd: system::SystemCommands,
    },

    /// Imperative workflow commands (LET paradigm)
    #[cfg(feature = "let")]
    Let {
        /// Target package/command (e.g., "ffmpeg", "cargo", "npm")
        target: String,
        
        /// Deploy/install the target
        #[arg(long)]
        deploy: bool,
        
        /// Show plan only
        #[arg(long)]
        plan: bool,
        
        /// Apply the plan
        #[arg(long)]
        apply: bool,
        
        /// Build/compile the target
        #[arg(long)]
        build: bool,
        
        /// Test the target
        #[arg(long)]
        test: bool,
        
        /// Clean/remove the target
        #[arg(long)]
        clean: bool,
        
        /// Update/upgrade the target
        #[arg(long)]
        update: bool,
        
        /// Additional arguments as key=value pairs
        #[arg(long = "arg", value_name = "k=v", num_args=0.., action=clap::ArgAction::Append)]
        args: Vec<String>,
        
        /// Execute in specific environment/container
        #[arg(long)]
        env: Option<String>,
        
        /// Parallel execution count
        #[arg(long, default_value = "1")]
        parallel: usize,
    },

    /// Workspace management commands
    Workspace {
        #[command(subcommand)]
        cmd: WorkspaceCommands,
    },

    /// Configuration management
    Config {
        #[command(subcommand)]
        cmd: ConfigCommands,
    },
}

#[derive(Subcommand)]
enum WorkspaceCommands {
    /// List all packages in workspace
    List {
        /// Output format (table, json, yaml)
        #[arg(long, default_value = "table")]
        format: String,
    },
    /// Synchronize all package managers
    Sync,
    /// Clean all build artifacts
    Clean,
    /// Update all dependencies
    Update,
    /// Check workspace health
    Check,
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Show current configuration
    Show,
    /// Set configuration value
    Set { key: String, value: String },
    /// Get configuration value
    Get { key: String },
    /// Reset configuration to defaults
    Reset,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    let log_level = if cli.verbose { "debug" } else { "info" };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level))
        .init();

    // Load configuration
    let config = config::Config::load(cli.config.as_deref()).await?;
    
    // Initialize workspace
    let workspace = workspace::Workspace::new(cli.workspace.as_deref(), config).await?;
    
    debug!("RCM CLI starting with command: {:?}", cli.cmd);
    
    let result = match cli.cmd {
        Commands::Init { managers, template } => {
            commands::init::run(&workspace, managers, &template).await
        }
        Commands::Add { spec, manager, dev } => {
            commands::add::run(&workspace, &spec, manager.as_deref(), dev).await
        }
        Commands::Remove { spec, manager } => {
            commands::remove::run(&workspace, &spec, manager.as_deref()).await
        }
        Commands::Ensure { managers } => {
            commands::ensure::run(&workspace, managers).await
        }
        Commands::Plan { managers, format } => {
            commands::plan::run(&workspace, managers, &format).await
        }
        Commands::Apply { managers, force } => {
            commands::apply::run(&workspace, managers, force).await
        }
        Commands::Snapshot { name, include_locks, format } => {
            commands::snapshot::run(&workspace, &name, include_locks, &format).await
        }
        Commands::Sbom { out, format, managers } => {
            commands::sbom::run(&workspace, &out, &format, managers).await
        }
        Commands::Provenance { out, format } => {
            commands::provenance::run(&workspace, &out, &format).await
        }
        
        #[cfg(feature = "npm")]
        Commands::Npm { cmd } => {
            npm::handle_command(&workspace, cmd).await
        }
        
        #[cfg(feature = "ppm")]
        Commands::Ppm { cmd } => {
            ppm::handle_command(&workspace, cmd).await
        }
        
        #[cfg(feature = "system")]
        Commands::System { cmd } => {
            system::handle_command(&workspace, cmd).await
        }
        
        #[cfg(feature = "let")]
        Commands::Let { 
            target, deploy, plan, apply, build, test, clean, update, 
            args, env, parallel 
        } => {
            commands::letcmd::run(
                &workspace, &target, deploy, plan, apply, build, test, 
                clean, update, args, env.as_deref(), parallel
            ).await
        }
        
        Commands::Workspace { cmd } => {
            commands::workspace::handle_command(&workspace, cmd).await
        }
        
        Commands::Config { cmd } => {
            commands::config::handle_command(&workspace, cmd).await
        }
    };

    match result {
        Ok(_) => {
            info!("RCM command completed successfully");
            Ok(())
        }
        Err(e) => {
            warn!("RCM command failed: {:?}", e);
            Err(e)
        }
    }
}

fn run_cli<I, S>(iter: I) -> Result<i32>
where
    I: IntoIterator<Item = S>,
    S: Into<std::ffi::OsString> + Clone,
{
    let rt = tokio::runtime::Runtime::new()?;
    match rt.block_on(async {
        let cli = Cli::parse_from(iter);
        // Set up minimal logging for FFI calls
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"))
            .init();
        
        let config = config::Config::default();
        let workspace = workspace::Workspace::new(None, config).await?;
        
        match cli.cmd {
            Commands::Init { managers, template } => {
                commands::init::run(&workspace, managers, &template).await?;
                Ok(0)
            }
            Commands::Add { spec, manager, dev } => {
                commands::add::run(&workspace, &spec, manager.as_deref(), dev).await?;
                Ok(0)
            }
            // Add other command mappings...
            _ => {
                eprintln!("Command not supported in FFI mode");
                Ok(1)
            }
        }
    }) {
        Ok(code) => Ok(code),
        Err(e) => {
            eprintln!("Runtime error: {:?}", e);
            Ok(1)
        }
    }
}

// --- FFI exports -------------------------------------------------------------

/// C ABI entry point: rcm_run(argc, argv)
/// Safety: argv must be an array of `argc` valid, null-terminated C strings.
#[no_mangle]
pub extern "C" fn rcm_run(argc: c_int, argv: *const *const c_char) -> c_int {
    let args: Vec<String> = unsafe {
        if argv.is_null() || argc <= 0 {
            Vec::new()
        } else {
            std::slice::from_raw_parts(argv, argc as usize)
                .iter()
                .map(|&p| {
                    if p.is_null() {
                        String::new()
                    } else {
                        CStr::from_ptr(p).to_string_lossy().into_owned()
                    }
                })
                .collect()
        }
    };

    let args = if args.is_empty() {
        vec!["rcm".to_string(), "--help".to_string()]
    } else {
        args
    };

    match run_cli(args) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("rcm error: {err:?}");
            1
        }
    }
}

/// Return a static version string
#[no_mangle]
pub extern "C" fn rcm_version() -> *const c_char {
    let s = CString::new(env!("CARGO_PKG_VERSION")).unwrap();
    Box::into_raw(Box::new(s)).as_ptr()
}
