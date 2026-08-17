//! Outcome of an RCM LET imperative — what callers bind after `rcm let <target>`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Final status of a single LET execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LetStatus {
    /// All gated actions finished successfully.
    Success,
    /// Constraints or a condition failed before/during actions (fail closed).
    Blocked,
    /// An action started but failed; see `actions` for which step.
    Failed,
    /// `--dry-run`: plan only, no side effects.
    DryRun,
}

/// One action after execution (or dry-run planning).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LetActionOutcome {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    /// Wall-clock duration in milliseconds, if executed.
    pub duration_ms: Option<u64>,
    pub exit_code: Option<i32>,
    pub stdout_tail: Option<String>,
    pub stderr_tail: Option<String>,
    pub skipped: bool,
    pub skip_reason: Option<String>,
}

/// Artifact produced or discovered by the run (paths, ids, urls).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LetArtifact {
    /// Stable key: "wasm", "contract_id", "bin", "model_path", …
    pub kind: String,
    pub path: Option<PathBuf>,
    pub value: Option<String>,
}

/// Bindable result of `LetExecutor::run` — language `let outcome = …`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LetOutcome {
    /// Spec target name, e.g. "cargo", "pmll-anchor".
    pub target: String,
    pub status: LetStatus,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    /// Manager resolved for this run, if any.
    pub manager: Option<String>,
    /// Version requested/resolved, if any.
    pub version: Option<String>,
    /// Workspace root used for the run.
    pub workspace: PathBuf,
    /// Spec file path, when loaded from disk.
    pub spec_path: Option<PathBuf>,
    /// Constraints that were evaluated (echo for agents/logs).
    pub constraints_ok: bool,
    pub constraint_notes: Vec<String>,
    /// Per-action results in order.
    pub actions: Vec<LetActionOutcome>,
    /// Named outputs to bind in scripts/agents.
    pub artifacts: Vec<LetArtifact>,
    /// Merged env that applied to actions (optional; redact secrets before serialize).
    pub env_keys: Vec<String>,
    /// Human-readable summary line.
    pub message: String,
    /// Extra machine-readable metadata.
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
