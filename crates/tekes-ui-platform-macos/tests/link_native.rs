#[cfg(target_os = "macos")]
fn main() {
    let checks = tekes_ui_platform_macos::selftest::link_role_contract();
    assert_eq!(checks, 6);
    println!("LINK_NATIVE_OBJECT_CONTRACT_OK checks={checks} windows=0 desktop_actions=0");
}
#[cfg(not(target_os = "macos"))]
fn main() {
    println!("LINK_NATIVE_OBJECT_UNAVAILABLE target_is_not_macos");
}
