#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use coretext::Text;
use tekes_ui_controls::{ControlKey, ToolbarControl, composition::*};
use tekes_ui_scene::{Point, Rect, Scene};
use tekes_ui_tokens::Theme;

const VIEW: Rect = Rect::new(0., 0., 800., 600.);
const ROOT: ComponentId = ComponentId(1);
const REGION: ComponentId = ComponentId(2);
const GROUP: ComponentId = ComponentId(10);
const COMMANDS: ComponentId = ComponentId(11);
const MENU: ComponentId = ComponentId(12);
#[derive(Clone, Debug, PartialEq, Eq)]
struct Source(u64);

fn text() -> Text {
    let mut text = Text::new(1.);
    text.services = true;
    text
}
fn build(region_frame: Rect, axis: tekes_ui_controls::extended::Axis) -> ToolbarHost<Source> {
    let mut host = ToolbarHost::new(ROOT, 41, VIEW).unwrap();
    let mut region = ToolbarRegion::new(REGION, region_frame);
    region.axis = axis;
    host.set_regions(41, VIEW, vec![region]).unwrap();
    let actions = ["Alpha", "Beta", "Gamma", "Delta"];
    let declarations = actions
        .iter()
        .enumerate()
        .map(|(index, title)| ToolbarButtonDeclaration {
            id: index as u64 + 1,
            icon: None,
            text: (*title).into(),
            hint: None,
            enabled: true,
        })
        .collect();
    let bindings = actions
        .iter()
        .enumerate()
        .map(|(index, _)| ToolbarCommandBinding {
            target: COMMANDS,
            action: Some(index as u64 + 1),
            command: 100 + index as u64,
            source: Source(index as u64),
        })
        .collect();
    let mut commands = toolbar_commands(COMMANDS, declarations, ToolbarGroupOverflow::Split);
    commands
        .component_mut::<ToolbarControl>()
        .unwrap()
        .set_axis(axis);
    host.register(ToolbarRegistration {
        region: REGION,
        order: 0,
        group: Content::new(
            GROUP,
            ToolbarGroup::new(ToolbarPlacement::Navigation, vec![commands]),
        ),
        bindings,
    })
    .unwrap();
    host
}
fn draw(host: &mut ToolbarHost<Source>, text: &mut Text) {
    host.render(&mut Scene::new(), text, Theme::LIGHT.into())
        .unwrap();
}
fn part(host: &ToolbarHost<Source>, name: &str) -> (ToolbarAccessibilityTarget, Rect, ContentRole) {
    part_in(host, COMMANDS, name)
}
fn part_in(
    host: &ToolbarHost<Source>,
    owner: ComponentId,
    name: &str,
) -> (ToolbarAccessibilityTarget, Rect, ContentRole) {
    host.semantics()
        .into_iter()
        .find(|node| node.node.id == owner)
        .unwrap()
        .parts
        .into_iter()
        .find(|(_, part)| part.semantics.name == name)
        .map(|(target, part)| (target, part.frame, part.semantics.role))
        .unwrap_or_else(|| panic!("missing toolbar part {name}"))
}
fn assert_single(host: &mut ToolbarHost<Source>, command: u64) {
    let found = host.drain_commands();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].command, command);
    assert_eq!(found[0].source, Source(command - 100));
    assert!(host.drain_commands().is_empty());
}

#[test]
fn vertical_region_stacks_original_commands_and_opens_side_menu_with_one_action() {
    for (x, opens_right) in [(12., true), (728., false)] {
        let region = Rect::new(x, 20., 60., VIEW.size.height);
        let mut host = build(region, tekes_ui_controls::extended::Axis::Vertical);
        let mut text = text();
        draw(&mut host, &mut text);
        let control = host
            .content_host()
            .get(COMMANDS)
            .unwrap()
            .component::<ToolbarControl>()
            .unwrap();
        let natural_height = control.measure(&mut text).height;
        let (_, measured_action, _) = part(&host, "Alpha");
        let allocation_height = natural_height - measured_action.size.height;
        assert!(allocation_height < natural_height);
        let region = Rect::new(x, 20., 60., allocation_height);
        let mut allocation = ToolbarRegion::new(REGION, region);
        allocation.axis = tekes_ui_controls::extended::Axis::Vertical;
        host.set_regions(41, VIEW, vec![allocation]).unwrap();
        draw(&mut host, &mut text);
        let (_, alpha, role) = part(&host, "Alpha");
        let (_, beta, _) = part(&host, "Beta");
        assert_eq!(role, ContentRole::Button);
        assert!(
            beta.min_y() > alpha.min_y(),
            "vertical items must follow model order"
        );
        assert!(
            (alpha.center().x - beta.center().x).abs() < 2.,
            "upright buttons share the cross-axis center"
        );
        let (more, more_frame, more_role) = part(&host, "More options");
        assert_eq!(more_role, ContentRole::MenuButton);
        assert!(
            more_frame.min_y() >= beta.max_y(),
            "More occupies its own bottom hit target"
        );
        assert!(
            host.accessibility_action(more, AccessibilityAction::Press)
                .consumed,
            "the original MenuButton consumes AX Press to open its menu"
        );
        draw(&mut host, &mut text);
        let (_, first_hidden, item_role) = part(&host, "Gamma");
        assert_eq!(item_role, ContentRole::MenuItem);
        if opens_right {
            assert!(
                first_hidden.min_x() >= more_frame.max_x(),
                "leading toolbar popup opens inward to the right"
            );
        } else {
            assert!(
                first_hidden.max_x() <= more_frame.min_x(),
                "trailing toolbar popup opens inward to the left"
            );
        }
        let (item, _, _) = part(&host, "Gamma");
        assert!(
            host.accessibility_action(item, AccessibilityAction::Press)
                .activated
        );
        assert_single(&mut host, 102);
    }
}

