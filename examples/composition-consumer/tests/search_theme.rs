#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::composition::*;
use tekes_ui_scene::{Primitive, Rect, Scene};
use tekes_ui_tokens::{Palette, Theme, proposal};
const SEARCH: ComponentId = ComponentId(29200);
fn draw(host: &mut ContentHost, text: &mut coretext::Text, theme: Theme) -> Scene {
    let mut scene = Scene::new();
    host.render(
        &mut scene,
        text,
        Rect::new(24., 24., 420., 60.),
        Rect::new(0., 0., 520., 160.),
        theme.into(),
    )
    .unwrap();
    assert!(scene.composition_error().is_none());
    scene
}
fn assert_query_ink(scene: &Scene, frame: Rect, theme: Theme, backing: f32) {
    let palette = Palette::from(theme);
    let (index, ground) = scene
        .primitives()
        .iter()
        .enumerate()
        .rev()
        .find_map(|(i, p)| match p {
            Primitive::Quad(q) if q.rect == frame => Some((i, q)),
            _ => None,
        })
        .expect("actual public Search single-line input own ground");
    assert_eq!(ground.fill, palette.color("transcript-input").unwrap());
    let labels: Vec<_> = scene.primitives()[index + 1..]
        .iter()
        .filter_map(|p| match p {
            Primitive::Glyphs(r)
                if !r.glyphs.is_empty()
                    && r.glyphs.iter().any(|g| {
                        let a = g.glyph;
                        frame
                            .intersection(&Rect::new(
                                g.origin.x + a.left as f32 / backing,
                                g.origin.y + a.top as f32 / backing,
                                a.width as f32 / backing,
                                a.height as f32 / backing,
                            ))
                            .is_some()
                    }) =>
            {
                Some(r)
            }
            _ => None,
        })
        .collect();
    assert!(
        !labels.is_empty(),
        "real CoreText query glyphs after own background"
    );
    assert!(
        labels
            .iter()
            .all(|r| r.color == palette.color("primary-text").unwrap())
    );
    assert_eq!((labels[0].color.a, ground.fill.a), (1., 1.));
    let luminance = |c: tekes_ui_tokens::Rgba| {
        let c = c.to_linear();
        0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
    };
    let (a, b) = (luminance(labels[0].color), luminance(ground.fill));
    assert!((a.max(b) + 0.05) / (a.min(b) + 0.05) >= if theme.is_increased() { 7. } else { 4.5 });
}
#[test]
fn search_retained_public_inline_content_keeps_query_theme_geometry_and_edit_clear() {
    let field = TextFieldContent::inline("Search", "notes 文档", 24.);
    let mut host = ContentHost::new(Content::new(SEARCH, field)).unwrap();
    host.set_action_events(true);
    let mut text = coretext::Text::new(1.);
    text.services = true;
    draw(&mut host, &mut text, Theme::LIGHT);
    assert!(host.focus(Some(SEARCH), true));
    let mut original_geometry = None;
    for theme in Theme::ALL {
        let scene = draw(&mut host, &mut text, theme);
        let field = host
            .get(SEARCH)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap();
        let surface = &field.surface;
        assert!(!field.show_metadata);
        assert!(!surface.input.multiline && !surface.input.read_only);
        assert_eq!(surface.role, proposal::TextRole::UiBody);
        assert_eq!(surface.input.text, "notes 文档");
        let geometry = (surface.outer, surface.inner);
        if let Some(original) = original_geometry {
            assert_eq!(geometry, original);
        } else {
            original_geometry = Some(geometry);
        }
        assert_query_ink(&scene, surface.outer, theme, text.backing);
        let nodes = host.semantics();
        let node = nodes.iter().find(|n| n.id == SEARCH).unwrap();
        assert_eq!(node.semantics.role, ContentRole::TextInput);
        assert_eq!(node.semantics.name, "Search");
        assert_eq!(node.semantics.value.as_deref(), Some("notes 文档"));
    }
    host.event(&ContentEvent::TextCommand("selectAll:".into()));
    assert!(
        host.event(&ContentEvent::Commit {
            text: "readme 查询".into(),
            replacement: None
        })
        .changed
    );
    draw(&mut host, &mut text, Theme::DARK_INCREASED);
    assert_eq!(
        host.semantics()[0].semantics.value.as_deref(),
        Some("readme 查询")
    );
    host.event(&ContentEvent::TextCommand("selectAll:".into()));
    assert!(
        host.event(&ContentEvent::TextCommand("deleteBackward:".into()))
            .changed
    );
    draw(&mut host, &mut text, Theme::DARK_INCREASED);
    assert_eq!(host.semantics()[0].semantics.value.as_deref(), Some(""));
    assert!(
        host.event(&ContentEvent::Commit {
            text: "notes 文档".into(),
            replacement: None
        })
        .changed
    );
    let scene = draw(&mut host, &mut text, Theme::DARK_INCREASED);
    let field = host
        .get(SEARCH)
        .unwrap()
        .component::<TextFieldContent>()
        .unwrap();
    assert_eq!(field.surface.input.text, "notes 文档");
    assert_query_ink(
        &scene,
        field.surface.outer,
        Theme::DARK_INCREASED,
        text.backing,
    );
    assert!(
        host.drain_action_records().is_empty(),
        "editing/query clear does not invent a business activation"
    );
}
