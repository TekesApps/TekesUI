use tekes_ui_controls::{
    BarChart, ButtonTextAlign, ButtonTextRenderer, ChartDatum,
    collections_view::{ChartOwnerCapacity, ChartOwnerFocusSpace, ChartView},
};
use tekes_ui_scene::{Primitive, Rect, Scene, Size};
use tekes_ui_tokens::{
    Palette, Rgba, Theme,
    proposal::TextRole,
    surface_palette::{SurfaceError, resolve_data_plot_surface, surface_names},
};
#[derive(Default)]
struct Text {
    drawn: Vec<(String, Rgba)>,
}
impl ButtonTextRenderer for Text {
    fn measure(&mut self, s: &str, _: TextRole, w: f32) -> Size {
        Size::new((s.len() as f32 * 7.).min(w), 18.)
    }
    fn draw(&mut self, _: &mut Scene, s: &str, _: TextRole, _: Rect, _: ButtonTextAlign, c: Rgba) {
        self.drawn.push((s.into(), c));
    }
}
#[test]
fn original_d1_uses_complete_surface_packet_and_preserves_geometry_selection_and_witness() {
    for mode in Theme::ALL {
        for surface in surface_names() {
            let palette: Palette = std::env::var("CHART_THEME_ID")
                .ok()
                .map(|id| {
                    Palette::for_theme_definition(mode, &id).expect("selected external Theme")
                })
                .unwrap_or_else(|| mode.into());
            let packet = resolve_data_plot_surface(palette, surface).unwrap();
            let mut m = BarChart::new(vec![
                ChartDatum::new("category", -1.),
                ChartDatum::new("category", 0.),
            ]);
            let mut v = ChartView::new();
            let mut text = Text::default();
            assert_eq!(
                v.arrange_owner(&mut text, &mut m, Rect::new(0., 0., 300., 200.), None),
                ChartOwnerCapacity::Sufficient
            );
            v.try_focus_owner(true, &mut m).unwrap();
            let layout = v.layout().clone();
            let before = m.clone();
            let witness = v.owner_focus_space(&m);
            let mut scene = Scene::new();
            assert_eq!(
                v.paint_owner_on_surface(&mut scene, &mut text, &mut m, palette, surface),
                Ok(witness)
            );
            assert_eq!(m, before);
            assert_eq!(v.layout().geometry.plot, layout.geometry.plot);
            assert_eq!(
                v.layout()
                    .geometry
                    .bars
                    .iter()
                    .map(|b| b.rect)
                    .collect::<Vec<_>>(),
                layout
                    .geometry
                    .bars
                    .iter()
                    .map(|b| b.rect)
                    .collect::<Vec<_>>()
            );
            assert!(text.drawn.iter().enumerate().all(|(i, (_, color))| *color
                == if i % 2 == 0 {
                    packet.plot.text
                } else {
                    packet.category.text
                }));
            assert!(
                scene
                    .primitives()
                    .iter()
                    .any(|p| matches!(p,Primitive::Quad(q) if Some(q.fill)==packet.plot.marker))
            );
            assert!(
                scene
                    .primitives()
                    .iter()
                    .any(|p| matches!(p,Primitive::Quad(q) if q.fill==packet.selection.background))
            );
            assert!(scene.primitives().iter().any(|p|matches!(p,Primitive::Quad(q) if q.border_width==2. && Some(q.border_color)==packet.plot.focus)));
            assert!(matches!(
                v.owner_focus_space(&m),
                ChartOwnerFocusSpace::Visible { .. }
            ));
        }
    }
}
#[test]
fn unknown_surface_rejects_before_any_scene_text_model_or_layout_effects() {
    let mut m = BarChart::new(vec![ChartDatum::new("legal", 0.)]);
    let mut v = ChartView::new();
    let mut text = Text::default();
    v.arrange_owner(&mut text, &mut m, Rect::new(0., 0., 300., 200.), None);
    v.try_focus_owner(true, &mut m).unwrap();
    let before = m.clone();
    let layout = v.layout().clone();
    let space = v.owner_focus_space(&m);
    let mut scene = Scene::new();
    assert_eq!(
        v.paint_owner_on_surface(
            &mut scene,
            &mut text,
            &mut m,
            Theme::LIGHT.into(),
            "unknown"
        ),
        Err(SurfaceError::UnknownCombination)
    );
    assert!(scene.primitives().is_empty());
    assert!(text.drawn.is_empty());
    assert_eq!(m, before);
    assert_eq!(v.layout(), &layout);
    assert_eq!(v.owner_focus_space(&m), space);
    v.arrange_owner(&mut text, &mut m, Rect::new(0., 0., 4., 4.), None);
    let before = m.clone();
    assert_eq!(
        v.paint_owner_on_surface(&mut scene, &mut text, &mut m, Theme::LIGHT.into(), "canvas"),
        Ok(ChartOwnerFocusSpace::Insufficient(
            ChartOwnerCapacity::InsufficientAllocation
        ))
    );
    assert_eq!(m, before);
    assert!(m.is_focused());
}

#[test]
fn interior_clip_keeps_visible_body_and_labels_without_faking_required_focus() {
    let mut m = BarChart::new(vec![
        ChartDatum::new("category", -1.),
        ChartDatum::new("category", 0.),
        ChartDatum::new("category", 1.),
    ]);
    m.select(1);
    let mut v = ChartView::new();
    let mut text = Text::default();
    let a = Rect::new(0., 0., 300., 240.);
    v.arrange_owner(&mut text, &mut m, a, None);
    v.try_focus_owner(true, &mut m).unwrap();
    assert_eq!(
        v.arrange_owner(&mut text, &mut m, a, Some(Rect::new(50., 30., 150., 195.))),
        ChartOwnerCapacity::Sufficient
    );
    assert_eq!(
        v.owner_focus_space(&m),
        ChartOwnerFocusSpace::NoVisibleWitness
    );
    let before = m.clone();
    let mut scene = Scene::new();
    assert_eq!(
        v.paint_owner_on_surface(&mut scene, &mut text, &mut m, Theme::LIGHT.into(), "canvas"),
        Ok(ChartOwnerFocusSpace::NoVisibleWitness)
    );
    assert_eq!(m, before);
    assert!(m.is_focused() && m.focus_visible());
    assert!(!scene.primitives().is_empty());
    assert!(!text.drawn.is_empty());
    assert!(scene.primitives().iter().any(|p|matches!(p,Primitive::Quad(q) if q.fill.a>0. && q.rect.size.width>0. && q.rect.size.height>0.)));
    assert!(
        !scene
            .primitives()
            .iter()
            .any(|p| matches!(p,Primitive::Quad(q) if q.border_width==2.))
    );
    assert_eq!(
        v.owner_focus_space(&m),
        ChartOwnerFocusSpace::NoVisibleWitness
    );
}
