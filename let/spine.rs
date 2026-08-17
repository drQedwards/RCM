------------------------------
----------- cli.rs -----------
------------------------------
//! CLI wiring for `rcm let <target>`.

use super::executor::LetExecutor;
use super::outcome::LetOutcome;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "let", about = "RCM prime imperative — bind a target, run constrained actions")]
pub struct LetCli {
    #[command(subcommand)]
    pub command: Option<LetCmd>,

    /// Target name when used as `rcm let <target>`
    pub target: Option<String>,

    /// Plan only; do not execute actions
    #[arg(long)]
    pub dry_run: bool,

    /// Emit LetOutcome as JSON
    #[arg(long)]
    pub json: bool,

    /// Workspace root (default: cwd)
    #[arg(long)]
    pub workspace: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub enum LetCmd {
    /// Initialize `.rcm/let` directory
    Init,
    /// Run a target (same as positional target)
    Run {
        target: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        json: bool,
    },
}

pub fn execute(cli: LetCli) -> anyhow::Result<LetOutcome> {
    let workspace = cli
        .workspace
        .unwrap_or_else(|| std::env::current_dir().expect("cwd"));
    let exec = LetExecutor::new(workspace);

    match cli.command {
        Some(LetCmd::Init) => {
            exec.initialize()?;
            let now = chrono::Utc::now();
            return Ok(LetOutcome {
                target: "init".into(),
                status: super::outcome::LetStatus::Success,
                started_at: now,
                finished_at: now,
                manager: None,
                version: None,
                workspace: exec.workspace.clone(),
                spec_path: None,
                constraints_ok: true,
                constraint_notes: vec!["created .rcm/let".into()],
                actions: vec![],
                artifacts: vec![],
                env_keys: vec![],
                message: "initialized .rcm/let".into(),
                meta: Default::default(),
            });
        }
        Some(LetCmd::Run {
            target,
            dry_run,
            json: _,
        }) => {
            return exec.run_blocking(&target, dry_run || cli.dry_run);
        }
        None => {}
    }

    let target = cli
        .target
        .ok_or_else(|| anyhow::anyhow!("usage: rcm let <target> | rcm let init | rcm let run <target>"))?;
    exec.run_blocking(&target, cli.dry_run)
}

pub fn print_outcome(outcome: &LetOutcome, json: bool) -> anyhow::Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(outcome)?);
    } else {
        println!("{}", outcome.message);
        for a in &outcome.actions {
            let mark = if a.skipped {
                "skip"
            } else if a.exit_code == Some(0) {
                "ok"
            } else {
                "fail"
            };
            println!("  [{mark}] {} — {} {:?}", a.name, a.command, a.args);
        }
        for art in &outcome.artifacts {
            match (&art.path, &art.value) {
                (Some(p), _) => println!("  artifact {}: {}", art.kind, p.display()),
                (_, Some(v)) => println!("  artifact {}: {}", art.kind, v),
                _ => println!("  artifact {}", art.kind),
            }
        }
    }
    Ok(())
}

------------------------------
-------- defaults.rs ---------
------------------------------
//! Built-in default LET specs for prime adoption targets.

use super::spec::{LetAction, LetConstraints, LetSpec};
use std::collections::HashMap;

/// Default spec for `cargo` — build release binary.
pub fn cargo_spec() -> LetSpec {
    LetSpec {
        target: "cargo".into(),
        version: None,
        manager: Some("cargo".into()),
        dependencies: vec![],
        actions: vec![LetAction {
            name: "build_release".into(),
            command: "cargo".into(),
            args: vec!["build".into(), "--release".into()],
            working_dir: None,
            env: HashMap::new(),
            conditions: vec![],
            parallel: false,
        }],
        environment: HashMap::new(),
        constraints: LetConstraints {
            platforms: vec![],
            min_memory_mb: None,
            required_commands: vec!["cargo".into(), "rustc".into()],
            required_env_vars: vec![],
        },
    }
}

