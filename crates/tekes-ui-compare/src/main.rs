//! `tekes-ui-compare --fixtures <dir> --reference <dir> --out <dir> [选项]`

#[cfg(target_os = "macos")]
fn main() {
    use std::path::PathBuf;
    use tekes_ui_compare::runner::{RunOptions, run};

    fn usage() -> ! {
        eprintln!(
            "用法：tekes-ui-compare --fixtures <夹具目录> --reference <参照目录> --out <输出目录>\n\
             \x20      [--only a,b] [--width 700,500] [--scale 1,2] [--no-images] [--origin-offset x,y]\n\
             退出码：全部通过为 0，有失败为 1，参数或环境错误为 2。"
        );
        std::process::exit(2);
    }

    let mut fixtures = None;
    let mut reference = None;
    let mut out = None;
    let mut only = None;
    let mut widths = None;
    let mut scales = None;
    let mut write_images = true;
    let mut origin_offset = (0.0_f32, 0.0_f32);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut value = || args.next().unwrap_or_else(|| usage());
        let list = |s: String| -> Vec<String> { s.split(',').map(str::to_owned).collect() };
        match a.as_str() {
            "--fixtures" => fixtures = Some(PathBuf::from(value())),
            "--reference" => reference = Some(PathBuf::from(value())),
            "--out" => out = Some(PathBuf::from(value())),
            "--only" => only = Some(list(value())),
            "--width" => {
                widths = Some(
                    list(value())
                        .iter()
                        .filter_map(|s| s.parse().ok())
                        .collect(),
                );
            }
            "--scale" => {
                scales = Some(
                    list(value())
                        .iter()
                        .filter_map(|s| s.parse().ok())
                        .collect(),
                );
            }
            "--no-images" => write_images = false,
            "--origin-offset" => {
                let v = value();
                let mut it = v.split(',').map(|s| s.parse::<f32>());
                match (it.next(), it.next()) {
                    (Some(Ok(x)), Some(Ok(y))) => origin_offset = (x, y),
                    _ => usage(),
                }
            }
            _ => usage(),
        }
    }
    let (Some(fixtures), Some(reference), Some(out)) = (fixtures, reference, out) else {
        usage()
    };
    let opts = RunOptions {
        fixtures,
        reference,
        out: out.clone(),
        only,
        widths,
        scales,
        write_images,
        origin_offset,
    };
    let report = match run(&opts) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("错误：{e}");
            std::process::exit(2);
        }
    };
    if let Err(e) = report.write_to(&out) {
        eprintln!("写报告失败：{e}");
        std::process::exit(2);
    }
    eprintln!("报告：{}/report.json、report.md", out.display());
    std::process::exit(if report.passed() { 0 } else { 1 });
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("tekes-ui-compare 目前只支持 macOS");
    std::process::exit(2);
}
