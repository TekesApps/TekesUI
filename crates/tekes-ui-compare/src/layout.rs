//! 排版数值的逐项比较（comparison-method 第 2.2 节）。平台无关：两边都是 `tekesui-layout/0`。
//!
//! 门槛：
//!
//! | 项 | 门槛 |
//! |---|---|
//! | 断行位置（每行的 UTF-16 范围） | 完全一致 |
//! | ascent / descent / leading / height / baseline_y | 1x ≤ 0.5pt，2x ≤ 0.25pt |
//! | 行宽（含与不含行尾空白） | ≤ 0.5pt |
//! | 字形 | glyph id 一致，原点 x、y ≤ 0.25pt |
//!
//! 只有范围一致的行才比较行内数值和字形；范围不同的行已经算失败。

use serde::Serialize;
use tekes_ui_text::export::LayoutExport;

/// 比较用的容差（pt）。
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Tolerances {
    pub metric: f64,
    pub width: f64,
    pub glyph_origin: f64,
    /// 段落总高度（comparison-method：内容总高度 ≤ 1pt，F3 起是门禁，F1 只记录）。
    pub total_height: f64,
}

impl Tolerances {
    pub fn for_scale(scale: f64) -> Tolerances {
        Tolerances {
            metric: if scale >= 1.5 { 0.25 } else { 0.5 },
            width: 0.5,
            glyph_origin: 0.25,
            total_height: 1.0,
        }
    }
}

/// 一条越界记录。
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Violation {
    pub line: usize,
    pub field: String,
    pub ours: f64,
    pub reference: f64,
}

/// 逐项比较的结果。
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LayoutDiff {
    pub tolerances: Tolerances,
    pub lines_ours: usize,
    pub lines_reference: usize,
    pub text_len_equal: bool,
    /// UTF-16 范围不同的行数（逐行下标比较，多出的行也算）。
    pub range_mismatch_lines: usize,
    /// 第一处断行不同：(行号, 我们的范围, 参照的范围)。
    pub first_range_mismatch: Option<(usize, [usize; 2], [usize; 2])>,
    pub lines_compared: usize,
    pub max_ascent: f64,
    pub max_descent: f64,
    pub max_leading: f64,
    pub max_height: f64,
    pub max_baseline: f64,
    pub max_width: f64,
    pub max_width_trimmed: f64,
    pub total_height_delta: f64,
    pub glyph_lines_compared: usize,
    /// 字形个数不同的行数。
    pub glyph_count_mismatch_lines: usize,
    pub glyphs_compared: usize,
    pub glyph_id_mismatches: usize,
    pub glyph_font_mismatches: usize,
    pub max_glyph_dx: f64,
    pub max_glyph_dy: f64,
    pub glyph_origin_violations: usize,
    /// 数值越界的记录，最多保留前 [`MAX_VIOLATIONS`] 条。
    pub violations: Vec<Violation>,
}

pub const MAX_VIOLATIONS: usize = 40;

impl LayoutDiff {
    pub fn breaks_ok(&self) -> bool {
        self.text_len_equal
            && self.lines_ours == self.lines_reference
            && self.range_mismatch_lines == 0
    }

    pub fn metrics_ok(&self) -> bool {
        let t = &self.tolerances;
        self.max_ascent <= t.metric
            && self.max_descent <= t.metric
            && self.max_leading <= t.metric
            && self.max_height <= t.metric
            && self.max_baseline <= t.metric
            && self.max_width <= t.width
            && self.max_width_trimmed <= t.width
    }

    pub fn glyphs_ok(&self) -> bool {
        self.glyph_count_mismatch_lines == 0
            && self.glyph_id_mismatches == 0
            && self.glyph_origin_violations == 0
    }

    pub fn passed(&self) -> bool {
        self.breaks_ok() && self.metrics_ok() && self.glyphs_ok()
    }
}

fn exceeds(a: f64, b: f64, tol: f64) -> bool {
    // 容差内的浮点噪声（1e-9）不算。
    (a - b).abs() > tol + 1e-9
}

