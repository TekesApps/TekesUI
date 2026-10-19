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
