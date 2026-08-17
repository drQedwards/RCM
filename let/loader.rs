//! Load LET specs from `.rcm/let/<target>.{toml,json}`.

use super::error::{LetError, LetResult};
use super::spec::LetSpec;
use std::path::PathBuf;

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
