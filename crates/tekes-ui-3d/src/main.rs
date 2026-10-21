#[cfg(target_os = "macos")]
mod clouds;
#[cfg(target_os = "macos")]
mod district;
#[cfg(target_os = "macos")]
mod road_wheels;
#[cfg(target_os = "macos")]
mod station;
#[cfg(target_os = "macos")]
mod transit;
#[cfg(target_os = "macos")]
mod viewer;
#[cfg(target_os = "macos")]
fn main() {
    if let Err(error) = viewer::run() {
        eprintln!("TekesUI3D: {error}");
        std::process::exit(1);
    }
}
#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("TekesUI3D's window host currently requires macOS.");
}
