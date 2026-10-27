#[cfg(target_os = "macos")]
fn main() {
    tekes_ui_platform_macos::selftest::exclusive_reentry_contract();
}
#[cfg(not(target_os = "macos"))]
fn main() {
    println!("EXCLUSIVE_REENTRY_NATIVE_UNAVAILABLE target_is_not_macos");
}
