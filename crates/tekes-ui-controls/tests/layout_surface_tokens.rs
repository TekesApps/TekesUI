#![cfg(feature = "token-audit")]
use tekes_ui_controls::{
    extended::{Axis, Stack},
    feedback_view::SurfaceView,
    layout_view::StackView,
};
use tekes_ui_scene::{Primitive, Rect, Scene};
use tekes_ui_tokens::{Contrast, Palette, Theme};
#[test]
fn actual_layout_and_surface_lookup_scopes_remain_distinct() {
    let mut cases = Vec::new();
    for theme in Theme::ALL {
        for brand in 0..3 {
            for scale in [1., 2.] {
                for panel in [false, true] {
                    for partial in [false, true] {
                        let palette = Palette::new(theme, brand).expect("registered Gallery brand");
                        let frame = Rect::new(8., 8., 280., 120.);
                        let clip = partial.then_some(Rect::new(20., 8., 90., 100.));
                        let expected = clip.unwrap_or(frame);
                        let mut stack = StackView::new(Stack {
                            axis: Axis::Horizontal,
                            gap: 4.,
                        });
                        let (geometry, geometry_reads) = tekes_ui_tokens::audit::capture(|| {
                            stack.render(frame, 3, clip).clone()
                        });
                        assert_eq!(geometry.visible, Some(expected));
                        assert_eq!(geometry.items.len(), 3);
                        assert!(geometry_reads.is_empty());
                        let mut surface = SurfaceView::default();
                        let mut scene = Scene::new();
                        let (layout, reads) = tekes_ui_tokens::audit::capture(|| {
                            surface.render_at_scale(&mut scene, frame, clip, palette, panel, scale)
                        });
                        assert_eq!(layout.frame, frame);
                        assert_eq!(layout.visible, Some(expected));
                        let q = scene
                            .primitives()
                            .iter()
                            .find_map(|p| match p {
                                Primitive::Quad(q) if q.rect == frame => Some(q),
                                _ => None,
                            })
                            .unwrap();
                        assert_eq!(
                            q.fill,
                            palette
                                .color(if panel { "surface-panel" } else { "surface-1" })
                                .unwrap()
                        );
                        assert_eq!(
                            q.border_width,
                            if !panel {
                                0.
                            } else if theme.contrast == Contrast::Increased {
                                1.
                            } else {
                                1. / scale
                            }
                        );
                        assert!(!reads.is_empty());
                        cases.push(serde_json::json!({"component":"tui.stack","content_state":"rest","state_asserted":true,"theme":format!("{theme:?}"),"brand":brand,"scale":scale,"panel":panel,"partial":partial,"lookups":geometry_reads,"no_dynamic_lookups":true,"zero_lookup_reason":"StackView owns geometry only. Actual themed SurfaceView painting runs outside this capture; child paint dependencies are not attributed to Stack."}));
                        cases.push(serde_json::json!({"component":"tui.surface","content_state":"rest","state_asserted":true,"theme":format!("{theme:?}"),"brand":brand,"scale":scale,"panel":panel,"partial":partial,"lookups":reads}));
                    }
                }
            }
        }
    }
    assert_eq!(cases.len(), 192);
    if let Some(path) = std::env::var_os("TEKESUI_LAYOUT_SURFACE_AUDIT_OUT") {
        std::fs::write(path,serde_json::to_vec_pretty(&serde_json::json!({"format":"tekesui-runtime-token-observation/1","scope":"Actual public StackView geometry and SurfaceView paint have separate nested-free lookup scopes.4themes/3brands/@1,@2/Canvas,Panel/ancestor clipping. Gallery and external consumer scale forwarding validated separately.","complete":false,"excluded":["child token dependencies attributed to layout owners","all caller and alias-closure branches","native and whole-page/ClientKit acceptance"],"cases":cases})).unwrap()).unwrap();
    }
}
