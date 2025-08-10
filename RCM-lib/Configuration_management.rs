//! Configuration management for RCM
//! 
//! Handles loading and saving configuration from files, environment variables, and command line

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use crate::util::get_os_info;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub version: String,
    pub core: CoreConfig,
    pub managers: HashMap<String, ManagerSettings>,
    pub registries: HashMap<String, RegistryConfig>,
    pub proxies: HashMap<String, ProxyConfig>,
    pub auth: HashMap<String, AuthConfig>,
    pub ui: UiConfig,
    pub telemetry: TelemetryConfig,
    pub cache: CacheConfig,
    pub security: SecurityConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CoreConfig {
    pub default_manager: Option<String>,
    pub parallel_jobs: usize,
    pub timeout_seconds: u64,
    pub retry_attempts: u32,
    pub offline_mode: bool,
    pub verify_checksums: bool,
    pub auto_update: bool,
    pub workspace_detection: bool,
    pub color_output: ColorMode,
    pub log_level: LogLevel,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ManagerSettings {
    pub enabled: bool,
    pub version: Option<String>,
    pub binary_path: Option<String>,
    pub install_dir: Option<String>,
    pub registry: Option<String>,
    pub proxy: Option<String>,
    pub auth: Option<String>,
    pub options: HashMap<String, serde_json::Value>,
    pub env_vars: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RegistryConfig {
    pub url: String,
    pub auth: Option<String>,
    pub mirror: Option<String>,
    pub timeout_seconds: u64,
    pub trusted: bool,
    pub verify_ssl: bool,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProxyConfig {
    pub http: Option<String>,
    pub https: Option<String>,
    pub no_proxy: Vec<String>,
    pub auth: Option<ProxyAuth>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProxyAuth {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthConfig {
    pub auth_type: AuthType,
    pub token: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub key_file: Option<String>,
    pub cert_file: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AuthType {
    Token,
    Basic,
    Certificate,
    SSH,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UiConfig {
    pub color: bool,
    pub unicode: bool,
    pub progress_bars: bool,
    pub confirm_prompts: bool,
    pub editor: Option<String>,
    pub pager: Option<String>,
    pub theme: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TelemetryConfig {
    pub enabled: bool,
    pub anonymous: bool,
    pub endpoint: Option<String>,
    pub collect_performance: bool,
    pub collect_errors: bool,
    pub collect_usage: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CacheConfig {
    pub enabled: bool,
    pub directory: Option<String>,
    pub max_size_mb: u64,
    pub ttl_hours: u64,
    pub compress: bool,
    pub cleanup_on_exit: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SecurityConfig {
    pub verify_signatures: bool,
    pub allow_insecure: bool,
    pub trusted_keys: Vec<String>,
    pub blocked_packages: Vec<String>,
    pub scan_for_vulnerabilities: bool,
    pub quarantine_suspicious: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ColorMode {
    Auto,
    Always,
    Never,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: "1.0.0".to_string(),
            core: CoreConfig::default(),
            managers: Self::default_managers(),
            registries: Self::default_registries(),
            proxies: HashMap::new(),
            auth: HashMap::new(),
            ui: UiConfig::default(),
            telemetry: TelemetryConfig::default(),
            cache: CacheConfig::default(),
            security: SecurityConfig::default(),
        }
    }
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            default_manager: None,
            parallel_jobs: num_cpus::get(),
            timeout_seconds: 300,
            retry_attempts: 3,
            offline_mode: false,
            verify_checksums: true,
            auto_update: false,
            workspace_detection: true,
            color_output: ColorMode::Auto,
            log_level: LogLevel::Info,
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            color: true,
            unicode: true,
            progress_bars: true,
            confirm_prompts: true,
            editor: std::env::var("EDITOR").ok(),
            pager: std::env::var("PAGER").ok(),
            theme: "default".to_string(),
        }
    }
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            anonymous: true,
            endpoint: None,
            collect_performance: true,
            collect_errors: true,
            collect_usage: true,
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            directory: None,
            max_size_mb: 1024,
            ttl_hours: 24,
            compress: true,
            cleanup_on_exit: false,
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            verify_signatures: true,
            allow_insecure: false,
            trusted_keys: vec![],
            blocked_packages: vec![],
            scan_for_vulnerabilities: true,
            quarantine_suspicious: true,
        }
    }
}

impl Config {
    /// Load configuration from file or create default
    pub async fn load(config_path: Option<&str>) -> Result<Self> {
        let config_file = if let Some(path) = config_path {
            PathBuf::from(path)
        } else {
            Self::default_config_path()?
        };

        if config_file.exists() {
            Self::load_from_file(&config_file).await
        } else {
            let config = Self::default();
            config.save_to_file(&config_file).await?;
            Ok(config)
        }
    }

    /// Get default configuration file path
    fn default_config_path() -> Result<PathBuf> {
        if let Some(config_dir) = dirs::config_dir() {
            Ok(config_dir.join("rcm").join("config.json"))
        } else {
            // Fallback to home directory
            if let Some(home_dir) = dirs::home_dir() {
                Ok(home_dir.join(".rcm").join("config.json"))
            } else {
                Ok(PathBuf::from(".rcm").join("config.json"))
            }
        }
    }

    /// Load configuration from file
    async fn load_from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path).await
            .context("Failed to read configuration file")?;

        let mut config: Self = serde_json::from_str(&content)
            .context("Failed to parse configuration file")?;

        // Apply environment variable overrides
        config.apply_env_overrides().await?;

        Ok(config)
    }

    /// Save configuration to file
    pub async fn save_to_file(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await
                .context("Failed to create config directory")?;
        }

        let content = serde_json::to_string_pretty(self)
            .context("Failed to serialize configuration")?;

        fs::write(path, content).await
            .context("Failed to write configuration file")
    }

    /// Apply environment variable overrides
    async fn apply_env_overrides(&mut self) -> Result<()> {
        // Core config overrides
        if let Ok(jobs) = std::env::var("RCM_PARALLEL_JOBS") {
            if let Ok(jobs) = jobs.parse::<usize>() {
                self.core.parallel_jobs = jobs;
            }
        }

        if let Ok(timeout) = std::env::var("RCM_TIMEOUT") {
            if let Ok(timeout) = timeout.parse::<u64>() {
                self.core.timeout_seconds = timeout;
            }
        }

        if let Ok(offline) = std::env::var("RCM_OFFLINE") {
            self.core.offline_mode = offline.parse().unwrap_or(false);
        }

        if let Ok(log_level) = std::env::var("RCM_LOG_LEVEL") {
            self.core.log_level = match log_level.to_lowercase().as_str() {
                "error" => LogLevel::Error,
                "warn" => LogLevel::Warn,
                "info" => LogLevel::Info,
                "debug" => LogLevel::Debug,
                "trace" => LogLevel::Trace,
                _ => LogLevel::Info,
            };
        }

        // Proxy settings
        if let Ok(http_proxy) = std::env::var("HTTP_PROXY") {
            self.proxies.insert("default".to_string(), ProxyConfig {
                http: Some(http_proxy.clone()),
                https: std::env::var("HTTPS_PROXY").ok().or(Some(http_proxy)),
                no_proxy: std::env::var("NO_PROXY")
                    .unwrap_or_default()
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect(),
                auth: None,
            });
        }

        // Manager-specific overrides
        for (manager, settings) in &mut self.managers {
            let env_prefix = format!("RCM_{}_", manager.to_uppercase());
            
            if let Ok(enabled) = std::env::var(format!("{}ENABLED", env_prefix)) {
                settings.enabled = enabled.parse().unwrap_or(settings.enabled);
            }
            
            if let Ok(binary) = std::env::var(format!("{}BINARY", env_prefix)) {
                settings.binary_path = Some(binary);
            }
            
            if let Ok(registry) = std::env::var(format!("{}REGISTRY", env_prefix)) {
                settings.registry = Some(registry);
            }
        }

        Ok(())
    }

    /// Default manager configurations
    fn default_managers() -> HashMap<String, ManagerSettings> {
        let mut managers = HashMap::new();

        // Cargo (Rust)
        managers.insert("cargo".to_string(), ManagerSettings {
            enabled: true,
            version: None,
            binary_path: None,
            install_dir: Some("target".to_string()),
            registry: Some("https://crates.io".to_string()),
            proxy: None,
            auth: None,
            options: HashMap::new(),
            env_vars: HashMap::new(),
        });

        // NPM (Node.js)
        managers.insert("npm".to_string(), ManagerSettings {
            enabled: true,
            version: None,
            binary_path: None,
            install_dir: Some("node_modules".to_string()),
            registry: Some("https://registry.npmjs.org".to_string()),
            proxy: None,
            auth: None,
            options: HashMap::new(),
            env_vars: HashMap::new(),
        });

        // Composer (PHP)
        managers.insert("composer".to_string(), ManagerSettings {
            enabled: true,
            version: None,
            binary_path: None,
            install_dir: Some("vendor".to_string()),
            registry: Some("https://packagist.org".to_string()),
            proxy: None,
            auth: None,
            options: HashMap::new(),
            env_vars: HashMap::new(),
        });

        // System package manager
        managers.insert("system".to_string(), ManagerSettings {
            enabled: true,
            version: None,
            binary_path: None,
            install_dir: None,
            registry: None,
            proxy: None,
            auth: None,
            options: HashMap::new(),
            env_vars: HashMap::new(),
        });

        managers
    }

    /// Default registry configurations
    fn default_registries() -> HashMap<String, RegistryConfig> {
        let mut registries = HashMap::new();

        registries.insert("crates.io".to_string(), RegistryConfig {
            url: "https://crates.io".to_string(),
            auth: None,
            mirror: None,
            timeout_seconds: 30,
            trusted: true,
            verify_ssl: true,
            metadata: HashMap::new(),
        });

        registries.insert("npmjs".to_string(), RegistryConfig {
            url: "https://registry.npmjs.org".to_string(),
            auth: None,
            mirror: None,
            timeout_seconds: 30,
            trusted: true,
            verify_ssl: true,
            metadata: HashMap::new(),
        });

        registries.insert("packagist".to_string(), RegistryConfig {
            url: "https://packagist.org".to_string(),
            auth: None,
            mirror: None,
            timeout_seconds: 30,
            trusted: true,
            verify_ssl: true,
            metadata: HashMap::new(),
        });

        registries
    }

    /// Get configuration value by key path
    pub fn get(&self, key: &str) -> Result<serde_json::Value> {
        let parts: Vec<&str> = key.split('.').collect();
        
        match parts.as_slice() {
            ["core", "parallel_jobs"] => Ok(serde_json::Value::Number(self.core.parallel_jobs.into())),
            ["core", "timeout_seconds"] => Ok(serde_json::Value::Number(self.core.timeout_seconds.into())),
            ["core", "offline_mode"] => Ok(serde_json::Value::Bool(self.core.offline_mode)),
            ["core", "verify_checksums"] => Ok(serde_json::Value::Bool(self.core.verify_checksums)),
            ["ui", "color"] => Ok(serde_json::Value::Bool(self.ui.color)),
            ["ui", "progress_bars"] => Ok(serde_json::Value::Bool(self.ui.progress_bars)),
            ["cache", "enabled"] => Ok(serde_json::Value::Bool(self.cache.enabled)),
            ["cache", "max_size_mb"] => Ok(serde_json::Value::Number(self.cache.max_size_mb.into())),
            ["telemetry", "enabled"] => Ok(serde_json::Value::Bool(self.telemetry.enabled)),
            _ => Err(anyhow!("Unknown configuration key: {}", key)),
        }
    }

    /// Set configuration value by key path
    pub fn set(&mut self, key: &str, value: &str) -> Result<()> {
        let parts: Vec<&str> = key.split('.').collect();
        
        match parts.as_slice() {
            ["core", "parallel_jobs"] => {
                self.core.parallel_jobs = value.parse()
                    .context("Invalid value for parallel_jobs")?;
            }
            ["core", "timeout_seconds"] => {
                self.core.timeout_seconds = value.parse()
                    .context("Invalid value for timeout_seconds")?;
            }
            ["core", "offline_mode"] => {
                self.core.offline_mode = value.parse()
                    .context("Invalid boolean value for offline_mode")?;
            }
            ["core", "verify_checksums"] => {
                self.core.verify_checksums = value.parse()
                    .context("Invalid boolean value for verify_checksums")?;
            }
            ["ui", "color"] => {
                self.ui.color = value.parse()
                    .context("Invalid boolean value for ui.color")?;
            }
            ["ui", "progress_bars"] => {
                self.ui.progress_bars = value.parse()
                    .context("Invalid boolean value for ui.progress_bars")?;
            }
            ["ui", "editor"] => {
                self.ui.editor = if value.is_empty() { None } else { Some(value.to_string()) };
            }
            ["cache", "enabled"] => {
                self.cache.enabled = value.parse()
                    .context("Invalid boolean value for cache.enabled")?;
            }
            ["cache", "max_size_mb"] => {
                self.cache.max_size_mb = value.parse()
                    .context("Invalid value for cache.max_size_mb")?;
            }
            ["telemetry", "enabled"] => {
                self.telemetry.enabled = value.parse()
                    .context("Invalid boolean value for telemetry.enabled")?;
            }
            _ => return Err(anyhow!("Unknown configuration key: {}", key)),
        }
        
        Ok(())
    }

    /// Reset configuration to defaults
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Get cache directory
    pub fn cache_dir(&self) -> PathBuf {
        if let Some(ref dir) = self.cache.directory {
            PathBuf::from(dir)
        } else if let Some(cache_dir) = dirs::cache_dir() {
            cache_dir.join("rcm")
        } else {
            PathBuf::from(".rcm").join("cache")
        }
    }

    /// Get data directory
    pub fn data_dir(&self) -> PathBuf {
        if let Some(data_dir) = dirs::data_dir() {
            data_dir.join("rcm")
        } else {
            PathBuf::from(".rcm").join("data")
        }
    }

    /// Check if manager is enabled
    pub fn is_manager_enabled(&self, manager: &str) -> bool {
        self.managers
            .get(manager)
            .map_or(false, |settings| settings.enabled)
    }

    /// Get manager settings
    pub fn get_manager_settings(&self, manager: &str) -> Option<&ManagerSettings> {
        self.managers.get(manager)
    }

    /// Get registry configuration
    pub fn get_registry(&self, registry: &str) -> Option<&RegistryConfig> {
        self.registries.get(registry)
    }

    /// Get proxy configuration
    pub fn get_proxy(&self, name: &str) -> Option<&ProxyConfig> {
        self.proxies.get(name)
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Check that at least one manager is enabled
        if !self.managers.values().any(|m| m.enabled) {
            return Err(anyhow!("At least one package manager must be enabled"));
        }

        // Validate parallel jobs
        if self.core.parallel_jobs == 0 {
            return Err(anyhow!("parallel_jobs must be greater than 0"));
        }

        // Validate timeout
        if self.core.timeout_seconds == 0 {
            return Err(anyhow!("timeout_seconds must be greater than 0"));
        }

        // Validate cache size
        if self.cache.max_size_mb == 0 && self.cache.enabled {
            return Err(anyhow!("cache.max_size_mb must be greater than 0 when cache is enabled"));
        }

        Ok(())
    }
}
