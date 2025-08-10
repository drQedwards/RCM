//! LET command module for imperative workflows in RCM
//! 
//! Implements the LET paradigm for declarative-imperative package and workflow management

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::fs;
use tokio::process::Command as AsyncCommand;
use crate::workspace::Workspace;
use crate::util::{self, execute_command, parse_key_value_args};
use crate::npm::{NpmManager, NpmManagerType};
use crate::ppm::ComposerManager;
use crate::system::SystemManager;

#[derive(Debug, Serialize, Deserialize)]
pub struct LetSpec {
    pub target: String,
    pub version: Option<String>,
    pub manager: Option<String>,
    pub dependencies: Vec<String>,
    pub actions: Vec<LetAction>,
    pub environment: HashMap<String, String>,
    pub constraints: LetConstraints,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LetAction {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub working_dir: Option<String>,
    pub env: HashMap<String, String>,
    pub conditions: Vec<LetCondition>,
    pub parallel: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LetCondition {
    pub condition_type: LetConditionType,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum LetConditionType {
    FileExists,
    CommandExists,
    EnvVar,
    Platform,
    PackageInstalled,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LetConstraints {
    pub platforms: Vec<String>,
    pub min_memory_mb: Option<u64>,
    pub required_commands: Vec<String>,
    pub required_env_vars: Vec<String>,
}

#[derive(Debug)]
pub struct LetExecutor {
    workspace: PathBuf,
    specs_dir: PathBuf,
}

impl LetExecutor {
    pub fn new(workspace_root: &Path) -> Self {
        let specs_dir = workspace_root.join(".rcm").join("let");
        
        Self {
            workspace: workspace_root.to_path_buf(),
            specs_dir,
        }
    }
    
    /// Initialize LET specs directory
    pub async fn initialize(&self) -> Result<()> {
        if !self.specs_dir.exists() {
            fs::create_dir_all(&self.specs_dir).await
                .context("Failed to create LET specs directory")?;
        }
        
        // Create default specs for common packages
        self.create_default_specs().await?;
        
        Ok(())
    }
    
    /// Create default LET specs for common packages
    async fn create_default_specs(&self) -> Result<()> {
        let specs = vec![
            self.create_ffmpeg_spec(),
            self.create_node_spec(),
            self.create_php_spec(),
            self.create_cargo_spec(),
            self.create_git_spec(),
        ];
        
        for spec in specs {
            let spec_path = self.specs_dir.join(format!("{}.json", spec.target));
            if !spec_path.exists() {
                let content = serde_json::to_string_pretty(&spec)?;
                fs::write(spec_path, content).await?;
            }
        }
        
        Ok(())
    }
    
    /// Create FFmpeg LET spec
    fn create_ffmpeg_spec(&self) -> LetSpec {
        LetSpec {
            target: "ffmpeg".to_string(),
            version: None,
            manager: Some("system".to_string()),
            dependencies: vec![],
            actions: vec![
                LetAction {
                    name: "install".to_string(),
                    command: "rcm".to_string(),
                    args: vec!["system", "install", "ffmpeg"].iter().map(|s| s.to_string()).collect(),
                    working_dir: None,
                    env: HashMap::new(),
                    conditions: vec![],
                    parallel: false,
                },
                LetAction {
                    name: "verify".to_string(),
                    command: "ffmpeg".to_string(),
                    args: vec!["-version"].iter().map(|s| s.to_string()).collect(),
                    working_dir: None,
                    env: HashMap::new(),
                    conditions: vec![LetCondition {
                        condition_type: LetConditionType::CommandExists,
                        value: "ffmpeg".to_string(),
                    }],
                    parallel: false,
                },
                LetAction {
                    name: "test".to_string(),
                    command: "ffmpeg".to_string(),
                    args: vec!["-f", "lavfi", "-i", "testsrc=duration=1:size=320x240:rate=1", 
                              "-f", "null", "-"].iter().map(|s| s.to_string()).collect(),
                    working_dir: None,
                    env: HashMap::new(),
                    conditions: vec![],
                    parallel: false,
                },
            ],
            environment: HashMap::new(),
            constraints: LetConstraints {
                platforms: vec!["linux".to_string(), "macos".to_string(), "windows".to_string()],
                min_memory_mb: Some(512),
                required_commands: vec![],
                required_env_vars: vec![],
            },
        }
    }
    
    /// Create Node.js LET spec
    fn create_node_spec(&self) -> LetSpec {
        LetSpec {
            target: "node".to_string(),
            version: Some(">=18".to_string()),
            manager: Some("system".to_string()),
            dependencies: vec!["npm".to_string()],
            actions: vec![
                LetAction {
                    name: "install".to_string(),
                    command: "rcm".to_string(),
                    args: vec!["system", "install", "node"].iter().map(|s| s.to_string()).collect(),
                    working_dir: None,
                    env: HashMap::new(),
                    conditions: vec![],
                    parallel: false,
                },
                LetAction {
                    name: "verify".to_string(),
                    command: "node".to_string(),
                    args: vec!["--version"].iter().map(|s| s.to_string()).collect(),
                    working_dir: None,
                    env: HashMap::new(),
                    conditions: vec![],
                    parallel: false,
                },
                LetAction {
                    name: "npm-init".to_string(),
                    command: "rcm".to_string(),
                    args: vec!["npm", "init", "--yes"].iter().map(|s| s.to_string()).collect(),
                    working_dir: Some(".".to_string()),
                    env: HashMap::new(),
                    conditions: vec![LetCondition {
                        condition_type: LetConditionType::FileExists,
                        value: "package.json".to_string(),
                    }],
                    parallel: false,
                },
            ],
            environment: HashMap::new(),
            constraints: LetConstraints {
                platforms: vec!["linux".to_string(), "macos".to_string(), "windows".to_string()],
                min_memory_mb: Some(256),
                required_commands: vec![],
                required_env_vars: vec![],
            },
        }
    }
    
    /// Create PHP LET spec
    fn create_php_spec(&self) -> LetSpec {
        LetSpec {
            target: "php".to_string(),
            version: Some(">=8.1".to_string()),
            manager: Some("system".to_string()),
            dependencies: vec!["composer".to_string()],
            actions: vec![
                LetAction {
                    name: "install".to_string(),
                    command: "rcm".to_string(),
                    args: vec!["system", "install", "php", "php-cli", "php-composer-installers"].iter().map(|s| s.to_string()).collect(),
                    working_dir: None,
                    env: HashMap::new(),
                    conditions: vec![],
                    parallel: false,
                },
                LetAction {
                    name: "composer-install".to_string(),
                    command: "rcm".to_string(),
                    args: vec!["system", "install", "composer"].iter().map(|s| s.to_string()).collect(),
                    working_dir: None,
                    env: HashMap::new(),
                    conditions: vec![],
                    parallel: false,
                },
                LetAction {
                    name: "verify".to_string(),
                    command: "php".to_string(),
                    args: vec!["--version"].iter().map(|s| s.to_string()).collect(),
                    working_dir: None,
                    env: HashMap::new(),
                    conditions: vec![],
                    parallel: false,
                },
                LetAction {
                    name: "composer-init".to_string(),
                    command: "rcm".to_string(),
                    args: vec!["ppm", "init"].iter().map(|s| s.to_string()).collect(),
                    working_dir: Some(".".to_string()),
                    env: HashMap::new(),
                    conditions: vec![LetCondition {
                        condition_type: LetConditionType::FileExists,
                        value: "composer.json".to_string(),
                    }],
                    parallel: false,
                },
            ],
            environment: HashMap::new(),
            constraints: LetConstraints {
                platforms: vec!["linux".to_string(), "macos".to_string(), "windows".to_string()],
                min_memory_mb: Some(512),
                required_commands: vec![],
                required_env_vars: vec![],
            },
        }
    }
    
    /// Create Cargo LET spec
    fn create_cargo_spec(&self) -> LetSpec {
        LetSpec {
            target: "cargo".to_string(),
            version: None,
            manager: Some("system".to_string()),
            dependencies: vec!["rust".to_string()],
            actions: vec![
                LetAction {
                    name: "install-rustup".to_string(),
                    command: "curl".to_string(),
                    args: vec!["--proto", "=https", "--tlsv1.2", "-sSf", "https://sh.rustup.rs"].iter().map(|s| s.to_string()).collect(),
                    working_dir: None,
                    env: HashMap::new(),
                    conditions: vec![LetCondition {
                        condition_type: LetConditionType::CommandExists,
                        value: "rustup".to_string(),
                    }],
                    parallel: false,
                },
                LetAction {
                    name: "verify".to_string(),
                    command: "cargo".to_string(),
                    args: vec!["--version"].iter().map(|s| s.to_string()).collect(),
                    working_dir: None,
                    env: HashMap::new(),
                    conditions: vec![],
                    parallel: false,
                },
                LetAction {
                    name: "init".to_string(),
                    command: "cargo".to_string(),
                    args: vec!["init", "--name", "project"].iter().map(|s| s.to_string()).collect(),
                    working_dir: Some(".".to_string()),
                    env: HashMap::new(),
                    conditions: vec![LetCondition {
                        condition_type: LetConditionType::FileExists,
                        value: "Cargo.toml".to_string(),
                    }],
                    parallel: false,
                },
                LetAction {
                    name: "build".to_string(),
                    command: "cargo".to_string(),
                    args: vec!["build"].iter().map(|s| s.to_string()).collect(),
                    working_dir: Some(".".to_string()),
                    env: HashMap::new(),
                    conditions: vec![],
                    parallel: false,
                },
                LetAction {
                    name: "test".to_string(),
                    command: "cargo".to_string(),
                    args: vec!["test"].iter().map(|s| s.to_string()).collect(),
                    working_dir: Some(".".to_string()),
                    env: HashMap::new(),
                    conditions: vec![],
                    parallel: false,
                },
            ],
            environment: HashMap::new(),
            constraints: LetConstraints {
                platforms: vec!["linux".to_string(), "macos".to_string(), "windows".to_string()],
                min_memory_mb: Some(1024),
                required_commands: vec!["curl".to_string()],
                required_env_vars: vec![],
            },
        }
    }
    
    /// Create Git LET spec
    fn create_git_spec(&self) -> LetSpec {
        LetSpec {
            target: "git".to_string(),
            version: None,
            manager: Some("system".to_string()),
            dependencies: vec![],
            actions: vec![
                LetAction {
                    name: "install".to_string(),
                    command: "rcm".to_string(),
                    args: vec!["system", "install", "git"].iter().map(|s| s.to_string()).collect(),
                    working_dir: None,
                    env: HashMap::new(),
                    conditions: vec![],
                    parallel: false,
                },
                LetAction {
                    name: "verify".to_string(),
                    command: "git".to_string(),
                    args: vec!["--version"].iter().map(|s| s.to_string()).collect(),
                    working_dir: None,
                    env: HashMap::new(),
                    conditions: vec![],
                    parallel: false,
                },
                LetAction {
                    name: "init".to_string(),
                    command: "git".to_string(),
                    args: vec!["init"].iter().map(|s| s.to_string()).collect(),
                    working_dir: Some(".".to_string()),
                    env: HashMap::new(),
                    conditions: vec![LetCondition {
                        condition_type: LetConditionType::FileExists,
                        value: ".git".to_string(),
                    }],
                    parallel: false,
                },
            ],
            environment: HashMap::new(),
            constraints: LetConstraints {
                platforms: vec!["linux".to_string(), "macos".to_string(), "windows".to_string()],
                min_memory_mb: Some(64),
                required_commands: vec![],
                required_env_vars: vec![],
            },
        }
    }
    
    /// Load LET spec for target
    pub async fn load_spec(&self, target: &str) -> Result<LetSpec> {
        let spec_path = self.specs_dir.join(format!("{}.json", target));
        
        if !spec_path.exists() {
            return Err(anyhow!("No LET spec found for target: {}", target));
        }
        
        let content = fs::read_to_string(spec_path).await
            .context("Failed to read LET spec")?;
        
        serde_json::from_str(&content)
            .context("Failed to parse LET spec")
    }
    
    /// Check if condition is met
    async fn check_condition(&self, condition: &LetCondition) -> Result<bool> {
        match condition.condition_type {
            LetConditionType::FileExists => {
                let path = if condition.value.starts_with('/') {
                    PathBuf::from(&condition.value)
                } else {
                    self.workspace.join(&condition.value)
                };
                Ok(path.exists())
            }
            LetConditionType::CommandExists => {
                Ok(util::command_exists(&condition.value).await)
            }
            LetConditionType::EnvVar => {
                Ok(std::env::var(&condition.value).is_ok())
            }
            LetConditionType::Platform => {
                let os = std::env::consts::OS;
                Ok(condition.value == os)
            }
            LetConditionType::PackageInstalled => {
                // Check if package is installed via any manager
                // This is a simplified check - could be enhanced
                util::command_exists(&condition.value).await.then(|| true).ok_or_else(|| anyhow!("Package check not implemented"))
            }
        }
    }
    
    /// Execute LET action
    async fn execute_action(&self, action: &LetAction, env: &HashMap<String, String>) -> Result<()> {
        // Check conditions
        for condition in &action.conditions {
            if !self.check_condition(condition).await? {
                println!("Skipping action '{}': condition not met", action.name);
                return Ok(());
            }
        }
        
        println!("Executing action: {}", action.name);
        
        let working_dir = if let Some(ref dir) = action.working_dir {
            if dir.starts_with('/') {
                PathBuf::from(dir)
            } else {
                self.workspace.join(dir)
            }
        } else {
            self.workspace.clone()
        };
        
        let mut cmd = AsyncCommand::new(&action.command);
        cmd.args(&action.args);
        cmd.current_dir(working_dir);
        
        // Set environment variables
        for (key, value) in env {
            cmd.env(key, value);
        }
        for (key, value) in &action.env {
            cmd.env(key, value);
        }
        
        let output = cmd.output().await
            .context(format!("Failed to execute command: {}", action.command))?;
        
        if !output.status.success() {
            return Err(anyhow!(
                "Command failed: {} {}\nStdout: {}\nStderr: {}",
                action.command,
                action.args.join(" "),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        
        // Print output if present
        if !output.stdout.is_empty() {
            println!("{}", String::from_utf8_lossy(&output.stdout));
        }
        if !output.stderr.is_empty() {
            eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        }
        
        Ok(())
    }
    
    /// Execute LET spec
    pub async fn execute(&self, target: &str, action_filter: Option<&str>, env: HashMap<String, String>) -> Result<()> {
        let spec = self.load_spec(target).await?;
        
        // Check constraints
        let current_platform = std::env::consts::OS;
        if !spec.constraints.platforms.is_empty() && !spec.constraints.platforms.contains(&current_platform.to_string()) {
            return Err(anyhow!("Target {} not supported on platform: {}", target, current_platform));
        }
        
        // Check required commands
        for required_cmd in &spec.constraints.required_commands {
            if !util::command_exists(required_cmd).await {
                return Err(anyhow!("Required command not found: {}", required_cmd));
            }
        }
        
        // Merge environment variables
        let mut combined_env = spec.environment.clone();
        combined_env.extend(env);
        
        // Execute actions
        for action in &spec.actions {
            if let Some(filter) = action_filter {
                if action.name != filter {
                    continue;
                }
            }
            
            self.execute_action(action, &combined_env).await?;
        }
        
        Ok(())
    }
}

/// Main LET command handler
pub async fn run(
    workspace: &Workspace,
    target: &str,
    deploy: bool,
    plan: bool,
    apply: bool,
    build: bool,
    test: bool,
    clean: bool,
    update: bool,
    args: Vec<String>,
    env: Option<&str>,
    parallel: usize,
) -> Result<()> {
    let executor = LetExecutor::new(workspace.root());
    executor.initialize().await?;
    
    // Parse additional arguments
    let parsed_args = parse_key_value_args(&args)?;
    
    // Determine action filter based on flags
    let action_filter = if deploy {
        Some("install")
    } else if build {
        Some("build")
    } else if test {
        Some("test")
    } else if clean {
        Some("clean")
    } else if update {
        Some("update")
    } else {
        None
    };
    
    // Add environment override if specified
    let mut env_vars = HashMap::new();
    if let Some(env_name) = env {
        env_vars.insert("RCM_ENV".to_string(), env_name.to_string());
    }
    env_vars.extend(parsed_args);
    
    if plan {
        println!("=== LET Plan for target: {} ===", target);
        let spec = executor.load_spec(target).await?;
        
        println!("Target: {}", spec.target);
        if let Some(version) = &spec.version {
            println!("Version: {}", version);
        }
        if let Some(manager) = &spec.manager {
            println!("Manager: {}", manager);
        }
        
        println!("\nActions:");
        for action in &spec.actions {
            if let Some(filter) = action_filter {
                if action.name != filter {
                    continue;
                }
            }
            println!("  - {}: {} {}", action.name, action.command, action.args.join(" "));
            
            // Check conditions
            for condition in &action.conditions {
                let met = executor.check_condition(condition).await.unwrap_or(false);
                println!("    Condition: {:?} = {} [{}]", 
                        condition.condition_type, condition.value, if met { "✓" } else { "✗" });
            }
        }
        
        println!("\nEnvironment:");
        for (key, value) in &env_vars {
            println!("  {}={}", key, value);
        }
        
        return Ok(());
    }
    
    if apply || (!plan && !deploy && !build && !test && !clean && !update) {
        executor.execute(target, action_filter, env_vars).await?;
    }
    
    Ok(())
}
