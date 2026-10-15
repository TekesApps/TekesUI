//! 夹具清单 `fixtures/text/manifest.json`（schema `tekesui-fixtures/0`）。

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Manifest {
    pub schema: String,
    pub fixtures: Vec<Fixture>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Fixture {
    pub name: String,
    pub file: String,
    #[serde(default)]
    pub category: String,
    pub style: FixtureStyle,
    pub widths: Vec<u32>,
    pub scales: Vec<u32>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct FixtureStyle {
    pub font: String,
    pub size: f32,
    #[serde(default)]
    pub line_spacing: f64,
}

impl Manifest {
    pub fn parse(text: &str) -> Result<Manifest, String> {
        let m: Manifest =
            serde_json::from_str(text).map_err(|e| format!("manifest 解析失败：{e}"))?;
        if m.schema != "tekesui-fixtures/0" {
            return Err(format!("不认识的 manifest schema：{}", m.schema));
        }
        Ok(m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_shape_of_the_manifest() {
        let m = Manifest::parse(
            r#"{"schema":"tekesui-fixtures/0","license":"MIT","fixtures":[
            {"name":"a","file":"a.txt","category":"c","style":{"font":"system-ui","size":14,"line_spacing":0},
             "widths":[700,500],"scales":[1,2],"notes":"x"}]}"#,
        )
        .unwrap();
        assert_eq!(m.fixtures[0].widths, [700, 500]);
        assert_eq!(m.fixtures[0].style.size, 14.0);
        assert!(Manifest::parse(r#"{"schema":"other","fixtures":[]}"#).is_err());
    }
}