/// Default spec for `pmll-anchor` — stellar contract build (host must have stellar CLI).
pub fn pmll_anchor_spec() -> LetSpec {
    LetSpec {
        target: "pmll-anchor".into(),
        version: Some("0.1.0".into()),
        manager: Some("stellar".into()),
        dependencies: vec![],
        actions: vec![LetAction {
            name: "contract_build".into(),
            command: "stellar".into(),
            args: vec!["contract".into(), "build".into()],
            working_dir: Some("pmll-anchor".into()),
            env: HashMap::new(),
            conditions: vec![],
            parallel: false,
        }],
        environment: HashMap::new(),
        constraints: LetConstraints {
            platforms: vec![],
            min_memory_mb: None,
            required_commands: vec!["stellar".into()],
            required_env_vars: vec![],
        },
    }
}

pub fn builtin(target: &str) -> Option<LetSpec> {
    match target {
        "cargo" => Some(cargo_spec()),
        "pmll-anchor" => Some(pmll_anchor_spec()),
        _ => None,
    }
}

------------------------------
---------- error.rs ----------
------------------------------
//! Errors for the RCM LET imperative spine.

use std::fmt;
use std::path::PathBuf;

#[derive(Debug)]
pub enum LetError {
    SpecNotFound { target: String, searched: String },
    InvalidSpec { path: PathBuf, reason: String },
    Blocked { target: String, reason: String },
    ActionFailed {
        action: String,
        code: Option<i32>,
        detail: String,
    },
    Io(std::io::Error),
    Msg(String),
}

