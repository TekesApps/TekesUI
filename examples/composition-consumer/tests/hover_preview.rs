#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_composition_consumer::hover_preview::*;
use tekes_ui_controls::{ControlKey, composition::*};
use tekes_ui_scene::{Rect, Scene};
use tekes_ui_tokens::Theme;
fn render(fixture: &mut Transcript) {
    let mut text = coretext::Text::new(1.);
    text.services = true;
    fixture
        .host
        .render(
            &mut Scene::new(),
            &mut text,
            Rect::new(0., 0., 420., 320.),
            Rect::new(0., 0., 420., 320.),
            Theme::LIGHT.into(),
        )
        .unwrap();
}
fn hover(fixture: &mut Transcript, id: ComponentId) {
    let point = fixture.host.get(id).unwrap().frame().center();
    fixture.host.event(&ContentEvent::PointerMove(point));
    render(fixture);
}
#[test]
fn hover_preview_independent_consumer_two_layers_stream_and_original_marked_composer() {
    let mut fixture = Transcript::new();
    render(&mut fixture);
    fixture.host.focus(Some(COMPOSER), true);
    fixture.host.event(&ContentEvent::Preedit {
        text: "中文".into(),
        selection: 1..1,
        clauses: vec![],
        replacement: None,
    });
    let before = {
        let input = &fixture
            .host
            .get(COMPOSER)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap()
            .surface
            .input;
        (input.text.clone(), input.marked.clone(), input.selection())
    };
    hover(&mut fixture, TICK_A);
    assert!(
        fixture
            .host
            .get(PREVIEW)
            .unwrap()
            .component::<HoverPreviewContent>()
            .unwrap()
            .is_open()
    );
    let old = fixture.version;
    fixture
        .update("Input A updated", "Output A streamed twice")
        .unwrap();
    render(&mut fixture);
    let semantics = fixture.host.semantics();
    assert!(
        semantics
            .iter()
            .any(|node| node.id == INPUT && node.semantics.name == "Input A updated")
    );
    assert!(
        semantics
            .iter()
            .any(|node| node.id == OUTPUT && node.semantics.name == "Output A streamed twice")
    );
    assert_eq!(
        fixture
            .host
            .update_hover_preview(old, content("old", "old")),
        Err(HoverPreviewError::StaleVersion)
    );
    assert_eq!(fixture.host.focused(), Some(COMPOSER));
    let input = &fixture
        .host
        .get(COMPOSER)
        .unwrap()
        .component::<TextFieldContent>()
        .unwrap()
        .surface
        .input;
    assert_eq!(
        (input.text.clone(), input.marked.clone(), input.selection()),
        before
    );
    fixture.host.event(&ContentEvent::Key {
        key: ControlKey::Escape,
        down: true,
    });
    assert!(
        !fixture
            .host
            .get(PREVIEW)
            .unwrap()
            .component::<HoverPreviewContent>()
            .unwrap()
            .is_open()
    );
}
#[test]
fn hover_preview_independent_consumer_rebinding_and_remount_reject_old_turn() {
    let mut fixture = Transcript::new();
    render(&mut fixture);
    hover(&mut fixture, TICK_A);
    let old = fixture.version;
    fixture.bind_b().unwrap();
    fixture.update("Input B", "Output B").unwrap();
    render(&mut fixture);
    hover(&mut fixture, TICK_B);
    assert_eq!(
        fixture
            .host
            .get(PREVIEW)
            .unwrap()
            .component::<HoverPreviewContent>()
            .unwrap()
            .version()
            .unwrap()
            .owner(),
        TICK_B
    );
    assert_eq!(
        fixture
            .host
            .update_hover_preview(old, content("A late", "A late")),
        Err(HoverPreviewError::StaleVersion)
    );
    let row = fixture.host.remove(TICK_B).unwrap();
    fixture.host.insert(ROOT, row).unwrap();
    render(&mut fixture);
    hover(&mut fixture, TICK_B);
    assert!(
        !fixture
            .host
            .get(PREVIEW)
            .unwrap()
            .component::<HoverPreviewContent>()
            .unwrap()
            .is_open()
    );
}
