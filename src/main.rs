//! RCM binary — `rcm` CLI with `let` as the prime subcommand.

use clap::{Parser, Subcommand};
use rcm::lets::cli::{self};
use rcm::lets::LetExecutor;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Debug, Parser)]
#[command(name = "rcm", version, about = "RCM — LET is the prime imperative")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Let {
        target: Option<String>,
        #[command(subcommand)]
        sub: Option<LetSub>,
        #[arg(long, global = true)]
        dry_run: bool,
        #[arg(long, global = true)]
        json: bool,
        #[arg(long, global = true)]
        workspace: Option<PathBuf>,
    },
    Version,
}

#[derive(Debug, Subcommand)]
enum LetSub {
    Init,
    Run { target: String },
}

fn main() -> ExitCode {
    if let Err(e) = real_main() {
        eprintln!("error: {e:#}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn real_main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Version => {
            println!("rcm {}", rcm::version());
            Ok(())
        }
        Commands::Let {
            target,
            sub,
            dry_run,
            json,
            workspace,
        } => {
            let workspace = workspace
                .unwrap_or_else(|| std::env::current_dir().expect("cwd"));
            let outcome = match sub {
                Some(LetSub::Init) => {
                    let exec = LetExecutor::new(&workspace);
                    exec.initialize()?;
                    let now = chrono::Utc::now();
                    rcm::LetOutcome {
                        target: "init".into(),
                        status: rcm::LetStatus::Success,
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
                    }
                }
                Some(LetSub::Run { target }) => {
                    LetExecutor::new(&workspace).run_blocking(&target, dry_run)?
                }
                None => {
                    let target = target.ok_or_else(|| {
                        anyhow::anyhow!(
                            "usage: rcm let <target> | rcm let init | rcm let run <target>"
                        )
                    })?;
                    LetExecutor::new(&workspace).run_blocking(&target, dry_run)?
                }
            };
            cli::print_outcome(&outcome, json)?;
            if outcome.ok() {
                Ok(())
            } else {
                Err(anyhow::anyhow!(outcome.message))
            }
        }
    }
}
