//! 对比运行器：排版、导出、与参照逐项比较，再用 atlas 加 wgpu 渲染并和参照位图做像素差异。
//!
//! 条件与参照侧一致：排版原点 (0, 0)，没有内边距、竖直偏移和 backing 相位，画布是
//! `宽度 × ceil(总高度) × scale`，文本是夹具文件的原样内容（包括末尾的换行）。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use sha2::{Digest, Sha256};
use tekes_ui_render::{RenderError, Renderer};
use tekes_ui_scene::{GlyphAtlas, Point, Scene};
use tekes_ui_text::compare::{DiffOptions, diff_rgba8};
use tekes_ui_text::coretext::{CoreTextFonts, CoreTextRasterizer, CoreTextShaper};
use tekes_ui_text::export::{ExporterInfo, LayoutMeta, validate_layout};
use tekes_ui_text::model::ShapeOptions;
use tekes_ui_text::push::push_paragraph;
use tekes_ui_text::style::{FontFamily, TextStyle};
use tekes_ui_text::traits::Shaper;
use tekes_ui_tokens::Rgba;

use crate::layout::compare_layout;
use crate::manifest::{Fixture, Manifest};
use crate::report::{CaseResult, Commit, FixtureHash, Machine, PixelResult, REPORT_SCHEMA, Report};

/// 运行选项。
#[derive(Clone, Debug)]
pub struct RunOptions {
    pub fixtures: PathBuf,
    pub reference: PathBuf,
    pub out: PathBuf,
    pub only: Option<Vec<String>>,
    pub widths: Option<Vec<u32>>,
    pub scales: Option<Vec<u32>>,
    /// 是否写热图和我们自己的渲染 PNG。
    pub write_images: bool,
    /// 诊断用：给我们这一侧的渲染原点加偏移（pt）。正式对比必须是 (0, 0)；
    /// 非零时用来确认比较确实对位置敏感（反向校验）。
    pub origin_offset: (f32, f32),
}

#[derive(Clone, Debug)]
struct Appearance {
    name: String,
    foreground: [u8; 3],
    background: [u8; 3],
}

fn builtin_appearances() -> Vec<Appearance> {
    vec![
        Appearance {
            name: "light".into(),
            foreground: [0, 0, 0],
            background: [255, 255, 255],
        },
        Appearance {
            name: "dark".into(),
            foreground: [230, 230, 230],
            background: [30, 30, 30],
        },
    ]
}

fn read_appearances(reference: &Path) -> Vec<Appearance> {
    let Ok(text) = fs::read_to_string(reference.join("appearances.json")) else {
        return builtin_appearances();
    };
    let Ok(v) = serde_json::from_str::<Value>(&text) else {
        return builtin_appearances();
    };
    let rgb = |v: &Value| -> Option<[u8; 3]> {
        let a = v.as_array()?;
        Some([
            a.first()?.as_u64()? as u8,
            a.get(1)?.as_u64()? as u8,
            a.get(2)?.as_u64()? as u8,
        ])
    };
    let parsed: Option<Vec<Appearance>> = v["appearances"].as_array().and_then(|list| {
        list.iter()
            .map(|a| {
                Some(Appearance {
                    name: a["name"].as_str()?.to_owned(),
                    foreground: rgb(&a["foreground"])?,
                    background: rgb(&a["background"])?,
                })
            })
            .collect()
    });
    parsed.unwrap_or_else(builtin_appearances)
}

