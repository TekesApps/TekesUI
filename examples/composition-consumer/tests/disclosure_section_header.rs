//! Public section-header consumption; CPU programmatic evidence, not GPU/native.
#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use std::rc::Rc;
use tekes_ui_controls::{ScrollArea, composition::*, layout_view::IconRenderer};
use tekes_ui_scene::{ImageId, Rect, Scene};
use tekes_ui_text::{LabelOptions, LabelOverflow};
use tekes_ui_tokens::{Rgba, Theme};
const ROOT: ComponentId = ComponentId(1);
const TRIGGER: ComponentId = ComponentId(2);
const TITLE: ComponentId = ComponentId(3);
const BODY: ComponentId = ComponentId(4);
const SOURCE: &str = "原始 Summary 标题与完整来源 retained title repeated repeated repeated repeated repeated repeated repeated repeated repeated";
#[derive(Default)]
struct Icons(Vec<String>);
impl IconRenderer for Icons {
    fn prepare_icon(
        &mut self,
        name: &str,
        _: f32,
        _: f32,
        _: Rgba,
    ) -> Result<IconImage, IconRenderError> {
        self.0.push(name.into());
        Ok(IconImage::new(ImageId(99), Rc::new(())))
    }
    fn draw_icon(&mut self, _: &mut Scene, _: &str, _: Rect, _: Rgba) {
        panic!("prepared icon service required")
    }
}
fn host(focusable: bool, section: bool) -> ContentHost {
    let title = Content::new(TITLE, TextContent::new(SOURCE))
        .with_label_layout(LabelOptions::single_line(LabelOverflow::TailEllipsis))
        .unwrap();
    let mut disclosure = DisclosureContent::new(
        TRIGGER,
        title,
        Content::new(BODY, TextFieldContent::new("Composer", "original draft")),
        true,
    );
    if !focusable {
        disclosure = disclosure.with_trigger_focusable(false);
    }
    if section {
        disclosure = disclosure.with_section_header_layout();
    }
    let mut host = ContentHost::new(Content::new(
        ComponentId(10),
        ScrollContent::new(ScrollArea::default(), Content::new(ROOT, disclosure)),
    ))
    .unwrap();
    host.set_action_events(true);
    host
}
fn render(
    h: &mut ContentHost,
    text: &mut coretext::Text,
    icons: &mut Icons,
    width: f32,
    clip: Rect,
    theme: Theme,
) {
    text.atlas.begin_frame();
    h.render(
        &mut Scene::new(),
        &mut WithIconRenderer::new(text, icons),
        Rect::new(0., 0., width, 300.),
        clip,
        theme.into(),
    )
    .unwrap();
}
fn text(scale: f32) -> coretext::Text {
    let mut t = coretext::Text::new(scale);
    t.services = true;
    t
}
fn intent(h: &mut ContentHost) -> DisclosureIntent {
    let records = h.drain_action_records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].action.target, TRIGGER);
    assert_eq!(records[0].mount, h.get(TRIGGER).unwrap().mount_identity());
    match &records[0].payload {
        Some(ContentActionPayload::DisclosureExpansion(intent)) => intent.clone(),
        other => panic!("original Disclosure intent required: {other:?}"),
    }
}
#[test]
fn nonfocus_trigger_refuses_focus_and_tab_but_press_keeps_original_marked_editor() {
    for section in [false, true] {
        let mut h = host(false, section);
        let mut t = text(1.);
        let mut icons = Icons::default();
        let frame = Rect::new(0., 0., 400., 300.);
        render(&mut h, &mut t, &mut icons, 400., frame, Theme::LIGHT);
        let mounts = (
            h.get(TRIGGER).unwrap().mount_identity(),
            h.get(BODY).unwrap().mount_identity(),
        );
        assert!(!h.focus_targets().iter().any(|(id, _)| *id == TRIGGER));
        assert!(h.enter_focus(false));
        assert_eq!(h.focused(), Some(BODY));
        assert!(!h.focus(Some(TRIGGER), true));
        let response = h.accessibility_action(TRIGGER, None, AccessibilityAction::Focus);
        assert!(!response.activated);
        assert_eq!(h.focused(), Some(BODY));
        h.event(&ContentEvent::Preedit {
            text: "ni".into(),
            selection: 2..2,
            clauses: std::iter::once(0..2).collect(),
            replacement: None,
        });
        render(&mut h, &mut t, &mut icons, 400., frame, Theme::LIGHT);
        let before = h.text_input_snapshot();
        assert!(before.as_ref().unwrap().marked.is_some());
        let point = h.get(TRIGGER).unwrap().frame().center();
        assert!(h.event(&ContentEvent::PointerDown(point)).capture_pointer);
        assert!(!h.pointer_focus_requested());
        let response = h.event(&ContentEvent::PointerUp(point));
        assert!(response.consumed && response.activated, "{response:?}");
        let request = intent(&mut h);
        let d = h
            .get_mut(ROOT)
            .unwrap()
            .component_mut::<DisclosureContent>()
            .unwrap();
        assert!(d.expanded());
        assert!(
            !d.confirm(&request, true),
            "marked body must refuse collapse"
        );
        assert!(d.confirm(&request, false));
        assert!(!d.confirm(&request, true));
        let response = h.accessibility_action(TRIGGER, None, AccessibilityAction::Press);
        assert!(response.consumed && response.activated, "{response:?}");
        let request = intent(&mut h);
        assert!(
            h.get_mut(ROOT)
                .unwrap()
                .component_mut::<DisclosureContent>()
                .unwrap()
                .confirm(&request, false)
        );
        assert_eq!(h.focused(), Some(BODY));
        assert_eq!(h.text_input_snapshot(), before);
        assert_eq!(
            (
                h.get(TRIGGER).unwrap().mount_identity(),
                h.get(BODY).unwrap().mount_identity()
            ),
            mounts
        );
        assert!(h.drain_action_records().is_empty());
    }
}
#[test]
fn default_focus_policy_remains_independent_of_section_title_layout() {
    for section in [false, true] {
        let mut h = host(true, section);
        let mut t = text(1.);
        let mut icons = Icons::default();
        let frame = Rect::new(0., 0., 400., 300.);
        render(&mut h, &mut t, &mut icons, 400., frame, Theme::LIGHT);
        assert!(h.focus_targets().iter().any(|(id, _)| *id == TRIGGER));
        assert!(h.enter_focus(false));
        assert_eq!(h.focused(), Some(TRIGGER));
        assert!(h.focus(Some(BODY), true));
        h.accessibility_action(TRIGGER, None, AccessibilityAction::Focus);
        assert_eq!(h.focused(), Some(TRIGGER));
        assert!(h.drain_action_records().is_empty());
    }
}
#[test]
fn section_title_uses_original_fixed_insets_positive_increasing_width_and_one_ax_owner() {
    for theme in [Theme::LIGHT, Theme::DARK] {
        for scale in [0.85, 1., 1.5] {
            for focusable in [false, true] {
                let mut h = host(focusable, true);
                let mut t = text(scale);
                let mut icons = Icons::default();
                let mut previous = 0.;
                let mounts = (
                    h.get(TRIGGER).unwrap().mount_identity(),
                    h.get(TITLE).unwrap().mount_identity(),
                );
                for width in [96., 240., 420.] {
                    let frame = Rect::new(0., 0., width, 300.);
                    render(&mut h, &mut t, &mut icons, width, frame, theme);
                    let header = h.get(TRIGGER).unwrap().frame();
                    let title = h.get(TITLE).unwrap().frame();
                    eprintln!(
                        "theme={theme:?} scale={scale} width={width} header={header:?} title={title:?} focused={:?}",
                        h.focused()
                    );
                    assert!((title.min_x() - header.min_x() - 16.).abs() < 0.01);
                    assert!((title.size.width - (width - 32.)).abs() < 0.01);
                    assert!(title.size.width > previous);
                    previous = title.size.width;
                    assert_eq!(
                        h.get(TITLE)
                            .unwrap()
                            .component::<TextContent>()
                            .unwrap()
                            .text,
                        SOURCE
                    );
                    let semantics = h.semantics();
                    assert_eq!(
                        semantics
                            .iter()
                            .filter(|n| n.semantics.name == SOURCE)
                            .count(),
                        1
                    );
                    assert_eq!(
                        semantics
                            .iter()
                            .find(|n| n.semantics.name == SOURCE)
                            .unwrap()
                            .id,
                        TRIGGER
                    );
                    assert_eq!(
                        (
                            h.get(TRIGGER).unwrap().mount_identity(),
                            h.get(TITLE).unwrap().mount_identity()
                        ),
                        mounts
                    );
                    assert!(
                        icons.0.is_empty(),
                        "historical plain Summary title has no indicator"
                    );
                }
            }
        }
    }
}
#[test]
fn default_sidebar_keeps_leading_indicator_and_nonfocus_does_not_hide_it() {
    for focusable in [false, true] {
        let mut h = host(focusable, false);
        let mut t = text(1.);
        let mut icons = Icons::default();
        let frame = Rect::new(0., 0., 400., 300.);
        render(&mut h, &mut t, &mut icons, 400., frame, Theme::LIGHT);
        assert_eq!(icons.0, ["tekes-expand"]);
        assert!(
            h.get(TITLE).unwrap().frame().min_x() > h.get(TRIGGER).unwrap().frame().min_x() + 16.
        );
        assert!(
            h.get_mut(ROOT)
                .unwrap()
                .component_mut::<DisclosureContent>()
                .unwrap()
                .set_expanded(false)
        );
        render(&mut h, &mut t, &mut icons, 400., frame, Theme::LIGHT);
        assert_eq!(icons.0.last().unwrap(), "tekes-forward");
    }
}
#[test]
fn section_arrange_and_paint_share_the_same_passive_title_frame() {
    let mut h = host(false, true);
    let mut t = text(1.5);
    let mut icons = Icons::default();
    let frame = Rect::new(0., 0., 96., 300.);
    render(&mut h, &mut t, &mut icons, 96., frame, Theme::LIGHT);
    let painted = h.get(TITLE).unwrap().frame();
    let root = h.get(ROOT).unwrap().frame();
    h.get_mut(ROOT).unwrap().arrange(&mut t, root, frame);
    assert_eq!(h.get(TITLE).unwrap().frame(), painted);
    assert!(icons.0.is_empty());
}
#[test]
fn nonfocus_section_keeps_unarranged_clipped_disabled_and_window_refusal() {
    let mut h = host(false, true);
    assert!(
        !h.accessibility_action(TRIGGER, None, AccessibilityAction::Press)
            .activated
    );
    let mut t = text(1.);
    let mut icons = Icons::default();
    let frame = Rect::new(0., 0., 400., 300.);
    render(
        &mut h,
        &mut t,
        &mut icons,
        400.,
        Rect::new(0., 200., 400., 100.),
        Theme::LIGHT,
    );
    assert!(
        !h.accessibility_action(TRIGGER, None, AccessibilityAction::Press)
            .activated
    );
    render(&mut h, &mut t, &mut icons, 400., frame, Theme::LIGHT);
    h.get_mut(ROOT)
        .unwrap()
        .component_mut::<DisclosureContent>()
        .unwrap()
        .set_disabled(true);
    render(&mut h, &mut t, &mut icons, 400., frame, Theme::LIGHT);
    assert!(
        !h.accessibility_action(TRIGGER, None, AccessibilityAction::Press)
            .activated
    );
    let d = h
        .get_mut(ROOT)
        .unwrap()
        .component_mut::<DisclosureContent>()
        .unwrap();
    d.set_disabled(false);
    d.set_window_active(false);
    d.set_window_active(true);
    assert!(
        !h.accessibility_action(TRIGGER, None, AccessibilityAction::Press)
            .activated
    );
    assert!(h.drain_action_records().is_empty());
    render(&mut h, &mut t, &mut icons, 400., frame, Theme::LIGHT);
    assert!(
        h.accessibility_action(TRIGGER, None, AccessibilityAction::Press)
            .activated
    );
    let request = intent(&mut h);
    assert!(
        h.get_mut(ROOT)
            .unwrap()
            .component_mut::<DisclosureContent>()
            .unwrap()
            .confirm(&request, false)
    );
    assert!(!h.focus_targets().iter().any(|(id, _)| *id == TRIGGER));
}
