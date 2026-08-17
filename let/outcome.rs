//! Outcome of an RCM LET imperative — what callers bind after `rcm let <target>`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LetStatus {
    Success,
    Blocked,
    Failed,
    DryRun,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LetActionOutcome {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub duration_ms: Option<u64>,
    pub exit_code: Option<i32>,
    pub stdout_tail: Option<String>,
    pub stderr_tail: Option<String>,
    pub skipped: bool,
    pub skip_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LetArtifact {
    pub kind: String,
    pub path: Option<PathBuf>,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LetOutcome {
    pub target: String,
    pub status: LetStatus,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub manager: Option<String>,
    pub version: Option<String>,
    pub workspace: PathBuf,
    pub spec_path: Option<PathBuf>,
    pub constraints_ok: bool,
    pub constraint_notes: Vec<String>,
    pub actions: Vec<LetActionOutcome>,
    pub artifacts: Vec<LetArtifact>,
    pub env_keys: Vec<String>,
    pub message: String,
    pub meta: HashMap<String, String>,
}

impl LetOutcome {
    pub fn ok(&self) -> bool {
        matches!(self.status, LetStatus::Success | LetStatus::DryRun)
    }

    pub fn artifact(&self, kind: &str) -> Option<&LetArtifact> {
        self.artifacts.iter().find(|a| a.kind == kind)
    }

    pub fn artifact_value(&self, kind: &str) -> Option<&str> {
        self.artifact(kind).and_then(|a| a.value.as_deref())
    }

    pub fn artifact_path(&self, kind: &str) -> Option<&PathBuf> {
        self.artifact(kind).and_then(|a| a.path.as_ref())
    }
}
