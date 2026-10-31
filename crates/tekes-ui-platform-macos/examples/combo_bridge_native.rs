//! Explicit original #116 native SDK probe. Requires its own allocated native window.
#[cfg(target_os = "macos")]
fn main() {
    let checks = tekes_ui_platform_macos::selftest::combo_text_role_contract();
    println!("COMBO_NATIVE_CHECKS {checks}");
    assert!(checks > 100);
}
#[cfg(not(target_os = "macos"))]
fn main() {
    panic!("ComboBox native contract requires macOS");
}
