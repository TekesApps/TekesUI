//! #12's narrow nested-scroll reading page. This is a visual fixture, not an input test.
use super::{POSITIONS, settle_scroll, stabilize_document, theme_slug, write_png};
use crate::gallery::{
    Gallery, Page,
    static_text::StaticText,
    workbench::{entries, story_docs},
};
use tekes_ui_render::Renderer;
use tekes_ui_scene::Scene;
use tekes_ui_tokens::Theme;

#[test]
#[ignore = "requires an allocated GPU window and a fresh TEKESUI_MASTER_READING_OUT directory"]
fn master_detail_narrow_reading_captures_real_page_and_nested_viewports() {
    let output = std::path::PathBuf::from(
        std::env::var_os("TEKESUI_MASTER_READING_OUT")
            .expect("fresh master reading output directory"),
    );
    std::fs::create_dir(&output).expect("master reading output must not already exist");
    let mut renderer = Renderer::new_headless().expect("actual GPU required for master reading");
    let index = entries()
        .iter()
        .position(|entry| entry.slug() == "pattern-master")
        .expect("registered master-detail pattern");
    assert_eq!(entries()[index].get("kind"), "pattern");
    assert_eq!(entries()[index].renderer(), "master");
    let route = "pattern-master/playground";
    let mut cases = Vec::new();
    for theme in Theme::ALL {
        for scale in [1.0_f32, 2.0] {
            let mut gallery = Gallery::with_static_text(StaticText::new(None));
            gallery.set_system_theme(theme);
            gallery.set_reduced_motion(true);
            gallery.workbench.text_scale = 2;
            gallery.text.set_scale(scale);
            assert!(gallery.open_route(route));
            assert_eq!(gallery.page, Page::Workbench(index));
            gallery.set_view_size(720.0, 512.0);
            stabilize_document(&mut gallery);
            let range = gallery.page_scroll_range().max(0.0);
            for (position, fraction) in POSITIONS {
                let (offset, actual_range) =
                    settle_scroll(&mut gallery, fraction, range * fraction);
                assert!((offset - actual_range * fraction).abs() < 0.5);
                let mut scene = Scene::new();
                gallery.build_scene(&mut scene);
                assert_eq!(gallery.page, Page::Workbench(index));
                assert!(!scene.is_empty());
                let state = gallery.workbench.state(index);
                let list = state.list_bounds;
                let detail = state.detail.frame_at(0);
                let list_offset = state.list_scroll.offset.y;
                assert_eq!(state.selected, 0, "initial selection remains unchanged");
                let width = (720.0 * scale) as u32;
                let height = (512.0 * scale) as u32;
                let rgba = renderer
                    .render_to_rgba8(
                        &scene,
                        gallery.atlas_mut(),
                        (width, height),
                        scale,
                        Gallery::background(theme),
                    )
                    .expect("master-detail whole-page GPU render");
                assert!(rgba.chunks_exact(4).any(|pixel| pixel != &rgba[0..4]));
                let file = format!(
                    "pattern-master__playground__{}__720x512__type-150__{}@{}x.png",
                    theme_slug(theme),
                    position,
                    scale as u32
                );
                write_png(output.join(&file), width, height, &rgba);
                cases.push(serde_json::json!({
                    "file": file, "route": route, "entry_index": index,
                    "renderer": "master", "description": story_docs(&entries()[index]),
                    "theme": theme_slug(theme), "brand": 0,
                    "window": {"width": 720, "height": 512},
                    "type_scale": 1.5, "backing_scale": scale, "position": position,
                    "actual_offset": offset, "actual_scroll_range": actual_range,
                    "document_height": gallery.page_document_height(),
                    "visible_list": [list.min_x(), list.min_y(), list.size.width, list.size.height],
                    "detail_frame": detail.map(|r| [r.min_x(), r.min_y(), r.size.width, r.size.height]),
                    "list_offset": list_offset, "selected_document": 1,
                    "visual_review": "unverified",
                    "evidence": "real initial master-detail scene; no nested scroll or input actions performed"
                }));
            }
        }
    }
    assert_eq!(cases.len(), 24);
    std::fs::write(
        output.join("manifest.json"),
        serde_json::to_vec_pretty(&cases).expect("serialize master reading manifest"),
    )
    .expect("write master reading manifest");
}
