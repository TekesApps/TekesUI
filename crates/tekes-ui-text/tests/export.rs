//! 布局导出（tekesui-layout/0）的测试：字段、往返、校验、与 JSON Schema 文件一致。
#![cfg(target_os = "macos")]

use serde_json::{Value, json};
use tekes_ui_text::coretext::{CoreTextFonts, CoreTextShaper};
use tekes_ui_text::export::{
    ExporterInfo, LayoutExport, LayoutMeta, SCHEMA, export_layout, validate_layout,
};
use tekes_ui_text::model::ShapeOptions;
use tekes_ui_text::style::TextStyle;
use tekes_ui_text::traits::Shaper;

const TEXT: &str = "Hi 你好，e\u{301} 😀\n第二段";

fn sample() -> (LayoutMeta, tekes_ui_text::model::Paragraph) {
    let style = TextStyle::system(14.0);
    let mut s = CoreTextShaper::new(CoreTextFonts::new());
    let p = s.shape(TEXT, &style, &ShapeOptions::with_width(60.0));
    let meta = LayoutMeta {
        fixture: "test/mixed".into(),
        style,
        width: Some(60.0),
        scale: 2.0,
        line_spacing: 0.0,
        exporter: ExporterInfo {
            id: "tekes-ui-text/coretext".into(),
            version: "0.1.0".into(),
            commit: Some("abc1234".into()),
        },
    };
    (meta, p)
}

#[test]
fn export_has_every_documented_field() {
    let (meta, p) = sample();
    let v = export_layout(&meta, &p);
    assert_eq!(v["schema"], SCHEMA);
    assert_eq!(v["fixture"], "test/mixed");
    assert_eq!(v["font"]["family"], "system-ui");
    assert_eq!(v["font"]["size"], 14.0);
    assert_eq!(v["font"]["weight"], 400);
    assert!(
        v["font"]["post_script_name"]
            .as_str()
            .unwrap()
            .contains("SF")
    );
    assert_eq!(v["width"], 60.0);
    assert_eq!(v["scale"], 2.0);
    assert_eq!(v["exporter"]["id"], "tekes-ui-text/coretext");
    assert_eq!(v["exporter"]["commit"], "abc1234");
    assert_eq!(v["units"]["text_offsets"], "utf16");
    assert_eq!(v["units"]["length"], "pt");
    assert_eq!(v["text_utf16_len"], TEXT.encode_utf16().count());
    let lines = v["lines"].as_array().unwrap();
    assert_eq!(lines.len(), p.lines.len());
    for (j, l) in lines.iter().zip(&p.lines) {
        for key in [
            "index",
            "utf16",
            "ascent",
            "descent",
            "leading",
            "height",
            "baseline_y",
            "width",
            "width_trimmed",
            "glyphs",
        ] {
            assert!(j.get(key).is_some(), "行缺少字段 {key}");
        }
        assert_eq!(j["utf16"], json!([l.utf16.start, l.utf16.end]));
        assert_eq!(j["baseline_y"], l.baseline_y);
        let glyphs = j["glyphs"].as_array().unwrap();
        assert_eq!(
            glyphs.len(),
            l.runs.iter().map(|r| r.glyphs.len()).sum::<usize>()
        );
        for g in glyphs {
            for key in ["id", "font", "x", "y", "cluster"] {
                assert!(g.get(key).is_some(), "字形缺少字段 {key}");
            }
            assert!(!g["font"].as_str().unwrap().is_empty());
        }
    }
    // 字形的 y 是段落坐标系里的原点：普通字形等于所在行的基线。
    let first = &lines[0]["glyphs"][0];
    assert_eq!(first["y"], lines[0]["baseline_y"]);
}

#[test]
fn export_round_trips_through_json_text() {
    let (meta, p) = sample();
    let v = export_layout(&meta, &p);
    let text = serde_json::to_string_pretty(&v).unwrap();
    let back: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v, back);
    let typed = validate_layout(&back).expect("导出的数据应通过校验");
    let again = serde_json::to_value(&typed).unwrap();
    assert_eq!(v, again);
    assert_eq!(typed.lines.len(), p.lines.len());
    // 不换行时 width 是 null，也能往返。
    let mut meta2 = meta.clone();
    meta2.width = None;
    let v2 = export_layout(&meta2, &p);
    assert!(v2["width"].is_null());
    validate_layout(&v2).unwrap();
}

#[test]
fn validation_rejects_broken_documents() {
    let (meta, p) = sample();
    let good = export_layout(&meta, &p);

    let mut v = good.clone();
    v["schema"] = json!("tekesui-layout/1");
    assert!(validate_layout(&v).is_err());

    let mut v = good.clone();
    v.as_object_mut().unwrap().remove("lines");
    assert!(validate_layout(&v).is_err());

    let mut v = good.clone();
    v["units"]["text_offsets"] = json!("utf8");
    assert!(validate_layout(&v).is_err());

    let mut v = good.clone();
    v["lines"][1]["utf16"] = json!([99, 100]);
    assert!(validate_layout(&v).unwrap_err().contains("不连续"));

    let mut v = good.clone();
    v["lines"][0]["width_trimmed"] = json!(1.0e6);
    assert!(validate_layout(&v).is_err());

    let mut v = good.clone();
    v["scale"] = json!(0.0);
    assert!(validate_layout(&v).is_err());

    let mut v = good.clone();
    v["lines"][0]["glyphs"][0]["cluster"] = json!([0, 99999]);
    assert!(validate_layout(&v).is_err());

    // 缺一行：没有覆盖全文。
    let mut v = good;
    v["lines"].as_array_mut().unwrap().pop();
    assert!(validate_layout(&v).is_err());
}

#[test]
fn export_matches_the_json_schema_file() {
    // 不引入 JSON Schema 校验库：核对 schema 文件里声明的必填字段与导出的字段一致。
    let schema_text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/schemas/tekesui-layout-0.schema.json"
    ))
    .expect("读取 schema 文件");
    let schema: Value = serde_json::from_str(&schema_text).unwrap();
    assert_eq!(schema["properties"]["schema"]["const"], SCHEMA);

    let (meta, p) = sample();
    let v = export_layout(&meta, &p);
    fn required(node: &Value) -> Vec<String> {
        let mut r: Vec<String> = node["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap().to_owned())
            .collect();
        r.sort();
        r
    }
    fn keys(v: &Value) -> Vec<String> {
        let mut k: Vec<String> = v.as_object().unwrap().keys().cloned().collect();
        k.sort();
        k
    }
    assert_eq!(required(&schema), keys(&v));
    for key in ["font", "exporter", "units"] {
        assert_eq!(required(&schema["properties"][key]), keys(&v[key]), "{key}");
    }
    let line_schema = &schema["properties"]["lines"]["items"];
    assert_eq!(required(line_schema), keys(&v["lines"][0]));
    let glyph_schema = &line_schema["properties"]["glyphs"]["items"];
    assert_eq!(required(glyph_schema), keys(&v["lines"][0]["glyphs"][0]));
}

#[test]
fn exporter_info_reads_the_commit_from_the_environment() {
    // 没有编译期变量时读运行时环境；这里只检查版本和 id，提交号可有可无。
    let info = ExporterInfo::current("tekes-ui-text/coretext");
    assert_eq!(info.id, "tekes-ui-text/coretext");
    assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
    if let Some(c) = &info.commit {
        assert!(!c.is_empty());
    }
    let _: Option<LayoutExport> = None;
}
