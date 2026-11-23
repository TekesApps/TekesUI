#[cfg(target_os = "macos")]
fn main() {
    tekes_ui_platform_macos::selftest::app_lifecycle_contract();
}
#[cfg(not(target_os = "macos"))]
fn main() {}
