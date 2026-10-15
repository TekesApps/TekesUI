//! 运行器的自测：不依赖私有参照数据。
//!
//! 参照目录由测试自己生成：排版导出加一张"逐字形 CTFontDrawGlyphs 画在整数像素上"的 PNG
//! （对照图，独立于 atlas 和 wgpu）。然后验证三件事：未被动过的参照通过；断行被改过的
//! 参照在排版门禁上失败；位图被改过的参照在像素门禁上失败。没有 GPU 适配器时跳过。
#![cfg(target_os = "macos")]

use std::fs;
use std::path::{Path, PathBuf};

use tekes_ui_compare::runner::{RunOptions, run};
use tekes_ui_scene::Point;
use tekes_ui_text::coretext::{
    CoreTextFonts, CoreTextShaper, OracleOptions, Placement, render_oracle,
};
use tekes_ui_text::export::{ExporterInfo, LayoutMeta, export_layout};
use tekes_ui_text::model::ShapeOptions;
use tekes_ui_text::style::TextStyle;
use tekes_ui_text::traits::Shaper;
use tekes_ui_tokens::Rgba;

const TEXT: &str = "测试 Hello, 世界。这一行会换行吗？\nSecond 行 abc 😀\n";
const WIDTH: u32 = 120;
const MANIFEST: &str = r#"{"schema":"tekesui-fixtures/0","license":"MIT","fixtures":[
  {"name":"tiny","file":"tiny.txt","category":"test",
   "style":{"font":"system-ui","size":14,"line_spacing":0},
   "widths":[120],"scales":[1,2],"notes":"自测"}]}"#;
const APPEARANCES: [(&str, [u8; 3], [u8; 3]); 2] = [
    ("light", [0, 0, 0], [255, 255, 255]),
    ("dark", [230, 230, 230], [30, 30, 30]),
];

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_png(path: &Path, w: u32, h: u32, rgba: &[u8]) {
    let file = fs::File::create(path).unwrap();
    let mut enc = png::Encoder::new(file, w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(rgba).unwrap();
}

/// 生成 fixtures 和 reference 目录。
fn make_dirs(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let root = scratch(name);
    let (fixtures, reference, out) = (
        root.join("fixtures"),
        root.join("reference"),
        root.join("out"),
    );
    fs::create_dir_all(&fixtures).unwrap();
    fs::write(fixtures.join("manifest.json"), MANIFEST).unwrap();
    fs::write(fixtures.join("tiny.txt"), TEXT).unwrap();
    let ref_dir = reference.join("tiny");
    fs::create_dir_all(&ref_dir).unwrap();
    let style = TextStyle::system(14.0);
    for scale in [1u32, 2] {
        let fonts = CoreTextFonts::new();
        let mut shaper = CoreTextShaper::new(fonts.clone());
        let p = shaper.shape(TEXT, &style, &ShapeOptions::with_width(f64::from(WIDTH)));
        let meta = LayoutMeta {
            fixture: "tiny".into(),
            style: style.clone(),
            width: Some(f64::from(WIDTH)),
            scale: f64::from(scale),
            line_spacing: 0.0,
            exporter: ExporterInfo {
                id: "selftest".into(),
                version: "0".into(),
                commit: None,
            },
        };
        let stem = format!("tiny-w{WIDTH}@{scale}x");
        fs::write(
            ref_dir.join(format!("{stem}.layout.json")),
            export_layout(&meta, &p).to_string(),
        )
        .unwrap();
        let size = (WIDTH * scale, (p.height.ceil() as u32) * scale);
        for (a, fg, bg) in APPEARANCES {
            let img = render_oracle(
                &fonts,
                &p,
                TEXT,
                &style,
                Point::new(0.0, 0.0),
                size,
                scale as f32,
                Rgba::rgb8(fg[0], fg[1], fg[2]),
                Rgba::rgb8(bg[0], bg[1], bg[2]),
                OracleOptions {
                    placement: Placement::GlyphsRounded,
                    font_smoothing: false,
                },
            );
            write_png(
                &ref_dir.join(format!("{stem}-{a}.png")),
                size.0,
                size.1,
                &img,
            );
        }
    }
    (fixtures, reference, out)
}

fn options(fixtures: &Path, reference: &Path, out: &Path) -> RunOptions {
    RunOptions {
        fixtures: fixtures.to_owned(),
        reference: reference.to_owned(),
        out: out.to_owned(),
        only: None,
        widths: None,
        scales: None,
        write_images: true,
        origin_offset: (0.0, 0.0),
    }
}

fn run_or_skip(opts: &RunOptions) -> Option<tekes_ui_compare::report::Report> {
    match run(opts) {
        Ok(r) => Some(r),
        Err(e) if e.contains("GPU") => {
            eprintln!("跳过：{e}");
            None
        }
        Err(e) => panic!("{e}"),
    }
}

#[test]
fn untouched_reference_passes_and_writes_artifacts() {
    let (fixtures, reference, out) = make_dirs("runner-pass");
    let Some(report) = run_or_skip(&options(&fixtures, &reference, &out)) else {
        return;
    };
    assert_eq!(report.cases.len(), 2);
    for c in &report.cases {
        assert!(c.passed, "{c:?}");
        assert_eq!(c.pixels.len(), 2);
        // 不是空比较：包围盒里有真实的文字。
        assert!(c.pixels.iter().all(|p| p.bbox_pixels > 100 && p.size_match));
        assert_eq!(c.lines, c.reference_lines);
        assert!(c.fonts.len() >= 2, "应记录回退字体：{:?}", c.fonts);
    }
    report.write_to(&out).unwrap();
    for f in [
        "report.json",
        "report.md",
        "tiny/tiny-w120@2x-light-heat.png",
        "tiny/tiny-w120@1x.layout.json",
    ] {
        assert!(out.join(f).exists(), "缺少 {f}");
    }
    let json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(out.join("report.json")).unwrap()).unwrap();
    assert_eq!(json["schema"], "tekesui-compare-report/0");
    assert!(json["machine"]["languages"].is_array());
    assert!(
        fs::read_to_string(out.join("report.md"))
            .unwrap()
            .contains("| tiny |")
    );
}

