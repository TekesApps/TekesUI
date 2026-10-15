//! 排版数据导出：JSON schema `tekesui-layout/0`，见 docs/schemas/layout-export.md。
//!
//! 格式只描述排版结果，不依赖任何平台类型，参照侧（Swift 的 compare 工具）与 TekesUI
//! 各自导出同一结构再逐项比较（comparison-method 第 2.2 节）。

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::model::Paragraph;
use crate::style::{FontFamily, TextStyle};

/// schema 标识，写在每个导出文件的 `schema` 字段里。
pub const SCHEMA: &str = "tekesui-layout/0";

/// 谁导出的这份数据。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExporterInfo {
    /// 导出方标识，如 `tekes-ui-text/coretext`。
    pub id: String,
    pub version: String,
    /// 导出方所依据的 git 提交。编译时由环境变量 `TEKES_UI_GIT_COMMIT` 提供，没有就是 `None`。
    pub commit: Option<String>,
}

impl ExporterInfo {
    /// 本 crate 的 CoreText 导出方。提交号取编译时环境变量 `TEKES_UI_GIT_COMMIT`，
    /// 没有时再取运行时的同名环境变量。
    pub fn current(id: &str) -> ExporterInfo {
        let commit = option_env!("TEKES_UI_GIT_COMMIT")
            .map(str::to_owned)
            .or_else(|| std::env::var("TEKES_UI_GIT_COMMIT").ok())
            .filter(|c| !c.is_empty());
        ExporterInfo {
            id: id.to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            commit,
        }
    }
}

