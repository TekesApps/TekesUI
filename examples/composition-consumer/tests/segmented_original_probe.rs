//! Original public View before/after extraction oracle; no adapter dependency.
#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{extended::SegmentedControl, layout_view::SegmentedControlView};
use tekes_ui_scene::{Rect, Scene};
use tekes_ui_tokens::{Theme, proposal};
#[test]
#[ignore = "explicit original View before/after fixture output"]
fn original_segmented_scene_and_layout_probe() {
    let out = std::path::PathBuf::from(
        std::env::var_os("TEKESUI_SEGMENT_PROBE_OUT").expect("explicit original probe output"),
    );
    std::fs::create_dir_all(&out).unwrap();
    let mut i = 0;
    for theme in Theme::ALL {
        for scale in [1., 1.5, 2.] {
            for selected in 0..3 {
                for focus in 0..3 {
                    for compact in [false, true] {
                        for surface in ["canvas", "sidebar", "panel"] {
                            let mut model = SegmentedControl::new(3);
                            model.select(selected);
                            let mut view = SegmentedControlView::new(3);
                            view.set_surface(surface).unwrap();
                            let mut t = coretext::Text::new(scale);
                            let width = if selected == 2 { 70. } else { 340. };
                            let f = Rect::new(10., 10., width, 90.);
                            let clip = Rect::new(4., 4., width + 12., 78.);
                            let ls = ["重复 Repeat", "", "重复 Repeat"];
                            view.render(
                                &mut Scene::new(),
                                &mut t,
                                &mut model,
                                &ls,
                                f,
                                Some(clip),
                                theme.into(),
                                proposal::TextRole::UiBody,
                                compact,
                            );
                            view.focus(focus);
                            let mut scene = Scene::new();
                            let layout = view.render(
                                &mut scene,
                                &mut t,
                                &mut model,
                                &ls,
                                f,
                                Some(clip),
                                theme.into(),
                                proposal::TextRole::UiBody,
                                compact,
                            );
                            std::fs::write(out.join(format!("{i:04}.txt")),format!("{theme:?} {scale} {selected} {focus} {compact} {surface}\n{layout:?}\n{scene:?}")).unwrap();
                            i += 1;
                        }
                    }
                }
            }
        }
    }
    println!("ORIGINAL_SEGMENTED_PROBE cases={i}");
}