#[test]
fn a_changed_line_break_fails_the_layout_gate() {
    let (fixtures, reference, out) = make_dirs("runner-break");
    let path = reference.join("tiny/tiny-w120@1x.layout.json");
    let mut v: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    let end0 = v["lines"][0]["utf16"][1].as_u64().unwrap();
    v["lines"][0]["utf16"][1] = (end0 - 1).into();
    v["lines"][1]["utf16"][0] = (end0 - 1).into();
    fs::write(&path, v.to_string()).unwrap();
    let Some(report) = run_or_skip(&options(&fixtures, &reference, &out)) else {
        return;
    };
    let bad = report.cases.iter().find(|c| c.scale == 1).unwrap();
    assert!(!bad.passed);
    let d = bad.layout.as_ref().unwrap();
    assert!(!d.breaks_ok());
    assert_eq!(d.range_mismatch_lines, 2);
    // 另一个倍率的用例不受影响。
    assert!(report.cases.iter().find(|c| c.scale == 2).unwrap().passed);
    assert!(!report.passed());
}

#[test]
fn a_shifted_reference_bitmap_fails_the_pixel_gate() {
    let (fixtures, reference, out) = make_dirs("runner-pixels");
    // 反向校验：运行器把我们这一侧挪 1pt，像素门禁必须失败，排版门禁不受影响。
    let mut opts = options(&fixtures, &reference, &out);
    opts.origin_offset = (1.0, 0.0);
    let Some(report) = run_or_skip(&opts) else {
        return;
    };
    for c in &report.cases {
        assert!(c.layout_passed);
        assert!(!c.pixels_passed && !c.passed, "{c:?}");
        assert!(c.pixels.iter().all(|p| p.ratio > p.max_ratio));
    }
    // 缺少参照位图时报告错误而不是 panic。
    fs::remove_file(reference.join("tiny/tiny-w120@1x-dark.png")).unwrap();
    opts.origin_offset = (0.0, 0.0);
    let report = run_or_skip(&opts).unwrap();
    let c = report.cases.iter().find(|c| c.scale == 1).unwrap();
    assert!(
        c.error.as_deref().is_some_and(|e| e.contains("dark.png")),
        "{:?}",
        c.error
    );
    assert!(!c.passed);
}
