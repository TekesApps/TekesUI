#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{
    collections_view::{CodeBlockView, PassiveCollectionView},
    extended::CodeBlock,
};
use tekes_ui_scene::{CornerRadii, Primitive, Rect, Scene};
use tekes_ui_tokens::{Palette, Theme, proposal};

#[test]
fn retained_code_block_themes_keep_real_source_wrap_clip_and_text_contrast() {
    let model = CodeBlock {
        language: "rust".into(),
        source: "let label = \"真实 source text with enough words to wrap in the narrow slot\";\n\nprintln!(\"{label}\");".into(),
    };
    assert_eq!(model.lines().count(), 3);
    let mut view = CodeBlockView::new();
    let mut text = coretext::Text::new(1.);
    let clip = Rect::new(0., 0., 700., 600.);
    let mut geometries = [None, None];
    let mut observed = Vec::new();
    for theme in Theme::ALL {
        let palette = Palette::from(theme);
        let mut heights = Vec::new();
        for (slot, width) in [220., 560.].into_iter().enumerate() {
            let frame = Rect::new(24., 24., width, 500.);
            let mut scene = Scene::new();
            let layout = view.render(
                &mut scene,
                &mut text,
                &model,
                frame,
                Some(clip),
                palette,
                1.,
            );
            assert!(scene.composition_error().is_none());
            assert_eq!(view.layout(), &layout);
            assert_eq!(layout.visible, Some(frame));
            assert_eq!(
                layout.blocks.iter().map(|b| b.id).collect::<Vec<_>>(),
                vec![0, 1, 2]
            );
            assert!(layout.blocks.iter().all(|b| b.visible == Some(b.frame)));
            if let Some(original) = &geometries[slot] {
                assert_eq!(&layout, original);
            } else {
                geometries[slot] = Some(layout.clone());
            }
            heights.push(layout.content_height);
            let ground = scene
                .primitives()
                .iter()
                .find_map(|p| match p {
                    Primitive::Quad(q) if q.rect == frame => Some(q),
                    _ => None,
                })
                .expect("own CodeBlock surface");
            assert_eq!(ground.fill, palette.color("code-surface").unwrap());
            assert_eq!(
                ground.corner_radii,
                CornerRadii::uniform(proposal::metric("radius", "c-code-radius").unwrap())
            );
            assert_eq!(ground.border_color, palette.color("separator").unwrap());
            assert_eq!(
                ground.border_width,
                if theme.is_increased() {
                    proposal::metric("stroke", "stroke-rule").unwrap()
                } else {
                    1. / text.backing
                }
            );
            for (source, block) in model.lines().zip(&layout.blocks) {
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
                if source.is_empty() {
                    assert!(
                        labels.is_empty(),
                        "empty source line keeps its line slot without ink"
                    );
                    continue;
                }
                assert!(
                    !labels.is_empty(),
                    "real CoreText glyphs in each nonempty source line"
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
            assert!(
                scene
                    .primitives()
                    .iter()
                    .any(|p| matches!(p,Primitive::PushClip(r) if *r==clip))
            );
            let before = view.layout().clone();
            view.cancel();
            assert_eq!(
                view.layout(),
                &before,
                "passive cancellation does not alter the document"
            );
            observed.push((ground.fill, palette.color("primary-text").unwrap()));
        }
        assert!(
            heights[0] > heights[1],
            "real narrow source wraps into more lines"
        );
    }
    assert!(observed.windows(2).any(|w| w[0] != w[1]));
    let mut hidden = Scene::new();
    let layout = view.render(
        &mut hidden,
        &mut text,
        &model,
        Rect::new(24., 24., 220., 500.),
        Some(Rect::new(800., 800., 20., 20.)),
        Theme::DARK.into(),
        1.,
    );
    assert!(layout.visible.is_none());
    assert!(layout.blocks.iter().all(|b| b.visible.is_none()));
    assert!(
        hidden.is_empty(),
        "fully clipped passive source paints nothing"
    );
    assert_eq!(model.language, "rust");
    assert_eq!(model.lines().nth(2), Some("println!(\"{label}\");"));
}
