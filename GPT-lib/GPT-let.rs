//! GPT-lib - AI Model Management & Serving for RCM
//! 
//! Provides LET imperatives for GPT model deployment, serving, and management
//! Compatible with Ollama, Hugging Face, and other model formats

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::fs;
use tokio::net::TcpListener;
use tokio::process::Command as AsyncCommand;
use reqwest;
use serde_json;

/// GPT model formats supported by RCM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelFormat {
    GGUF,
    ONNX,
    PyTorch,
    TensorFlow,
    Safetensors,
    Ollama,
}

/// Model serving backends
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServingBackend {
    Ollama,
    LlamaCpp,
    Onnx,
    Candle,
    TorchServe,
    TensorFlowServing,
    Custom(String),
}

/// Model deployment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub name: String,
    pub version: String,
    pub format: ModelFormat,
    pub backend: ServingBackend,
    pub model_path: PathBuf,
    pub config_path: Option<PathBuf>,
    pub tokenizer_path: Option<PathBuf>,
    pub parameters: ModelParameters,
    pub serving_config: ServingConfig,
}

/// Model runtime parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelParameters {
    pub context_length: usize,
    pub batch_size: usize,
    pub temperature: f32,
    pub top_p: f32,
    pub top_k: u32,
    pub repetition_penalty: f32,
    pub max_tokens: usize,
    pub gpu_layers: Option<u32>,
    pub cpu_threads: Option<u32>,
}

/// Serving configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServingConfig {
    pub host: String,
    pub port: u16,
    pub api_version: String,
    pub enable_cors: bool,
    pub auth_token: Option<String>,
    pub rate_limit: Option<u32>,
    pub timeout_seconds: u64,
    pub health_check_path: String,
}

/// Model registry for managing available models
#[derive(Debug, Serialize, Deserialize)]
pub struct ModelRegistry {
    pub models: HashMap<String, ModelConfig>,
    pub active_models: HashMap<String, ModelInstance>,
    pub default_model: Option<String>,
    pub registry_path: PathBuf,
}

/// Running model instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInstance {
    pub config: ModelConfig,
    pub process_id: Option<u32>,
    pub endpoint: String,
    pub status: ModelStatus,
    pub started_at: String,
    pub memory_usage: Option<u64>,
    pub gpu_usage: Option<f32>,
}

/// Model status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelStatus {
    Stopped,
    Starting,
    Running,
    Error(String),
    Updating,
}

/// GPT CLI commands
#[derive(Subcommand)]
pub enum GptCommands {
    /// Serve a GPT model
    Serve {
        /// Model name or path
        model: String,
        /// Deploy and start serving
        #[arg(long)]
        deploy: bool,
        /// Port to serve on
        #[arg(long, default_value = "11434")]
        port: u16,
        /// Host to bind to
        #[arg(long, default_value = "localhost")]
        host: String,
        /// GPU layers to use
        #[arg(long)]
        gpu_layers: Option<u32>,
        /// CPU threads
        #[arg(long)]
        threads: Option<u32>,
        /// Context length
        #[arg(long, default_value = "2048")]
        context: usize,
        /// Creativity level (temperature)
        #[arg(long, default_value = "0.7")]
        creativity: f32,
        /// Serving backend
        #[arg(long, default_value = "ollama")]
        backend: String,
    },
    
    /// Download and install a model
    Install {
        /// Model name (e.g., llama2, codellama, mistral)
        model: String,
        /// Model version or tag
        #[arg(long)]
        version: Option<String>,
        /// Source registry
        #[arg(long, default_value = "ollama")]
        source: String,
        /// Force reinstall
        #[arg(long)]
        force: bool,
    },
    
    /// Remove a model
    Remove {
        /// Model name
        model: String,
        /// Remove all versions
        #[arg(long)]
        all_versions: bool,
    },
    
    /// List available models
    List {
        /// Show only running models
        #[arg(long)]
        running: bool,
        /// Output format
        #[arg(long, default_value = "table")]
        format: String,
    },
    
    /// Stop a running model
    Stop {
        /// Model name
        model: String,
    },
    
    /// Model health check and status
    Status {
        /// Model name (all if not specified)
        model: Option<String>,
        /// Detailed status information
        #[arg(long)]
        detailed: bool,
    },
    
    /// Update model to latest version
    Update {
        /// Model name
        model: String,
    },
    
    /// Chat with a model
    Chat {
        /// Model name
        model: String,
        /// Chat message
        message: Option<String>,
        /// Interactive mode
        #[arg(long)]
        interactive: bool,
    },
    
