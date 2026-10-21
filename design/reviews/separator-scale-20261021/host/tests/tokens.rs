#![cfg(feature = "token-audit")]
use tekes_ui_controls::{extended::Axis, layout_view::SeparatorView};
use tekes_ui_scene::{Primitive, Rect, Scene};
use tekes_ui_tokens::{Contrast, Palette, Theme};
#[test]
fn real_separator_uses_shared_color_and_applicable_stroke_lookups() {
    let mut cases = Vec::new();
    for theme in Theme::ALL {
        for brand in 0..3 {
            for scale in [1., 2.] {
                for vertical in [false, true] {
                    for phase in [0., 0.25, 0.5] {
                        let palette = Palette { theme, brand };
                        let frame = Rect::new(8. + phase, 12. + phase, 100., 12.);
                        let axis = if vertical {
                            Axis::Vertical
                        } else {
                            Axis::Horizontal
                        };
                        let mut view = SeparatorView::default();
                        let mut scene = Scene::new();
                        let (layout, reads) = tekes_ui_tokens::audit::capture(|| {
                            view.render_at_scale(&mut scene, frame, axis, None, palette, scale)
                        });
                        assert_eq!(layout.frame, frame);
                        assert_eq!(layout.visible, Some(frame));
                        let q = scene
                            .primitives()
                            .iter()
                            .find_map(|p| {
                                if let Primitive::Quad(q) = p {
                                    Some(q)
                                } else {
                                    None
                                }
                            })
                            .unwrap();
                        assert_eq!(q.fill, palette.color("separator").unwrap());
                        let width = if vertical {
                            q.rect.size.width
                        } else {
                            q.rect.size.height
                        };
                        assert_eq!(
                            width,
                            if theme.contrast == Contrast::Increased {
                                1.
                            } else {
                                1. / scale
                            }
                        );
                        assert!(!reads.is_empty());
                        cases.push(serde_json::json!({"component":"tui.separator","content_state":"rest","scenario":"scale-aware-static-separator","state_asserted":true,"theme":format!("{theme:?}"),"brand":brand,"scale":scale,"vertical":vertical,"phase":phase,"lookups":reads}));
                    }
                }
            }
        }
    }
    assert_eq!(cases.len(), 144);
    let path = std::env::var_os("TEKESUI_SEPARATOR_TOKEN_OUT").expect("explicit output");
    std::fs::write(path,serde_json::to_vec_pretty(&serde_json::json!({"format":"tekesui-runtime-token-observation/1","scope":"Public SeparatorView render_at_scale, actual normal/IC width and separator color;4themes3brands,axes,scales and fractional phases. No interaction or native claim.","complete":false,"excluded":["nonlookup geometry/constants","all external caller/layout branches","native/whole-page/ClientKit acceptance"],"cases":cases})).unwrap()).unwrap();
}