/// 导出的元数据：不由排版结果本身决定的那部分。
#[derive(Clone, Debug)]
pub struct LayoutMeta {
    /// 夹具名。
    pub fixture: String,
    pub style: TextStyle,
    /// 换行宽度（pt），`None` 表示不换行。
    pub width: Option<f64>,
    pub scale: f64,
    pub line_spacing: f64,
    pub exporter: ExporterInfo,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FontSpec {
    /// `system-ui` 或字体名。
    pub family: String,
    pub size: f64,
    pub weight: u16,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Units {
    pub length: String,
    pub text_offsets: String,
    pub y_axis: String,
    pub glyph_origin: String,
}

impl Units {
    fn current() -> Units {
        Units {
            length: "pt".into(),
            text_offsets: "utf16".into(),
            y_axis: "down".into(),
            glyph_origin: "paragraph-top-left".into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExportGlyph {
    pub id: u16,
    /// 字体的 PostScript 名。
    pub font: String,
    /// 字形原点（段落坐标系，pt）：行原点加字形相对位置。
    pub x: f64,
    pub y: f64,
    /// 对应的 UTF-16 范围 `[start, end)`。
    pub cluster: [usize; 2],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExportLine {
    pub index: usize,
    /// UTF-16 范围 `[start, end)`，包含行尾换行符。
    pub utf16: [usize; 2],
    pub ascent: f64,
    pub descent: f64,
    pub leading: f64,
    pub height: f64,
    /// 基线 y（段落坐标系，pt），不取整。
    pub baseline_y: f64,
    /// 含行尾空白的宽度。
    pub width: f64,
    /// 不含行尾空白的宽度。
    pub width_trimmed: f64,
    pub glyphs: Vec<ExportGlyph>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LayoutExport {
    pub schema: String,
    pub fixture: String,
    pub font: FontSpec,
    /// 换行宽度（pt）；不换行时为 `null`。
    pub width: Option<f64>,
    pub scale: f64,
    pub line_spacing: f64,
    pub exporter: ExporterInfo,
    pub units: Units,
    /// 文本的 UTF-16 长度。
    pub text_utf16_len: usize,
    /// 段落总高度（pt）。
    pub height: f64,
    pub lines: Vec<ExportLine>,
}

/// 把排版结果转成可序列化的结构。
pub fn layout_export(meta: &LayoutMeta, paragraph: &Paragraph) -> LayoutExport {
    let lines = paragraph
        .lines
        .iter()
        .map(|line| {
            let mut glyphs = Vec::new();
            for run in &line.runs {
                let font = paragraph
                    .font(run.font)
                    .map(|f| f.post_script_name.clone())
                    .unwrap_or_default();
                for g in &run.glyphs {
                    glyphs.push(ExportGlyph {
                        id: g.id,
                        font: font.clone(),
                        x: g.x,
                        y: line.baseline_y + g.y,
                        cluster: [g.cluster_utf16.start, g.cluster_utf16.end],
                    });
                }
            }
            ExportLine {
                index: line.index,
                utf16: [line.utf16.start, line.utf16.end],
                ascent: line.ascent,
                descent: line.descent,
                leading: line.leading,
                height: line.height,
                baseline_y: line.baseline_y,
                width: line.width,
                width_trimmed: line.width_trimmed,
                glyphs,
            }
        })
        .collect();
    LayoutExport {
        schema: SCHEMA.to_owned(),
        fixture: meta.fixture.clone(),
        font: FontSpec {
            family: match &meta.style.family {
                FontFamily::SystemUi => "system-ui".to_owned(),
                FontFamily::Named(n) => n.clone(),
            },
            size: f64::from(meta.style.size),
            weight: meta.style.weight,
        },
        width: meta.width,
        scale: meta.scale,
        line_spacing: meta.line_spacing,
        exporter: meta.exporter.clone(),
        units: Units::current(),
        text_utf16_len: paragraph.utf16_len,
        height: paragraph.height,
        lines,
    }
}

/// 导出为 JSON 值。
pub fn export_layout(meta: &LayoutMeta, paragraph: &Paragraph) -> Value {
    serde_json::to_value(layout_export(meta, paragraph)).expect("导出结构总能序列化")
}

/// 检查一个 JSON 值是否符合 `tekesui-layout/0`：能解析成 [`LayoutExport`]，并满足
/// 文档里写明的不变量（schema 标识、单位、行范围连续且递增、数值有限等）。
pub fn validate_layout(value: &Value) -> Result<LayoutExport, String> {
    let export: LayoutExport =
        serde_json::from_value(value.clone()).map_err(|e| format!("字段不符：{e}"))?;
    if export.schema != SCHEMA {
        return Err(format!("schema 应为 {SCHEMA}，实际为 {}", export.schema));
    }
    if export.units.text_offsets != "utf16" || export.units.length != "pt" {
        return Err("units 与 schema 约定不符".into());
    }
    if !(export.scale.is_finite() && export.scale > 0.0) {
        return Err("scale 必须是正数".into());
    }
    let mut expect_start = 0usize;
    for (i, line) in export.lines.iter().enumerate() {
        if line.index != i {
            return Err(format!("第 {i} 行的 index 为 {}", line.index));
        }
        if line.utf16[0] != expect_start || line.utf16[1] < line.utf16[0] {
            return Err(format!("第 {i} 行的 utf16 范围 {:?} 不连续", line.utf16));
        }
        expect_start = line.utf16[1];
        let metrics = [
            line.ascent,
            line.descent,
            line.leading,
            line.height,
            line.baseline_y,
            line.width,
            line.width_trimmed,
        ];
        if metrics.iter().any(|v| !v.is_finite()) {
            return Err(format!("第 {i} 行有非有限数值"));
        }
        if line.width_trimmed > line.width + 1e-9 {
            return Err(format!("第 {i} 行的 width_trimmed 大于 width"));
        }
        for g in &line.glyphs {
            if !(g.x.is_finite() && g.y.is_finite()) {
                return Err(format!("第 {i} 行有非有限的字形位置"));
            }
            if g.cluster[1] < g.cluster[0] || g.cluster[1] > export.text_utf16_len {
                return Err(format!("第 {i} 行的字形簇范围 {:?} 越界", g.cluster));
            }
        }
    }
    if expect_start != export.text_utf16_len && !export.lines.is_empty() {
        return Err(format!(
            "各行没有覆盖全文：到 {expect_start}，全文 {}",
            export.text_utf16_len
        ));
    }
    Ok(export)
}
