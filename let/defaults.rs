//! Built-in default LET specs for prime adoption targets.

use super::spec::{LetAction, LetConstraints, LetSpec};
use std::collections::HashMap;

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
