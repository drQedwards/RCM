//! RCM-cli.rs: Rust side of a C-callable CLI.
//!
//! Add to Cargo.toml:
//! [lib]
//! name = "rcm_cli"
//! crate-type = ["cdylib", "rlib"]
//!
//! Then `cargo build -p rcm --release` will produce a shared lib you can link from C.

mod lockfile;
mod sbom;
mod provenance;
mod registry;
mod util;
mod commands;

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "rcm", version, about = "RCM – Rust Cargo Manager")]
struct Cli {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize RCM in the current directory
    Init,
    /// Add a crate requirement (name[@version])
    Add { spec: String },
    /// Check environment, ensure lockfile exists, validate metadata
    Ensure,
    /// Show what would change (dry-run)
    Plan,
    /// Apply the plan (no-op in stub)
    Apply,
    /// Create a snapshot
    Snapshot { #[arg(long)] name: String },
    /// Generate SBOM JSON
    Sbom { #[arg(long)] out: String },
    /// Generate provenance JSON
    Provenance { #[arg(long)] out: String },

    /// Imperative macro (e.g., `rcm let cargo --deploy`)
    #[cfg(feature = "let")]
    Let {
        target: String,
        #[arg(long)] deploy: bool,
        #[arg(long)] plan: bool,
        #[arg(long)] apply: bool,
        #[arg(long = "arg", value_name = "k=v", num_args=0.., action=clap::ArgAction::Append)]
        args: Vec<String>,
    },
}

fn run_cli<I, S>(iter: I) -> Result<i32>
where
    I: IntoIterator<Item = S>,
    S: Into<std::ffi::OsString> + Clone,
{
    let cli = Cli::parse_from(iter);
    match cli.cmd {
        Commands::Init => {
            commands::init::run()?;
            Ok(0)
        }
        Commands::Add { spec } => {
            commands::add::run(&spec)?;
            Ok(0)
        }
        Commands::Ensure => {
            commands::ensure::run()?;
            Ok(0)
        }
        Commands::Plan => {
            commands::plan::run()?;
            Ok(0)
        }
        Commands::Apply => {
            commands::apply::run()?;
            Ok(0)
        }
        Commands::Snapshot { name } => {
            commands::snapshot::run(&name)?;
            Ok(0)
        }
        Commands::Sbom { out } => {
            commands::sbom::run(&out)?;
            Ok(0)
        }
        Commands::Provenance { out } => {
            commands::provenance::run(&out)?;
            Ok(0)
        }
        #[cfg(feature = "let")]
        Commands::Let { target, deploy, plan, apply, args } => {
            commands::letcmd::run(&target, deploy, plan, apply, args)?;
            Ok(0)
        }
    }
}

// --- FFI exports -------------------------------------------------------------

/// C ABI entry point: rcm_run(argc, argv)
/// Safety: argv must be an array of `argc` valid, null-terminated C strings.
#[no_mangle]
pub extern "C" fn rcm_run(argc: c_int, argv: *const *const c_char) -> c_int {
    // Safety: we trust the C caller on argc/argv validity.
    let args: Vec<String> = unsafe {
        if argv.is_null() || argc <= 0 {
            Vec::new()
        } else {
            std::slice::from_raw_parts(argv, argc as usize)
                .iter()
                .map(|&p| {
                    if p.is_null() {
                        String::new()
                    } else {
                        CStr::from_ptr(p).to_string_lossy().into_owned()
                    }
                })
                .collect()
        }
    };

    // If caller passed no args, behave like "rcm --help".
    let args = if args.is_empty() {
        vec!["rcm".to_string(), "--help".to_string()]
    } else {
        args
    };

    match run_cli(args) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("rcm error: {err:?}");
            1
        }
    }
}

/// Return a static version string ("6.5.0" or similar).
#[no_mangle]
pub extern "C" fn rcm_version() -> *const c_char {
    // Note: leak the CString to keep it alive for the process lifetime
    // (simplest option for a C-accessible static).
    let s = CString::new(env!("CARGO_PKG_VERSION")).unwrap();
    Box::into_raw(Box::new(s)).as_ptr()
}
