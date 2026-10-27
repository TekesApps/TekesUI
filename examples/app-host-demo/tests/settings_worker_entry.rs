#![cfg(target_os = "macos")]

#[test]
fn real_config_child_bypasses_consumer_settings_migration() {
    let isolated_home = std::env::temp_dir().join(format!("settings-worker-home-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    let settings = isolated_home.join("Library/Application Support/TekesUI AppHost Prototype/settings");
    let legacy = settings.join("general/settings.toml");
    let destination = settings.join("general-percent/settings.toml");
    std::fs::create_dir_all(legacy.parent().unwrap()).unwrap();
    // Calling the consumer migration would attempt to parse this file and fail
    // before the real child registry can acknowledge its handshake.
    let sentinel = b"invalid = [";
    std::fs::write(&legacy, sentinel).unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tekes-ui-app-host-demo"))
        .arg("--verify-settings-undo")
        .env("HOME", &isolated_home)
        .output().unwrap();
    assert!(output.status.success(),"real child failed: stdout={} stderr={} fixture={}",String::from_utf8_lossy(&output.stdout),String::from_utf8_lossy(&output.stderr),isolated_home.display());
    assert!(String::from_utf8_lossy(&output.stdout).contains("SETTINGS_UNDO_IPC_PASS"));
    assert_eq!(std::fs::read(&legacy).unwrap().as_slice(), sentinel);
    assert!(!legacy.with_extension("lock").exists(),"consumer migration touched legacy provider");
    assert!(!destination.exists(),"headless worker created consumer configuration");
    std::fs::remove_dir_all(isolated_home).unwrap();
}
