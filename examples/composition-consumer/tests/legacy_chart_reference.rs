#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{BarChart, ChartDatum, collections_view::ChartView};
use tekes_ui_scene::{Primitive, Rect, Scene};
use tekes_ui_tokens::Theme;
#[test]
#[ignore = "explicit original Chart View before/after fixture output"]
fn bare_original_view_cpu_reference() {
    let mut rows = vec![];
    for (name, count, value, label, width, height) in [
        ("positive", 64, 1., "", 480., 240.),
        ("negative", 64, -1., "", 480., 240.),
        ("zero", 4, 0., "same", 480., 240.),
        ("narrow", 64, 1., "", 120., 240.),
        (
            "long",
            64,
            1.,
            "Long duplicate category label with original complete text",
            480.,
            240.,
        ),
    ] {
        for focused in [false, true] {
            let mut m = BarChart::new((0..count).map(|_| ChartDatum::new(label, value)).collect());
            m.select(2);
            let mut v = ChartView::new();
            if focused {
                v.focus(true, &mut m);
            }
            let mut t = coretext::Text::new(1.);
            t.backing = 1.;
            let mut scene = Scene::new();
            let l = v.render(
                &mut scene,
                &mut t,
                &mut m,
                Rect::new(16., 16., width, height),
                None,
                Theme::ALL[0].into(),
                1.,
            );
            let primitives = scene
                .primitives()
                .iter()
                .map(|p| match p {
                    Primitive::Glyphs(g) => format!(
                        "glyph {:?} {:?}",
                        g.color,
                        g.glyphs.iter().map(|g| g.origin).collect::<Vec<_>>()
                    ),
                    other => format!("{other:?}"),
                })
                .collect::<Vec<_>>();
            rows.push(serde_json::json!({"case":name,"focused":focused,"geometry":format!("{l:?}"),"data":format!("{:?}",m.data()),"selection":m.selected(),"primitives":primitives}));
        }
    }
    std::fs::write(
        std::env::var_os("CHART_LEGACY_OUT").unwrap(),
        serde_json::to_vec_pretty(&rows).unwrap(),
    )
    .unwrap();
}
