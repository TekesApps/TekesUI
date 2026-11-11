use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer,
    composition::{
        AccessibilityAction, Component, ComponentId, Content, ContentEvent, ContentHost,
        ListContent, SelectableListContent,
    },
};
use tekes_ui_controls::{ControlKey, extended::List};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal};

struct ListText;
impl ButtonTextRenderer for ListText {
    fn measure(&mut self, value: &str, _: proposal::TextRole, width: f32) -> Size {
        Size::new((value.chars().count() as f32 * 8.).min(width), 24.)
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

const ROOT: ComponentId = ComponentId(9000);
const ITEMS: ComponentId = ComponentId(9001);
const WRITING: ComponentId = ComponentId(9002);

fn scrollable_legacy_list() -> ContentHost {
    let mut writing = SelectableListContent::new("Writing", vec!["First".into(), "Second".into()]);
    writing.model.selected = Some(0);
    ContentHost::new(Content::new(
        ROOT,
        ListContent::scrollable("Workspace", ITEMS, vec![Content::new(WRITING, writing)]),
    ))
    .unwrap()
}

fn second_row(host: &ContentHost) -> Rect {
    host.get(WRITING)
        .unwrap()
        .component::<SelectableListContent>()
        .unwrap()
        .semantic_parts()
        .into_iter()
        .find(|part| part.index == 1)
        .unwrap()
        .frame
}

fn selected(host: &ContentHost) -> Option<usize> {
    host.get(WRITING)
        .unwrap()
        .component::<SelectableListContent>()
        .unwrap()
        .model
        .selected
}

fn render(host: &mut ContentHost, text: &mut ListText) {
    let frame = Rect::new(0., 0., 360., 420.);
    host.render(
        &mut Scene::new(),
        text,
        frame,
        frame,
        Palette::from(Theme::LIGHT),
    )
    .unwrap();
}
#[test]
fn independent_public_consumer_preserves_literal_destructure_new_and_legacy_navigation() {
    let mut literal = List {
        selected: Some(1),
        count: 3,
    };
    let copy = literal.clone();
    let List { selected, count } = copy;
    assert_eq!((selected, count), (Some(1), 3));
    assert!(literal.key(ControlKey::End));
    assert_eq!(
        literal,
        List {
            selected: Some(2),
            count: 3
        }
    );
    assert!(!literal.select(3));
    literal.count = 0;
    literal.selected = None;
    assert!(!literal.key(ControlKey::Home));
    let mut fresh = List::new(2);
    assert!(fresh.key(ControlKey::Down));
    assert_eq!(fresh.selected, Some(0));
}

#[test]
fn legacy_selectable_list_fields_and_constructor_remain_public() {
    use tekes_ui_controls::{collections_view::ListView, composition::SelectableListContent};

    let literal = SelectableListContent {
        model: List {
            selected: Some(0),
            count: 1,
        },
        view: ListView::new(),
        labels: vec!["One".into()],
        name: "Legacy".into(),
    };
    assert_eq!(literal.model.selected, Some(0));
    assert_eq!(literal.labels, ["One"]);
    let constructed = SelectableListContent::new("New", vec!["Two".into()]);
    assert_eq!(constructed.model.count, 1);
    assert_eq!(constructed.name, "New");
}

#[test]
fn legacy_selectable_list_inside_scroll_accepts_pointer() {
    let mut pointer_host = scrollable_legacy_list();
    pointer_host.set_action_events(true);
    let mut text = ListText;
    render(&mut pointer_host, &mut text);
    let target = second_row(&pointer_host).center();
    assert!(
        pointer_host
            .event(&ContentEvent::PointerDown(target))
            .consumed
    );
    assert_eq!(selected(&pointer_host), Some(1));
    pointer_host.event(&ContentEvent::PointerUp(target));
}

#[test]
fn legacy_selectable_list_inside_scroll_accepts_ax_press() {
    let mut ax_host = scrollable_legacy_list();
    ax_host.set_action_events(true);
    let mut text = ListText;
    render(&mut ax_host, &mut text);
    let response = ax_host.accessibility_action(WRITING, Some(1), AccessibilityAction::Press);
    assert!(
        response.consumed,
        "the actual visible ListItem Press is admissible"
    );
    assert_eq!(selected(&ax_host), Some(1));
}
