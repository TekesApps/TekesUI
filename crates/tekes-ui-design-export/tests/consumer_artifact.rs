use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use tekes_ui_design_export::{verify_consumer_inputs, verify_inputs};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Artifact {
    root: PathBuf,
    package: PathBuf,
    document: Value,
}
impl Artifact {
    fn new() -> Self {
        let package = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let document: Value =
            serde_json::from_slice(&fs::read(package.join("design/tokens.json")).unwrap()).unwrap();
        let root = std::env::temp_dir().join(format!(
            "tekesui-consumer-artifact-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        for path in document["merge"]["inputs_sha256"]
            .as_object()
            .unwrap()
            .keys()
        {
            let target = root.join(path);
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::copy(package.join(path), target).unwrap();
        }
        Self {
            root,
            package,
            document,
        }
    }
}
impl Drop for Artifact {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}
#[test]
fn consumer_artifact_requires_exact_package_inputs_and_absolute_explicit_root() {
    let mut a = Artifact::new();
    let watched = verify_consumer_inputs(&a.package, &a.root, &a.document).unwrap();
    assert!(watched.iter().any(|p| p.starts_with(&a.root)));
    assert!(watched.iter().any(|p| p.starts_with(&a.package)));
    assert!(verify_consumer_inputs(&a.package, &PathBuf::from("relative"), &a.document).is_err());
    let path = "design/brand/policy.json";
    let bytes = b"modified incompatible policy";
    fs::write(a.root.join(path), bytes).unwrap();
    let digest = Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    a.document["merge"]["inputs_sha256"][path] = digest.into();
    verify_inputs(&a.root, &a.document).unwrap();
    assert!(
        verify_consumer_inputs(&a.package, &a.root, &a.document)
            .unwrap_err()
            .join(" ")
            .contains("incompatible")
    );
}
#[test]
fn consumer_artifact_rejects_stale_source_missing_input_and_modified_projection() {
    let mut a = Artifact::new();
    let path = a.root.join("design/brand/config.json");
    let before = fs::read(&path).unwrap();
    fs::write(&path, b"{}").unwrap();
    assert!(verify_consumer_inputs(&a.package, &a.root, &a.document).is_err());
    fs::write(&path, before).unwrap();
    a.document["sets"]["light"]["color"]["accent"]["$value"]["alpha"] = 0.0.into();
    assert!(verify_consumer_inputs(&a.package, &a.root, &a.document).is_err());
    a.document["merge"]["inputs_sha256"]
        .as_object_mut()
        .unwrap()
        .remove("design/source/layout.json");
    assert!(verify_consumer_inputs(&a.package, &a.root, &a.document).is_err());
}
