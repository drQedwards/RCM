//! GPT-lib Integration with RCM
//! 
//! Integration layer for GPT model management in RCM LET commands

// Add to main Cargo.toml:
// [dependencies]
// gpt-lib = { path = "GPT-lib", optional = true }
// 
// [features]
// gpt = ["dep:gpt-lib"]
// experimental = ["let", "npm", "ppm", "system", "arm", "gpt"]

// Integration in main src/main.rs:
use anyhow::Result;

#[cfg(feature = "gpt")]
use gpt_lib;

// Add to main Commands enum:
#[derive(clap::Subcommand)]
enum Commands {
    // ... existing commands ...
    
    #[cfg(feature = "gpt")]
    /// GPT model management and serving
    Gpt {
        #[command(subcommand)]
        cmd: gpt_lib::GptCommands,
    },
}

// In main command handler:
#[cfg(feature = "gpt")]
Commands::Gpt { cmd } => {
    gpt_lib::handle_command(&workspace, cmd).await
}

// Enhanced LET command integration for GPT operations
use crate::commands::letcmd;

/// Enhanced LET command that includes GPT model operations
pub async fn enhanced_let_command(
    workspace: &crate::workspace::Workspace,
    target: &str,
    deploy: bool,
    build: bool,
    test: bool,
    args: Vec<String>,
) -> Result<()> {
    // Check if target is a GPT model operation
    if target.starts_with("gpt") || is_model_name(target) {
        return handle_gpt_let_command(workspace, target, deploy, build, test, args).await;
    }
    
    // Fall back to existing LET implementation
    letcmd::run(workspace, target, deploy, false, false, build, test, false, false, args, None, 1).await
}

/// Handle GPT-specific LET commands
async fn handle_gpt_let_command(
    workspace: &crate::workspace::Workspace,
    target: &str,
    deploy: bool,
    _build: bool,
    test: bool,
    args: Vec<String>,
) -> Result<()> {
    let mut gpt_manager = gpt_lib::GptManager::new(workspace.root()).await?;
    
    if target == "gpt" {
        // Parse GPT subcommand from args
        if args.is_empty() {
            return Err(anyhow::anyhow!("GPT LET command requires model specification"));
        }
        
        let subcommand = &args[0];
        match subcommand.as_str() {
            "serve" => {
                if args.len() < 2 {
                    return Err(anyhow::anyhow!("GPT serve requires model name"));
                }
                
                let model = &args[1];
                let mut creativity = 0.7;
                let mut port = 11434;
                
                // Parse additional arguments
                for arg in &args[2..] {
                    if arg.starts_with("--creativity=") {
                        creativity = arg.strip_prefix("--creativity=").unwrap().parse()?;
                    } else if arg.starts_with("--port=") {
                        port = arg.strip_prefix("--port=").unwrap().parse()?;
                    }
                }
                
                let cmd = gpt_lib::GptCommands::Serve {
                    model: model.clone(),
                    deploy,
                    port,
                    host: "localhost".to_string(),
                    gpu_layers: None,
                    threads: None,
                    context: 2048,
                    creativity,
                    backend: "ollama".to_string(),
                };
                
                gpt_manager.serve_model(&cmd).await?;
            }
            _ => {
                return Err(anyhow::anyhow!("Unknown GPT subcommand: {}", subcommand));
            }
        }
    } else {
        // Direct model name - assume serve operation
        let mut creativity = 0.7;
        let mut port = 11434;
        
        // Parse arguments for model-specific parameters
        for arg in &args {
            if arg.starts_with("--creativity=") {
                creativity = arg.strip_prefix("--creativity=").unwrap().parse()?;
            } else if arg.starts_with("--port=") {
                port = arg.strip_prefix("--port=").unwrap().parse()?;
            }
        }
        
        let cmd = gpt_lib::GptCommands::Serve {
            model: target.to_string(),
            deploy,
            port,
            host: "localhost".to_string(),
            gpu_layers: None,
            threads: None,
            context: 2048,
            creativity,
            backend: "ollama".to_string(),
        };
        
        gpt_manager.serve_model(&cmd).await?;
        
        // Run test if requested
        if test {
            let test_prompt = "Hello, this is a test prompt.";
            let response = gpt_manager.generate_text(target, test_prompt, 50, creativity).await?;
            println!("🧪 Test Response: {}", response);
        }
    }
    
    Ok(())
}

