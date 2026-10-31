//! Original Tabs panel slots containing the original controlled Disclosure/editor.
//! CPU/API + CoreText only; the icon lease is not SVG/GPU/native/physical IME evidence.
#[path = "support/coretext.rs"]
mod coretext;
use std::{ops::Range, rc::Rc, time::Duration};
use tekes_ui_controls::{
    ButtonFramePolicy, ControlKey,
    composition::*,
    extended::{Axis, Stack},
    layout_view::IconRenderer,
    paint::ButtonVariant,
};
use tekes_ui_scene::{ImageId, Primitive, Rect, Scene};
use tekes_ui_tokens::{Rgba, Theme};
const TABS: ComponentId = ComponentId(38_100);
const PANEL: ComponentId = ComponentId(38_101);
const DISCLOSURE: ComponentId = ComponentId(38_102);
const TRIGGER: ComponentId = ComponentId(38_103);
const BODY: ComponentId = ComponentId(38_104);
const EDITOR: ComponentId = ComponentId(38_105);
const ACTION: ComponentId = ComponentId(38_106);
#[derive(Default)]
struct Icons(usize);
impl IconRenderer for Icons {
    fn prepare_icon(
        &mut self,
        _: &str,
        _: f32,
        _: f32,
        _: Rgba,
    ) -> Result<IconImage, IconRenderError> {
        self.0 += 1;
        Ok(IconImage::new(ImageId(99), Rc::new(())))
    }
    fn draw_icon(&mut self, _: &mut Scene, _: &str, _: Rect, _: Rgba) {
        unreachable!("original checked icon-resource seam")
    }
}
fn stack(children: Vec<Content>) -> StackContent {
    StackContent {
        model: Stack {
            axis: Axis::Vertical,
            gap: 8.,
        },
        flexible_child: None,
        children,
    }
}
fn fixture() -> (ContentHost, coretext::Text, Icons) {
    let mut action = Button::new("Original body action");
    action.variant = ButtonVariant::Primary;
    action.button.frame_policy = ButtonFramePolicy::Exact;
    let disclosure = DisclosureContent::new(
        TRIGGER,
        Content::new(
            ComponentId(38_107),
            TextContent::new("Disclosure in original panel"),
        ),
        Content::new(
            BODY,
            stack(vec![
                // Sample the leading retained control, not a trailing child
                // that can legitimately be outside the shrinking reveal clip.
                Content::new(ACTION, action),
                Content::new(
                    EDITOR,
                    TextFieldContent::new("Retained editor", "A😀B draft"),
                ),
            ]),
        ),
        true,
    );
    let mut h = ContentHost::new(Content::new(
        TABS,
        TabsContent::new(
            vec![
                TabItem::new(
                    Content::new(ComponentId(38_108), TextContent::new("First")),
                    Content::new(PANEL, stack(vec![Content::new(DISCLOSURE, disclosure)])),
                ),
                TabItem::new(
                    Content::new(ComponentId(38_109), TextContent::new("Second")),
                    Content::new(
                        ComponentId(38_110),
                        TextContent::new("Other original panel"),
                    ),
                ),
            ],
            0,
        ),
    ))
    .unwrap();
    h.set_action_events(true);
    let mut text = coretext::Text::new(1.);
    text.services = true;
    text.reduced_motion = false;
    (h, text, Icons::default())
}
fn render(h: &mut ContentHost, t: &mut coretext::Text, icons: &mut Icons) -> Scene {
    icons.0 = 0;
    t.atlas.begin_frame();
    let mut scene = Scene::new();
    h.render(
        &mut scene,
        &mut WithIconRenderer::new(t, icons),
        Rect::new(12., 12., 460., 340.),
        Rect::new(0., 0., 484., 364.),
        Theme::LIGHT.into(),
    )
    .unwrap();
    scene
}
fn disclosure(h: &mut ContentHost) -> &mut DisclosureContent {
    h.get_mut(DISCLOSURE).unwrap().component_mut().unwrap()
}
fn select(h: &mut ContentHost, index: usize) -> bool {
    h.get_mut(TABS)
        .unwrap()
        .component_mut::<TabsContent>()
        .unwrap()
        .select(index)
}
fn draft(h: &ContentHost) -> (String, Range<usize>, Option<Range<usize>>) {
    let input = &h
        .get(EDITOR)
        .unwrap()
        .component::<TextFieldContent>()
        .unwrap()
        .surface
        .input;
    (input.text.clone(), input.selection(), input.marked.clone())
}
fn mounts(h: &ContentHost) -> Vec<u64> {
    [PANEL, DISCLOSURE, TRIGGER, BODY, EDITOR, ACTION]
        .iter()
        .map(|id| h.get(*id).unwrap().mount_identity())
        .collect()
}
fn body_retired(h: &mut ContentHost) {
    for id in [EDITOR, ACTION] {
        assert!(!h.focus_targets().iter().any(|(target, _)| *target == id));
        assert!(!h.semantics().iter().any(|node| node.id == id));
        assert!(!h.focus(Some(id), true));
        assert!(
            !h.accessibility_action(id, None, AccessibilityAction::Press)
                .activated
        );
    }
    assert!(h.semantic_text_input_snapshot(EDITOR).is_none());
    for down in [true, false] {
        h.event(&ContentEvent::Key {
            key: ControlKey::Tab { reverse: false },
            down,
        });
        assert!(
            !h.focused().is_some_and(|id| [EDITOR, ACTION].contains(&id)),
            "actual Tab routing must exclude the retired body"
        );
    }
}
fn request(h: &mut ContentHost) -> DisclosureIntent {
    assert!(
        h.accessibility_action(TRIGGER, None, AccessibilityAction::Press)
            .activated
    );
    let records = h.drain_action_records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].action.target, TRIGGER);
    assert_eq!(records[0].action.source, ContentActionSource::Accessibility);
    let Some(ContentActionPayload::DisclosureExpansion(intent)) = &records[0].payload else {
        panic!("original controlled Disclosure intent required")
    };
    intent.clone()
}
fn paints_action(scene: &Scene, frame: Rect) -> bool {
    let mut clips = vec![Some(Rect::new(0., 0., 484., 364.))];
    let mut painted = false;
    for primitive in scene.primitives() {
        match primitive {
            Primitive::PushClip(rect) => clips.push(
                clips
                    .last()
                    .copied()
                    .flatten()
                    .and_then(|clip| clip.intersection(rect)),
            ),
            Primitive::PopClip => {
                assert!(clips.len() > 1);
                clips.pop();
            }
            Primitive::Quad(quad) if quad.rect == frame && quad.fill.a >= 0.999 => {
                painted |= clips
                    .last()
                    .copied()
                    .flatten()
                    .and_then(|clip| clip.intersection(&frame))
                    .is_some();
            }
            _ => {}
        }
    }
    assert_eq!(clips.len(), 1);
    painted
}

