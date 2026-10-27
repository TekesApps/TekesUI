#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer,
    collections_view::{DiffViewRuntime, DocumentLayout, PassiveCollectionView},
    extended::{DiffKind, DiffView},
};
use tekes_ui_scene::{Primitive, Quad, Rect, Scene, Size};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal};
struct Draw {
    source: String,
    role: proposal::TextRole,
    frame: Rect,
    ink: Rgba,
    primitives: std::ops::Range<usize>,
}
struct Text {
    actual: coretext::Text,
    draws: Vec<Draw>,
}
impl Text {
    fn new() -> Self {
        Self {
            actual: coretext::Text::new(1.),
            draws: Vec::new(),
        }
    }
}
impl ButtonTextRenderer for Text {
    fn measure(&mut self, source: &str, role: proposal::TextRole, width: f32) -> Size {
        self.actual.measure(source, role, width)
    }
    fn resolved_font_size(&mut self, role: proposal::TextRole) -> f32 {
        self.actual.resolved_font_size(role)
    }
    fn backing_scale(&self) -> f32 {
        self.actual.backing
    }
    fn draw(
        &mut self,
        scene: &mut Scene,
        source: &str,
        role: proposal::TextRole,
        frame: Rect,
        align: ButtonTextAlign,
        ink: Rgba,
    ) {
        let start = scene.len();
        self.actual.draw(scene, source, role, frame, align, ink);
        self.draws.push(Draw {
            source: source.into(),
            role,
            frame,
            ink,
            primitives: start..scene.len(),
        });
    }
}
fn contrast(a: Rgba, b: Rgba) -> f32 {
    let l = |c: Rgba| {
        let c = c.to_linear();
        0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
    };
    let (a, b) = (l(a), l(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}
fn assert_draw(scene: &Scene, draw: &Draw, palette: Palette, ground: Rgba, role: &str) {
    assert_eq!(draw.role, proposal::TextRole::Code);
    assert_eq!(draw.ink, palette.color(role).unwrap());
    let runs: Vec<_> = scene.primitives()[draw.primitives.clone()]
        .iter()
        .filter_map(|p| match p {
            Primitive::Glyphs(run) if !run.glyphs.is_empty() => Some(run),
            _ => None,
        })
        .collect();
    assert!(
        !runs.is_empty(),
        "actual CoreText diff glyphs for complete rendered line"
    );
    for run in runs {
        assert_eq!(run.color, draw.ink);
        assert_eq!(run.color.a, 1.);
        assert!(
            contrast(run.color, ground)
                >= if palette.theme.is_increased() {
                    7.
                } else {
                    4.5
                },
            "actual diff text/parent ground contrast"
        );
    }
}
fn render(
    view: &mut DiffViewRuntime,
    model: &DiffView,
    text: &mut Text,
    theme: Theme,
    frame: Rect,
    clip: Option<Rect>,
) -> (Scene, DocumentLayout) {
    text.draws.clear();
    let palette: Palette = theme.into();
    let mut scene = Scene::new();
    scene.push_quad(Quad::new(
        Rect::new(0., 0., 800., 800.),
        palette.color("document-canvas").unwrap(),
    ));
    let scale = text.actual.scale;
    let layout = view.render(&mut scene, text, model, frame, clip, palette, scale);
    (scene, layout)
}
#[test]
fn diff_view_retained_themes_keep_lcs_prefixes_line_identity_real_ink_and_passive_clipping() {
    let model = DiffView::between(
        "common\nold setting alpha beta gamma delta\nend",
        "common\nnew setting alpha beta gamma delta\nend\nadded",
    );
    let original = model.clone();
    assert_eq!(
        model.lines.iter().map(|line| line.kind).collect::<Vec<_>>(),
        [
            DiffKind::Equal,
            DiffKind::Removed,
            DiffKind::Added,
            DiffKind::Equal,
            DiffKind::Added
        ]
    );
    let expected = [
        "  common",
        "− old setting alpha beta gamma delta",
        "+ new setting alpha beta gamma delta",
        "  end",
        "+ added",
    ];
    let mut view = DiffViewRuntime::new();
    let mut text = Text::new();
    let frame = Rect::new(20., 30., 460., 400.);
    let mut initial = None;
    for theme in Theme::ALL {
        let palette: Palette = theme.into();
        let (scene, layout) = render(&mut view, &model, &mut text, theme, frame, None);
        assert_eq!(model, original);
        assert_eq!(
            layout
                .blocks
                .iter()
                .map(|block| block.id)
                .collect::<Vec<_>>(),
            [0, 1, 2, 3, 4]
        );
        if let Some(before) = &initial {
            assert_eq!(&layout, before);
        } else {
            initial = Some(layout.clone());
        }
        assert_eq!(
            text.draws
                .iter()
                .map(|draw| draw.source.as_str())
                .collect::<Vec<_>>(),
            expected
        );
        let Primitive::Quad(parent) = &scene.primitives()[0] else {
            panic!("real caller canvas")
        };
        assert_eq!(parent.fill, palette.color("document-canvas").unwrap());
        for ((line, block), draw) in model.lines.iter().zip(&layout.blocks).zip(&text.draws) {
            assert_eq!(draw.frame, block.frame);
            assert!(block.visible.is_some());
            assert_draw(
                &scene,
                draw,
                palette,
                parent.fill,
                match line.kind {
                    DiffKind::Equal => "primary-text",
                    DiffKind::Added => "status-succeeded",
                    DiffKind::Removed => "status-failed",
                },
            );
        }
    }
    let initial = initial.unwrap();
    text.actual.scale = 1.5;
    let palette: Palette = Theme::DARK_INCREASED.into();
    let (scene, narrow) = render(
        &mut view,
        &model,
        &mut text,
        Theme::DARK_INCREASED,
        Rect::new(20., 30., 120., 700.),
        None,
    );
    assert_eq!(
        text.draws
            .iter()
            .map(|draw| draw.source.as_str())
            .collect::<Vec<_>>(),
        expected
    );
    assert_eq!(
        narrow
            .blocks
            .iter()
            .map(|block| block.id)
            .collect::<Vec<_>>(),
        [0, 1, 2, 3, 4]
    );
    assert!(
        narrow.blocks[1].frame.size.height > initial.blocks[1].frame.size.height,
        "real narrow large text wraps the full removed line"
    );
    for (line, draw) in model.lines.iter().zip(&text.draws) {
        assert_draw(
            &scene,
            draw,
            palette,
            palette.color("document-canvas").unwrap(),
            match line.kind {
                DiffKind::Equal => "primary-text",
                DiffKind::Added => "status-succeeded",
                DiffKind::Removed => "status-failed",
            },
        );
    }
    text.actual.scale = 1.;
    let (scene, clipped) = render(
        &mut view,
        &model,
        &mut text,
        Theme::DARK_INCREASED,
        frame,
        Some(Rect::new(20., 30., 460., 28.)),
    );
    assert_eq!(
        clipped
            .blocks
            .iter()
            .map(|block| block.id)
            .collect::<Vec<_>>(),
        [0, 1, 2, 3, 4]
    );
    assert!(clipped.blocks[0].visible.is_some());
    assert!(
        clipped.blocks[1..]
            .iter()
            .all(|block| block.visible.is_none())
    );
    assert_eq!(text.draws.len(), 1);
    assert_eq!(text.draws[0].source, "  common");
    assert_draw(
        &scene,
        &text.draws[0],
        palette,
        palette.color("document-canvas").unwrap(),
        "primary-text",
    );
    let empty = DiffView::between("", "");
    let (scene, layout) = render(&mut view, &empty, &mut text, Theme::LIGHT, frame, None);
    assert!(layout.blocks.is_empty());
    assert_eq!(layout.content_height, 0.);
    assert!(text.draws.is_empty());
    assert!(
        !scene
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Glyphs(_)))
    );
    view.cancel();
    let (_, restored) = render(&mut view, &model, &mut text, Theme::LIGHT, frame, None);
    assert_eq!(restored, initial);
    assert_eq!(model, original);
    assert_eq!(text.draws.len(), expected.len());
}