fn run_cmd(program: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(program).args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

/// 解析 `defaults read -g AppleLanguages` 的括号列表输出。
pub fn parse_languages(text: &str) -> Vec<String> {
    text.lines()
        .map(|l| {
            l.trim()
                .trim_matches(|c| c == '(' || c == ')' || c == ',' || c == '"')
        })
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
        .collect()
}

pub fn machine_info() -> Machine {
    Machine {
        model: run_cmd("sysctl", &["-n", "hw.model"]).unwrap_or_default(),
        chip: run_cmd("sysctl", &["-n", "machdep.cpu.brand_string"]).unwrap_or_default(),
        macos_version: run_cmd("sw_vers", &["-productVersion"]).unwrap_or_default(),
        macos_build: run_cmd("sw_vers", &["-buildVersion"]).unwrap_or_default(),
        languages: run_cmd("defaults", &["read", "-g", "AppleLanguages"])
            .map(|t| parse_languages(&t))
            .unwrap_or_default(),
    }
}

/// 取一个 git 检出的提交号和是否有未提交改动。
fn git_commit(dir: &Path) -> Commit {
    let d = dir.to_string_lossy().into_owned();
    let commit =
        run_cmd("git", &["-C", &d, "rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".into());
    let dirty = run_cmd("git", &["-C", &d, "status", "--porcelain"]).is_some_and(|s| !s.is_empty());
    Commit { commit, dirty }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

fn read_png_rgba(path: &Path) -> Result<(u32, u32, Vec<u8>), String> {
    let file = fs::File::open(path).map_err(|e| format!("{}：{e}", path.display()))?;
    let mut reader = png::Decoder::new(std::io::BufReader::new(file))
        .read_info()
        .map_err(|e| format!("{}：{e}", path.display()))?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| format!("{}：{e}", path.display()))?;
    if info.bit_depth != png::BitDepth::Eight {
        return Err(format!("{}：只支持 8 位 PNG", path.display()));
    }
    let data = &buf[..info.buffer_size()];
    let rgba = match info.color_type {
        png::ColorType::Rgba => data.to_vec(),
        png::ColorType::Rgb => data
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        other => return Err(format!("{}：不支持的颜色类型 {other:?}", path.display())),
    };
    Ok((info.width, info.height, rgba))
}

fn write_png_rgba(path: &Path, w: u32, h: u32, rgba: &[u8]) -> Result<(), String> {
    let file = fs::File::create(path).map_err(|e| format!("{}：{e}", path.display()))?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()
        .and_then(|mut w| w.write_image_data(rgba))
        .map_err(|e| format!("{}：{e}", path.display()))
}

fn case_stem(fixture: &str, width: u32, scale: u32) -> String {
    format!("{fixture}-w{width}@{scale}x")
}

fn text_style(f: &Fixture) -> TextStyle {
    let family = if f.style.font == "system-ui" {
        FontFamily::SystemUi
    } else {
        FontFamily::Named(f.style.font.clone())
    };
    TextStyle {
        family,
        size: f.style.size,
        weight: 400,
    }
}

fn rgba8(c: [u8; 3]) -> Rgba {
    Rgba::rgb8(c[0], c[1], c[2])
}

/// 运行全部用例。
pub fn run(opts: &RunOptions) -> Result<Report, String> {
    let manifest_text = fs::read_to_string(opts.fixtures.join("manifest.json"))
        .map_err(|e| format!("读取 manifest 失败：{e}"))?;
    let manifest = Manifest::parse(&manifest_text)?;
    let appearances = read_appearances(&opts.reference);
    let meta: Value = fs::read_to_string(opts.reference.join("meta.json"))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or(Value::Null);

    let mut renderer = match Renderer::new_headless() {
        Ok(r) => r,
        Err(RenderError::NoAdapter) => return Err("没有可用的 GPU 适配器".into()),
        Err(e) => return Err(format!("创建渲染器失败：{e}")),
    };
    fs::create_dir_all(&opts.out).map_err(|e| format!("创建输出目录失败：{e}"))?;

    let mut fixtures_hashes = Vec::new();
    let mut cases = Vec::new();
    let selected: Vec<&Fixture> = manifest
        .fixtures
        .iter()
        .filter(|f| opts.only.as_ref().is_none_or(|o| o.contains(&f.name)))
        .collect();
    if selected.is_empty() {
        return Err("没有选中任何夹具".into());
    }

    for f in selected {
        let bytes = fs::read(opts.fixtures.join(&f.file))
            .map_err(|e| format!("读取夹具 {} 失败：{e}", f.file))?;
        let sha = sha256_hex(&bytes);
        let reference_sha = meta["fixtures"]
            .as_array()
            .and_then(|l| l.iter().find(|x| x["name"] == f.name.as_str()))
            .and_then(|x| x["sha256"].as_str().map(str::to_owned));
        fixtures_hashes.push(FixtureHash {
            name: f.name.clone(),
            matches_reference: reference_sha.as_deref() == Some(sha.as_str()),
            sha256: sha,
            reference_sha256: reference_sha,
        });
        // 原样内容，不规范化，保留末尾的换行。
        let text =
            String::from_utf8(bytes).map_err(|e| format!("夹具 {} 不是 UTF-8：{e}", f.file))?;

        for &width in &f.widths {
            if opts.widths.as_ref().is_some_and(|w| !w.contains(&width)) {
                continue;
            }
            for &scale in &f.scales {
                if opts.scales.as_ref().is_some_and(|s| !s.contains(&scale)) {
                    continue;
                }
                let case = run_case(opts, &mut renderer, &appearances, f, &text, width, scale);
                eprintln!(
                    "{} w{} @{}x：{}",
                    f.name,
                    width,
                    scale,
                    if case.passed { "通过" } else { "失败" }
                );
                cases.push(case);
            }
        }
    }

    let tekesui_dir = opts
        .fixtures
        .ancestors()
        .find(|p| p.join(".git").exists())
        .unwrap_or(&opts.fixtures);
    let reference_kit = Commit {
        commit: meta["reference"]["commit"]
            .as_str()
            .unwrap_or("unknown")
            .to_owned(),
        dirty: meta["reference"]["dirty"].as_bool().unwrap_or(false),
    };
    let reference_tool = Commit {
        commit: meta["tool_repo"]["commit"]
            .as_str()
            .unwrap_or("unknown")
            .to_owned(),
        dirty: meta["tool_repo"]["dirty"].as_bool().unwrap_or(false),
    };
    Ok(Report {
        schema: REPORT_SCHEMA.to_owned(),
        generated_at: run_cmd("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"]).unwrap_or_default(),
        machine: machine_info(),
        tekesui: git_commit(tekesui_dir),
        reference_kit,
        reference_tool,
        fixtures: fixtures_hashes,
        cases,
    })
}

#[allow(clippy::too_many_arguments)]
fn run_case(
    opts: &RunOptions,
    renderer: &mut Renderer,
    appearances: &[Appearance],
    f: &Fixture,
    text: &str,
    width: u32,
    scale: u32,
) -> CaseResult {
    let mut result = CaseResult {
        fixture: f.name.clone(),
        width,
        scale,
        lines: 0,
        reference_lines: 0,
        layout: None,
        layout_passed: false,
        fonts: Vec::new(),
        pixels: Vec::new(),
        pixels_passed: false,
        passed: false,
        error: None,
    };
    match try_case(
        opts,
        renderer,
        appearances,
        f,
        text,
        width,
        scale,
        &mut result,
    ) {
        Ok(()) => {}
        Err(e) => result.error = Some(e),
    }
    result.passed = result.error.is_none() && result.layout_passed && result.pixels_passed;
    result
}

#[allow(clippy::too_many_arguments)]
fn try_case(
    opts: &RunOptions,
    renderer: &mut Renderer,
    appearances: &[Appearance],
    f: &Fixture,
    text: &str,
    width: u32,
    scale: u32,
    result: &mut CaseResult,
) -> Result<(), String> {
    let style = text_style(f);
    let fonts = CoreTextFonts::new();
    let mut shaper = CoreTextShaper::new(fonts.clone());
    let mut rasterizer = CoreTextRasterizer::new(fonts);
    let paragraph = shaper.shape(
        text,
        &style,
        &ShapeOptions {
            width: Some(f64::from(width)),
            line_spacing: f.style.line_spacing,
        },
    );
    result.lines = paragraph.lines.len();
    result.fonts = paragraph
        .fonts
        .iter()
        .map(|f| f.post_script_name.clone())
        .collect();

    // 排版导出，写到输出目录，再和参照逐项比较。
    let stem = case_stem(&f.name, width, scale);
    let meta = LayoutMeta {
        fixture: f.name.clone(),
        style: style.clone(),
        width: Some(f64::from(width)),
        scale: f64::from(scale),
        line_spacing: f.style.line_spacing,
        exporter: ExporterInfo::current("tekes-ui-text/coretext"),
    };
    let ours_json = tekes_ui_text::export::export_layout(&meta, &paragraph);
    let dir = opts.out.join(&f.name);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    fs::write(
        dir.join(format!("{stem}.layout.json")),
        serde_json::to_string(&ours_json).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let ours = validate_layout(&ours_json)?;

    let ref_dir = opts.reference.join(&f.name);
    let ref_layout_text = fs::read_to_string(ref_dir.join(format!("{stem}.layout.json")))
        .map_err(|e| format!("缺少参照排版 {stem}.layout.json：{e}"))?;
    let ref_value: Value =
        serde_json::from_str(&ref_layout_text).map_err(|e| format!("参照排版解析失败：{e}"))?;
    let reference =
        validate_layout(&ref_value).map_err(|e| format!("参照排版不符合 schema：{e}"))?;
    result.reference_lines = reference.lines.len();
    let diff = compare_layout(&ours, &reference, f64::from(scale));
    result.layout_passed = diff.passed();
    result.layout = Some(diff);

    // 渲染：同样的画布、颜色。
    let height_pt = paragraph.height.ceil();
    let size = (
        (f64::from(width) * f64::from(scale)).ceil() as u32,
        (height_pt * f64::from(scale)).ceil() as u32,
    );
    let mut all_ok = true;
    for a in appearances {
        let mut atlas = GlyphAtlas::default();
        atlas.begin_frame();
        let mut scene = Scene::new();
        push_paragraph(
            &mut scene,
            &mut atlas,
            &mut rasterizer,
            &paragraph,
            Point::new(opts.origin_offset.0, opts.origin_offset.1),
            scale as f32,
            rgba8(a.foreground),
        );
        let gpu = renderer
            .render_to_rgba8(&scene, &mut atlas, size, scale as f32, rgba8(a.background))
            .map_err(|e| format!("渲染失败：{e}"))?;
        let ref_png = ref_dir.join(format!("{stem}-{}.png", a.name));
        let (rw, rh, ref_rgba) = read_png_rgba(&ref_png)?;
        let size_match = (rw, rh) == size;
        let mut px = PixelResult {
            appearance: a.name.clone(),
            our_size: [size.0, size.1],
            reference_size: [rw, rh],
            size_match,
            over_threshold: 0,
            bbox_pixels: 0,
            ratio: 1.0,
            max_ratio: 0.0,
            max_component_area: 0,
            max_component_limit: 0,
            max_pixel_diff: 255,
            passed: false,
            heatmap: None,
        };
        if size_match {
            let bg = [a.background[0], a.background[1], a.background[2], 255];
            let dopts = DiffOptions::for_scale(
                f64::from(scale),
                f64::from(f.style.size) * f64::from(scale),
                bg,
            );
            let rep = diff_rgba8(&ref_rgba, &gpu, size.0, size.1, dopts);
            px.over_threshold = rep.over_threshold;
            px.bbox_pixels = rep.bbox_pixels;
            px.ratio = rep.ratio;
            px.max_ratio = dopts.max_ratio;
            px.max_component_area = rep.max_component_area;
            px.max_component_limit = dopts.max_component_area;
            px.max_pixel_diff = rep.max_pixel_diff;
            px.passed = rep.passed();
            if opts.write_images {
                let heat = format!("{stem}-{}-heat.png", a.name);
                write_png_rgba(&dir.join(&heat), size.0, size.1, &rep.heatmap)?;
                write_png_rgba(
                    &dir.join(format!("{stem}-{}-ours.png", a.name)),
                    size.0,
                    size.1,
                    &gpu,
                )?;
                px.heatmap = Some(format!("{}/{heat}", f.name));
            }
        } else if opts.write_images {
            write_png_rgba(
                &dir.join(format!("{stem}-{}-ours.png", a.name)),
                size.0,
                size.1,
                &gpu,
            )?;
        }
        all_ok &= px.passed;
        result.pixels.push(px);
    }
    result.pixels_passed = all_ok;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_defaults_language_list() {
        let text = "(\n    \"en-JP\",\n    \"ja-JP\"\n)\n";
        assert_eq!(parse_languages(text), ["en-JP", "ja-JP"]);
    }

    #[test]
    fn sha256_matches_a_known_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