/// 比较两份导出。`scale` 决定行度量的容差。
pub fn compare_layout(ours: &LayoutExport, reference: &LayoutExport, scale: f64) -> LayoutDiff {
    let tol = Tolerances::for_scale(scale);
    let mut d = LayoutDiff {
        tolerances: tol,
        lines_ours: ours.lines.len(),
        lines_reference: reference.lines.len(),
        text_len_equal: ours.text_utf16_len == reference.text_utf16_len,
        range_mismatch_lines: 0,
        first_range_mismatch: None,
        lines_compared: 0,
        max_ascent: 0.0,
        max_descent: 0.0,
        max_leading: 0.0,
        max_height: 0.0,
        max_baseline: 0.0,
        max_width: 0.0,
        max_width_trimmed: 0.0,
        total_height_delta: (ours.height - reference.height).abs(),
        glyph_lines_compared: 0,
        glyph_count_mismatch_lines: 0,
        glyphs_compared: 0,
        glyph_id_mismatches: 0,
        glyph_font_mismatches: 0,
        max_glyph_dx: 0.0,
        max_glyph_dy: 0.0,
        glyph_origin_violations: 0,
        violations: Vec::new(),
    };
    let common = ours.lines.len().min(reference.lines.len());
    d.range_mismatch_lines = ours.lines.len().abs_diff(reference.lines.len());
    for i in 0..common {
        let (a, b) = (&ours.lines[i], &reference.lines[i]);
        if a.utf16 != b.utf16 {
            d.range_mismatch_lines += 1;
            if d.first_range_mismatch.is_none() {
                d.first_range_mismatch = Some((i, a.utf16, b.utf16));
            }
            continue;
        }
        d.lines_compared += 1;
        let record = |d: &mut LayoutDiff,
                      field: &str,
                      x: f64,
                      y: f64,
                      tol: f64,
                      max: fn(&mut LayoutDiff) -> &mut f64| {
            let delta = (x - y).abs();
            let slot = max(d);
            if delta > *slot {
                *slot = delta;
            }
            if exceeds(x, y, tol) && d.violations.len() < MAX_VIOLATIONS {
                d.violations.push(Violation {
                    line: i,
                    field: field.to_owned(),
                    ours: x,
                    reference: y,
                });
            }
        };
        record(&mut d, "ascent", a.ascent, b.ascent, tol.metric, |d| {
            &mut d.max_ascent
        });
        record(&mut d, "descent", a.descent, b.descent, tol.metric, |d| {
            &mut d.max_descent
        });
        record(&mut d, "leading", a.leading, b.leading, tol.metric, |d| {
            &mut d.max_leading
        });
        record(&mut d, "height", a.height, b.height, tol.metric, |d| {
            &mut d.max_height
        });
        record(
            &mut d,
            "baseline_y",
            a.baseline_y,
            b.baseline_y,
            tol.metric,
            |d| &mut d.max_baseline,
        );
        record(&mut d, "width", a.width, b.width, tol.width, |d| {
            &mut d.max_width
        });
        record(
            &mut d,
            "width_trimmed",
            a.width_trimmed,
            b.width_trimmed,
            tol.width,
            |d| &mut d.max_width_trimmed,
        );

        d.glyph_lines_compared += 1;
        if a.glyphs.len() != b.glyphs.len() {
            d.glyph_count_mismatch_lines += 1;
            if d.violations.len() < MAX_VIOLATIONS {
                d.violations.push(Violation {
                    line: i,
                    field: "glyph_count".into(),
                    ours: a.glyphs.len() as f64,
                    reference: b.glyphs.len() as f64,
                });
            }
            continue;
        }
        for (j, (ga, gb)) in a.glyphs.iter().zip(&b.glyphs).enumerate() {
            d.glyphs_compared += 1;
            if ga.id != gb.id {
                d.glyph_id_mismatches += 1;
                if d.violations.len() < MAX_VIOLATIONS {
                    d.violations.push(Violation {
                        line: i,
                        field: format!("glyph[{j}].id"),
                        ours: f64::from(ga.id),
                        reference: f64::from(gb.id),
                    });
                }
            }
            if ga.font != gb.font {
                d.glyph_font_mismatches += 1;
            }
            let (dx, dy) = ((ga.x - gb.x).abs(), (ga.y - gb.y).abs());
            d.max_glyph_dx = d.max_glyph_dx.max(dx);
            d.max_glyph_dy = d.max_glyph_dy.max(dy);
            if exceeds(ga.x, gb.x, tol.glyph_origin) || exceeds(ga.y, gb.y, tol.glyph_origin) {
                d.glyph_origin_violations += 1;
                if d.violations.len() < MAX_VIOLATIONS {
                    d.violations.push(Violation {
                        line: i,
                        field: format!("glyph[{j}].origin"),
                        ours: ga.x.max(ga.y),
                        reference: gb.x.max(gb.y),
                    });
                }
            }
        }
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;
    use tekes_ui_text::export::{ExportGlyph, ExportLine, ExporterInfo, FontSpec, SCHEMA};

    fn sample() -> LayoutExport {
        let glyph = |id: u16, x: f64| ExportGlyph {
            id,
            font: "f".into(),
            x,
            y: 13.5,
            cluster: [0, 1],
        };
        LayoutExport {
            schema: SCHEMA.into(),
            fixture: "t".into(),
            font: FontSpec {
                family: "system-ui".into(),
                post_script_name: "f".into(),
                size: 14.0,
                weight: 400,
            },
            width: Some(100.0),
            scale: 1.0,
            line_spacing: 0.0,
            exporter: ExporterInfo {
                id: "x".into(),
                version: "0".into(),
                commit: None,
            },
            units: serde_json::from_value(serde_json::json!({
                "length": "pt", "text_offsets": "utf16", "y_axis": "down",
                "glyph_origin": "paragraph-top-left"
            }))
            .unwrap(),
            text_utf16_len: 4,
            height: 34.0,
            lines: vec![
                ExportLine {
                    index: 0,
                    utf16: [0, 2],
                    ascent: 13.5,
                    descent: 3.0,
                    leading: 0.0,
                    height: 17.0,
                    baseline_y: 13.5,
                    width: 20.0,
                    width_trimmed: 18.0,
                    glyphs: vec![glyph(1, 0.0), glyph(2, 10.0)],
                },
                ExportLine {
                    index: 1,
                    utf16: [2, 4],
                    ascent: 13.5,
                    descent: 3.0,
                    leading: 0.0,
                    height: 17.0,
                    baseline_y: 30.5,
                    width: 20.0,
                    width_trimmed: 20.0,
                    glyphs: vec![glyph(3, 0.0), glyph(4, 10.0)],
                },
            ],
        }
    }

    #[test]
    fn identical_layouts_pass() {
        let a = sample();
        let d = compare_layout(&a, &a.clone(), 1.0);
        assert!(d.passed());
        assert_eq!((d.lines_compared, d.glyphs_compared), (2, 4));
    }

    #[test]
    fn a_different_break_fails_exactly() {
        let a = sample();
        let mut b = a.clone();
        b.lines[0].utf16 = [0, 3];
        b.lines[1].utf16 = [3, 4];
        let d = compare_layout(&a, &b, 1.0);
        assert!(!d.breaks_ok());
        assert_eq!(d.range_mismatch_lines, 2);
        assert_eq!(d.first_range_mismatch, Some((0, [0, 2], [0, 3])));
        let mut c = a.clone();
        c.lines.pop();
        assert!(!compare_layout(&a, &c, 1.0).breaks_ok());
    }

    #[test]
    fn tolerances_depend_on_scale() {
        let a = sample();
        let mut b = a.clone();
        b.lines[0].baseline_y += 0.4;
        assert!(compare_layout(&a, &b, 1.0).metrics_ok());
        assert!(!compare_layout(&a, &b, 2.0).metrics_ok());
        let mut w = a.clone();
        w.lines[1].width_trimmed += 0.6;
        assert!(!compare_layout(&a, &w, 1.0).metrics_ok());
    }

    #[test]
    fn glyph_ids_must_match_and_origins_within_a_quarter_point() {
        let a = sample();
        let mut b = a.clone();
        b.lines[0].glyphs[1].x += 0.2;
        assert!(compare_layout(&a, &b, 1.0).glyphs_ok());
        b.lines[0].glyphs[1].x += 0.1;
        let d = compare_layout(&a, &b, 1.0);
        assert!(!d.glyphs_ok());
        assert_eq!(d.glyph_origin_violations, 1);
        let mut c = a.clone();
        c.lines[1].glyphs[0].id = 99;
        let d = compare_layout(&a, &c, 1.0);
        assert_eq!(d.glyph_id_mismatches, 1);
        assert!(!d.glyphs_ok());
    }
}
