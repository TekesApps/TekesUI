#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{TextSurface, TextSurfaceHost, composition::*};
use tekes_ui_scene::{Primitive, Quad, Rect, Scene};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal};
fn contrast(a: Rgba, b: Rgba) -> f32 {
    let l = |c: Rgba| {
        let c = c.to_linear();
        0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
    };
    let (a, b) = (l(a), l(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}
fn assert_input(
    scene: &Scene,
    frame: Rect,
    palette: Palette,
    state: (bool, bool, bool),
    ink_role: &str,
    backing: f32,
) {
    let (read_only, disabled, invalid) = state;
    let (index, ground) = scene
        .primitives()
        .iter()
        .enumerate()
        .rev()
        .find_map(|(i, p)| match p {
            Primitive::Quad(q) if q.rect == frame => Some((i, *q)),
            _ => None,
        })
        .expect("actual original TextField input ground");
    assert_eq!(
        ground.fill,
        palette
            .color(if read_only {
                "surface-1"
            } else {
                "transcript-input"
            })
            .unwrap()
    );
    assert_eq!(
        ground.corner_radii,
        tekes_ui_scene::CornerRadii::uniform(proposal::metric("radius", "radius-input").unwrap())
    );
    assert_eq!(
        ground.border_color,
        palette
            .color(
                proposal::control_boundary(
                    if invalid { "error" } else { "required-input" },
                    palette.theme
                )
                .unwrap()
            )
            .unwrap()
    );
    if invalid && !disabled {
        assert!(
            contrast(ground.border_color, ground.fill) >= 3.,
            "actual invalid boundary/input ground"
        );
    }
    let inks: Vec<_> = scene.primitives()[index + 1..]
        .iter()
        .filter_map(|p| match p {
            Primitive::Glyphs(run)
                if run.glyphs.iter().any(|g| {
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
                Some(run.color)
            }
            _ => None,
        })
        .collect();
    assert!(
        !inks.is_empty(),
        "real CoreText input or declared Gallery placeholder glyphs after own input ground"
    );
    for ink in inks {
        assert_eq!(ink, palette.color(ink_role).unwrap());
        if !disabled {
            assert!(
                contrast(ink, ground.fill)
                    >= if palette.theme.is_increased() {
                        7.
                    } else {
                        4.5
                    },
                "actual TextField glyph/input ground contrast"
            );
        }
    }
}
fn has_ring(scene: &Scene, frame: Rect, palette: Palette) -> bool {
    scene.primitives().iter().any(|p|matches!(p,Primitive::Quad(q)if q.fill.a==0. && q.border_width>0.
        && q.rect.center()==frame.center() && q.border_color==palette.color("border-focused").unwrap()))
}

fn render_surface(
    surface: &mut TextSurface,
    text: &mut coretext::Text,
    theme: Theme,
    focused: bool,
) -> Scene {
    let role = surface.role;
    let scale = text.scale;
    let backing = text.backing;
    surface.layout_with(
        Rect::new(20., 30., 420., 48.),
        Some(Rect::new(0., 0., 540., 160.)),
        backing,
        scale,
        |source, width| TextSurfaceHost::shape_surface(text, source, role, width),
    );
    let mut scene = Scene::new();
    surface.draw(text, &mut scene, theme, focused, false);
    scene
}
#[test]
fn text_field_primary_single_line_surface_retains_theme_and_declared_state_ink() {
    let mut field = TextSurface::new("Project draft", false, false);
    let mut text = coretext::Text::new(1.);
    assert_eq!(field.role, proposal::TextRole::UiBody);
    assert!(!field.input.multiline);
    let selection = field.input.selection();
    let mut initial = None;
    for theme in Theme::ALL {
        let scene = render_surface(&mut field, &mut text, theme, false);
        let palette: Palette = theme.into();
        assert_eq!(field.input.text, "Project draft");
        assert_eq!(field.input.selection(), selection);
        if let Some(before) = initial {
            assert_eq!((field.outer, field.inner), before);
        } else {
            initial = Some((field.outer, field.inner));
        }
        assert_input(
            &scene,
            field.outer,
            palette,
            (false, false, false),
            "primary-text",
            text.backing,
        );
        assert!(!has_ring(&scene, field.outer, palette));
    }
    let theme = Theme::DARK_INCREASED;
    let palette: Palette = theme.into();
    let scene = render_surface(&mut field, &mut text, theme, true);
    assert!(has_ring(&scene, field.outer, palette));
    field.error = true;
    let scene = render_surface(&mut field, &mut text, theme, true);
    assert_input(
        &scene,
        field.outer,
        palette,
        (false, false, true),
        "primary-text",
        text.backing,
    );
    assert!(has_ring(&scene, field.outer, palette));
    field.error = false;
    field.input.read_only = true;
    let scene = render_surface(&mut field, &mut text, theme, true);
    assert_input(
        &scene,
        field.outer,
        palette,
        (true, false, false),
        "primary-text",
        text.backing,
    );
    assert!(!field.input.insert("blocked", None));
    assert_eq!(field.input.text, "Project draft");
    field.input.read_only = false;
    field.input.disabled = true;
    let scene = render_surface(&mut field, &mut text, theme, true);
    assert_input(
        &scene,
        field.outer,
        palette,
        (false, true, false),
        "disabled-text",
        text.backing,
    );
    assert!(!has_ring(&scene, field.outer, palette));
    assert!(!field.input.insert("blocked", None));
    assert_eq!(field.input.text, "Project draft");
}
fn render_host(host: &mut ContentHost, text: &mut coretext::Text, theme: Theme) -> Scene {
    let mut scene = Scene::new();
    let palette: Palette = theme.into();
    scene.push_quad(Quad::new(
        Rect::new(0., 0., 640., 360.),
        palette.color("document-canvas").unwrap(),
    ));
    host.render(
        &mut scene,
        text,
        Rect::new(20., 30., 500., 240.),
        Rect::new(0., 0., 640., 360.),
        palette,
    )
    .unwrap();
    scene
}
#[test]
fn text_field_content_retained_themes_use_real_opted_in_host_and_original_input_semantics() {
    let mut field = TextFieldContent::new("Project name", "draft");
    field.field.help = "Choose a name".into();
    field.field.required = true;
    let mut host = ContentHost::new(Content::new(ComponentId(1), field)).unwrap();
    let mut text = coretext::Text::new(1.);
    text.services = true; // Existing required TextSurfaceHost opt-in; actual CoreText, not a no-op renderer.
    let mut initial = None;
    for theme in Theme::ALL {
        let scene = render_host(&mut host, &mut text, theme);
        let field = host
            .get(ComponentId(1))
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap();
        let frame = field.surface.outer;
        if let Some(before) = initial {
            assert_eq!(frame, before);
        } else {
            initial = Some(frame);
        }
        assert_eq!(field.surface.role, proposal::TextRole::UiBody);
        assert_input(
            &scene,
            frame,
            theme.into(),
            (false, false, false),
            "primary-text",
            text.backing,
        );
        let node = &host.semantics()[0];
        assert_eq!(node.semantics.role, ContentRole::TextInput);
        assert_eq!(node.semantics.name, "Project name");
        assert_eq!(node.semantics.value.as_deref(), Some("draft"));
        assert!(node.semantics.required);
        assert_eq!(node.semantics.description.as_deref(), Some("Choose a name"));
    }
    let theme = Theme::DARK_INCREASED;
    let palette: Palette = theme.into();
    host.focus(Some(ComponentId(1)), true);
    assert!(
        host.event(&ContentEvent::Commit {
            text: "renamed".into(),
            replacement: Some(0..5)
        })
        .changed
    );
    let scene = render_host(&mut host, &mut text, theme);
    let frame = host.semantics()[0].frame;
    assert!(has_ring(&scene, frame, palette));
    assert_eq!(
        host.semantics()[0].semantics.value.as_deref(),
        Some("renamed")
    );
    host.get_mut(ComponentId(1))
        .unwrap()
        .component_mut::<TextFieldContent>()
        .unwrap()
        .surface
        .input
        .read_only = true;
    let scene = render_host(&mut host, &mut text, theme);
    assert_input(
        &scene,
        frame,
        palette,
        (true, false, false),
        "primary-text",
        text.backing,
    );
    assert!(
        !host
            .event(&ContentEvent::Commit {
                text: "blocked".into(),
                replacement: None
            })
            .changed
    );
    assert!(
        !host
            .accessibility_action(
                ComponentId(1),
                None,
                AccessibilityAction::SetValue("blocked".into())
            )
            .changed
    );
    let field = host
        .get_mut(ComponentId(1))
        .unwrap()
        .component_mut::<TextFieldContent>()
        .unwrap();
    field.surface.input.read_only = false;
    field.field.error = Some("Name requires review".into());
    let scene = render_host(&mut host, &mut text, theme);
    let field = host
        .get(ComponentId(1))
        .unwrap()
        .component::<TextFieldContent>()
        .unwrap();
    assert_input(
        &scene,
        field.surface.outer,
        palette,
        (false, false, true),
        "primary-text",
        text.backing,
    );
    assert!(host.semantics()[0].semantics.invalid);
    let message = field.view.layout().message;
    let errors: Vec<_> = scene
        .primitives()
        .iter()
        .filter_map(|p| match p {
            Primitive::Glyphs(run)
                if !run.glyphs.is_empty()
                    && run.glyphs.iter().all(|g| message.contains(g.origin)) =>
            {
                Some(run.color)
            }
            _ => None,
        })
        .collect();
    assert!(!errors.is_empty());
    for ink in errors {
        assert_eq!(ink, palette.color("status-failed").unwrap());
        assert!(contrast(ink, palette.color("document-canvas").unwrap()) >= 7.);
    }
    let field = host
        .get_mut(ComponentId(1))
        .unwrap()
        .component_mut::<TextFieldContent>()
        .unwrap();
    field.field.error = None;
    field.surface.input.disabled = true;
    let scene = render_host(&mut host, &mut text, theme);
    let frame = host.semantics()[0].frame;
    assert_input(
        &scene,
        frame,
        palette,
        (false, true, false),
        "disabled-text",
        text.backing,
    );
    assert!(!has_ring(&scene, frame, palette));
    assert!(host.semantics()[0].semantics.disabled);
    assert!(
        !host
            .event(&ContentEvent::Commit {
                text: "blocked".into(),
                replacement: None
            })
            .changed
    );
    assert_eq!(
        host.semantics()[0].semantics.value.as_deref(),
        Some("renamed")
    );
    host.get_mut(ComponentId(1))
        .unwrap()
        .component_mut::<TextFieldContent>()
        .unwrap()
        .surface
        .input
        .disabled = false;
    host.focus(Some(ComponentId(1)), true);
    assert!(
        host.event(&ContentEvent::Commit {
            text: "ready".into(),
            replacement: Some(0..7)
        })
        .changed
    );
    render_host(&mut host, &mut text, theme);
    assert_eq!(
        host.semantics()[0].semantics.value.as_deref(),
        Some("ready")
    );
}
