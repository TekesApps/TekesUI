//! TekesUI Gallery: default AppHost showcase, headless snapshots, and selftest/perf harnesses.

#[cfg(target_os = "macos")]
mod gallery;
#[cfg(all(target_os = "macos", feature = "selftest"))]
mod perf;
#[cfg(all(target_os = "macos", feature = "selftest"))]
mod selftest_scripts;

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("Gallery 目前只支持 macOS");
}

#[cfg(target_os = "macos")]
fn main() {
    let startup_args: Vec<String> = std::env::args().collect();
    if let Some(args) = gallery::apphost_workbench::apphost_arguments(&startup_args) {
        if let Err(error) = gallery::apphost_workbench::run(&args) {
            eprintln!("Gallery AppHost: {error:#}");
            std::process::exit(1);
        }
        return;
    }

    let mut args = startup_args.iter().skip(1).peekable();
    if args.peek().is_some_and(|arg| arg.as_str() == "--worker") {
        if let Err(error) = gallery::apphost_workbench::run(&startup_args) {
            eprintln!("Gallery AppHost: {error:#}");
            std::process::exit(1);
        }
        return;
    }

    match args.next().map(String::as_str) {
        Some("--snapshot") => {
            let Some(dir) = args.next() else {
                eprintln!("用法：tekes-ui-gallery --snapshot <输出目录>");
                std::process::exit(2);
            };
            if let Some(extra) = args.next() {
                eprintln!(
                    "未知参数 {extra}\n用法：tekes-ui-gallery [--apphost | --snapshot <输出目录>]"
                );
                std::process::exit(2);
            }
            match gallery::snapshot::write_all(std::path::Path::new(dir)) {
                Ok(files) => {
                    for f in &files {
                        println!("{}", f.display());
                    }
                    eprintln!("共写出 {} 张", files.len());
                }
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
        }
        Some(other) => {
            eprintln!(
                "未知参数 {other}\n用法：tekes-ui-gallery [--apphost | --snapshot <输出目录>]"
            );
            std::process::exit(2);
        }
        None => unreachable!(),
    }
}
