#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_composition_consumer::combo_status::{QUERY, Scenario};
use tekes_ui_controls::{
    ControlKey, composition::*, extended::ComboBox, layout_view::ComboBoxView,
};
use tekes_ui_scene::{Primitive, Rect, Scene};
use tekes_ui_tokens::{Palette, Theme, proposal};
fn draw(s: &mut Scenario, text: &mut coretext::Text, theme: Theme) -> Scene {
    let mut scene = Scene::new();
    s.render(
        &mut scene,
        text,
        Rect::new(24., 24., 400., 440.),
        Rect::new(0., 0., 520., 560.),
        theme.into(),
    )
    .unwrap();
    assert!(scene.composition_error().is_none());
    scene
}
fn assert_ink(scene: &Scene, frame: Rect, theme: Theme, ground_role: &str, backing: f32) {
    let p = Palette::from(theme);
    let (index, ground) = scene
        .primitives()
        .iter()
        .enumerate()
        .find_map(|(i, p)| match p {
            Primitive::Quad(q) if q.rect == frame => Some((i, q)),
            _ => None,
        })
        .expect("actual input/candidate own ground");
    assert_eq!(ground.fill, p.color(ground_role).unwrap());
    let inks: Vec<_> = scene.primitives()[index + 1..]
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
                Some(r.color)
            }
            _ => None,
        })
        .collect();
    assert!(
        !inks.is_empty(),
        "actual CoreText query/candidate glyphs after own ground"
    );
    for ink in inks {
        assert_eq!(ink, p.color("primary-text").unwrap());
        assert_eq!((ink.a, ground.fill.a), (1., 1.));
        let luminance = |c: tekes_ui_tokens::Rgba| {
            let c = c.to_linear();
            0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
        };
        let (a, b) = (luminance(ink), luminance(ground.fill));
        assert!(
            (a.max(b) + 0.05) / (a.min(b) + 0.05) >= if theme.is_increased() { 7. } else { 4.5 }
        );
    }
}
#[test]
fn combo_box_retained_public_composition_keeps_query_source_indices_and_real_commit() {
    let mut s = Scenario::new(
        ["Alpha", "Beta", "Bravo", "Gamma"]
            .map(str::to_owned)
            .to_vec(),
    )
    .unwrap();
    s.host.set_action_events(true);
    let mut text = coretext::Text::new(1.);
    text.services = true;
    draw(&mut s, &mut text, Theme::LIGHT);
    assert!(s.host.focus(Some(QUERY), true));
    assert!(
        s.host
            .event(&ContentEvent::Commit {
                text: "B".into(),
                replacement: None
            })
            .changed
    );
    draw(&mut s, &mut text, Theme::LIGHT);
    s.host.event(&ContentEvent::Key {
        key: ControlKey::Down,
        down: true,
    });
    let mut geometry = None;
    let mut choice = None;
    for theme in Theme::ALL {
        let scene = draw(&mut s, &mut text, theme);
        assert!(s.open());
        assert_eq!(s.query(), "B");
        assert_eq!(s.active(), Some(2));
        assert_eq!(s.selected(), None);
        let input = s.host.focused_text().unwrap().outer;
        let node = s
            .host
            .semantics()
            .into_iter()
            .find(|n| n.id == QUERY)
            .unwrap();
        assert_eq!(node.semantics.expanded, Some(true));
        assert_eq!(
            node.parts
                .iter()
                .map(|p| (p.index, p.semantics.name.as_str()))
                .collect::<Vec<_>>(),
            vec![(1, "Beta"), (2, "Bravo")]
        );
        let row = node.parts.iter().find(|p| p.index == 2).unwrap();
        assert!(row.focused && !row.semantics.selected);
        let current = (
            input,
            s.popup_frame().unwrap(),
            node.parts.iter().map(|p| p.frame).collect::<Vec<_>>(),
        );
        if let Some(original) = &geometry {
            assert_eq!(&current, original);
        } else {
            geometry = Some(current);
        }
        assert_ink(&scene, input, theme, "transcript-input", text.backing);
        assert_ink(&scene, row.frame, theme, "row-selection", text.backing);
        choice = Some(row.frame.center());
        assert!(
            s.host.drain_action_records().is_empty(),
            "Theme/query/navigation does not confirm a choice"
        );
    }
    let point = choice.unwrap();
    assert!(
        !s.host.event(&ContentEvent::PointerUp(point)).activated,
        "release without press"
    );
    s.host.event(&ContentEvent::PointerDown(point));
    assert!(s.host.event(&ContentEvent::PointerUp(point)).activated);
    assert_eq!(s.selected(), Some("Bravo"));
    assert_eq!(s.query(), "Bravo");
    assert!(!s.open());
    let actions = s.host.drain_action_records();
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].action.target, QUERY);
    assert_eq!(actions[0].action.source, ContentActionSource::Pointer);
    draw(&mut s, &mut text, Theme::DARK_INCREASED);
    assert!(!s.is_presented());
    s.host
        .event(&ContentEvent::TextCommand("selectAll:".into()));
    s.host.event(&ContentEvent::Commit {
        text: "B".into(),
        replacement: None,
    });
    draw(&mut s, &mut text, Theme::DARK_INCREASED);
    s.host.event(&ContentEvent::Key {
        key: ControlKey::Down,
        down: true,
    });
    assert!(
        s.host
            .event(&ContentEvent::Key {
                key: ControlKey::Enter,
                down: true
            })
            .activated
    );
    assert_eq!(s.selected(), Some("Bravo"));
    assert_eq!(
        s.host.drain_action_records().len(),
        1,
        "confirming the same original value remains an event"
    );
    s.host
        .event(&ContentEvent::TextCommand("selectAll:".into()));
    s.host.event(&ContentEvent::Commit {
        text: "B".into(),
        replacement: None,
    });
    draw(&mut s, &mut text, Theme::DARK_INCREASED);
    s.host.event(&ContentEvent::Key {
        key: ControlKey::Escape,
        down: true,
    });
    assert_eq!(s.query(), "B");
    assert_eq!(s.selected(), Some("Bravo"));
    assert!(!s.open());
    assert!(s.host.drain_action_records().is_empty());
}
#[test]
fn combo_box_original_view_cancels_disabled_arm_and_rejects_empty_snapshot() {
    let mut view = ComboBoxView::default();
    view.options.set_reduced_motion(true);
    let mut model = ComboBox {
        items: vec!["Alpha".into(), "Beta".into()],
        ..Default::default()
    };
    model.set_query("B");
    let mut text = coretext::Text::new(1.);
    text.services = true;
    let mut paint = |view: &mut ComboBoxView, model: &mut ComboBox| {
        view.render(
            &mut Scene::new(),
            &mut text,
            model,
            Rect::new(24., 24., 240., 44.),
            Rect::new(0., 0., 520., 500.),
            None,
            Theme::DARK_INCREASED.into(),
            proposal::TextRole::UiBody,
            false,
        )
    };
    let popup = paint(&mut view, &mut model).unwrap();
    assert_eq!(view.options.original_index(0), Some(1));
    view.pointer_down(popup.items[0].frame.center());
    model.disabled = true;
    assert!(paint(&mut view, &mut model).is_none());
    assert!(!model.open && !view.options.is_presented());
    assert_eq!(
        view.pointer_up(popup.items[0].frame.center(), &mut model),
        None
    );
    assert_eq!(model.selected, None);
    model.disabled = false;
    model.set_query("absent");
    assert!(paint(&mut view, &mut model).is_none());
    assert!(model.open);
    assert_eq!(model.active, None);
    assert!(!view.focus_option(1, &mut model));
    assert_eq!(view.activate_option(1, &mut model), None);
    model.set_query("B");
    paint(&mut view, &mut model);
    assert_eq!(view.activate_option(1, &mut model), Some(1));
    assert_eq!(model.selected, Some(1));
    assert_eq!(model.query, "Beta");
}