#[test]
fn vertical_arrow_navigation_and_axis_change_retire_old_pointer_capture() {
    let mut host = build(
        Rect::new(12., 20., 68., 300.),
        tekes_ui_controls::extended::Axis::Vertical,
    );
    let mut text = text();
    draw(&mut host, &mut text);
    host.event(&ContentEvent::Key {
        key: ControlKey::Tab { reverse: false },
        down: true,
    });
    host.event(&ContentEvent::Key {
        key: ControlKey::Down,
        down: true,
    });
    host.event(&ContentEvent::Key {
        key: ControlKey::Enter,
        down: true,
    });
    assert_single(&mut host, 101);
    host.event(&ContentEvent::Key {
        key: ControlKey::Enter,
        down: false,
    });
    host.event(&ContentEvent::Key {
        key: ControlKey::Up,
        down: true,
    });
    host.event(&ContentEvent::Key {
        key: ControlKey::Enter,
        down: true,
    });
    host.event(&ContentEvent::Key {
        key: ControlKey::Enter,
        down: false,
    });
    assert_single(&mut host, 100);

    draw(&mut host, &mut text);
    let (_, alpha, _) = part(&host, "Alpha");
    assert!(
        host.event(&ContentEvent::PointerDown(alpha.center()))
            .capture_pointer
    );
    let mut horizontal = ToolbarRegion::new(REGION, Rect::new(12., 20., 68., 300.));
    horizontal.axis = tekes_ui_controls::extended::Axis::Horizontal;
    host.set_regions(41, VIEW, vec![horizontal]).unwrap();
    assert!(
        host.content_host().pointer_capture().is_none(),
        "axis mutation retires the original capture before the next paint"
    );
    draw(&mut host, &mut text);
    host.event(&ContentEvent::PointerUp(alpha.center()));
    assert!(
        host.drain_commands().is_empty(),
        "axis change retires the old pointer capture"
    );
}

#[test]
fn toolbar_menu_rows_escape_bar_clip_but_remain_hittable_and_region_bounded() {
    for axis in [
        tekes_ui_controls::extended::Axis::Horizontal,
        tekes_ui_controls::extended::Axis::Vertical,
    ] {
        let (x, y, initial_width) = if axis == tekes_ui_controls::extended::Axis::Horizontal {
            (100., 180., VIEW.size.width - 200.)
        } else {
            (12., 20., 60.)
        };
        let initial_region = Rect::new(x, y, initial_width, VIEW.size.height);
        let mut host = build(initial_region, axis);
        let mut text = text();
        draw(&mut host, &mut text);
        let control = host
            .content_host()
            .get(COMMANDS)
            .unwrap()
            .component::<ToolbarControl>()
            .unwrap();
        let natural = control.measure(&mut text);
        let row_height = control.row_height(&mut text);
        let (_, measured_action, _) = part(&host, "Alpha");
        let region = if axis == tekes_ui_controls::extended::Axis::Horizontal {
            Rect::new(x, y, natural.width - measured_action.size.width, row_height)
        } else {
            Rect::new(
                x,
                y,
                initial_width,
                natural.height - measured_action.size.height,
            )
        };
        let mut allocation = ToolbarRegion::new(REGION, region);
        allocation.axis = axis;
        host.set_regions(41, VIEW, vec![allocation]).unwrap();
        draw(&mut host, &mut text);
        let (more, _, role) = part(&host, "More options");
        assert_eq!(role, ContentRole::MenuButton);
        assert!(
            host.accessibility_action(more, AccessibilityAction::Press)
                .consumed
        );
        draw(&mut host, &mut text);
        let (_, delta_frame, role) = part(&host, "Delta");
        assert_eq!(role, ContentRole::MenuItem);
        assert!(
            delta_frame.size.height >= 30.,
            "the second popup row keeps its full menu hit frame outside the toolbar clip: {axis:?} {delta_frame:?}"
        );
        if axis == tekes_ui_controls::extended::Axis::Horizontal {
            assert!(
                delta_frame.max_y() > region.max_y() || delta_frame.min_y() < region.min_y(),
                "horizontal popup row lies outside the toolbar row"
            );
        } else {
            assert!(
                delta_frame.max_x() > region.max_x() || delta_frame.min_x() < region.min_x(),
                "vertical popup row lies beside the toolbar column"
            );
        }
        host.event(&ContentEvent::PointerDown(delta_frame.center()));
        host.event(&ContentEvent::PointerUp(delta_frame.center()));
        assert_single(&mut host, 103);

        let outside_bar = Point::new(region.max_x() + 1., region.center().y);
        host.event(&ContentEvent::PointerDown(outside_bar));
        host.event(&ContentEvent::PointerUp(outside_bar));
        assert!(
            host.drain_commands().is_empty(),
            "ordinary toolbar hits remain bounded by the allocated region"
        );
    }
}

