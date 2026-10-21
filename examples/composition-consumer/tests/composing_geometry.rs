#![cfg(target_os = "macos")]
use std::rc::Rc;
use tekes_ui_controls::TextSurface;
use tekes_ui_scene::Rect;
use tekes_ui_text::{
    coretext::{CoreTextFonts, CoreTextShaper},
    model::ShapeOptions,
    style::TextStyle,
    traits::Shaper,
};

fn layout(surface: &mut TextSurface, frame: Rect, clip: Rect, backing: f32, scale: f32) {
    let mut shaper = CoreTextShaper::new(CoreTextFonts::new());
    let size = surface.role.style().size * scale;
    surface.layout_with(frame, Some(clip), backing, scale, |text, width| {
        Rc::new(shaper.shape(
            text,
            &TextStyle::system(size),
            &ShapeOptions {
                width: width.map(f64::from),
                line_spacing: 0.,
            },
        ))
    });
}
#[test]
fn composing_caret_survives_type_scale_clip_and_resize_without_editor_mutation() {
    for backing in [1., 2.] {
        let mut s = TextSurface::new(&"中文 English 😀 é 12345 ".repeat(140), true, false);
        let initial = Rect::new(16., 484., 968., 224.);
        let full = Rect::new(0., 0., 1000., 760.);
        layout(&mut s, initial, full, backing, 1.);
        s.input.select(2210, false);
        s.reveal();
        s.input.preedit("ni", 2..2, None);
        s.reveal();
        layout(&mut s, initial, full, backing, 1.);
        let editor = format!("{:?}", s.input);
        for (frame, clip, scale) in [
            (initial, full, 1.5),
            (initial, Rect::new(32., 0., 872., 760.), 2.),
            (
                Rect::new(16., 309., 688., 224.),
                Rect::new(32., 0., 592., 600.),
                2.,
            ),
            (initial, full, 1.),
        ] {
            layout(&mut s, frame, clip, backing, scale);
            assert_eq!(
                format!("{:?}", s.input),
                editor,
                "geometry cannot reset composition/selection/history"
            );
            assert!(
                s.visible_frame().unwrap().contains(s.caret().center()),
                "composing caret clipped at backing={backing} scale={scale}: caret={:?} visible={:?}",
                s.caret(),
                s.visible_frame()
            );
        }
        s.scroll.scroll_by(0., -40.);
        let offset = s.scroll.offset;
        layout(&mut s, initial, full, backing, 1.);
        assert_eq!(
            s.scroll.offset, offset,
            "unchanged layout must respect an explicit user scroll"
        );
    }
}
