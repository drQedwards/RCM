//! LET specification types — target, constraints, actions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LetSpec {
    pub target: String,
    pub version: Option<String>,
    pub manager: Option<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub actions: Vec<LetAction>,
    #[serde(default)]
    pub environment: HashMap<String, String>,
    #[serde(default)]
    pub constraints: LetConstraints,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LetAction {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub working_dir: Option<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub conditions: Vec<LetCondition>,
    #[serde(default)]
    pub parallel: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LetCondition {
    pub condition_type: LetConditionType,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LetConditionType {
    FileExists,
    CommandExists,
    EnvVar,
    Platform,
    PackageInstalled,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LetConstraints {
    #[serde(default)]
    pub platforms: Vec<String>,
    pub min_memory_mb: Option<u64>,
    #[serde(default)]
    pub required_commands: Vec<String>,
    #[serde(default)]
    pub required_env_vars: Vec<String>,
}

impl LetSpec {
    pub fn minimal(target: impl Into<String>) -> Self {
        Self {
            target: target.into(),
            version: None,
            manager: None,
            dependencies: Vec::new(),
            actions: Vec::new(),
            environment: HashMap::new(),
            constraints: LetConstraints::default(),
        }
    }
}