#[test]
fn original_menu_content_popup_rows_use_host_clip_in_horizontal_and_vertical_regions() {
    for (axis, x, y, width) in [
        (
            tekes_ui_controls::extended::Axis::Horizontal,
            100.,
            180.,
            240.,
        ),
        (tekes_ui_controls::extended::Axis::Vertical, 12., 20., 60.),
    ] {
        let initial_region = Rect::new(x, y, width, VIEW.size.height);
        let mut host = ToolbarHost::new(ROOT, 41, VIEW).unwrap();
        let mut allocation = ToolbarRegion::new(REGION, initial_region);
        allocation.axis = axis;
        host.set_regions(41, VIEW, vec![allocation]).unwrap();
        host.register(ToolbarRegistration {
            region: REGION,
            order: 0,
            group: Content::new(
                GROUP,
                ToolbarGroup::new(
                    ToolbarPlacement::Navigation,
                    vec![Content::new(
                        MENU,
                        MenuContent::new("Documents", vec!["First".into(), "Second".into()]),
                    )],
                ),
            ),
            bindings: vec![],
        })
        .unwrap();
        let mut text = text();
        draw(&mut host, &mut text);
        let toolbar_height = if axis == tekes_ui_controls::extended::Axis::Horizontal {
            host.content_host().get(REGION).unwrap().frame().size.height
        } else {
            host.content_host().get(MENU).unwrap().frame().size.height
        };
        let region = Rect::new(x, y, width, toolbar_height);
        let mut allocation = ToolbarRegion::new(REGION, region);
        allocation.axis = axis;
        host.set_regions(41, VIEW, vec![allocation]).unwrap();
        draw(&mut host, &mut text);
        let owner = host
            .semantics()
            .into_iter()
            .find(|node| node.node.id == MENU)
            .unwrap()
            .target;
        assert!(
            host.accessibility_action(owner, AccessibilityAction::Press)
                .consumed
        );
        draw(&mut host, &mut text);
        let (_, second, role) = part_in(&host, MENU, "Second");
        assert_eq!(role, ContentRole::MenuItem);
        assert!(
            second.size.height >= 30.,
            "MenuContent row is clipped by its toolbar ancestor in {axis:?}: {second:?}"
        );
        let second_visible = host
            .content_host()
            .get(MENU)
            .unwrap()
            .component::<MenuContent>()
            .unwrap()
            .popup
            .as_ref()
            .unwrap()
            .items[1]
            .visible;
        assert!(
            second_visible.is_some_and(|visible| visible.size.height >= 30.),
            "host viewport must expose the full row outside the toolbar clip in {axis:?}: {second_visible:?}"
        );
        assert!(
            second.max_y() > region.max_y() || second.min_y() < region.min_y(),
            "the popup extends past the toolbar allocation"
        );
        host.event(&ContentEvent::PointerDown(second.center()));
        let released = host.event(&ContentEvent::PointerUp(second.center()));
        assert!(
            released.activated,
            "pointer release activates the visible menu row"
        );
        assert_eq!(
            host.content_host()
                .get(MENU)
                .unwrap()
                .component::<MenuContent>()
                .unwrap()
                .selected,
            Some(1)
        );
        assert!(host.drain_commands().is_empty());

        let outside = Point::new(region.max_x() + 1., region.center().y);
        host.event(&ContentEvent::PointerDown(outside));
        host.event(&ContentEvent::PointerUp(outside));
        assert!(host.drain_commands().is_empty());
    }
}