impl fmt::Display for LetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LetError::SpecNotFound { target, searched } => {
                write!(f, "spec not found for target `{target}` (looked in {searched})")
            }
            LetError::InvalidSpec { path, reason } => {
                write!(f, "invalid spec `{}`: {reason}", path.display())
            }
            LetError::Blocked { target, reason } => {
                write!(f, "constraint blocked target `{target}`: {reason}")
            }
            LetError::ActionFailed {
                action,
                code,
                detail,
            } => write!(f, "action `{action}` failed (exit {code:?}): {detail}"),
            LetError::Io(e) => write!(f, "io error: {e}"),
            LetError::Msg(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for LetError {}

impl From<std::io::Error> for LetError {
    fn from(e: std::io::Error) -> Self {
        LetError::Io(e)
    }
}

impl From<anyhow::Error> for LetError {
    fn from(e: anyhow::Error) -> Self {
        LetError::Msg(e.to_string())
    }
}

pub type LetResult<T> = Result<T, LetError>;

------------------------------
-------- executor.rs ---------
------------------------------
//! LET executor — constraints → actions → LetOutcome.

use super::defaults;
use super::error::{LetError, LetResult};
use super::loader::{command_exists, current_platform, SpecLoader};
use super::outcome::{LetActionOutcome, LetArtifact, LetOutcome, LetStatus};
use super::spec::{LetAction, LetConditionType, LetConstraints, LetSpec};
use chrono::Utc;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

pub struct LetExecutor {
    pub workspace: PathBuf,
    loader: SpecLoader,
}

impl LetExecutor {
    pub fn new(workspace: impl Into<PathBuf>) -> Self {
        let workspace = workspace.into();
        let loader = SpecLoader::new(workspace.clone());
        Self { workspace, loader }
    }

    pub fn initialize(&self) -> LetResult<()> {
        self.loader.ensure_dir()
    }

    /// Resolve spec: disk first, then built-in defaults.
    pub fn resolve_spec(&self, target: &str) -> LetResult<(LetSpec, Option<PathBuf>)> {
        match self.loader.load(target) {
            Ok((spec, path)) => Ok((spec, Some(path))),
            Err(LetError::SpecNotFound { .. }) => {
                if let Some(spec) = defaults::builtin(target) {
                    Ok((spec, None))
                } else {
                    Err(LetError::SpecNotFound {
                        target: target.to_string(),
                        searched: format!(
                            "{}, builtins",
                            self.loader
                                .candidates(target)
                                .iter()
                                .map(|p| p.display().to_string())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    })
                }
            }
            Err(e) => Err(e),
        }
    }

    pub fn check_constraints(&self, c: &LetConstraints) -> (bool, Vec<String>) {
        let mut notes = Vec::new();
        let mut ok = true;
        let plat = current_platform();

        if !c.platforms.is_empty() && !c.platforms.iter().any(|p| p == &plat || p == "any") {
            ok = false;
            notes.push(format!(
                "platform `{plat}` not in allowed {:?}",
                c.platforms
            ));
        }

        for cmd in &c.required_commands {
            if !command_exists(cmd) {
                ok = false;
                notes.push(format!("required command not found: `{cmd}`"));
            } else {
                notes.push(format!("command ok: `{cmd}`"));
            }
        }

        for key in &c.required_env_vars {
            if std::env::var_os(key).is_none() {
                ok = false;
                notes.push(format!("required env var missing: `{key}`"));
            }
        }

        (ok, notes)
    }

    fn conditions_hold(&self, action: &LetAction) -> (bool, Option<String>) {
        for cond in &action.conditions {
            let pass = match cond.condition_type {
                LetConditionType::FileExists => {
                    Path::new(&cond.value).is_file()
                        || self.workspace.join(&cond.value).is_file()
                }
                LetConditionType::CommandExists => command_exists(&cond.value),
                LetConditionType::EnvVar => std::env::var_os(&cond.value).is_some(),
                LetConditionType::Platform => {
                    cond.value == current_platform() || cond.value == "any"
                }
                LetConditionType::PackageInstalled => command_exists(&cond.value),
            };
            if !pass {
                return (
                    false,
                    Some(format!("{:?}({}) failed", cond.condition_type, cond.value)),
                );
            }
        }
        (true, None)
    }

    fn run_action(&self, action: &LetAction, dry_run: bool) -> LetActionOutcome {
        let (ok, reason) = self.conditions_hold(action);
        if !ok {
            return LetActionOutcome {
                name: action.name.clone(),
                command: action.command.clone(),
                args: action.args.clone(),
                duration_ms: None,
                exit_code: None,
                stdout_tail: None,
                stderr_tail: None,
                skipped: true,
                skip_reason: reason,
            };
        }

        if dry_run {
            return LetActionOutcome {
                name: action.name.clone(),
                command: action.command.clone(),
                args: action.args.clone(),
                duration_ms: None,
                exit_code: None,
                stdout_tail: Some("(dry-run)".into()),
                stderr_tail: None,
                skipped: true,
                skip_reason: Some("dry_run".into()),
            };
        }

        let cwd = action
            .working_dir
            .as_ref()
            .map(|d| self.workspace.join(d))
            .unwrap_or_else(|| self.workspace.clone());

        let started = Instant::now();
        let mut cmd = Command::new(&action.command);
        cmd.args(&action.args).current_dir(&cwd);
        for (k, v) in &action.env {
            cmd.env(k, v);
        }

        match cmd.output() {
            Ok(out) => {
                let tail = |b: &[u8]| {
                    let s = String::from_utf8_lossy(b);
                    let t = s.trim();
                    if t.len() > 400 {
                        Some(t[t.len() - 400..].to_string())
                    } else if t.is_empty() {
                        None
                    } else {
                        Some(t.to_string())
                    }
                };
                LetActionOutcome {
                    name: action.name.clone(),
                    command: action.command.clone(),
                    args: action.args.clone(),
                    duration_ms: Some(started.elapsed().as_millis() as u64),
                    exit_code: out.status.code(),
                    stdout_tail: tail(&out.stdout),
                    stderr_tail: tail(&out.stderr),
                    skipped: false,
                    skip_reason: None,
                }
            }
            Err(e) => LetActionOutcome {
                name: action.name.clone(),
                command: action.command.clone(),
                args: action.args.clone(),
                duration_ms: Some(started.elapsed().as_millis() as u64),
                exit_code: None,
                stdout_tail: None,
                stderr_tail: Some(e.to_string()),
                skipped: false,
                skip_reason: None,
            },
        }
    }

    pub async fn run(&self, target: &str, dry_run: bool) -> LetResult<LetOutcome> {
        // async surface for RCM; body is sync-friendly
        self.run_blocking(target, dry_run)
    }

    pub fn run_blocking(&self, target: &str, dry_run: bool) -> LetResult<LetOutcome> {
        let started_at = Utc::now();
        let (spec, spec_path) = self.resolve_spec(target)?;
        let (constraints_ok, constraint_notes) = self.check_constraints(&spec.constraints);

        if !constraints_ok && !dry_run {
            let finished_at = Utc::now();
            return Ok(LetOutcome {
                target: spec.target.clone(),
                status: LetStatus::Blocked,
                started_at,
                finished_at,
                manager: spec.manager.clone(),
                version: spec.version.clone(),
                workspace: self.workspace.clone(),
                spec_path,
                constraints_ok: false,
                constraint_notes,
                actions: vec![],
                artifacts: vec![],
                env_keys: spec.environment.keys().cloned().collect(),
                message: format!("blocked: constraints failed for `{}`", spec.target),
                meta: Default::default(),
            });
        }

        let mut actions = Vec::with_capacity(spec.actions.len());
        for action in &spec.actions {
            actions.push(self.run_action(action, dry_run));
        }

        let failed = actions.iter().any(|a| {
            !a.skipped && a.exit_code != Some(0) && a.exit_code.is_some()
                || (!a.skipped && a.exit_code.is_none() && a.stderr_tail.is_some())
        });

        let status = if dry_run {
            LetStatus::DryRun
        } else if failed {
            LetStatus::Failed
        } else {
            LetStatus::Success
        };

        let mut artifacts = Vec::new();
        // Heuristic: cargo release binary
        let release_dir = self.workspace.join("target/release");
        if release_dir.is_dir() {
            artifacts.push(LetArtifact {
                kind: "release_dir".into(),
                path: Some(release_dir),
                value: None,
            });
        }
        let wasm = self
            .workspace
            .join("target/wasm32v1-none/release");
        if wasm.is_dir() {
            artifacts.push(LetArtifact {
                kind: "wasm_dir".into(),
                path: Some(wasm),
                value: None,
            });
        }

        let finished_at = Utc::now();
        let message = match status {
            LetStatus::Success => format!("let `{}` succeeded ({} actions)", spec.target, actions.len()),
            LetStatus::DryRun => format!("let `{}` dry-run ({} actions)", spec.target, actions.len()),
            LetStatus::Failed => format!("let `{}` failed", spec.target),
            LetStatus::Blocked => format!("let `{}` blocked", spec.target),
        };

        Ok(LetOutcome {
            target: spec.target,
            status,
            started_at,
            finished_at,
            manager: spec.manager,
            version: spec.version,
            workspace: self.workspace.clone(),
            spec_path,
            constraints_ok,
            constraint_notes,
            actions,
            artifacts,
            env_keys: spec.environment.keys().cloned().collect(),
            message,
            meta: Default::default(),
        })
    }
}

------------------------------
------- LetOutcome.rs --------
------------------------------
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

------------------------------
--------- loader.rs ----------
------------------------------
//! Load LET specs from `.rcm/let/<target>.{toml,json}`.

use super::error::{LetError, LetResult};
use super::spec::LetSpec;
use std::path::{Path, PathBuf};

pub struct SpecLoader {
    pub workspace: PathBuf,
    pub specs_dir: PathBuf,
}

impl SpecLoader {
    pub fn new(workspace: impl Into<PathBuf>) -> Self {
        let workspace = workspace.into();
        let specs_dir = workspace.join(".rcm").join("let");
        Self {
            workspace,
            specs_dir,
        }
    }

    pub fn candidates(&self, target: &str) -> Vec<PathBuf> {
        vec![
            self.specs_dir.join(format!("{target}.toml")),
            self.specs_dir.join(format!("{target}.json")),
            self.specs_dir.join(target).join("spec.toml"),
            self.specs_dir.join(target).join("spec.json"),
        ]
    }

    pub fn load(&self, target: &str) -> LetResult<(LetSpec, PathBuf)> {
        let paths = self.candidates(target);
        for path in &paths {
            if path.is_file() {
                let raw = std::fs::read_to_string(path)?;
                let spec = if path.extension().and_then(|e| e.to_str()) == Some("json") {
                    serde_json::from_str::<LetSpec>(&raw).map_err(|e| LetError::InvalidSpec {
                        path: path.clone(),
                        reason: e.to_string(),
                    })?
                } else {
                    toml::from_str::<LetSpec>(&raw).map_err(|e| LetError::InvalidSpec {
                        path: path.clone(),
                        reason: e.to_string(),
                    })?
                };
                return Ok((spec, path.clone()));
            }
        }
        Err(LetError::SpecNotFound {
            target: target.to_string(),
            searched: paths
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", "),
        })
    }

    pub fn ensure_dir(&self) -> LetResult<()> {
        std::fs::create_dir_all(&self.specs_dir)?;
        Ok(())
    }
}

pub fn command_exists(cmd: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths).any(|dir| {
                let p = dir.join(cmd);
                if p.is_file() {
                    return true;
                }
                #[cfg(windows)]
                {
                    return dir.join(format!("{cmd}.exe")).is_file();
                }
                #[cfg(not(windows))]
                {
                    false
                }
            })
        })
        .unwrap_or(false)
}

