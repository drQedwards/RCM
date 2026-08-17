//! RCM LET — prime bonded imperative.
//!
//! ```text
//! rcm let <target>  →  LetSpec  →  constraints  →  actions  →  LetOutcome
//! ```
//!
//! In `lib.rs` expose as:
//! ```ignore
//! #[path = "../let/mod.rs"]
//! pub mod lets;
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
