//! Independent public API: original Host, one retained editor, no private View access.
use std::{cell::Cell, rc::Rc};
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlKey, TextSurfaceHost, TextSurfaceTextRenderer,
    composition::*,
    extended::{Axis, Stack},
    layout::MeasuredChild,
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_text::{
    coretext::{CoreTextFonts, CoreTextShaper},
    model::{Paragraph, ShapeOptions},
    style::TextStyle,
    traits::Shaper,
};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal};
struct Text {
    shaper: CoreTextShaper,
    labels: Vec<String>,
}
impl Text {
    fn new() -> Self {
        Self {
            shaper: CoreTextShaper::new(CoreTextFonts::new()),
            labels: Vec::new(),
        }
    }
}
impl ButtonTextRenderer for Text {
    fn text_surface_host(&mut self) -> Option<&mut dyn tekes_ui_controls::TextSurfaceHost> {
        Some(self)
    }
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        let p = self.shaper.shape(
            s,
            &TextStyle::system(r.style().size).with_weight(r.style().weight),
            &ShapeOptions {
                width: Some(w.into()),
                line_spacing: 0.,
            },
        );
        Size::new(p.width as f32, p.height as f32)
    }
    fn draw(
        &mut self,
        _: &mut Scene,
        s: &str,
        _: proposal::TextRole,
        _: Rect,
        _: ButtonTextAlign,
        _: Rgba,
    ) {
        self.labels.push(s.into());
    }
}
impl TextSurfaceHost for Text {
    fn shape_surface(&mut self, s: &str, r: proposal::TextRole, w: Option<f32>) -> Rc<Paragraph> {
        Rc::new(self.shaper.shape(
            s,
            &TextStyle::system(r.style().size).with_weight(r.style().weight),
            &ShapeOptions {
                width: w.map(f64::from),
                line_spacing: 0.,
            },
        ))
    }
}
impl TextSurfaceTextRenderer for Text {
    fn push_visible(&mut self, _: &mut Scene, _: &Paragraph, _: Point, _: Rgba, _: Rect) {}
}
const COMBO: ComponentId = ComponentId(9090);
fn items() -> Vec<String> {
    ["", "Compact", "Compact", "Comfortable"]
        .map(String::from)
        .to_vec()
}
fn model(h: &ContentHost) -> &ComboBoxContent {
    h.get(COMBO)
        .unwrap()
        .component::<ComboBoxContent>()
        .unwrap()
}
fn model_mut(h: &mut ContentHost) -> &mut ComboBoxContent {
    h.get_mut(COMBO)
        .unwrap()
        .component_mut::<ComboBoxContent>()
        .unwrap()
}
fn render(h: &mut ContentHost, width: f32, palette: Palette) {
    h.render(
        &mut Scene::new(),
        &mut Text::new(),
        Rect::new(0., 0., width, 450.),
        Rect::new(0., 0., width, 450.),
        palette,
    )
    .unwrap();
}
fn key(h: &mut ContentHost, key: ControlKey) -> ContentResponse {
    h.event(&ContentEvent::Key { key, down: true })
}
fn host() -> ContentHost {
    let mut h =
        ContentHost::new(Content::new(COMBO, ComboBoxContent::new("Layout", items()))).unwrap();
    h.set_action_events(true);
    render(&mut h, 400., Theme::LIGHT.into());
    assert!(h.focus(Some(COMBO), true));
    h
}
#[test]
fn public_editor_confirm_duplicate_index_payload_and_once_survive_close() {
    let mut h = host();
    assert!(
        h.event(&ContentEvent::Commit {
            text: "Com".into(),
            replacement: None
        })
        .consumed
    );
    assert_eq!(model(&h).model().filtered(), vec![1, 2, 3]);
    assert_eq!(model(&h).model().active, Some(1));
    key(&mut h, ControlKey::Down);
    assert_eq!(model(&h).model().active, Some(2));
    assert!(key(&mut h, ControlKey::Enter).activated);
    assert_eq!(model(&h).model().selected, Some(2));
    assert_eq!(h.focused_text().unwrap().input.text, "Compact");
    assert!(!model(&h).model().open);
    let records = h.drain_action_records();
    assert_eq!(records.len(), 1);
    let ContentActionPayload::ComboBoxSelection(selected) = records[0].payload.as_ref().unwrap()
    else {
        panic!("selection payload")
    };
    assert_eq!(selected.index(), 2);
    assert_eq!(selected.label(), "Compact");
    assert_eq!(records[0].action.kind, ContentActionKind::Activate);
    assert!(h.drain_action_records().is_empty());
    assert!(
        !h.event(&ContentEvent::Key {
            key: ControlKey::Enter,
            down: false
        })
        .activated
    );
    assert!(key(&mut h, ControlKey::Enter).consumed);
    assert!(model(&h).model().open);
    key(&mut h, ControlKey::Down);
    assert!(key(&mut h, ControlKey::Enter).activated);
    assert_eq!(h.drain_action_records().len(), 1);
    assert!(h.drain_actions().is_empty());
}
#[test]
fn public_marked_updates_retain_the_same_editor_selection_and_undo() {
    let mut h = host();
    h.event(&ContentEvent::Commit {
        text: "Com".into(),
        replacement: None,
    });
    h.event(&ContentEvent::Preedit {
        text: "候補😀".into(),
        selection: 1..3,
        replacement: Some(0..3),
        clauses: vec![0..2, 2..4],
    });
    let baseline = h.focused_text().unwrap().input.external_baseline();
    let clauses = h.focused_text().unwrap().clauses.clone();
    assert!(
        model_mut(&mut h)
            .set_items(vec!["".into(), "候補😀".into(), "候補😀".into()])
            .unwrap()
    );
    render(&mut h, 220., Theme::DARK.into());
    assert_eq!(
        h.focused_text().unwrap().input.external_baseline(),
        baseline
    );
    assert_eq!(h.focused_text().unwrap().clauses, clauses);
    assert_eq!(h.focused(), Some(COMBO));
    assert!(!key(&mut h, ControlKey::Enter).activated);
    assert!(!key(&mut h, ControlKey::Tab { reverse: false }).consumed);
    assert_eq!(
        h.focused_text().unwrap().input.external_baseline(),
        baseline
    );
    assert_eq!(
        model_mut(&mut h).set_query("replace"),
        Err(ComboBoxUpdateError::Composing)
    );
    assert!(key(&mut h, ControlKey::Escape).consumed);
    assert_eq!(h.focused_text().unwrap().input.text, "Com");
    assert!(h.focused_text().unwrap().input.marked.is_none());
    assert!(model(&h).model().open);
    assert!(key(&mut h, ControlKey::Escape).consumed);
    assert!(!model(&h).model().open);
    assert!(h.drain_actions().is_empty());
    assert!(h.event(&ContentEvent::TextCommand("undo:".into())).consumed);
    assert_eq!(h.focused_text().unwrap().input.text, "");
    assert!(h.event(&ContentEvent::TextCommand("redo:".into())).consumed);
    assert_eq!(h.focused_text().unwrap().input.text, "Com");
}
#[test]
fn public_program_replace_revokes_queued_selection_without_rewriting_raw_index() {
    let mut h = host();
    model_mut(&mut h).set_query("Com").unwrap();
    assert!(key(&mut h, ControlKey::Enter).activated);
    model_mut(&mut h).set_items(vec![]).unwrap();
    assert_eq!(model(&h).model().selected, Some(1));
    assert!(model(&h).model().items.is_empty());
    assert!(h.drain_action_records().is_empty());
    assert_eq!(h.focused_text().unwrap().input.text, "Compact");
    assert!(model_mut(&mut h).set_items(items()).unwrap());
    model_mut(&mut h).set_query("Com").unwrap();
    assert!(key(&mut h, ControlKey::Enter).activated);
    model_mut(&mut h).set_query("Compact").unwrap();
    assert!(h.drain_action_records().is_empty());
}
#[derive(Debug)]
struct Background {
    events: Rc<Cell<usize>>,
}
impl Component for Background {
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(a.width, 35.))
    }
    fn paint(
        &mut self,
        _: &mut Scene,
        _: &mut dyn ButtonTextRenderer,
        _: Rect,
        _: Rect,
        _: ContentTheme,
    ) {
    }
    fn event(&mut self, e: &ContentEvent) -> ContentResponse {
        if matches!(e, ContentEvent::PointerDown(_) | ContentEvent::PointerUp(_)) {
            self.events.set(self.events.get() + 1);
            return ContentResponse {
                activated: true,
                ..ContentResponse::consumed(true)
            };
        }
        ContentResponse::default()
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::Button, "Background")
    }
    fn focusable(&self) -> bool {
        true
    }
}
#[test]
fn real_host_outside_dismissal_owns_down_and_up_without_background_delivery() {
    let count = Rc::new(Cell::new(0));
    let mut h = ContentHost::new(Content::new(
        ComponentId(9099),
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 0.,
            },
            flexible_child: None,
            children: vec![
                Content::new(COMBO, ComboBoxContent::new("Layout", items())),
                Content::new(
                    ComponentId(9091),
                    Background {
                        events: count.clone(),
                    },
                ),
            ],
        },
    ))
    .unwrap();
    h.set_action_events(true);
    render(&mut h, 400., Theme::LIGHT.into());
    h.focus(Some(COMBO), true);
    // Empty candidates still have the original logical overlay; no phantom row.
    model_mut(&mut h)
        .set_query("No matching candidate")
        .unwrap();
    render(&mut h, 400., Theme::LIGHT.into());
    let p = h
        .semantics()
        .iter()
        .find(|n| n.id == ComponentId(9091))
        .unwrap()
        .frame
        .center();
    assert!(h.event(&ContentEvent::PointerDown(p)).consumed);
    assert!(!model(&h).model().open);
    assert_eq!(h.pointer_capture().map(|v| v.0), Some(COMBO));
    assert!(h.event(&ContentEvent::PointerUp(p)).consumed);
    assert!(h.pointer_capture().is_none());
    assert_eq!(count.get(), 0);
    assert!(h.drain_actions().is_empty());
}
#[test]
fn actual_admission_confirms_ax_candidate_and_replacement_parts_never_reuse_tokens() {
    let mut h = host();
    model_mut(&mut h).set_query("Com").unwrap();
    render(&mut h, 400., Theme::LIGHT.into());
    let old = h
        .semantics()
        .into_iter()
        .find(|n| n.id == COMBO)
        .unwrap()
        .parts;
    assert!(old.len() > 1);
    let old_token = old[1].index;
    assert!(
        h.accessibility_action(COMBO, Some(old_token), AccessibilityAction::Press)
            .activated
    );
    assert_eq!(h.drain_action_records().len(), 1);
    model_mut(&mut h).set_query("Com").unwrap();
    model_mut(&mut h).set_items(vec!["new".into()]).unwrap();
    model_mut(&mut h).set_items(items()).unwrap();
    render(&mut h, 400., Theme::LIGHT.into());
    let parts = h
        .semantics()
        .into_iter()
        .find(|n| n.id == COMBO)
        .unwrap()
        .parts;
    assert!(
        parts
            .iter()
            .all(|p| !old.iter().any(|old| old.index == p.index))
    );
    assert!(
        !h.accessibility_action(COMBO, Some(old_token), AccessibilityAction::Press)
            .activated
    );
    assert_eq!(model(&h).model().selected, Some(1));
}
#[test]
fn passive_background_program_updates_do_not_acquire_scope_or_end_neighbor_marked() {
    const COMPOSER: ComponentId = ComponentId(9092);
    let mut h = ContentHost::new(Content::new(
        ComponentId(9099),
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 0.,
            },
            flexible_child: None,
            children: vec![
                Content::new(COMBO, ComboBoxContent::new("Layout", items())),
                Content::new(COMPOSER, TextFieldContent::new("Composer", "draft")),
            ],
        },
    ))
    .unwrap();
    h.set_action_events(true);
    render(&mut h, 400., Theme::LIGHT.into());
    assert!(h.focus(Some(COMPOSER), true));
    h.event(&ContentEvent::Preedit {
        text: "候補".into(),
        selection: 1..1,
        replacement: Some(0..5),
        clauses: std::iter::once(0..2).collect(),
    });
    let before = h.focused_text().unwrap().input.external_baseline();
    model_mut(&mut h)
        .set_items(vec!["".into(), "Com".into(), "Com".into()])
        .unwrap();
    model_mut(&mut h).set_query("Com").unwrap();
    render(&mut h, 260., Theme::DARK.into());
    assert_eq!(h.focused(), Some(COMPOSER));
    assert_eq!(h.focused_text().unwrap().input.external_baseline(), before);
    assert!(
        model(&h).model().open,
        "original set_query policy remains logical open"
    );
    let semantics = h.semantics();
    let combo = semantics.iter().find(|n| n.id == COMBO).unwrap();
    assert_eq!(combo.semantics.expanded, Some(false));
    assert!(combo.parts.is_empty());
    assert!(h.drain_action_records().is_empty());
}
#[test]
fn original_menu_pointer_confirmation_and_replaced_capture_release_are_once() {
    let mut h = host();
    model_mut(&mut h).set_query("Com").unwrap();
    render(&mut h, 400., Theme::LIGHT.into());
    let row = h
        .semantics()
        .into_iter()
        .find(|n| n.id == COMBO)
        .unwrap()
        .parts[2]
        .frame
        .center();
    assert!(h.event(&ContentEvent::PointerDown(row)).consumed);
    assert_eq!(h.pointer_capture().map(|v| v.0), Some(COMBO));
    assert!(h.event(&ContentEvent::PointerUp(row)).activated);
    assert_eq!(model(&h).model().selected, Some(2));
    assert_eq!(h.focused_text().unwrap().input.text, "Compact");
    assert_eq!(h.drain_action_records().len(), 1);
    assert!(h.drain_actions().is_empty());
    model_mut(&mut h).set_query("Com").unwrap();
    render(&mut h, 400., Theme::LIGHT.into());
    let row = h
        .semantics()
        .into_iter()
        .find(|n| n.id == COMBO)
        .unwrap()
        .parts[1]
        .frame
        .center();
    h.event(&ContentEvent::PointerDown(row));
    model_mut(&mut h)
        .set_items(vec!["Com changed".into()])
        .unwrap();
    assert!(
        h.pointer_capture().is_none(),
        "original Host capture authorization is revoked immediately"
    );
    render(&mut h, 400., Theme::LIGHT.into());
    assert!(!h.event(&ContentEvent::PointerUp(row)).activated);
    assert!(h.pointer_capture().is_none());
    assert_eq!(model(&h).model().selected, Some(2));
    assert!(h.drain_action_records().is_empty());
    let part = h
        .semantics()
        .into_iter()
        .find(|n| n.id == COMBO)
        .unwrap()
        .parts[1]
        .index;
    assert!(
        h.accessibility_action(COMBO, Some(part), AccessibilityAction::Press)
            .activated
    );
    assert_eq!(h.drain_action_records().len(), 1);
}
#[test]
fn accepted_host_sources_reject_ax_before_effect_and_allow_original_pointer_transfer() {
    const OTHER: ComponentId = ComponentId(9092);
    let mut h = ContentHost::new(Content::new(
        ComponentId(9099),
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 8.,
            },
            flexible_child: None,
            children: vec![
                Content::new(COMBO, ComboBoxContent::new("Layout", items())),
                Content::new(OTHER, TextFieldContent::new("Composer", "draft")),
            ],
        },
    ))
    .unwrap();
    h.set_action_events(true);
    render(&mut h, 400., Theme::LIGHT.into());
    assert!(h.focus(Some(OTHER), true));
    h.event(&ContentEvent::Preedit {
        text: "候補".into(),
        selection: 1..1,
        replacement: Some(0..5),
        clauses: std::iter::once(0..2).collect(),
    });
    let before = h.focused_text().unwrap().input.external_baseline();
    assert!(
        !h.accessibility_action(COMBO, None, AccessibilityAction::SetExpanded(true))
            .consumed
    );
    assert_eq!(h.focused(), Some(OTHER));
    assert_eq!(h.focused_text().unwrap().input.external_baseline(), before);
    assert!(!model(&h).model().open);
    assert!(h.drain_actions().is_empty());
    let p = model(&h).text_surface().inner.center();
    assert!(h.event(&ContentEvent::PointerDown(p)).consumed);
    assert_eq!(h.focused(), Some(COMBO));
    assert!(model(&h).model().open);
    assert!(h.text_surface(OTHER).unwrap().input.marked.is_none());
    h.event(&ContentEvent::PointerUp(p));
    h.event(&ContentEvent::Commit {
        text: "Com".into(),
        replacement: None,
    });
    render(&mut h, 400., Theme::LIGHT.into());
    let nodes = tekes_ui_platform_macos::content_accessibility::content_nodes(&h);
    assert!(
        nodes
            .iter()
            .any(|n| n.role == tekes_ui_platform_macos::AccessibilityRole::ComboBox)
    );
    assert!(!tekes_ui_platform_macos::content_accessibility::content_text_layouts(&h).is_empty());
    assert!(h.drain_actions().is_empty());
    key(&mut h, ControlKey::Tab { reverse: false });
    assert_eq!(h.focused(), Some(OTHER));
    assert!(!model(&h).model().open);
}
#[test]
fn editor_drag_retains_original_capture_and_popup_wheel_does_not_scroll_reader() {
    let mut combo =
        ComboBoxContent::new("Layout", (0..24).map(|i| format!("Item {i:02}")).collect());
    combo.set_query("").unwrap();
    let stack = Content::new(
        ComponentId(9080),
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 0.,
            },
            flexible_child: None,
            children: vec![
                Content::new(COMBO, combo),
                Content::new(
                    ComponentId(9081),
                    TextContent::new("Reader history\n".repeat(80)),
                ),
            ],
        },
    );
    let mut scroll = ScrollContent::new(tekes_ui_controls::ScrollArea::default(), stack);
    scroll.viewport_height = Some(160.);
    let mut h = ContentHost::new(Content::new(ComponentId(9082), scroll)).unwrap();
    h.set_action_events(true);
    render(&mut h, 400., Theme::LIGHT.into());
    h.focus(Some(COMBO), true);
    model_mut(&mut h).set_query("").unwrap();
    render(&mut h, 400., Theme::LIGHT.into());
    let point = model(&h).text_surface().inner.center();
    h.event(&ContentEvent::PointerDown(point));
    assert_eq!(h.pointer_capture().map(|p| p.0), Some(COMBO));
    let editor = model(&h).text_surface().outer;
    h.event(&ContentEvent::PointerMove(Point::new(
        editor.max_x() - 2.,
        editor.max_y() + 10.,
    )));
    assert_eq!(h.pointer_capture().map(|p| p.0), Some(COMBO));
    h.event(&ContentEvent::PointerUp(point));
    assert!(h.pointer_capture().is_none());
    render(&mut h, 400., Theme::LIGHT.into());
    let popup = model(&h).popup_layout().unwrap().visible.unwrap().center();
    let before = h
        .get(ComponentId(9082))
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .scroll
        .offset;
    assert!(
        h.event(&ContentEvent::Scroll {
            position: popup,
            delta: Point::new(0., 32.),
            precise: true,
            begin: true,
            end: false,
            momentum: false,
            time: 1.
        })
        .consumed
    );
    assert_eq!(
        h.get(ComponentId(9082))
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .scroll
            .offset,
        before
    );
    assert!(h.drain_actions().is_empty());
}
#[test]
fn empty_label_and_repeated_same_value_confirmations_are_distinct_once_events() {
    let mut h = host();
    model_mut(&mut h).set_query("").unwrap();
    assert!(key(&mut h, ControlKey::Enter).activated);
    assert_eq!(model(&h).model().selected, Some(0));
    assert_eq!(h.focused_text().unwrap().input.text, "");
    assert!(
        !h.event(&ContentEvent::Key {
            key: ControlKey::Enter,
            down: false
        })
        .activated
    );
    assert!(!key(&mut h, ControlKey::Enter).activated);
    assert!(key(&mut h, ControlKey::Enter).activated);
    let records = h.drain_action_records();
    assert_eq!(records.len(), 2);
    for r in records {
        let Some(ContentActionPayload::ComboBoxSelection(s)) = r.payload else {
            panic!("original selection")
        };
        assert_eq!(s.index(), 0);
        assert_eq!(s.label(), "");
    }
    assert!(h.drain_actions().is_empty());
}
