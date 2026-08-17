//! CLI wiring for `rcm let <target>`.

use super::executor::LetExecutor;
use super::outcome::{LetOutcome, LetStatus};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "let", about = "RCM prime imperative — bind a target, run constrained actions")]
pub struct LetCli {
    #[command(subcommand)]
    pub command: Option<LetCmd>,
    pub target: Option<String>,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub json: bool,
    #[arg(long)]
    pub workspace: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub enum LetCmd {
    Init,
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
                status: LetStatus::Success,
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
        Some(LetCmd::Run { target, dry_run, json: _ }) => {
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