    /// Generate text completion
    Generate {
        /// Model name
        model: String,
        /// Prompt text
        prompt: String,
        /// Maximum tokens to generate
        #[arg(long, default_value = "100")]
        max_tokens: usize,
        /// Temperature (creativity)
        #[arg(long, default_value = "0.7")]
        temperature: f32,
    },
    
    /// Configure model settings
    Config {
        /// Model name
        model: String,
        /// Configuration key=value pairs
        #[arg(long, value_delimiter = ',')]
        set: Vec<String>,
        /// Show current configuration
        #[arg(long)]
        show: bool,
    },
}

/// GPT model manager
pub struct GptManager {
    registry: ModelRegistry,
    workspace_root: PathBuf,
    models_dir: PathBuf,
    configs_dir: PathBuf,
}

impl Default for ModelParameters {
    fn default() -> Self {
        Self {
            context_length: 2048,
            batch_size: 1,
            temperature: 0.7,
            top_p: 0.9,
            top_k: 40,
            repetition_penalty: 1.1,
            max_tokens: 256,
            gpu_layers: None,
            cpu_threads: None,
        }
    }
}

impl Default for ServingConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 11434,
            api_version: "v1".to_string(),
            enable_cors: true,
            auth_token: None,
            rate_limit: None,
            timeout_seconds: 30,
            health_check_path: "/health".to_string(),
        }
    }
}

impl GptManager {
    /// Create new GPT manager
    pub async fn new(workspace_root: &Path) -> Result<Self> {
        let models_dir = workspace_root.join(".rcm").join("models");
        let configs_dir = workspace_root.join(".rcm").join("gpt-configs");
        let registry_path = configs_dir.join("registry.json");
        
        // Create directories
        fs::create_dir_all(&models_dir).await?;
        fs::create_dir_all(&configs_dir).await?;
        
        // Load or create registry
        let registry = if registry_path.exists() {
            let content = fs::read_to_string(&registry_path).await?;
            serde_json::from_str(&content)?
        } else {
            ModelRegistry {
                models: HashMap::new(),
                active_models: HashMap::new(),
                default_model: None,
                registry_path: registry_path.clone(),
            }
        };
        
        Ok(Self {
            registry,
            workspace_root: workspace_root.to_path_buf(),
            models_dir,
            configs_dir,
        })
    }
    
    /// Serve a model with LET imperative
    pub async fn serve_model(&mut self, cmd: &GptCommands) -> Result<()> {
        if let GptCommands::Serve { 
            model, deploy, port, host, gpu_layers, threads, 
            context, creativity, backend 
        } = cmd {
            
            println!("🚀 RCM LET GPT serve {} --deploy", model);
            
            // Check if model exists
            if !self.model_exists(model).await? {
                println!("📥 Model '{}' not found, downloading...", model);
                self.install_model(model, None, "ollama", false).await?;
            }
            
            // Configure model parameters
            let mut model_config = self.get_or_create_model_config(model).await?;
            model_config.parameters.context_length = *context;
            model_config.parameters.temperature = *creativity;
            model_config.parameters.gpu_layers = *gpu_layers;
            model_config.parameters.cpu_threads = *threads;
            model_config.serving_config.host = host.clone();
            model_config.serving_config.port = *port;
            model_config.backend = self.parse_backend(backend)?;
            
            if *deploy {
                self.deploy_model(&model_config).await?;
            } else {
                self.configure_model(&model_config).await?;
            }
            
            Ok(())
        } else {
            Err(anyhow!("Invalid serve command"))
        }
    }
    
    /// Install a model
    pub async fn install_model(&mut self, model: &str, version: Option<&str>, source: &str, force: bool) -> Result<()> {
        println!("📦 Installing model: {} from {}", model, source);
        
        match source {
            "ollama" => self.install_ollama_model(model, version, force).await,
            "huggingface" => self.install_huggingface_model(model, version, force).await,
            "local" => self.install_local_model(model, version).await,
            _ => Err(anyhow!("Unsupported model source: {}", source)),
        }
    }
    
