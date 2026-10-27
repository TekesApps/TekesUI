#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{
    collections_view::{MarkdownView, PassiveCollectionView},
    extended::{Markdown, MarkdownBlock},
};
use tekes_ui_scene::{Primitive, Quad, Rect, Scene};
use tekes_ui_tokens::{Palette, Theme};

#[test]
fn retained_markdown_themes_preserve_subset_wrap_clip_and_caller_ground_contrast() {
    let model = Markdown::parse(
        "# Release 更新\n\nA readable paragraph with enough words to wrap across several lines in a narrow document.\nSecond source line joins the paragraph.\n\n- Stable item\n\n```rust\nlet ready = true;\nprintln!(\"{ready}\");\n```",
    );
    assert_eq!(model.blocks, vec![
        MarkdownBlock::Heading { level: 1, text: "Release 更新".into() },
        MarkdownBlock::Paragraph("A readable paragraph with enough words to wrap across several lines in a narrow document. Second source line joins the paragraph.".into()),
        MarkdownBlock::Bullet("Stable item".into()),
        MarkdownBlock::Code { language: "rust".into(), source: "let ready = true;\nprintln!(\"{ready}\");".into() },
    ]);
    let original_model = model.clone();
    let mut view = MarkdownView::new();
    let mut text = coretext::Text::new(1.);
    let mut original_layouts = [None, None];
    let mut observed = Vec::new();
    for theme in Theme::ALL {
        let palette = Palette::from(theme);
        let mut paragraph_heights = Vec::new();
        for (slot, width) in [220., 560.].into_iter().enumerate() {
            let frame = Rect::new(24., 24., width, 760.);
            let mut scene = Scene::new();
            // Markdown is transparent; this real caller supplies its document surface.
            scene.push_quad(Quad::new(frame, palette.color("document-canvas").unwrap()));
            let layout = view.render(
                &mut scene,
                &mut text,
                &model,
                frame,
                Some(frame),
                palette,
                1.,
            );
            assert!(scene.composition_error().is_none());
            assert_eq!(view.layout(), &layout);
            assert_eq!(layout.visible, Some(frame));
            assert_eq!(
                layout.blocks.iter().map(|b| b.id).collect::<Vec<_>>(),
                vec![0, 1, 2, 3]
            );
            assert!(layout.blocks.iter().all(|b| b.visible == Some(b.frame)));
            assert!(
                layout
                    .blocks
                    .windows(2)
                    .all(|b| b[0].frame.max_y() < b[1].frame.min_y())
            );
            paragraph_heights.push(layout.blocks[1].frame.size.height);
            if let Some(original) = &original_layouts[slot] {
                assert_eq!(&layout, original);
            } else {
                original_layouts[slot] = Some(layout.clone());
            }
            assert_eq!(
                scene
                    .primitives()
                    .iter()
                    .filter(|p| matches!(p, Primitive::Quad(_)))
                    .count(),
                1,
                "only caller background; no invented Markdown surface"
            );
            let Primitive::Quad(ground) = &scene.primitives()[0] else {
                panic!("caller ground")
            };
            for block in &layout.blocks {
                let labels: Vec<_> = scene
                    .primitives()
                    .iter()
                    .filter_map(|p| match p {
                        Primitive::Glyphs(run)
                            if !run.glyphs.is_empty()
                                && run.glyphs.iter().all(|g| block.frame.contains(g.origin)) =>
                        {
                            Some(run)
                        }
                        _ => None,
                    })
                    .collect();
                assert!(
                    !labels.is_empty(),
                    "real heading/paragraph/bullet/fenced-code glyphs"
                );
                assert!(
                    labels
                        .iter()
                        .all(|run| run.color == palette.color("primary-text").unwrap())
                );
                assert_eq!((labels[0].color.a, ground.fill.a), (1., 1.));
                let luminance = |c: tekes_ui_tokens::Rgba| {
                    let c = c.to_linear();
                    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
                };
                let (a, b) = (luminance(labels[0].color), luminance(ground.fill));
                assert!(
                    (a.max(b) + 0.05) / (a.min(b) + 0.05)
                        >= if theme.is_increased() { 7. } else { 4.5 }
                );
            }
            let before = view.layout().clone();
            view.cancel();
            assert_eq!(view.layout(), &before);
            assert_eq!(model, original_model);
            observed.push((ground.fill, palette.color("primary-text").unwrap()));
        }
        assert!(
            paragraph_heights[0] > paragraph_heights[1],
            "real CoreText narrow paragraph wraps"
        );
    }
    assert!(observed.windows(2).any(|w| w[0] != w[1]));
    let frame = Rect::new(24., 24., 220., 760.);
    let mut scene = Scene::new();
    let layout = view.render(
        &mut scene,
        &mut text,
        &model,
        frame,
        Some(Rect::new(24., 24., 220., 35.)),
        Theme::DARK.into(),
        1.,
    );
    assert!(layout.blocks[0].visible.is_some());
    assert!(layout.blocks[1..].iter().all(|b| b.visible.is_none()));
    assert!(scene.composition_error().is_none());
    scene.clear();
    let layout = view.render(
        &mut scene,
        &mut text,
        &model,
        frame,
        Some(Rect::new(900., 900., 10., 10.)),
        Theme::DARK.into(),
        1.,
    );
    assert!(layout.visible.is_none());
    assert!(layout.blocks.iter().all(|b| b.visible.is_none()));
    assert!(scene.is_empty());
}