#[test]
fn selected_closing_body_retires_input_tab_ax_and_inactive_panel_cannot_repaint_it() {
    let (mut h, mut t, mut icons) = fixture();
    let expanded = render(&mut h, &mut t, &mut icons);
    let original = mounts(&h);
    let saved = draft(&h);
    let action = h.get(ACTION).unwrap().frame();
    assert!(
        paints_action(&expanded, action),
        "original leading action paints before close"
    );
    assert!(h.focus(Some(EDITOR), true));
    assert!(disclosure(&mut h).set_expanded(false));
    t.now = Duration::from_millis(10);
    render(&mut h, &mut t, &mut icons);
    t.now = Duration::from_millis(40);
    let closing = render(&mut h, &mut t, &mut icons);
    let reveal = disclosure(&mut h).body_layout();
    assert!(reveal.progress > 0. && reveal.progress < 1. && !reveal.interactive);
    assert_eq!(
        h.get(ACTION).unwrap().frame(),
        action,
        "sampled original control must not move"
    );
    assert!(
        reveal.visible_clip.unwrap().intersection(&action).is_some(),
        "leading sample must actually intersect reveal: action={action:?}, reveal={reveal:?}"
    );
    assert!(
        paints_action(&closing, action),
        "positive oracle: selected closing body still paints; action={action:?}, reveal={reveal:?}"
    );
    body_retired(&mut h);
    assert_ne!(h.focused(), Some(EDITOR));
    assert!(
        !h.event(&ContentEvent::PointerDown(action.center()))
            .capture_pointer
    );
    assert!(!h.event(&ContentEvent::PointerUp(action.center())).activated);
    assert!(h.drain_action_records().is_empty());
    assert!(select(&mut h, 1));
    let hidden = render(&mut h, &mut t, &mut icons);
    assert!(
        !paints_action(&hidden, action),
        "Tabs inactivity overrides retained Disclosure paint"
    );
    assert_eq!(
        icons.0, 0,
        "inactive Disclosure must not demand icon services"
    );
    assert!(
        !h.semantics()
            .iter()
            .any(|n| [PANEL, DISCLOSURE, TRIGGER, BODY].contains(&n.id))
    );
    body_retired(&mut h);
    // Actual Tab routing may already restore the retained roving stop.
    // select() reports a change, not success; verify both the exact return
    // contract and the desired selected panel instead of assuming a change.
    let selected_before = h
        .get(TABS)
        .unwrap()
        .component::<TabsContent>()
        .unwrap()
        .selected();
    assert_eq!(select(&mut h, 0), selected_before != 0);
    assert_eq!(
        h.get(TABS)
            .unwrap()
            .component::<TabsContent>()
            .unwrap()
            .selected(),
        0
    );
    t.now = Duration::from_millis(500);
    render(&mut h, &mut t, &mut icons);
    assert!(!disclosure(&mut h).expanded());
    assert_eq!(disclosure(&mut h).body_layout().progress, 0.);
    body_retired(&mut h);
    assert_eq!(mounts(&h), original);
    assert_eq!(draft(&h), saved);
    assert!(h.drain_action_records().is_empty());
}
#[test]
fn marked_editor_blocks_both_hides_then_panel_switch_retires_capture_without_losing_draft() {
    let (mut h, mut t, mut icons) = fixture();
    render(&mut h, &mut t, &mut icons);
    let original = mounts(&h);
    assert!(h.focus(Some(EDITOR), true));
    h.event(&ContentEvent::Preedit {
        text: "ni".into(),
        selection: 1..1,
        clauses: std::iter::once(0..2).collect(),
        replacement: Some(0..1),
    });
    let marked = draft(&h);
    assert_eq!(marked, ("ni😀B draft".into(), 1..1, Some(0..2)));
    assert!(!select(&mut h, 1));
    assert!(!disclosure(&mut h).set_expanded(false));
    assert_eq!(h.focused(), Some(EDITOR));
    assert_eq!(draft(&h), marked);
    assert_eq!(mounts(&h), original);
    h.event(&ContentEvent::Unmark);
    let unmarked = draft(&h);
    assert_eq!(unmarked, ("ni😀B draft".into(), 1..1, None));
    let point = h.get(ACTION).unwrap().frame().center();
    assert!(h.event(&ContentEvent::PointerDown(point)).capture_pointer);
    assert_eq!(h.pointer_capture().unwrap().0, ACTION);
    assert!(select(&mut h, 1));
    render(&mut h, &mut t, &mut icons);
    assert!(h.pointer_capture().is_none());
    body_retired(&mut h);
    assert!(!h.focus(Some(TRIGGER), true));
    assert!(
        !h.accessibility_action(TRIGGER, None, AccessibilityAction::Press)
            .activated
    );
    assert!(!h.event(&ContentEvent::PointerUp(point)).activated);
    h.event(&ContentEvent::Commit {
        text: "must not enter hidden draft".into(),
        replacement: None,
    });
    assert_eq!(draft(&h), unmarked);
    assert!(h.drain_action_records().is_empty());
    assert!(select(&mut h, 0));
    render(&mut h, &mut t, &mut icons);
    assert!(disclosure(&mut h).expanded());
    assert_eq!(mounts(&h), original);
    assert_eq!(draft(&h), unmarked);
    assert!(h.focus_targets().iter().any(|(id, _)| *id == EDITOR));
    assert!(
        !h.event(&ContentEvent::PointerUp(
            h.get(ACTION).unwrap().frame().center()
        ))
        .activated
    );
    assert!(h.drain_action_records().is_empty());
    assert!(
        h.accessibility_action(ACTION, None, AccessibilityAction::Press)
            .activated
    );
    let fresh = h.drain_action_records();
    assert_eq!(fresh.len(), 1);
    assert_eq!(fresh[0].action.target, ACTION);
    assert_eq!(fresh[0].action.source, ContentActionSource::Accessibility);
}
#[test]
fn inactive_panel_retires_pending_disclosure_intent_and_reselection_needs_fresh_generation() {
    let (mut h, mut t, mut icons) = fixture();
    render(&mut h, &mut t, &mut icons);
    let original = mounts(&h);
    let old = request(&mut h);
    assert!(!old.expanded());
    assert!(select(&mut h, 1));
    render(&mut h, &mut t, &mut icons);
    assert!(!disclosure(&mut h).confirm(&old, true));
    assert!(disclosure(&mut h).expanded());
    assert!(h.pointer_capture().is_none());
    assert!(!h.focus(Some(TRIGGER), true));
    assert!(
        !h.accessibility_action(TRIGGER, None, AccessibilityAction::Press)
            .activated
    );
    h.event(&ContentEvent::Key {
        key: ControlKey::Space,
        down: false,
    });
    assert!(h.drain_action_records().is_empty());
    assert!(select(&mut h, 0));
    render(&mut h, &mut t, &mut icons);
    assert_eq!(mounts(&h), original);
    assert!(!disclosure(&mut h).confirm(&old, true));
    let fresh = request(&mut h);
    assert_ne!(fresh.generation(), old.generation());
    assert!(disclosure(&mut h).confirm(&fresh, true));
    assert!(!disclosure(&mut h).expanded());
    body_retired(&mut h);
    assert!(h.drain_action_records().is_empty());
}