    /// Install model via Ollama
    async fn install_ollama_model(&mut self, model: &str, version: Option<&str>, force: bool) -> Result<()> {
        let model_spec = if let Some(ver) = version {
            format!("{}:{}", model, ver)
        } else {
            model.to_string()
        };
        
        // Check if Ollama is available
        if !self.check_ollama_available().await {
            return Err(anyhow!("Ollama is not installed or not running. Install from https://ollama.ai/"));
        }
        
        let mut cmd = AsyncCommand::new("ollama");
        cmd.arg("pull").arg(&model_spec);
        
        if force {
            cmd.arg("--force");
        }
        
        let output = cmd.output().await?;
        
        if !output.status.success() {
            return Err(anyhow!(
                "Failed to pull model: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        
        // Register model in RCM registry
        let config = ModelConfig {
            name: model.to_string(),
            version: version.unwrap_or("latest").to_string(),
            format: ModelFormat::Ollama,
            backend: ServingBackend::Ollama,
            model_path: self.models_dir.join(model),
            config_path: None,
            tokenizer_path: None,
            parameters: ModelParameters::default(),
            serving_config: ServingConfig::default(),
        };
        
        self.registry.models.insert(model.to_string(), config);
        self.save_registry().await?;
        
        println!("✅ Model '{}' installed successfully", model);
        Ok(())
    }
    
    /// Install model from Hugging Face
    async fn install_huggingface_model(&mut self, model: &str, version: Option<&str>, force: bool) -> Result<()> {
        println!("📥 Downloading from Hugging Face: {}", model);
        
        // Use huggingface-hub or git clone
        let model_dir = self.models_dir.join(model);
        
        if model_dir.exists() && !force {
            return Err(anyhow!("Model already exists. Use --force to reinstall."));
        }
        
        // Clone from Hugging Face
        let repo_url = format!("https://huggingface.co/{}", model);
        let mut cmd = AsyncCommand::new("git");
        cmd.arg("clone")
           .arg(&repo_url)
           .arg(&model_dir);
        
        if let Some(ver) = version {
            cmd.arg("--branch").arg(ver);
        }
        
        let output = cmd.output().await?;
        
        if !output.status.success() {
            return Err(anyhow!(
                "Failed to clone model: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        
        // Auto-detect model format
        let format = self.detect_model_format(&model_dir).await?;
        
        let config = ModelConfig {
            name: model.to_string(),
            version: version.unwrap_or("main").to_string(),
            format,
            backend: ServingBackend::LlamaCpp, // Default for HF models
            model_path: model_dir,
            config_path: None,
            tokenizer_path: None,
            parameters: ModelParameters::default(),
            serving_config: ServingConfig::default(),
        };
        
        self.registry.models.insert(model.to_string(), config);
        self.save_registry().await?;
        
        println!("✅ Model '{}' downloaded from Hugging Face", model);
        Ok(())
    }
    
    /// Deploy and start serving a model
    async fn deploy_model(&mut self, config: &ModelConfig) -> Result<()> {
        println!("🚀 Deploying model: {}", config.name);
        
        match config.backend {
            ServingBackend::Ollama => self.deploy_ollama_model(config).await,
            ServingBackend::LlamaCpp => self.deploy_llamacpp_model(config).await,
            ServingBackend::Candle => self.deploy_candle_model(config).await,
            _ => Err(anyhow!("Backend not yet implemented: {:?}", config.backend)),
        }
    }
    
    /// Deploy model using Ollama
    async fn deploy_ollama_model(&mut self, config: &ModelConfig) -> Result<()> {
        let mut cmd = AsyncCommand::new("ollama");
        cmd.arg("serve");
        
        // Set environment variables for configuration
        cmd.env("OLLAMA_HOST", format!("{}:{}", config.serving_config.host, config.serving_config.port));
        
        if let Some(gpu_layers) = config.parameters.gpu_layers {
            cmd.env("OLLAMA_NUM_GPU", gpu_layers.to_string());
        }
        
        if let Some(threads) = config.parameters.cpu_threads {
            cmd.env("OLLAMA_NUM_THREAD", threads.to_string());
        }
        
        // Start ollama serve in background
        let child = cmd.spawn()?;
        
        // Wait a moment for server to start
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        
        // Load the specific model
        let mut load_cmd = AsyncCommand::new("ollama");
        load_cmd.arg("run").arg(&config.name).arg("--verbose");
        
        let output = load_cmd.output().await?;
        
        if !output.status.success() {
            return Err(anyhow!(
                "Failed to load model: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        
        // Register as active model
        let instance = ModelInstance {
            config: config.clone(),
            process_id: child.id(),
            endpoint: format!("http://{}:{}", config.serving_config.host, config.serving_config.port),
            status: ModelStatus::Running,
            started_at: chrono::Utc::now().to_rfc3339(),
            memory_usage: None,
            gpu_usage: None,
        };
        
        self.registry.active_models.insert(config.name.clone(), instance);
        self.save_registry().await?;
        
        println!("✅ Model '{}' deployed and running on {}:{}", 
                config.name, config.serving_config.host, config.serving_config.port);
        println!("🌐 API endpoint: http://{}:{}/api/generate", 
                config.serving_config.host, config.serving_config.port);
        
        Ok(())
    }
    
    /// Deploy model using llama.cpp
    async fn deploy_llamacpp_model(&mut self, config: &ModelConfig) -> Result<()> {
        // Check if llama.cpp server is available
        if !self.check_llamacpp_available().await {
            return Err(anyhow!("llama.cpp server not found. Install llama.cpp or use different backend."));
        }
        
        let mut cmd = AsyncCommand::new("llama-server");
        cmd.arg("--model").arg(&config.model_path)
           .arg("--host").arg(&config.serving_config.host)
           .arg("--port").arg(config.serving_config.port.to_string())
           .arg("--ctx-size").arg(config.parameters.context_length.to_string());
        
        if let Some(gpu_layers) = config.parameters.gpu_layers {
            cmd.arg("--n-gpu-layers").arg(gpu_layers.to_string());
        }
        
        if let Some(threads) = config.parameters.cpu_threads {
            cmd.arg("--threads").arg(threads.to_string());
        }
        
        let child = cmd.spawn()?;
        
        // Register as active model
        let instance = ModelInstance {
            config: config.clone(),
            process_id: child.id(),
            endpoint: format!("http://{}:{}", config.serving_config.host, config.serving_config.port),
            status: ModelStatus::Running,
            started_at: chrono::Utc::now().to_rfc3339(),
            memory_usage: None,
            gpu_usage: None,
        };
        
        self.registry.active_models.insert(config.name.clone(), instance);
        self.save_registry().await?;
        
        println!("✅ Model '{}' deployed with llama.cpp on {}:{}", 
                config.name, config.serving_config.host, config.serving_config.port);
        
        Ok(())
    }
    
    /// List available models
    pub async fn list_models(&self, running_only: bool, format: &str) -> Result<()> {
        match format {
            "table" => self.list_models_table(running_only).await,
            "json" => self.list_models_json(running_only).await,
            _ => Err(anyhow!("Unsupported format: {}", format)),
        }
    }
    
    /// List models in table format
    async fn list_models_table(&self, running_only: bool) -> Result<()> {
        use tabled::{Table, Tabled};
        
        #[derive(Tabled)]
        struct ModelRow {
            #[tabled(rename = "Name")]
            name: String,
            #[tabled(rename = "Version")]
            version: String,
            #[tabled(rename = "Backend")]
            backend: String,
            #[tabled(rename = "Status")]
            status: String,
            #[tabled(rename = "Endpoint")]
            endpoint: String,
        }
        
        let mut rows = Vec::new();
        
        for (name, config) in &self.registry.models {
            if running_only && !self.registry.active_models.contains_key(name) {
                continue;
            }
            
            let (status, endpoint) = if let Some(instance) = self.registry.active_models.get(name) {
                (format!("{:?}", instance.status), instance.endpoint.clone())
            } else {
                ("Stopped".to_string(), "N/A".to_string())
            };
            
            rows.push(ModelRow {
                name: name.clone(),
                version: config.version.clone(),
                backend: format!("{:?}", config.backend),
                status,
                endpoint,
            });
        }
        
        if rows.is_empty() {
            println!("No models found.");
        } else {
            let table = Table::new(rows);
            println!("{}", table);
        }
        
        Ok(())
    }
    
    /// Generate text using a model
    pub async fn generate_text(&self, model: &str, prompt: &str, max_tokens: usize, temperature: f32) -> Result<String> {
        let instance = self.registry.active_models.get(model)
            .ok_or_else(|| anyhow!("Model '{}' is not running", model))?;
        
        match instance.config.backend {
            ServingBackend::Ollama => self.generate_ollama(instance, prompt, max_tokens, temperature).await,
            ServingBackend::LlamaCpp => self.generate_llamacpp(instance, prompt, max_tokens, temperature).await,
            _ => Err(anyhow!("Text generation not implemented for backend: {:?}", instance.config.backend)),
        }
    }
    
    /// Generate text using Ollama API
    async fn generate_ollama(&self, instance: &ModelInstance, prompt: &str, max_tokens: usize, temperature: f32) -> Result<String> {
        let client = reqwest::Client::new();
        let url = format!("{}/api/generate", instance.endpoint);
        
        let request_body = serde_json::json!({
            "model": instance.config.name,
            "prompt": prompt,
            "stream": false,
            "options": {
                "num_predict": max_tokens,
                "temperature": temperature,
            }
        });
        
        let response = client.post(&url)
            .json(&request_body)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(anyhow!("API request failed: {}", response.status()));
        }
        
        let result: serde_json::Value = response.json().await?;
        let generated_text = result["response"]
            .as_str()
            .ok_or_else(|| anyhow!("Invalid response format"))?;
        
        Ok(generated_text.to_string())
    }
    
    // Helper methods
    async fn model_exists(&self, model: &str) -> Result<bool> {
        Ok(self.registry.models.contains_key(model))
    }
    
    async fn check_ollama_available(&self) -> bool {
        AsyncCommand::new("ollama")
            .arg("--version")
            .output()
            .await
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
    
    async fn check_llamacpp_available(&self) -> bool {
        AsyncCommand::new("llama-server")
            .arg("--help")
            .output()
            .await
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
    
    async fn detect_model_format(&self, model_dir: &Path) -> Result<ModelFormat> {
        if model_dir.join("config.json").exists() {
            Ok(ModelFormat::Safetensors)
        } else if model_dir.join("pytorch_model.bin").exists() {
            Ok(ModelFormat::PyTorch)
        } else if model_dir.join("model.onnx").exists() {
            Ok(ModelFormat::ONNX)
        } else {
            Ok(ModelFormat::GGUF) // Default assumption
        }
    }
    
    async fn get_or_create_model_config(&mut self, model: &str) -> Result<ModelConfig> {
        if let Some(config) = self.registry.models.get(model) {
            Ok(config.clone())
        } else {
            // Create default config
            let config = ModelConfig {
                name: model.to_string(),
                version: "latest".to_string(),
                format: ModelFormat::Ollama,
                backend: ServingBackend::Ollama,
                model_path: self.models_dir.join(model),
                config_path: None,
                tokenizer_path: None,
                parameters: ModelParameters::default(),
                serving_config: ServingConfig::default(),
            };
            Ok(config)
        }
    }
    
    fn parse_backend(&self, backend: &str) -> Result<ServingBackend> {
        match backend.to_lowercase().as_str() {
            "ollama" => Ok(ServingBackend::Ollama),
            "llamacpp" | "llama.cpp" => Ok(ServingBackend::LlamaCpp),
            "onnx" => Ok(ServingBackend::Onnx),
            "candle" => Ok(ServingBackend::Candle),
            "torchserve" => Ok(ServingBackend::TorchServe),
            "tensorflow" | "tfserving" => Ok(ServingBackend::TensorFlowServing),
            _ => Ok(ServingBackend::Custom(backend.to_string())),
        }
    }
    
    async fn configure_model(&mut self, config: &ModelConfig) -> Result<()> {
        self.registry.models.insert(config.name.clone(), config.clone());
        self.save_registry().await?;
        println!("⚙️ Model '{}' configured", config.name);
        Ok(())
    }
    
    async fn save_registry(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.registry)?;
        fs::write(&self.registry.registry_path, content).await?;
        Ok(())
    }
    
    // Placeholder implementations
    async fn install_local_model(&mut self, _model: &str, _version: Option<&str>) -> Result<()> {
        todo!("Local model installation")
    }
    
    async fn deploy_candle_model(&mut self, _config: &ModelConfig) -> Result<()> {
        todo!("Candle backend deployment")
    }
    
    async fn list_models_json(&self, _running_only: bool) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.registry)?;
        println!("{}", json);
        Ok(())
    }
    
    async fn generate_llamacpp(&self, _instance: &ModelInstance, _prompt: &str, _max_tokens: usize, _temperature: f32) -> Result<String> {
        todo!("LlamaCpp text generation")
    }
}

/// Handle GPT commands
pub async fn handle_command(workspace: &crate::workspace::Workspace, cmd: GptCommands) -> Result<()> {
    let mut gpt_manager = GptManager::new(workspace.root()).await?;
    
    match cmd {
        GptCommands::Serve { .. } => {
            gpt_manager.serve_model(&cmd).await
        }
        GptCommands::Install { model, version, source, force } => {
            gpt_manager.install_model(&model, version.as_deref(), &source, force).await
        }
        GptCommands::List { running, format } => {
            gpt_manager.list_models(running, &format).await
        }
        GptCommands::Generate { model, prompt, max_tokens, temperature } => {
            let result = gpt_manager.generate_text(&model, &prompt, max_tokens, temperature).await?;
            println!("{}", result);
            Ok(())
        }
        _ => {
            println!("Command not yet implemented: {:?}", cmd);
            Ok(())
        }
    }
}