pub fn current_platform() -> String {
    std::env::consts::OS.to_string()
}

------------------------------
----------- mod.rs -----------
------------------------------
//! RCM LET — prime bonded imperative.
//!
//! ```text
//! rcm let <target>  →  LetSpec  →  constraints  →  actions  →  LetOutcome
//! ```
//!
//! In `lib.rs` expose as:
//! ```ignore
//! #[path = "let/mod.rs"]
//! pub mod lets;
//! // or: pub mod r#let;
//! ```

pub mod cli;
pub mod defaults;
pub mod error;
pub mod executor;
pub mod loader;
pub mod outcome;
pub mod spec;

pub use error::{LetError, LetResult};
pub use executor::LetExecutor;
pub use outcome::{LetActionOutcome, LetArtifact, LetOutcome, LetStatus};
pub use spec::{LetAction, LetCondition, LetConditionType, LetConstraints, LetSpec};

------------------------------
--------- outcome.rs ---------
------------------------------
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

------------------------------
--------- README.md ----------
------------------------------
# RCM LET — prime bonded imperative

```text
rcm let <target>  →  LetSpec  →  constraints  →  actions  →  LetOutcome
```

| File | Role |
|------|------|
| `mod.rs` | Re-exports |
| `spec.rs` | LetSpec / actions / constraints |
| `executor.rs` | Resolve → check → run → outcome |
| `outcome.rs` | LetOutcome (canonical; same as LetOutcome.rs) |
| `LetOutcome.rs` | Keep if already on main; prefer single `outcome.rs` long-term |
| `error.rs` | LetError |
| `loader.rs` | `.rcm/let/<target>.toml` |
| `defaults.rs` | Built-ins: `cargo`, `pmll-anchor` |
| `cli.rs` | `rcm let` clap surface |

Wire in crate root:

```rust
#[path = "let/mod.rs"]
pub mod lets;
```

Deps: `serde`, `serde_json`, `toml`, `chrono`, `anyhow`, `clap`.

------------------------------
---------- spec.rs -----------
------------------------------
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