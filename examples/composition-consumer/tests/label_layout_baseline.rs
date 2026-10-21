//! Compatibility baseline: natural wrapping is preserved, not a #31 acceptance.
use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer, composition::*};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_text::{
    ShapeOptions, Shaper, TextStyle,
    coretext::{CoreTextFonts, CoreTextShaper},
};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text {
    shaper: CoreTextShaper,
    scale: f32,
}
impl ButtonTextRenderer for Text {
    fn measure(&mut self, text: &str, role: proposal::TextRole, width: f32) -> Size {
        let p = self.shaper.shape(
            text,
            &TextStyle::system(role.style().size * self.scale).with_weight(role.style().weight),
            &ShapeOptions::with_width(width as f64),
        );
        Size::new(p.width as f32, p.height as f32)
    }
    fn draw(
        &mut self,
        _: &mut Scene,
        _: &str,
        _: proposal::TextRole,
        _: Rect,
        _: ButtonTextAlign,
        _: Rgba,
    ) {
    }
}
#[test]
fn real_text_button_and_compound_consumers_currently_wrap_without_mutating_original_ax_names() {
    let original = "很长的真实标题 English 👨‍👩‍👧‍👦 e\u{301} שלום العربية 123，用于核对窄宽与字体倍率，不应在消费者截断字符串。";
    let mut cases = Vec::new();
    for theme in Theme::ALL {
        for scale in [1., 1.5, 2.] {
            for kind in 0..3 {
                let mut heights = Vec::new();
                for width in [180., 600.] {
                    let id = ComponentId(9861);
                    let node = match kind {
                        0 => Content::new(id, TextContent::new(original)),
                        1 => Content::new(id, ButtonContent::new(original)),
                        _ => Content::new(
                            id,
                            CompoundButton::new(
                                original,
                                Content::new(ComponentId(9862), TextContent::new(original)),
                            )
                            .unwrap(),
                        ),
                    };
                    let mut host = ContentHost::new(node).unwrap();
                    let mut text = Text {
                        shaper: CoreTextShaper::new(CoreTextFonts::new()),
                        scale,
                    };
                    let mut scene = Scene::new();
                    let frame = Rect::new(10., 10., width, 1.);
                    host.render(
                        &mut scene,
                        &mut text,
                        frame,
                        Rect::new(0., 0., 800., 1800.),
                        theme.into(),
                    )
                    .unwrap();
                    let height = host.get(id).unwrap().frame().size.height;
                    heights.push(height);
                    let semantics = host.semantics();
                    let owner = semantics.iter().find(|n| n.id == id).unwrap();
                    assert_eq!(owner.semantics.name, original);
                    if kind == 2 {
                        assert_eq!(semantics.len(), 1, "compound children are decorative");
                    }
                    cases.push(serde_json::json!({"theme":format!("{theme:?}"),"scale":scale,"kind":kind,"width":width,"height":height,"full_AX_name":owner.semantics.name}));
                }
                assert!(
                    heights[0] > heights[1],
                    "current natural wrapping is real, not single-line ellipsis"
                );
            }
        }
    }
    assert_eq!(cases.len(), 72);
    if let Some(path) = std::env::var_os("TEKESUI_LABEL_BASELINE_OUT") {
        std::fs::write(path,serde_json::to_vec_pretty(&serde_json::json!({"scope":"Existing public TextContent/ButtonContent/CompoundButton actual composition and CoreText measurement; no-op glyph sink; current default compatibility and missing opt-in evidence, not #31 acceptance","cases":cases})).unwrap()).unwrap();
    }
}

#[test]
fn independent_consumer_can_use_the_new_readonly_core_layout_without_truncating_source() {
    use tekes_ui_text::{LabelOptions, LabelOverflow};
    let mut shaper = CoreTextShaper::new(CoreTextFonts::new());
    for original in [
        "很长的标题 English 👨‍👩‍👧‍👦 e\u{301} שלום العربية 123，需要完整保留原文",
        "first\nsecond\nthird",
        "אבג English 123 中文 😀 המשך",
    ] {
        for scale in [1., 1.5, 2.] {
            for width in [1., 80., 320.] {
                let layout = shaper.shape_label(
                    original,
                    &TextStyle::system(17. * scale),
                    &ShapeOptions::with_width(width),
                    LabelOptions::single_line(LabelOverflow::TailEllipsis),
                );
                assert_eq!(layout.source(), original);
                assert_eq!(
                    layout.paragraph().utf16_len,
                    original.encode_utf16().count()
                );
                assert_eq!(layout.paragraph().lines.len(), 1);
                assert!(layout.width() <= width);
                assert!(layout.height().is_finite());
            }
        }
    }
}