/// Check if a string is a known model name
fn is_model_name(target: &str) -> bool {
    let known_models = [
        "llama2", "llama3", "codellama", "mistral", "mixtral", "phi", "gemma",
        "qwen", "solar", "deepseek", "openchat", "starling", "zephyr",
        "vicuna", "alpaca", "wizard", "orca", "falcon"
    ];
    
    known_models.iter().any(|&model| target.starts_with(model))
}

/// GPT-lib configuration for RCM workspace
pub mod config {
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    
    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct GptWorkspaceConfig {
        pub enabled: bool,
        pub default_backend: String,
        pub default_model: Option<String>,
        pub models_directory: String,
        pub serving_defaults: ServingDefaults,
        pub model_aliases: HashMap<String, String>,
    }
    
    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct ServingDefaults {
        pub host: String,
        pub port: u16,
        pub context_length: usize,
        pub temperature: f32,
        pub max_tokens: usize,
        pub gpu_layers: Option<u32>,
    }
    
    impl Default for GptWorkspaceConfig {
        fn default() -> Self {
            Self {
                enabled: true,
                default_backend: "ollama".to_string(),
                default_model: None,
                models_directory: ".rcm/models".to_string(),
                serving_defaults: ServingDefaults::default(),
                model_aliases: HashMap::new(),
            }
        }
    }
    
    impl Default for ServingDefaults {
        fn default() -> Self {
            Self {
                host: "localhost".to_string(),
                port: 11434,
                context_length: 2048,
                temperature: 0.7,
                max_tokens: 256,
                gpu_layers: None,
            }
        }
    }
}

/// Examples and usage patterns
pub mod examples {
    use super::*;
    
    /// Example: Basic model serving workflow
    pub async fn basic_serving_example() -> Result<()> {
        println!("🚀 RCM GPT Basic Serving Example");
        
        // Initialize workspace (would be done by RCM)
        let workspace = crate::workspace::Workspace::new(None, crate::config::Config::default()).await?;
        
        // Install and serve a model
        let install_cmd = gpt_lib::GptCommands::Install {
            model: "llama2".to_string(),
            version: None,
            source: "ollama".to_string(),
            force: false,
        };
        
        gpt_lib::handle_command(&workspace, install_cmd).await?;
        
        // Serve the model with custom creativity
        let serve_cmd = gpt_lib::GptCommands::Serve {
            model: "llama2".to_string(),
            deploy: true,
            port: 11434,
            host: "localhost".to_string(),
            gpu_layers: None,
            threads: None,
            context: 4096,
            creativity: 0.8,
            backend: "ollama".to_string(),
        };
        
        gpt_lib::handle_command(&workspace, serve_cmd).await?;
        
        println!("✅ Model serving example completed");
        Ok(())
    }
    
    /// Example: LET command workflow
    pub async fn let_command_example() -> Result<()> {
        println!("⚡ RCM LET GPT Command Example");
        
        let workspace = crate::workspace::Workspace::new(None, crate::config::Config::default()).await?;
        
        // Using enhanced LET commands
        enhanced_let_command(
            &workspace,
            "llama2",
            true,  // deploy
            false, // build
            true,  // test
            vec!["--creativity=0.9".to_string(), "--port=8080".to_string()],
        ).await?;
        
        println!("✅ LET command example completed");
        Ok(())
    }
    
    /// Example: Multi-model serving
    pub async fn multi_model_example() -> Result<()> {
        println!("🔄 Multi-Model Serving Example");
        
        let workspace = crate::workspace::Workspace::new(None, crate::config::Config::default()).await?;
        let mut gpt_manager = gpt_lib::GptManager::new(workspace.root()).await?;
        
        // Install multiple models
        let models = ["llama2", "codellama", "mistral"];
        
        for model in &models {
            let install_cmd = gpt_lib::GptCommands::Install {
                model: model.to_string(),
                version: None,
                source: "ollama".to_string(),
                force: false,
            };
            gpt_lib::handle_command(&workspace, install_cmd).await?;
        }
        
        // Serve models on different ports
        for (i, model) in models.iter().enumerate() {
            let port = 11434 + i as u16;
            let serve_cmd = gpt_lib::GptCommands::Serve {
                model: model.to_string(),
                deploy: true,
                port,
                host: "localhost".to_string(),
                gpu_layers: None,
                threads: None,
                context: 2048,
                creativity: 0.7,
                backend: "ollama".to_string(),
            };
            
            gpt_lib::handle_command(&workspace, serve_cmd).await?;
            println!("✅ {} serving on port {}", model, port);
        }
        
        // List all running models
        let list_cmd = gpt_lib::GptCommands::List {
            running: true,
            format: "table".to_string(),
        };
        
        gpt_lib::handle_command(&workspace, list_cmd).await?;
        
        println!("✅ Multi-model serving example completed");
        Ok(())
    }
    
