//! Consumer-owned migration; old configuration and its undo/nav files stay intact.
use anyhow::{Result, ensure};
use std::path::Path;
use tekes_ui_settings::{FileProvider, Schema, Snapshot, Submission, Value};

pub fn migrate(destination: &Path) -> Result<()> {
    if destination.exists() {
        return Ok(());
    }
    let old = destination
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("general/settings.toml");
    if !old.exists() {
        return Ok(());
    }
    let source = FileProvider::open(old, "apphost-demo.general".into())?.load()?;
    let mut overrides = source.overrides;
    if let Some(general) = overrides.get_mut("general").and_then(Value::as_table_mut)
        && let Some(step) = general.remove("text_size_step")
    {
        let step = step
            .as_integer()
            .ok_or_else(|| anyhow::anyhow!("invalid legacy text zoom step"))?;
        ensure!(
            (-5..=8).contains(&step),
            "legacy text zoom step out of range"
        );
        let ramp = [
            50, 60, 75, 85, 92, 100, 110, 125, 140, 160, 180, 200, 250, 300,
        ];
        general.insert(
            "text_size_percent".into(),
            Value::Integer(ramp[(step + 5) as usize]),
        );
    }
    let schema = Schema::parse(include_str!("../settings.schema.toml"))?;
    let next = Snapshot {
        revision: 1,
        last_commit_id: "migrate-general-percent-v1".into(),
        overrides,
    };
    schema.validate_snapshot(&next)?;
    let provider = FileProvider::open(destination.into(), schema.namespace)?;
    provider.commit(&Submission::replacement(&Snapshot::default(), &next)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_zoom_is_migrated_once_without_touching_old_configuration_or_history() {
        let root = std::env::temp_dir().join(format!(
            "settings-percent-migration-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let old = root.join("general/settings.toml");
        let provider = FileProvider::open(old.clone(), "apphost-demo.general".into()).unwrap();
        let mut table = Snapshot::default().overrides;
        table.insert(
            "general".into(),
            Value::Table(
                [
                    ("text_size_step".into(), Value::Integer(1)),
                    ("appearance".into(), Value::String("dark".into())),
                ]
                .into_iter()
                .collect(),
            ),
        );
        let after = Snapshot {
            revision: 1,
            last_commit_id: "old".into(),
            overrides: table,
        };
        provider
            .commit(&Submission::replacement(&Snapshot::default(), &after).unwrap())
            .unwrap();
        drop(provider);
        let original = std::fs::read(&old).unwrap();
        std::fs::write(root.join("general/undo.json"), "preserved history").unwrap();
        let dest = root.join("general-percent/settings.toml");
        migrate(&dest).unwrap();
        migrate(&dest).unwrap();
        let next = FileProvider::open(dest, "apphost-demo.general-percent".into())
            .unwrap()
            .load()
            .unwrap();
        assert_eq!(
            next.overrides["general"]["text_size_percent"].as_integer(),
            Some(110)
        );
        assert_eq!(
            next.overrides["general"]["appearance"].as_str(),
            Some("dark")
        );
        assert_eq!(std::fs::read(old).unwrap(), original);
        assert_eq!(
            std::fs::read_to_string(root.join("general/undo.json")).unwrap(),
            "preserved history"
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
