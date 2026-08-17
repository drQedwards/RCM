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
            notes.push(format!("platform `{plat}` not in allowed {:?}", c.platforms));
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
                    Path::new(&cond.value).is_file() || self.workspace.join(&cond.value).is_file()
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
            (!a.skipped && a.exit_code != Some(0) && a.exit_code.is_some())
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
        let release_dir = self.workspace.join("target/release");
        if release_dir.is_dir() {
            artifacts.push(LetArtifact {
                kind: "release_dir".into(),
                path: Some(release_dir),
                value: None,
            });
        }
        let wasm = self.workspace.join("target/wasm32v1-none/release");
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
