#[cfg(target_os = "macos")]
fn main() {
    let rows = tekes_ui_platform_macos::selftest::titlebar_contract();
    assert_eq!(rows.len(), 2);
    println!(
        "{}",
        serde_json::json!({"kind":"hidden native object contract, not physical input or GPU acceptance", "titlebar_overlap_rows":rows,"origin_checks":"down retained; drag preserves original down; secondary does not replace; primary up, blur and finish clear; late drag cannot restore; synthetic callback cannot start native drag","windows_shown":0})
    );
}
#[cfg(not(target_os = "macos"))]
fn main() {}
