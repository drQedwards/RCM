//! Commands module for RCM
//! 
//! Contains implementations for all RCM commands

pub mod init;
pub mod add;
pub mod remove;
pub mod ensure;
pub mod plan;
pub mod apply;
pub mod snapshot;
pub mod sbom;
pub mod provenance;
pub mod workspace;
pub mod config;
pub mod letcmd;

use anyhow::Result;
use crate::workspace::Workspace;

/// Common trait for all commands
pub trait Command {
    async fn execute(&self, workspace: &Workspace) -> Result<()>;
}

/// Command execution context
pub struct CommandContext {
    pub workspace: Workspace,
    pub dry_run: bool,
    pub verbose: bool,
    pub force: bool,
}

impl CommandContext {
    pub fn new(workspace: Workspace) -> Self {
        Self {
            workspace,
            dry_run: false,
            verbose: false,
            force: false,
        }
    }
    
    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }
    
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }
    
    pub fn with_force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }
}
