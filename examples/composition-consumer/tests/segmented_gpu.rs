//! Explicit actual Metal/CoreText oracle. No windows or physical input.
#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{
    composition::*, extended::SegmentedControl, layout_view::SegmentedControlView,
};
use tekes_ui_scene::{Rect, Scene};
use tekes_ui_tokens::{Palette, Theme, proposal};
#[test]
#[ignore = "explicit Metal lane using the original shared GPU target and lock"]
fn segmented_actual_metal_original_view_default_and_external_theme() {
    let out = std::path::PathBuf::from(
        std::env::var_os("TEKESUI_SEGMENT_GPU_OUT").expect("explicit evidence directory"),
    );
    std::fs::create_dir_all(&out).unwrap();
    let mut gpu = tekes_ui_render::Renderer::new_headless().expect("actual Metal GPU");
    let external = std::env::var("TEKESUI_SEGMENT_THEME_IDS").ok();
    let mut rows = vec![];
    let ids: Vec<_> = external
        .as_deref()
        .map(|s| s.split(',').collect())
        .unwrap_or(vec![""]);
    for id in ids {
        for (theme_index, theme) in Theme::ALL.into_iter().enumerate() {
            for (scale_index, scale) in [1., 1.5, 2.].into_iter().enumerate() {
                for selected in 0..3 {
                    for surface in ["canvas", "sidebar", "panel"] {
                        let palette = if id.is_empty() {
                            Palette::from(theme)
                        } else {
                            Palette::for_theme_definition(theme, id)
                                .expect("real compiled external Theme")
                        };
                        if !id.is_empty() {
                            assert_eq!(palette.brand_index(), None);
                        }
                        let focus = (selected + 1) % 3;
                        let compact = selected == 1;
                        let labels = vec![
                            "首段 First".to_owned(),
                            "中段 Middle".into(),
                            "末段 Last".into(),
                        ];
                        let ls = labels.iter().map(String::as_str).collect::<Vec<_>>();
                        let mut control = SegmentedControlContent::new("Segment", labels.clone());
                        control.set_selected(selected).unwrap();
                        control.set_compact(compact);
                        let mut host = ContentHost::new(
                            Content::new(ComponentId(110), control)
                                .with_surface(surface)
                                .unwrap(),
                        )
                        .unwrap();
                        let width = if selected == 2 { 150. } else { 370. };
                        let frame = Rect::new(12., 12., width, 90.);
                        let clip = Rect::new(4., 4., width + 13., 86.);
                        let mut text = coretext::Text::new(scale);
                        text.atlas.begin_frame();
                        host.render(&mut Scene::new(), &mut text, frame, clip, palette)
                            .unwrap();
                        let token = host
                            .get(ComponentId(110))
                            .unwrap()
                            .component::<SegmentedControlContent>()
                            .unwrap()
                            .semantic_parts()[focus]
                            .index;
                        assert!(
                            host.accessibility_action(
                                ComponentId(110),
                                Some(token),
                                AccessibilityAction::Focus
                            )
                            .consumed
                        );
                        let mut actual = Scene::new();
                        host.render(&mut actual, &mut text, frame, clip, palette)
                            .unwrap();
                        let allocated = host.get(ComponentId(110)).unwrap().frame();
                        let mut model = SegmentedControl::new(3);
                        model.select(selected);
                        let mut view = SegmentedControlView::new(3);
                        view.set_surface(surface).unwrap();
                        let mut original_text = coretext::Text::new(scale);
                        original_text.atlas.begin_frame();
                        view.render(
                            &mut Scene::new(),
                            &mut original_text,
                            &mut model,
                            &ls,
                            allocated,
                            Some(clip),
                            palette,
                            proposal::TextRole::UiBody,
                            compact,
                        );
                        view.focus(focus);
                        let mut reference = Scene::new();
                        reference.push_clip(clip);
                        view.render(
                            &mut reference,
                            &mut original_text,
                            &mut model,
                            &ls,
                            allocated,
                            Some(clip),
                            palette,
                            proposal::TextRole::UiBody,
                            compact,
                        );
                        reference.pop_clip();
                        assert_eq!(
                            *host
                                .get(ComponentId(110))
                                .unwrap()
                                .component::<SegmentedControlContent>()
                                .unwrap()
                                .view()
                                .layout(),
                            *view.layout()
                        );
                        let size = (800, 240);
                        let clear = palette.color("surface-1").unwrap();
                        let a = gpu
                            .render_to_rgba8(&actual, &mut text.atlas, size, 2., clear)
                            .unwrap();
                        let b = gpu
                            .render_to_rgba8(&reference, &mut original_text.atlas, size, 2., clear)
                            .unwrap();
                        let differences = a.iter().zip(&b).filter(|(a, b)| a != b).count();
                        assert_eq!(
                            differences, 0,
                            "{id} {theme_index} {scale} {selected} {surface}"
                        );
                        assert!(a.chunks_exact(4).any(|p| p != &a[0..4]), "actual ink");
                        let name =
                            format!("{id}-t{theme_index}-s{scale_index}-v{selected}-{surface}");
                        for (suffix, data) in [("actual", a), ("original", b)] {
                            resvg::tiny_skia::Pixmap::from_vec(
                                data,
                                resvg::tiny_skia::IntSize::from_wh(size.0, size.1).unwrap(),
                            )
                            .unwrap()
                            .save_png(out.join(format!("{name}-{suffix}.png")))
                            .unwrap();
                        }
                        rows.push(serde_json::json!({"name":name,"theme":format!("{theme:?}"),"source":format!("{:?}",palette.source()),"surface":surface,"scale":scale,"selected":selected,"focus":focus,"compact":compact,"differences":differences,"pixels":size,"backing_scale":2,"renderer":"actual Metal / CoreText","native_window":false}));
                    }
                }
            }
        }
    }
    std::fs::write(
        out.join("comparisons.json"),
        serde_json::to_vec_pretty(&rows).unwrap(),
    )
    .unwrap();
    println!(
        "SEGMENT_GPU_ORACLE cases={} exact_rgba=true windows=0",
        rows.len()
    );
}
