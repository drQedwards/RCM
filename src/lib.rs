//! RCM library — LET is the prime bonded imperative.

#[path = "../let/mod.rs"]
pub mod lets;

pub use lets::{
    LetAction, LetActionOutcome, LetArtifact, LetCondition, LetConditionType, LetConstraints,
    LetError, LetExecutor, LetOutcome, LetResult, LetSpec, LetStatus,
};

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
