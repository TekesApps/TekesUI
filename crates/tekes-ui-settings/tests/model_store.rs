use std::path::PathBuf;
use tekes_ui_settings::{Draft, FileProvider, Schema, Snapshot, Value};
fn schema() -> Schema {
    Schema::parse(include_str!("fixtures/layout.schema.toml")).unwrap()
}
fn temp() -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    std::env::temp_dir()
        .join(format!(
            "tekes-settings-test-{}-{}-{}",
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
        .join("settings.toml")
}

#[test]
fn schema_reports_unsupported_and_duplicate_or_overlapping_definitions() {
    let source = include_str!("fixtures/layout.schema.toml");
    assert!(Schema::parse(&source.replace("schema_version = 1", "schema_version = 99")).is_err());
    assert!(
        Schema::parse(&source.replace("id = \"autosave\"", "id = \"workspace-name\"")).is_err()
    );
    assert!(Schema::parse(&source.replace("type = \"integer\"", "type = \"action\"")).is_err());
    assert!(
        Schema::parse(&source.replace("[\"workspace\", \"autosave\"]", "[\"workspace\"]")).is_err()
    );
}

#[test]
fn range_validation_preserves_invalid_draft_and_cancel_is_local() {
    let s = schema();
    let mut draft = Draft::new(Snapshot::default());
    draft.edit(&s.settings[3], "999".into());
    assert!(draft.submit(&s, "bad".into()).is_err());
    assert_eq!(draft.display(&s.settings[3]), "999");
    draft.cancel();
    assert!(!draft.dirty());
    assert_eq!(draft.display(&s.settings[3]), "14");
    assert_eq!(draft.base.revision, 0);
}

#[test]
fn nested_typed_round_trip_reset_unsets_and_preserves_unknown_values() {
    let path = temp();
    let s = schema();
    let provider = FileProvider::open(path.clone(), s.namespace.clone()).unwrap();
    let mut base = Snapshot::default();
    base.overrides
        .insert("future".into(), Value::String("preserve".into()));
    let mut draft = Draft::new(base);
    for (f, value) in s
        .settings
        .iter()
        .zip(["研究工作区", "false", "en", "18", "1.75"])
    {
        draft.edit(f, value.into());
    }
    let request = draft.submit(&s, "one".into()).unwrap();
    let saved = provider.commit(&request).unwrap();
    draft.accept(&s, &request, saved.clone()).unwrap();
    assert!(!draft.dirty());
    assert_eq!(
        saved.overrides["reading"]["font_size"].as_integer(),
        Some(18)
    );
    assert_eq!(
        saved.overrides["reading"]["advanced"]["line_spacing"].as_float(),
        Some(1.75)
    );
    assert_eq!(
        saved.overrides["workspace"]["autosave"].as_bool(),
        Some(false)
    );
    draft.reset(&s);
    assert!(draft.dirty());
    assert_eq!(provider.load().unwrap(), saved);
    let reset = draft.submit(&s, "reset".into()).unwrap();
    let receipt = provider.commit(&reset).unwrap();
    assert_eq!(receipt.overrides.len(), 1);
    assert_eq!(receipt.overrides["future"].as_str(), Some("preserve"));
    draft.accept(&s, &reset, receipt).unwrap();
    assert!(!draft.dirty());
    drop(provider);
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[test]
fn lost_receipt_duplicate_precedes_revision_check_and_survives_provider_restart() {
    let path = temp();
    let s = schema();
    let mut draft = Draft::new(Snapshot::default());
    draft.edit(&s.settings[0], "Saved".into());
    let request = draft.submit(&s, "lost-receipt".into()).unwrap();
    let provider = FileProvider::open(path.clone(), s.namespace.clone()).unwrap();
    let first = provider.commit(&request).unwrap();
    drop(provider);
    let provider = FileProvider::open(path.clone(), s.namespace.clone()).unwrap();
    assert_eq!(provider.query(&request).unwrap(), Some(first.clone()));
    assert_eq!(provider.commit(&request).unwrap(), first);
    draft.accept(&s, &request, first).unwrap();
    assert!(!draft.dirty());
    drop(provider);
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[test]
fn successful_batch_cannot_erase_newer_typing_even_when_reverted_to_original() {
    let s = schema();
    for newer in ["Newer", "My Workspace"] {
        let mut draft = Draft::new(Snapshot::default());
        draft.edit(&s.settings[0], "Sent".into());
        let request = draft.submit(&s, "in-flight".into()).unwrap();
        draft.edit(&s.settings[0], newer.into());
        let snapshot = Snapshot {
            revision: 1,
            last_commit_id: request.commit_id.clone(),
            overrides: request.overrides.clone(),
        };
        draft.accept(&s, &request, snapshot).unwrap();
        assert!(draft.dirty());
        assert_eq!(draft.display(&s.settings[0]), newer);
    }
}

#[test]
fn provider_rejects_second_writer_stale_commit_and_corruption_without_overwrite() {
    let path = temp();
    let s = schema();
    let provider = FileProvider::open(path.clone(), s.namespace.clone()).unwrap();
    assert!(FileProvider::open(path.clone(), s.namespace.clone()).is_err());
    let mut draft = Draft::new(Snapshot::default());
    draft.edit(&s.settings[0], "First".into());
    let one = draft.submit(&s, "one".into()).unwrap();
    provider.commit(&one).unwrap();
    draft.edit(&s.settings[0], "Second".into());
    let stale = draft.submit(&s, "stale".into()).unwrap();
    let before = std::fs::read(&path).unwrap();
    assert!(provider.commit(&stale).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    std::fs::write(&path, "malformed = [").unwrap();
    assert!(provider.load().is_err());
    assert!(provider.commit(&one).is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "malformed = [");
    drop(provider);
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[test]
fn group_item_field_identity_and_item_filter_do_not_depend_on_labels() {
    let source = include_str!("fixtures/layout.schema.toml");
    let s = schema();
    assert_eq!(s.items().count(), 3);
    let item = s.for_item("reading/text").unwrap();
    assert_eq!(item.settings.len(), 2);
    assert!(item.settings.iter().all(|f| f.item == "reading/text"));
    let renamed = Schema::parse(&source.replace("基本设置", "General settings")).unwrap();
    assert_eq!(s.settings[0].id, renamed.settings[0].id);
    assert_eq!(s.settings[0].path, renamed.settings[0].path);
    let minimal = source
        .replace("[[pages]]\nid = \"general\"\nlabel = \"通用\"", "")
        .replace("page = \"general\"", "");
    assert_eq!(Schema::parse(&minimal).unwrap().items().count(), 3);
}

#[test]
fn discard_before_late_receipt_never_resurrects_cancelled_editor() {
    let s = schema();
    let mut draft = Draft::new(Snapshot::default());
    draft.edit(&s.settings[0], "Sent".into());
    let request = draft.submit(&s, "late".into()).unwrap();
    draft.cancel();
    let snapshot = Snapshot {
        revision: 1,
        last_commit_id: request.commit_id.clone(),
        overrides: request.overrides.clone(),
    };
    draft.accept(&s, &request, snapshot).unwrap();
    assert!(!draft.dirty());
    assert_eq!(draft.display(&s.settings[0]), "Sent");
}

#[test]
fn known_write_failure_is_not_unknown_and_does_not_replace_original() {
    let path = temp();
    let s = schema();
    let provider = FileProvider::open(path.clone(), s.namespace.clone()).unwrap();
    let mut draft = Draft::new(Snapshot::default());
    draft.edit(&s.settings[0], "Test".into());
    let request = draft.submit(&s, "failure".into()).unwrap();
    std::fs::create_dir(path.with_extension("pending")).unwrap();
    let error = provider.commit_checked(&request).unwrap_err();
    assert!(!error.unknown);
    assert!(!path.exists());
    assert!(draft.dirty());
    drop(provider);
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[test]
fn apphost_general_defaults_only_include_appearance_and_text_zoom() {
    let s = Schema::parse(include_str!(
        "../../../examples/app-host-demo/settings.schema.toml"
    ))
    .unwrap();
    let draft = Draft::new(Snapshot::default());
    let field = |id: &str| s.settings.iter().find(|f| f.id.ends_with(id)).unwrap();
    assert_eq!(draft.display(field("/appearance")), "system");
    assert_eq!(draft.display(field("/text-size")), "100");
    assert_eq!(field("/text-size").min, Some(50.));
    assert_eq!(field("/text-size").max, Some(300.));
    assert_eq!(s.settings.len(), 2);
    assert!(
        s.settings
            .iter()
            .all(|f| f.id.ends_with("/appearance") || f.id.ends_with("/text-size"))
    );
    assert_eq!(
        s.items().map(|(id, _)| id).collect::<Vec<_>>(),
        vec!["general/appearance"]
    );
}
