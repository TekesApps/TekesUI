//! Fixed-catalogue entry point shared by AppHost and other container consumers.
use crate::ModuleDefinition;
use anyhow::{Result, ensure};

fn selection(modules: &[ModuleDefinition], args: &[String]) -> Result<Option<usize>> {
    if args.get(1).map(String::as_str) != Some("--worker") {
        return Ok(None);
    }
    ensure!(args.len() == 4, "invalid module worker arguments");
    ensure!(!args[3].is_empty(), "missing module transport endpoint");
    let mut matches = modules.iter().enumerate().filter(|(_, m)| m.id == args[2]);
    let Some((index, _)) = matches.next() else {
        anyhow::bail!("worker module not in fixed catalogue");
    };
    ensure!(matches.next().is_none(), "ambiguous worker module identity");
    Ok(Some(index))
}

/// Handle the existing `--worker MODULE ENDPOINT` entry before starting UI.
/// Returns false for an ordinary parent launch and true after the worker exits.
/// Factories are selected only from the consumer's fixed catalogue, and are
/// never called in the ordinary parent path. No plugin/executable discovery.
pub fn run_worker_from_args(modules: &[ModuleDefinition], args: &[String]) -> Result<bool> {
    let Some(index) = selection(modules, args)? else {
        return Ok(false);
    };
    #[cfg(target_os = "macos")]
    {
        crate::run((modules[index].create)(), &args[3])?;
        Ok(true)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = index;
        anyhow::bail!("native module GPU transport currently requires macOS")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn never_create() -> Box<dyn crate::PanelModule> {
        panic!("selection must not instantiate a module")
    }
    fn args(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| s.to_string()).collect()
    }
    fn catalogue() -> [ModuleDefinition; 2] {
        [
            ModuleDefinition {
                id: "navigation",
                create: never_create,
            },
            ModuleDefinition {
                id: "run-view",
                create: never_create,
            },
        ]
    }
    #[test]
    fn parent_path_never_instantiates_or_starts_a_worker() {
        assert!(!run_worker_from_args(&catalogue(), &args(&["review"])).unwrap());
        assert!(!run_worker_from_args(&catalogue(), &args(&["review", "--verify"])).unwrap());
    }
    #[test]
    fn fixed_identity_is_selected_without_executing_either_factory() {
        assert_eq!(
            selection(
                &catalogue(),
                &args(&["review", "--worker", "run-view", "owned-endpoint"])
            )
            .unwrap(),
            Some(1)
        );
    }
    #[test]
    fn malformed_unknown_or_ambiguous_worker_entry_is_rejected() {
        for a in [
            args(&["review", "--worker"]),
            args(&["review", "--worker", "unknown", "endpoint"]),
            args(&["review", "--worker", "run-view", ""]),
            args(&["review", "--worker", "run-view", "endpoint", "extra"]),
        ] {
            assert!(run_worker_from_args(&catalogue(), &a).is_err());
        }
        let duplicate = [catalogue()[0], catalogue()[0]];
        assert!(
            run_worker_from_args(
                &duplicate,
                &args(&["review", "--worker", "navigation", "endpoint"])
            )
            .is_err()
        );
    }
}
