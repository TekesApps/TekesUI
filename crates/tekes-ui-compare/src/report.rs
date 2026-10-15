//! 对比报告（comparison-method 第 4 节）：机器可读的 JSON 和 Markdown 摘要表。

use crate::layout::LayoutDiff;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct Machine {
    pub model: String,
    pub chip: String,
    pub macos_version: String,
    pub macos_build: String,
    /// `defaults read -g AppleLanguages`：汉字用哪个回退字体取决于它，必须记录。
    pub languages: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Commit {
    pub commit: String,
    pub dirty: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct FixtureHash {
    pub name: String,
    pub sha256: String,
    /// 参照侧 meta.json 里记录的哈希；缺失时为 `None`。
    pub reference_sha256: Option<String>,
    pub matches_reference: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct PixelResult {
    pub appearance: String,
    pub our_size: [u32; 2],
    pub reference_size: [u32; 2],
    pub size_match: bool,
    pub over_threshold: usize,
    pub bbox_pixels: usize,
    /// 超阈值像素占文字包围盒的比例。
    pub ratio: f64,
    pub max_ratio: f64,
    pub max_component_area: usize,
    pub max_component_limit: usize,
    pub max_pixel_diff: u8,
    pub passed: bool,
    /// 热图文件名（相对输出目录）。
    pub heatmap: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CaseResult {
    pub fixture: String,
    pub width: u32,
    pub scale: u32,
    pub lines: usize,
    pub reference_lines: usize,
    pub layout: Option<LayoutDiff>,
    pub layout_passed: bool,
    /// 排版实际用到的字体（PostScript 名），含回退字体。
    pub fonts: Vec<String>,
    pub pixels: Vec<PixelResult>,
    pub pixels_passed: bool,
    pub passed: bool,
    /// 该用例没能完成时的原因（缺少参照文件等）。
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Report {
    pub schema: String,
    pub generated_at: String,
    pub machine: Machine,
    pub tekesui: Commit,
    pub reference_kit: Commit,
    pub reference_tool: Commit,
    pub fixtures: Vec<FixtureHash>,
    pub cases: Vec<CaseResult>,
}

pub const REPORT_SCHEMA: &str = "tekesui-compare-report/0";

fn mark(ok: bool) -> &'static str {
    if ok { "通过" } else { "**失败**" }
}

impl Report {
    pub fn passed(&self) -> bool {
        self.cases.iter().all(|c| c.passed)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("报告总能序列化")
    }

    /// 把 `report.json` 和 `report.md` 写进目录。
    pub fn write_to(&self, dir: &std::path::Path) -> std::io::Result<()> {
        std::fs::write(dir.join("report.json"), self.to_json())?;
        std::fs::write(dir.join("report.md"), self.to_markdown())
    }

    /// Markdown 摘要表。
    pub fn to_markdown(&self) -> String {
        let mut s = String::new();
        s.push_str("# TekesUI 与参照实现的对比报告\n\n");
        s.push_str(&format!("- 生成时间：{}\n", self.generated_at));
        s.push_str(&format!(
            "- 机器：{}（{}），macOS {}（{}）\n",
            self.machine.model,
            self.machine.chip,
            self.machine.macos_version,
            self.machine.macos_build
        ));
        s.push_str(&format!(
            "- 系统语言：{}\n",
            self.machine.languages.join("、")
        ));
        s.push_str(&format!(
            "- TekesUI 提交：`{}`{}\n",
            self.tekesui.commit,
            if self.tekesui.dirty {
                "（工作区有未提交改动）"
            } else {
                ""
            }
        ));
        s.push_str(&format!(
            "- 参照实现提交：`{}`{}；对比工具提交：`{}`\n\n",
            self.reference_kit.commit,
            if self.reference_kit.dirty {
                "（有改动）"
            } else {
                ""
            },
            self.reference_tool.commit
        ));
        let bad: Vec<_> = self
            .fixtures
            .iter()
            .filter(|f| !f.matches_reference)
            .collect();
        if bad.is_empty() {
            s.push_str("夹具哈希：与参照侧记录一致。\n\n");
        } else {
            s.push_str("**夹具哈希与参照侧不一致：**");
            for f in bad {
                s.push_str(&format!(" {}", f.name));
            }
            s.push_str("\n\n");
        }
        let total = self.cases.len();
        let ok = self.cases.iter().filter(|c| c.passed).count();
        s.push_str(&format!("共 {total} 个用例，通过 {ok} 个。\n\n"));
        s.push_str("| 夹具 | 宽 | 倍率 | 行数（我/参） | 断行 | 行度量 | 字形 | 浅色：比例 / 连通域 / 最大差 | 深色：比例 / 连通域 / 最大差 | 结论 |\n");
        s.push_str("|---|---|---|---|---|---|---|---|---|---|\n");
        for c in &self.cases {
            let (breaks, metrics, glyphs) = match &c.layout {
                Some(d) => (
                    if d.breaks_ok() {
                        "一致".to_owned()
                    } else {
                        format!("{} 行不同", d.range_mismatch_lines)
                    },
                    format!(
                        "基线 {:.3} / 宽 {:.3}",
                        d.max_baseline,
                        d.max_width.max(d.max_width_trimmed)
                    ),
                    format!(
                        "id 差 {} / 位置越界 {}（最大 {:.3}, {:.3}）",
                        d.glyph_id_mismatches,
                        d.glyph_origin_violations,
                        d.max_glyph_dx,
                        d.max_glyph_dy
                    ),
                ),
                None => ("-".into(), "-".into(), "-".into()),
            };
            let px = |name: &str| -> String {
                match c.pixels.iter().find(|p| p.appearance == name) {
                    Some(p) => format!(
                        "{:.4}% / {} / {}{}",
                        p.ratio * 100.0,
                        p.max_component_area,
                        p.max_pixel_diff,
                        if p.size_match {
                            ""
                        } else {
                            "（尺寸不同）"
                        }
                    ),
                    None => "-".into(),
                }
            };
            let verdict = match &c.error {
                Some(e) => format!("**失败**：{e}"),
                None => mark(c.passed).to_owned(),
            };
            s.push_str(&format!(
                "| {} | {} | {}x | {}/{} | {} | {} | {} | {} | {} | {} |\n",
                c.fixture,
                c.width,
                c.scale,
                c.lines,
                c.reference_lines,
                breaks,
                metrics,
                glyphs,
                px("light"),
                px("dark"),
                verdict
            ));
        }
        s.push('\n');
        s.push_str("排版门槛：断行完全一致；ascent/descent/leading/height/baseline_y 1x ≤ 0.5pt、2x ≤ 0.25pt；行宽 ≤ 0.5pt；字形 id 一致、原点 ≤ 0.25pt。像素门槛：通道差 24/255，超阈值比例 ≤ 0.5%（1x）/ 0.3%（2x），连通域不超过一个字形面积。\n");
        s
    }
}