    /// Example: Development workflow with code generation
    pub async fn development_workflow_example() -> Result<()> {
        println!("💻 Development Workflow Example");
        
        let workspace = crate::workspace::Workspace::new(None, crate::config::Config::default()).await?;
        
        // Install and serve code generation model
        enhanced_let_command(
            &workspace,
            "codellama",
            true,  // deploy
            false, // build
            false, // test
            vec!["--creativity=0.2".to_string()], // Low creativity for code
        ).await?;
        
        // Generate some code
        let mut gpt_manager = gpt_lib::GptManager::new(workspace.root()).await?;
        
        let code_prompt = "Write a Rust function to calculate fibonacci numbers:";
        let generated_code = gpt_manager.generate_text("codellama", code_prompt, 200, 0.2).await?;
        
        println!("🤖 Generated Code:\n{}", generated_code);
        
        // Test the model with different prompts
        let prompts = [
            "Explain this Rust error: borrow checker",
            "Optimize this loop for performance",
            "Write unit tests for a sorting function",
        ];
        
        for prompt in &prompts {
            let response = gpt_manager.generate_text("codellama", prompt, 150, 0.3).await?;
            println!("\n🔹 Prompt: {}\n🤖 Response: {}", prompt, response);
        }
        
        println!("✅ Development workflow example completed");
        Ok(())
    }
}

/// CLI command examples for documentation
pub mod cli_examples {
    pub const BASIC_USAGE: &str = r#"
# Basic GPT model operations
rcm gpt install llama2
rcm gpt serve llama2 --deploy --port 11434 --creativity 0.7

# LET imperative syntax
rcm let gpt serve llama2 --deploy
rcm let llama2 --deploy --creativity 0.8
rcm let codellama --deploy --port 8080 --test

# List and manage models
rcm gpt list
rcm gpt list --running --format json
rcm gpt status llama2 --detailed

# Generate text
rcm gpt generate llama2 "Write a story about AI" --max-tokens 200
rcm gpt chat llama2 --interactive

# Install from different sources
rcm gpt install microsoft/DialoGPT-medium --source huggingface
rcm gpt install llama2:13b --source ollama

# Multi-model serving
rcm let llama2 --deploy --port 11434
rcm let codellama --deploy --port 11435  
rcm let mistral --deploy --port 11436

# Configuration
rcm gpt config llama2 --set temperature=0.8,context_length=4096
rcm config set gpt.default_model llama2
rcm config set gpt.serving_defaults.creativity 0.7
"#;

    pub const ADVANCED_USAGE: &str = r#"
# Industrial automation with GPT
rcm robot analyze-logs --model codellama --creativity 0.1
rcm let gpt serve llama2 --deploy --env production --parallel 4

# Integration with existing RCM features
rcm add transformers  # Python package
rcm let cargo --build --features gpt
rcm let gpt serve llama2 --deploy --arm-optimized

# Workspace integration
rcm workspace check --include-gpt-models
rcm snapshot --name "with-gpt-models" --include-gpt-state
rcm sbom --out sbom.json --include-gpt-models

# Performance optimization
rcm let llama2 --deploy --gpu-layers 32 --threads 8
rcm arm let simd --deploy --computation llm --vector-size 512
rcm let gpt serve llama2 --deploy --arm-accelerated

# Custom backends and formats
rcm gpt install ./my-model.gguf --source local
rcm gpt serve my-model --backend llamacpp --deploy
rcm gpt serve onnx-model --backend onnx --deploy
"#;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gpt_manager_creation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let manager = gpt_lib::GptManager::new(temp_dir.path()).await;
        assert!(manager.is_ok());
    }

    #[test]
    fn test_model_name_detection() {
        assert!(is_model_name("llama2"));
        assert!(is_model_name("codellama-instruct"));
        assert!(is_model_name("mistral-7b"));
        assert!(!is_model_name("cargo"));
        assert!(!is_model_name("npm"));
    }

    #[test]
    fn test_gpt_config_defaults() {
        let config = config::GptWorkspaceConfig::default();
        assert_eq!(config.default_backend, "ollama");
        assert_eq!(config.serving_defaults.port, 11434);
        assert_eq!(config.serving_defaults.temperature, 0.7);
    }
}
