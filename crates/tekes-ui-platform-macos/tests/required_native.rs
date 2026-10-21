#[cfg(target_os = "macos")]
fn main() {
    let checks = tekes_ui_platform_macos::selftest::required_metadata_contract();
    assert_eq!(checks, 36);
    println!("REQUIRED_NATIVE_OBJECT_CONTRACT_OK checks={checks} windows=0 desktop_actions=0");
}
#[cfg(not(target_os = "macos"))]
fn main() {
    println!("REQUIRED_NATIVE_OBJECT_UNAVAILABLE target_is_not_macos");
}
