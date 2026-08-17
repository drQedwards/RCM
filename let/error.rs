//! Errors for the RCM LET imperative spine.

use std::fmt;
use std::path::PathBuf;

#[derive(Debug)]
pub enum LetError {
    SpecNotFound {
        target: String,
        searched: String,
    },
    InvalidSpec {
        path: PathBuf,
        reason: String,
    },
    Blocked {
        target: String,
        reason: String,
    },
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
                write!(
                    f,
                    "spec not found for target `{target}` (looked in {searched})"
                )
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
