use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer, composition::*};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text;
impl ButtonTextRenderer for Text {
    fn text_surface_host(&mut self) -> Option<&mut dyn tekes_ui_controls::TextSurfaceHost> {
        Some(self)
    }
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        Size::new((s.len() as f32 * 8.).min(w), r.style().line_height)
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
impl tekes_ui_controls::TextSurfaceHost for Text {
    fn shape_surface(
        &mut self,
        text: &str,
        role: proposal::TextRole,
        width: Option<f32>,
    ) -> std::rc::Rc<tekes_ui_text::model::Paragraph> {
        use tekes_ui_text::traits::Shaper;
        let mut shaper = tekes_ui_text::coretext::CoreTextShaper::new(
            tekes_ui_text::coretext::CoreTextFonts::new(),
        );
        std::rc::Rc::new(shaper.shape(
            text,
            &tekes_ui_text::style::TextStyle::system(role.style().size),
            &tekes_ui_text::model::ShapeOptions {
                width: width.map(f64::from),
                line_spacing: 0.,
            },
        ))
    }
}
impl tekes_ui_controls::TextSurfaceTextRenderer for Text {
    fn push_visible(
        &mut self,
        _: &mut Scene,
        _: &tekes_ui_text::model::Paragraph,
        _: tekes_ui_scene::Point,
        _: Rgba,
        _: Rect,
    ) {
    }
}

use tekes_ui_controls::{
    ControlKey, ScrollArea, TextSurface,
    extended::{Axis, Pane, Stack},
};
const BUDGET: usize = 64 * 1024 * 1024;
fn id(n: u64) -> ComponentId {
    ComponentId(n)
}
fn split(leading: Content, trailing: Content) -> Content {
    Content::new(
        id(1),
        SplitViewContent::try_new(SplitViewConstruction {
            config: SplitViewConfig::default(),
            slots: SplitViewSlotIds {
                leading: id(2),
                trailing: id(3),
                divider: id(4),
            },
            leading,
            trailing,
            budget: BUDGET,
        })
        .unwrap(),
    )
}
fn render(h: &mut ContentHost, w: f32, hgt: f32) {
    let frame = Rect::new(0., 0., w, hgt);
    h.render_checked(
        &mut Scene::new(),
        &mut Text,
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .unwrap();
}
fn composer(n: u64) -> Content {
    let mut field = TextFieldContent::new("Message", "");
    field.surface = TextSurface::new("draft", true, false);
    field.viewport = TextViewport::Grow {
        min: 56.,
        max: 120.,
    };
    field.submit_policy = TextSubmitPolicy::CommandEnter;
    Content::new(id(n), field)
}
fn request(h: &ContentHost, pane: Pane, replacement: Content) -> SplitViewReplacement {
    SplitViewReplacement {
        owner: id(1),
        expected: h.split_view_expected(id(1)).unwrap(),
        pane,
        replacement,
        budget: BUDGET,
    }
}
#[test]
fn genuine_editor_composer_button_scroll_and_owned_replacement() {
    let trailing = Content::new(
        id(20),
        ScrollContent::new(
            ScrollArea::default(),
            Content::new(
                id(21),
                StackContent {
                    model: Stack {
                        axis: Axis::Vertical,
                        gap: 6.,
                    },
                    flexible_child: None,
                    children: vec![composer(22), Content::new(id(23), Button::new("Send"))],
                },
            ),
        ),
    );
    let mut host = ContentHost::try_new_prepared(
        split(
            Content::new(id(10), TextFieldContent::new("Name", "Alice")),
            trailing,
        ),
        BUDGET,
    )
    .unwrap();
    host.set_action_events(true);
    render(&mut host, 640., 320.);
    let expected = host.split_view_expected(id(1)).unwrap();
    assert_eq!(
        host.focus_targets().iter().map(|v| v.0).collect::<Vec<_>>(),
        vec![id(10), id(22), id(23), id(4)]
    );
    assert!(host.focus(Some(id(22)), true));
    host.event(&ContentEvent::Commit {
        text: " hello".into(),
        replacement: None,
    });
    let binding = host.native_key_binding();
    host.event(&ContentEvent::Preedit {
        text: "中".into(),
        selection: 1..1,
        replacement: None,
        clauses: vec![],
    });
    assert!(host.is_composing());
    let before = host
        .get(id(1))
        .unwrap()
        .component::<SplitViewContent>()
        .unwrap()
        .config();
    let divider = host
        .get(id(1))
        .unwrap()
        .component::<SplitViewContent>()
        .unwrap()
        .layout()
        .unwrap()
        .divider
        .frame()
        .center();
    host.event(&ContentEvent::PointerDown(divider));
    assert!(host.pointer_capture().is_none());
    assert_eq!(
        host.get(id(1))
            .unwrap()
            .component::<SplitViewContent>()
            .unwrap()
            .config(),
        before
    );
    let input = request(&host, Pane::Trailing, composer(30));
    let rejected = host.replace_split_view_pane(input).unwrap_err();
    assert_eq!(rejected.error, SplitViewError::Marked);
    drop(rejected.input);
    assert_eq!(host.native_key_binding(), binding);
    host.event(&ContentEvent::Unmark);
    host.event(&ContentEvent::KeyInput(ContentKeyEvent {
        key: Some(ControlKey::Enter),
        down: true,
        repeat: false,
        modifiers: InputModifiers {
            command: true,
            ..Default::default()
        },
        native_code: 36,
        characters: "\r".into(),
        characters_ignoring_modifiers: "\r".into(),
    }));
    assert!(
        host.drain_action_records()
            .iter()
            .any(|a| a.action.kind == ContentActionKind::Submit)
    );
    let input = request(
        &host,
        Pane::Leading,
        Content::new(id(10), Button::new("Replaced")),
    );
    let old = host.replace_split_view_pane(input).unwrap();
    assert!(old.component::<TextFieldContent>().is_some());
    assert_ne!(old.mount_identity(), expected.leading_mount);
    let after = host.split_view_expected(id(1)).unwrap();
    assert_eq!(after.trailing_mount, expected.trailing_mount);
    assert_eq!(after.mount, expected.mount);
    render(&mut host, 700., 340.);
    assert_eq!(host.focused(), Some(id(22)));
}
#[derive(Debug)]
struct WrappedField(TextFieldContent);
impl Component for WrappedField {
    fn table_transaction_compatibility(&self) -> TableTransactionCompatibility {
        TableTransactionCompatibility::StableHooks
    }
    fn prepared_effects_work(&self, n: usize) -> Result<usize, PreparedEffectsError> {
        self.0.prepared_effects_work(n)
    }
    fn prepare_effects(&self, h: &[PreparedHook]) -> Result<PreparedEffects, PreparedEffectsError> {
        self.0.prepare_effects(h)
    }
    fn lifecycle_prepared(&mut self, e: Lifecycle, c: &mut PreparedLifecycleContext) {
        self.0.lifecycle_prepared(e, c);
    }
    fn lifecycle(&mut self, e: Lifecycle) {
        self.0.lifecycle(e);
    }
    fn window_active_changed(&mut self, a: bool) {
        self.0.window_active_changed(a);
    }
    fn presentation_runnable_changed(&mut self, a: bool) {
        self.0.presentation_runnable_changed(a);
    }
    fn measure(
        &mut self,
        t: &mut dyn ButtonTextRenderer,
        a: Size,
    ) -> tekes_ui_controls::layout::MeasuredChild {
        self.0.measure(t, a)
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        self.0.arrange(t, f, c);
    }
    fn paint(
        &mut self,
        s: &mut Scene,
        t: &mut dyn ButtonTextRenderer,
        f: Rect,
        c: Rect,
        theme: ContentTheme,
    ) {
        self.0.paint(s, t, f, c, theme);
    }
    fn text_surface(&self) -> Option<&TextSurface> {
        self.0.text_surface()
    }
    fn requires_text_services(&self) -> bool {
        true
    }
    fn focusable(&self) -> bool {
        self.0.focusable()
    }
    fn set_focus(&mut self, f: bool, v: bool) {
        self.0.set_focus(f, v);
    }
    fn composing(&self) -> bool {
        self.0.composing()
    }
    fn key_event(&mut self, k: &ContentKeyEvent) -> ContentResponse {
        self.0.key_event(k)
    }
    fn pointer_event(&mut self, p: &PointerEvent) -> ContentResponse {
        self.0.pointer_event(p)
    }
    fn action_kinds(&self, r: &ContentResponse) -> Vec<ContentActionKind> {
        self.0.action_kinds(r)
    }
    fn event(&mut self, e: &ContentEvent) -> ContentResponse {
        self.0.event(e)
    }
    fn semantics(&self) -> ContentSemantics {
        self.0.semantics()
    }
    fn prepare_text_input(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        self.0.prepare_text_input(t, f, c);
    }
}
#[derive(Debug)]
struct WrappedScroll(ScrollContent);
impl Component for WrappedScroll {
    fn table_transaction_compatibility(&self) -> TableTransactionCompatibility {
        TableTransactionCompatibility::StableHooks
    }
    fn prepared_effects_work(&self, n: usize) -> Result<usize, PreparedEffectsError> {
        self.0.prepared_effects_work(n)
    }
    fn prepare_effects(&self, h: &[PreparedHook]) -> Result<PreparedEffects, PreparedEffectsError> {
        self.0.prepare_effects(h)
    }
    fn lifecycle_prepared(&mut self, e: Lifecycle, c: &mut PreparedLifecycleContext) {
        self.0.lifecycle_prepared(e, c);
    }
    fn lifecycle(&mut self, e: Lifecycle) {
        self.0.lifecycle(e);
    }
    fn window_active_changed(&mut self, a: bool) {
        self.0.window_active_changed(a);
    }
    fn presentation_runnable_changed(&mut self, a: bool) {
        self.0.presentation_runnable_changed(a);
    }
    fn children(&self) -> &[Content] {
        self.0.children()
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        self.0.children_mut()
    }
    fn measure(
        &mut self,
        t: &mut dyn ButtonTextRenderer,
        a: Size,
    ) -> tekes_ui_controls::layout::MeasuredChild {
        self.0.measure(t, a)
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        self.0.arrange(t, f, c);
    }
    fn event(&mut self, e: &ContentEvent) -> ContentResponse {
        self.0.event(e)
    }
}
#[test]
fn compliant_external_field_and_scroll_delegate_finite_original_context() {
    let mut custom = TextFieldContent::new("Custom", "value");
    custom.submit_policy = TextSubmitPolicy::Enter;
    let left = Content::new(id(10), WrappedField(custom));
    let mut reader = ScrollContent::new(ScrollArea::default(), composer(21));
    let reader_version = reader
        .initialize_reading_document("wrapped-reader", 3, vec![id(21)])
        .unwrap();
    let right = Content::new(id(20), WrappedScroll(reader));
    let mut host = ContentHost::try_new_prepared(split(left, right), BUDGET).unwrap();
    render(&mut host, 640., 320.);
    host.set_action_events(true);
    host.focus(Some(id(10)), true);
    host.event(&ContentEvent::Commit {
        text: "edited".into(),
        replacement: None,
    });
    assert!(host.text_input_snapshot().unwrap().text.contains("edited"));
    host.event(&ContentEvent::KeyInput(ContentKeyEvent {
        key: Some(ControlKey::Enter),
        down: true,
        repeat: false,
        modifiers: InputModifiers::default(),
        native_code: 36,
        characters: "\r".into(),
        characters_ignoring_modifiers: "\r".into(),
    }));
    assert_eq!(
        host.drain_action_records()
            .iter()
            .filter(|a| a.action.kind == ContentActionKind::Submit)
            .count(),
        1
    );
    let input = request(
        &host,
        Pane::Leading,
        Content::new(id(30), WrappedField(TextFieldContent::new("Next", "next"))),
    );
    let old = host.replace_split_view_pane(input).unwrap();
    assert!(old.component::<WrappedField>().is_some());
    assert_eq!(
        host.get(id(20))
            .unwrap()
            .component::<WrappedScroll>()
            .unwrap()
            .0
            .reading_document_version(),
        Some(&reader_version)
    );
    assert!(
        host.get_mut(id(20))
            .unwrap()
            .component_mut::<WrappedScroll>()
            .unwrap()
            .0
            .initialize_reading_document("illegal-mounted", 4, vec![id(21)])
            .is_err()
    );
    let input = request(
        &host,
        Pane::Trailing,
        Content::new(
            id(31),
            WrappedScroll(ScrollContent::new(ScrollArea::default(), composer(32))),
        ),
    );
    let old = host.replace_split_view_pane(input).unwrap();
    let returned = &old.component::<WrappedScroll>().unwrap().0;
    let version = returned.reading_document_version().unwrap();
    assert_ne!(version.attachment(), reader_version.attachment());
    assert_eq!(version.key(), reader_version.key());
    assert_eq!(version.revision(), 3);
}
#[test]
fn zero_allocation_preserves_marked_binding_and_restores_new_geometry() {
    let mut host =
        ContentHost::try_new_prepared(split(composer(10), composer(20)), BUDGET).unwrap();
    render(&mut host, 600., 300.);
    host.focus(Some(id(10)), true);
    host.event(&ContentEvent::Preedit {
        text: "中".into(),
        selection: 1..1,
        replacement: None,
        clauses: vec![],
    });
    let before = host.text_input_snapshot().unwrap();
    let binding = host.native_key_binding();
    let mount = host.get(id(10)).unwrap().mount_identity();
    render(&mut host, 0., 0.);
    let zero = host.text_input_snapshot().unwrap();
    assert_eq!(zero.marked, before.marked);
    assert_eq!(zero.binding, before.binding);
    assert!(zero.caret.is_none() && zero.clip.is_none() && zero.carets.is_empty());
    assert_eq!(host.native_key_binding(), binding);
    assert_eq!(host.focused(), Some(id(10)));
    assert!(!host.semantics().iter().any(|s| s.id == id(10)));
    assert!(host.focus_targets().is_empty());
    render(&mut host, 650., 350.);
    assert!(host.text_input_snapshot().unwrap().caret.is_some());
    assert_eq!(host.get(id(10)).unwrap().mount_identity(), mount);
}
#[test]
fn authoritative_ratio_and_original_drag_resize_and_invalid_raw_refusal() {
    let mut host = ContentHost::try_new_prepared(
        split(
            Content::new(id(10), Button::new("A")),
            Content::new(id(20), Button::new("B")),
        ),
        BUDGET,
    )
    .unwrap();
    render(&mut host, 600., 300.);
    let split = host
        .get(id(1))
        .unwrap()
        .component::<SplitViewContent>()
        .unwrap();
    let p = split.layout().unwrap().divider.frame().center();
    host.event(&ContentEvent::PointerDown(p));
    assert!(host.pointer_capture().is_some());
    host.event(&ContentEvent::PointerMove(tekes_ui_scene::Point::new(
        p.x + 30.,
        p.y,
    )));
    render(&mut host, 600., 300.);
    assert!(host.pointer_capture().is_some());
    render(&mut host, 700., 300.);
    assert!(host.pointer_capture().is_none());
    let expected = host.split_view_expected(id(1)).unwrap();
    let mut config = host
        .get(id(1))
        .unwrap()
        .component::<SplitViewContent>()
        .unwrap()
        .config()
        .unwrap();
    config.ratio = 0.3;
    assert!(
        host.update_split_view(id(1), expected, config, BUDGET)
            .unwrap()
    );
    render(&mut host, 700., 300.);
    assert_eq!(
        host.get(id(1))
            .unwrap()
            .component::<SplitViewContent>()
            .unwrap()
            .config()
            .unwrap()
            .ratio,
        0.3
    );
    let before = host.split_view_expected(id(1));
    let dirty = host.needs_render();
    let mut scene = Scene::new();
    assert_eq!(
        host.render_checked(
            &mut scene,
            &mut Text,
            Rect::new(0., 0., f32::NAN, 1.),
            Rect::new(0., 0., 1., 1.),
            Theme::LIGHT.into()
        ),
        Err(SplitViewError::InvalidAllocation)
    );
    assert_eq!(host.split_view_expected(id(1)), before);
    assert_eq!(host.needs_render(), dirty);
}
#[derive(Debug)]
struct UnknownProbe(std::rc::Rc<std::cell::Cell<usize>>);
impl Component for UnknownProbe {
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        self.0.set(self.0.get() + 1);
        None
    }
    fn measure(
        &mut self,
        _: &mut dyn ButtonTextRenderer,
        _: Size,
    ) -> tekes_ui_controls::layout::MeasuredChild {
        tekes_ui_controls::layout::MeasuredChild::new(Size::new(80., 24.))
    }
}
fn row(children: Vec<Content>) -> Content {
    Content::new(
        id(100),
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 0.,
            },
            flexible_child: None,
            children,
        },
    )
}
#[test]
fn preceding_unknown_getter_untouched_on_actual_success_and_refusal() {
    let probe = std::rc::Rc::new(std::cell::Cell::new(0));
    let mut host = ContentHost::new(row(vec![
        Content::new(id(101), UnknownProbe(probe.clone())),
        split(
            Content::new(id(10), Button::new("A")),
            Content::new(id(20), Button::new("B")),
        ),
    ]))
    .unwrap();
    render(&mut host, 600., 400.);
    probe.set(0);
    let input = request(
        &host,
        Pane::Leading,
        Content::new(id(30), Button::new("new")),
    );
    let old = host.replace_split_view_pane(input).unwrap();
    assert_eq!(probe.get(), 0);
    drop(old);
    let mut input = request(
        &host,
        Pane::Leading,
        Content::new(id(20), Button::new("duplicate")),
    );
    let original = input.replacement.mount_identity();
    input.budget = BUDGET;
    assert!(
        matches!(host.replace_split_view_pane(input).unwrap_err(),Rejected{error:SplitViewError::Composition(CompositionError::DuplicateId(_)),input} if input.replacement.mount_identity()==original)
    );
    assert_eq!(probe.get(), 0);
}
#[derive(Debug)]
struct CrossOwner(std::rc::Rc<std::cell::Cell<bool>>);
impl Component for CrossOwner {
    fn table_transaction_compatibility(&self) -> TableTransactionCompatibility {
        TableTransactionCompatibility::StableHooks
    }
    fn measure(
        &mut self,
        _: &mut dyn ButtonTextRenderer,
        a: Size,
    ) -> tekes_ui_controls::layout::MeasuredChild {
        tekes_ui_controls::layout::MeasuredChild::new(a)
    }
    fn lifecycle(&mut self, e: Lifecycle) {
        if e == Lifecycle::Mounted {
            self.0.set(false);
        }
    }
}
#[test]
fn stable_cross_owner_effect_without_preparation_is_refused_before_callback() {
    let mut host = ContentHost::try_new_prepared(
        split(
            Content::new(id(10), Button::new("A")),
            Content::new(id(20), Button::new("B")),
        ),
        BUDGET,
    )
    .unwrap();
    let flag = std::rc::Rc::new(std::cell::Cell::new(true));
    let expected = host.split_view_expected(id(1));
    let input = request(
        &host,
        Pane::Leading,
        Content::new(id(30), CrossOwner(flag.clone())),
    );
    let rejected = host.replace_split_view_pane(input).unwrap_err();
    assert_eq!(
        rejected.error,
        SplitViewError::Effects(PreparedEffectsError::Unavailable)
    );
    assert!(flag.get());
    assert_eq!(host.split_view_expected(id(1)), expected);
    drop(rejected.input);
}
fn dialog() -> Content {
    Content::new(
        id(10),
        DialogContent::new(
            "Modal",
            Content::new(id(11), TextContent::new("Heading")),
            Content::new(id(12), Button::new("Body")),
            Content::new(id(13), Button::new("Close")),
        ),
    )
}
#[test]
fn actual_modal_close_full_cancel_unmount_positive_and_unknown_early_refusal() {
    let mut host = ContentHost::try_new_prepared(
        split(dialog(), Content::new(id(20), Button::new("retained"))),
        BUDGET,
    )
    .unwrap();
    render(&mut host, 600., 400.);
    host.show_dialog(id(10)).unwrap();
    render(&mut host, 600., 400.);
    assert!(host.input_overlay_owner().is_some());
    let input = request(
        &host,
        Pane::Leading,
        Content::new(id(30), Button::new("next")),
    );
    let old = host.replace_split_view_pane(input).unwrap();
    assert!(!old.component::<DialogContent>().unwrap().is_open());
    assert!(host.input_overlay_owner().is_none());
    assert_eq!(host.focused(), None);
    let probe = std::rc::Rc::new(std::cell::Cell::new(0));
    let mut host = ContentHost::new(row(vec![
        Content::new(id(101), UnknownProbe(probe.clone())),
        split(dialog(), Content::new(id(20), Button::new("retained"))),
    ]))
    .unwrap();
    render(&mut host, 600., 400.);
    host.show_dialog(id(10)).unwrap();
    render(&mut host, 600., 400.);
    probe.set(0);
    let before = host.split_view_expected(id(1));
    let focused = host.focused();
    let input = request(
        &host,
        Pane::Leading,
        Content::new(id(30), Button::new("next")),
    );
    let rejected = host.replace_split_view_pane(input).unwrap_err();
    assert_eq!(
        rejected.error,
        SplitViewError::Effects(PreparedEffectsError::Unavailable)
    );
    assert_eq!(probe.get(), 0);
    assert_eq!(host.focused(), focused);
    assert_eq!(host.split_view_expected(id(1)), before);
    assert!(host.input_overlay_owner().is_some());
    drop(rejected.input);
}
#[test]
fn same_original_layout_matrix_and_structural_tamper_never_self_heals() {
    use tekes_ui_controls::{extended::SplitView, layout_view::SplitViewView};
    for axis in [Axis::Horizontal, Axis::Vertical] {
        for extent in [0., 0.25, 1., 7.5, 99.5, 400.] {
            let config = SplitViewConfig {
                axis,
                ratio: 0.35,
                min_pane: 20.,
                ..Default::default()
            };
            let root = Content::new(
                id(1),
                SplitViewContent::try_new(SplitViewConstruction {
                    config,
                    slots: SplitViewSlotIds {
                        leading: id(2),
                        trailing: id(3),
                        divider: id(4),
                    },
                    leading: Content::new(id(10), Button::new("A")),
                    trailing: Content::new(id(20), Button::new("B")),
                    budget: BUDGET,
                })
                .unwrap(),
            );
            let mut host = ContentHost::try_new_prepared(root, BUDGET).unwrap();
            render(&mut host, extent, extent);
            let actual = host
                .get(id(1))
                .unwrap()
                .component::<SplitViewContent>()
                .unwrap()
                .layout()
                .unwrap();
            let mut model = SplitView::new(config.ratio);
            model.min_pane = config.min_pane;
            model.divider = config.divider;
            let expected = SplitViewView::new(axis).arrange(
                &mut model,
                Rect::new(0., 0., extent, extent),
                Some(Rect::new(0., 0., extent, extent)),
            );
            assert_eq!(actual, expected);
        }
    }
    let mut host = ContentHost::try_new_prepared(
        split(
            Content::new(id(10), Button::new("A")),
            Content::new(id(20), Button::new("B")),
        ),
        BUDGET,
    )
    .unwrap();
    host.get_mut(id(1))
        .unwrap()
        .component_mut::<SplitViewContent>()
        .unwrap()
        .children_mut()
        .unwrap()
        .swap(0, 1);
    assert!(host.split_view_expected(id(1)).is_none());
    assert!(
        host.get(id(1))
            .unwrap()
            .component::<SplitViewContent>()
            .unwrap()
            .validate()
            .is_err()
    );
    assert!(
        host.get(id(1))
            .unwrap()
            .component::<SplitViewContent>()
            .unwrap()
            .layout()
            .is_none()
    );
}
#[test]
fn original_reveal_moves_offscreen_composer_inside_the_actual_split_pane_clip() {
    let mut children: Vec<Content> = (30..60)
        .map(|n| Content::new(id(n), TextContent::new(format!("History {n}"))))
        .collect();
    children.push(composer(60));
    let reader = Content::new(
        id(20),
        ScrollContent::new(
            ScrollArea::default(),
            Content::new(
                id(21),
                StackContent {
                    model: Stack {
                        axis: Axis::Vertical,
                        gap: 8.,
                    },
                    flexible_child: None,
                    children,
                },
            ),
        ),
    );
    let mut host = ContentHost::try_new_prepared(
        split(Content::new(id(10), Button::new("A")), reader),
        BUDGET,
    )
    .unwrap();
    render(&mut host, 600., 220.);
    assert!(
        host.focus(Some(id(60)), true),
        "original pure reveal must understand the authoritative Slot clip"
    );
    render(&mut host, 600., 220.);
    let scroll = host
        .get(id(20))
        .unwrap()
        .component::<ScrollContent>()
        .unwrap();
    assert!(scroll.scroll.offset.y > 0.);
    let snapshot = host.text_input_snapshot().unwrap();
    let pane = host
        .get(id(1))
        .unwrap()
        .component::<SplitViewContent>()
        .unwrap()
        .layout()
        .unwrap()
        .trailing
        .frame();
    assert!(
        snapshot
            .caret
            .is_some_and(|r| r.intersection(&pane).is_some())
    );
}
#[test]
fn actual_tab_order_reaches_divider_then_wraps_and_old_actions_cannot_replay() {
    let mut host = ContentHost::try_new_prepared(
        split(
            Content::new(id(10), Button::new("A")),
            Content::new(id(20), Button::new("B")),
        ),
        BUDGET,
    )
    .unwrap();
    render(&mut host, 600., 300.);
    host.set_action_events(true);
    host.enter_focus(false);
    assert_eq!(host.focused(), Some(id(10)));
    for expected in [id(20), id(4), id(10)] {
        host.event(&ContentEvent::Key {
            key: ControlKey::Tab { reverse: false },
            down: true,
        });
        assert_eq!(host.focused(), Some(expected));
    }
    host.event(&ContentEvent::Key {
        key: ControlKey::Enter,
        down: true,
    });
    host.event(&ContentEvent::Key {
        key: ControlKey::Enter,
        down: false,
    });
    let input = request(
        &host,
        Pane::Leading,
        Content::new(id(10), Button::new("new same ID")),
    );
    let old = host.replace_split_view_pane(input).unwrap();
    assert!(host.drain_action_records().is_empty());
    drop(old);
    render(&mut host, 600., 300.);
    host.focus(Some(id(10)), true);
    host.event(&ContentEvent::Key {
        key: ControlKey::Enter,
        down: true,
    });
    host.event(&ContentEvent::Key {
        key: ControlKey::Enter,
        down: false,
    });
    assert_eq!(host.drain_action_records().len(), 1);
}
#[test]
fn raw_config_failures_and_zero_hide_marked_keep_all_logical_input() {
    let mut host =
        ContentHost::try_new_prepared(split(composer(10), composer(20)), BUDGET).unwrap();
    render(&mut host, 600., 300.);
    host.focus(Some(id(10)), true);
    host.event(&ContentEvent::Preedit {
        text: "中".into(),
        selection: 1..1,
        replacement: None,
        clauses: vec![],
    });
    let expected = host.split_view_expected(id(1)).unwrap();
    let config = host
        .get(id(1))
        .unwrap()
        .component::<SplitViewContent>()
        .unwrap()
        .config()
        .unwrap();
    let binding = host.native_key_binding();
    let input = host.text_input_snapshot().unwrap();
    for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -1., 1.01] {
        let mut bad = config;
        bad.ratio = invalid;
        assert_eq!(
            host.update_split_view(id(1), expected, bad, BUDGET),
            Err(SplitViewError::InvalidConfig)
        );
        assert_eq!(host.split_view_expected(id(1)), Some(expected));
        assert_eq!(host.native_key_binding(), binding);
        assert_eq!(host.text_input_snapshot().unwrap(), input);
    }
    let mut hide = config;
    hide.collapsed = Some(Pane::Leading);
    assert_eq!(
        host.update_split_view(id(1), expected, hide, BUDGET),
        Err(SplitViewError::Marked)
    );
    assert_eq!(host.text_input_snapshot().unwrap(), input);
}
#[test]
fn original_view_and_adapter_share_primary_secondary_doubleclick_and_keyboard_reducer() {
    use tekes_ui_controls::{extended::SplitView, layout_view::SplitViewView};
    use tekes_ui_scene::Point;
    let mut host = ContentHost::try_new_prepared(
        split(
            Content::new(id(10), Button::new("A")),
            Content::new(id(20), Button::new("B")),
        ),
        BUDGET,
    )
    .unwrap();
    render(&mut host, 600., 300.);
    let frame = Rect::new(0., 0., 600., 300.);
    let mut model = SplitView::new(0.5);
    let mut view = SplitViewView::new(Axis::Horizontal);
    view.arrange(&mut model, frame, Some(frame));
    let p = view.layout().divider.frame().center();
    for button in [PointerButton::Secondary, PointerButton::Middle] {
        host.event(&ContentEvent::Pointer(PointerEvent {
            position: p,
            phase: PointerPhase::Down,
            button: Some(button),
            modifiers: Default::default(),
            click_count: 1,
        }));
        assert!(host.pointer_capture().is_none());
        assert_eq!(
            host.get(id(1))
                .unwrap()
                .component::<SplitViewContent>()
                .unwrap()
                .config()
                .unwrap()
                .ratio,
            model.ratio()
        );
    }
    view.pointer_down(p, &mut model);
    host.event(&ContentEvent::PointerDown(p));
    for delta in [3.9, 4., 30.] {
        let p = Point::new(p.x + delta, p.y);
        view.pointer_move(p, &mut model);
        host.event(&ContentEvent::PointerMove(p));
        view.arrange(&mut model, frame, Some(frame));
        render(&mut host, 600., 300.);
        assert_eq!(
            host.get(id(1))
                .unwrap()
                .component::<SplitViewContent>()
                .unwrap()
                .config()
                .unwrap()
                .ratio,
            model.ratio()
        );
    }
    view.pointer_up(&mut model);
    host.event(&ContentEvent::PointerUp(p));
    let p = view.layout().divider.frame().center();
    view.double_click(p, &mut model);
    host.event(&ContentEvent::Pointer(PointerEvent {
        position: p,
        phase: PointerPhase::Down,
        button: Some(PointerButton::Primary),
        modifiers: Default::default(),
        click_count: 2,
    }));
    assert!(host.pointer_capture().is_none());
    assert_eq!(
        host.get(id(1))
            .unwrap()
            .component::<SplitViewContent>()
            .unwrap()
            .config()
            .unwrap()
            .ratio,
        model.ratio()
    );
    host.focus(Some(id(4)), true);
    for key in [
        ControlKey::Left,
        ControlKey::Right,
        ControlKey::Home,
        ControlKey::End,
        ControlKey::Enter,
        ControlKey::Space,
    ] {
        view.key(key, &mut model);
        host.event(&ContentEvent::Key { key, down: true });
        view.arrange(&mut model, frame, Some(frame));
        render(&mut host, 600., 300.);
        let actual = host
            .get(id(1))
            .unwrap()
            .component::<SplitViewContent>()
            .unwrap()
            .config()
            .unwrap();
        assert_eq!(actual.ratio, model.ratio());
        assert_eq!(actual.collapsed, model.collapsed);
    }
}
#[derive(Debug)]
struct ActivityLimited(std::rc::Rc<std::cell::Cell<usize>>);
impl Component for ActivityLimited {
    fn table_transaction_compatibility(&self) -> TableTransactionCompatibility {
        TableTransactionCompatibility::StableHooks
    }
    fn prepared_effects_work(&self, n: usize) -> Result<usize, PreparedEffectsError> {
        PreparedEffects::local_work(n)
    }
    fn prepare_effects(&self, h: &[PreparedHook]) -> Result<PreparedEffects, PreparedEffectsError> {
        let calls = h
            .iter()
            .filter(|h| matches!(h.kind, PreparedHookKind::WindowActive(_)))
            .count();
        if self.0.get() + calls > 1 {
            return Err(PreparedEffectsError::CounterCapacity);
        }
        Ok(PreparedEffects::local(self, h))
    }
    fn window_active_changed(&mut self, _: bool) {
        self.0.set(self.0.get() + 1);
    }
    fn measure(
        &mut self,
        _: &mut dyn ButtonTextRenderer,
        a: Size,
    ) -> tekes_ui_controls::layout::MeasuredChild {
        tekes_ui_controls::layout::MeasuredChild::new(a)
    }
}
#[test]
fn activity_preparation_counts_actual_notifications_not_noop_framework_updates() {
    let calls = std::rc::Rc::new(std::cell::Cell::new(0));
    let mut original = ContentHost::new(row(vec![Content::new(
        id(30),
        ActivityLimited(calls.clone()),
    )]))
    .unwrap();
    // Current ordinary Host construction starts with a notified-active cache.
    // Establish the one real notification without changing that Host contract.
    original.set_window_active(false);
    assert_eq!(calls.get(), 1);
    let retained = original.remove(id(30)).unwrap();
    let mut host = ContentHost::try_new_prepared(
        split(
            Content::new(id(10), Button::new("A")),
            Content::new(id(20), Button::new("B")),
        ),
        BUDGET,
    )
    .unwrap();
    host.set_window_active(false);
    let input = request(&host, Pane::Leading, retained);
    host.replace_split_view_pane(input).unwrap();
    assert_eq!(
        calls.get(),
        1,
        "unchanged effective activation does not invoke component hook again"
    );

    // Keep the original active-to-active obligation under this baseline's
    // zero-initial-notification contract: neither preparation nor commit may
    // invent a redundant true notification for a retained active owner.
    let active_calls = std::rc::Rc::new(std::cell::Cell::new(0));
    let mut active_original = ContentHost::new(row(vec![Content::new(
        id(30),
        ActivityLimited(active_calls.clone()),
    )]))
    .unwrap();
    assert_eq!(active_calls.get(), 0);
    let active_retained = active_original.remove(id(30)).unwrap();
    let mut active_host = ContentHost::try_new_prepared(
        split(
            Content::new(id(10), Button::new("A")),
            Content::new(id(20), Button::new("B")),
        ),
        BUDGET,
    )
    .unwrap();
    let input = request(&active_host, Pane::Leading, active_retained);
    active_host.replace_split_view_pane(input).unwrap();
    assert_eq!(
        active_calls.get(),
        0,
        "active-to-active replacement adds no notification"
    );
}
#[test]
fn checked_render_refuses_a_broken_split_without_blurring_original_marked_owner() {
    let mut host = ContentHost::try_new_prepared(
        split(composer(10), Content::new(id(20), Button::new("B"))),
        BUDGET,
    )
    .unwrap();
    render(&mut host, 600., 300.);
    host.focus(Some(id(10)), true);
    host.event(&ContentEvent::Preedit {
        text: "中".into(),
        selection: 1..1,
        replacement: None,
        clauses: vec![],
    });
    host.get_mut(id(1))
        .unwrap()
        .component_mut::<SplitViewContent>()
        .unwrap()
        .children_mut()
        .unwrap()
        .swap(0, 1);
    let field = host
        .get(id(10))
        .unwrap()
        .component::<TextFieldContent>()
        .unwrap();
    let marked = field.surface.input.marked.clone();
    let text = field.surface.input.text.clone();
    let epoch = host.focus_epoch();
    let focused = host.focused();
    let frame = Rect::new(0., 0., 600., 300.);
    assert!(
        host.render_checked(
            &mut Scene::new(),
            &mut Text,
            frame,
            frame,
            Theme::LIGHT.into()
        )
        .is_err()
    );
    assert_eq!(host.focused(), focused);
    assert_eq!(host.focus_epoch(), epoch);
    let field = host
        .get(id(10))
        .unwrap()
        .component::<TextFieldContent>()
        .unwrap();
    assert_eq!(field.surface.input.marked, marked);
    assert_eq!(field.surface.input.text, text);
}
#[test]
fn doubleclick_terminates_the_original_host_capture_even_without_an_intervening_up() {
    let mut host = ContentHost::try_new_prepared(
        split(
            Content::new(id(10), Button::new("A")),
            Content::new(id(20), Button::new("B")),
        ),
        BUDGET,
    )
    .unwrap();
    render(&mut host, 600., 300.);
    let p = host
        .get(id(1))
        .unwrap()
        .component::<SplitViewContent>()
        .unwrap()
        .layout()
        .unwrap()
        .divider
        .frame()
        .center();
    host.event(&ContentEvent::PointerDown(p));
    assert!(host.pointer_capture().is_some());
    host.event(&ContentEvent::Pointer(PointerEvent {
        position: p,
        phase: PointerPhase::Down,
        button: Some(PointerButton::Primary),
        modifiers: Default::default(),
        click_count: 2,
    }));
    assert!(host.pointer_capture().is_none());
    assert_eq!(
        host.get(id(1))
            .unwrap()
            .component::<SplitViewContent>()
            .unwrap()
            .config()
            .unwrap()
            .ratio,
        0.5
    );
}

#[derive(Debug)]
struct RecordedButton {
    button: Button,
    name: &'static str,
    log: std::rc::Rc<std::cell::RefCell<Vec<String>>>,
}
impl Component for RecordedButton {
    fn table_transaction_compatibility(&self) -> TableTransactionCompatibility {
        TableTransactionCompatibility::StableHooks
    }
    fn prepared_effects_work(&self, n: usize) -> Result<usize, PreparedEffectsError> {
        self.button.prepared_effects_work(n)
    }
    fn prepare_effects(&self, h: &[PreparedHook]) -> Result<PreparedEffects, PreparedEffectsError> {
        self.button.prepare_effects(h)?;
        Ok(PreparedEffects::local(self, h))
    }
    fn measure(
        &mut self,
        t: &mut dyn ButtonTextRenderer,
        s: Size,
    ) -> tekes_ui_controls::layout::MeasuredChild {
        self.button.measure(t, s)
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        self.button.arrange(t, f, c);
    }
    fn focusable(&self) -> bool {
        self.button.focusable()
    }
    fn set_focus(&mut self, f: bool, v: bool) {
        self.log
            .borrow_mut()
            .push(format!("{} focus {f}", self.name));
        self.button.set_focus(f, v);
    }
    fn lifecycle(&mut self, e: Lifecycle) {
        self.log.borrow_mut().push(format!("{} {e:?}", self.name));
        self.button.lifecycle(e);
    }
    fn event(&mut self, e: &ContentEvent) -> ContentResponse {
        if matches!(e, ContentEvent::Cancel) {
            self.log.borrow_mut().push(format!("{} cancel", self.name));
        }
        self.button.event(e)
    }
    fn semantics(&self) -> ContentSemantics {
        self.button.semantics()
    }
}
#[test]
fn real_modal_replacement_restores_retained_target_in_original_order_before_unmount() {
    let log = std::rc::Rc::new(std::cell::RefCell::new(vec![]));
    let recorded = |n, name| {
        Content::new(
            id(n),
            RecordedButton {
                button: Button::new(name),
                name,
                log: log.clone(),
            },
        )
    };
    let modal = Content::new(
        id(10),
        DialogContent::new(
            "Modal",
            Content::new(id(11), TextContent::new("Title")),
            recorded(12, "body"),
            Content::new(id(13), Button::new("Close")),
        ),
    );
    let mut host =
        ContentHost::try_new_prepared(split(modal, recorded(20, "retained")), BUDGET).unwrap();
    render(&mut host, 600., 400.);
    assert!(host.focus(Some(id(20)), true));
    let restore = host.capture_focus().unwrap();
    host.show_dialog(id(10)).unwrap();
    render(&mut host, 600., 400.);
    assert_eq!(host.focused(), Some(id(12)));
    let old_mount = host.get(id(10)).unwrap().mount_identity();
    let child_mount = host.get(id(12)).unwrap().mount_identity();
    log.borrow_mut().clear();
    let input = request(
        &host,
        Pane::Leading,
        Content::new(id(30), Button::new("new")),
    );
    let old = host.replace_split_view_pane(input).unwrap();
    assert_eq!(host.focused(), Some(id(20)));
    assert_eq!(host.capture_focus(), Some(restore));
    assert!(host.input_overlay_owner().is_none());
    assert!(!old.component::<DialogContent>().unwrap().is_open());
    assert_ne!(old.mount_identity(), old_mount);
    assert_ne!(
        old.component::<DialogContent>().unwrap().children()[1].mount_identity(),
        child_mount
    );
    assert_eq!(
        *log.borrow(),
        vec![
            "body cancel",
            "retained cancel",
            "body focus false",
            "retained focus true",
            "body cancel",
            "body focus false",
            "body Unmounted"
        ]
    );
}
#[test]
fn real_modal_replacement_uses_original_pure_reveal_for_offscreen_retained_composer() {
    let mut children: Vec<Content> = (30..60)
        .map(|n| Content::new(id(n), TextContent::new(format!("History {n}"))))
        .collect();
    children.push(composer(60));
    let reader = Content::new(
        id(20),
        ScrollContent::new(
            ScrollArea::default(),
            Content::new(
                id(21),
                StackContent {
                    model: Stack {
                        axis: Axis::Vertical,
                        gap: 8.,
                    },
                    flexible_child: None,
                    children,
                },
            ),
        ),
    );
    let mut host = ContentHost::try_new_prepared(split(dialog(), reader), BUDGET).unwrap();
    render(&mut host, 600., 240.);
    assert!(host.focus(Some(id(60)), true));
    render(&mut host, 600., 240.);
    let lease = host.capture_focus().unwrap();
    let binding = host.native_key_binding().unwrap();
    host.show_dialog(id(10)).unwrap();
    render(&mut host, 600., 240.);
    host.get_mut(id(20))
        .unwrap()
        .component_mut::<ScrollContent>()
        .unwrap()
        .scroll
        .offset
        .y = 0.;
    render(&mut host, 600., 240.);
    assert_eq!(
        host.get(id(20))
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .scroll
            .offset
            .y,
        0.
    );
    let old = host
        .replace_split_view_pane(request(
            &host,
            Pane::Leading,
            Content::new(id(70), Button::new("new")),
        ))
        .unwrap();
    assert!(!old.component::<DialogContent>().unwrap().is_open());
    assert_eq!(host.capture_focus(), Some(lease));
    assert_eq!(host.native_key_binding().as_ref(), Some(&binding));
    assert!(
        host.get(id(20))
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .scroll
            .offset
            .y
            > 0.
    );
    render(&mut host, 600., 240.);
    assert!(host.text_input_snapshot().unwrap().caret.is_some());
}
#[test]
fn same_id_replacement_and_remount_reject_old_focus_leases_and_accept_new_identity() {
    let mut host = ContentHost::try_new_prepared(
        split(composer(10), Content::new(id(20), Button::new("retained"))),
        BUDGET,
    )
    .unwrap();
    render(&mut host, 600., 300.);
    host.focus(Some(id(10)), true);
    let original = host.capture_focus().unwrap();
    let old_binding = host.native_key_binding().unwrap();
    let retired = host
        .replace_split_view_pane(request(&host, Pane::Leading, composer(10)))
        .unwrap();
    render(&mut host, 600., 300.);
    assert!(host.focus(Some(id(10)), true));
    let replaced = host.capture_focus().unwrap();
    assert_ne!(replaced, original);
    assert_ne!(host.native_key_binding().unwrap(), old_binding);
    host.restore_focus(Some(original));
    assert_eq!(host.focused(), None);
    assert!(host.restore_focus(Some(replaced)));
    assert_eq!(host.focused(), Some(id(10)));
    let second = host
        .replace_split_view_pane(request(&host, Pane::Leading, retired))
        .unwrap();
    render(&mut host, 600., 300.);
    assert!(host.focus(Some(id(10)), true));
    let remounted = host.capture_focus().unwrap();
    assert_ne!(remounted, original);
    assert_ne!(remounted, replaced);
    for stale in [original, replaced] {
        host.restore_focus(Some(stale));
        assert_eq!(host.focused(), None);
        assert!(host.restore_focus(Some(remounted)));
    }
    drop(second);
}
#[test]
fn real_modal_return_lease_cannot_authorize_a_same_id_replacement_but_new_lease_can() {
    let mut host = ContentHost::try_new_prepared(split(dialog(), composer(20)), BUDGET).unwrap();
    render(&mut host, 600., 350.);
    assert!(host.focus(Some(id(20)), true));
    let stale = host.capture_focus().unwrap();
    host.show_dialog(id(10)).unwrap();
    render(&mut host, 600., 350.);
    let retired = host
        .replace_split_view_pane(request(&host, Pane::Trailing, composer(20)))
        .unwrap();
    render(&mut host, 600., 350.);
    let closed = host
        .replace_split_view_pane(request(
            &host,
            Pane::Leading,
            Content::new(id(70), Button::new("new")),
        ))
        .unwrap();
    assert_eq!(host.focused(), None);
    render(&mut host, 600., 350.);
    assert!(host.focus(Some(id(20)), true));
    let fresh = host.capture_focus().unwrap();
    assert_ne!(stale, fresh);
    host.restore_focus(Some(stale));
    assert_eq!(host.focused(), None);
    assert!(host.restore_focus(Some(fresh)));
    drop((retired, closed));
}

#[test]
fn a_valid_return_target_inside_the_retiring_pane_is_refused_instead_of_silently_dropped() {
    let mut host = ContentHost::try_new_prepared(
        split(
            row(vec![composer(15), dialog()]),
            Content::new(id(20), Button::new("retained")),
        ),
        BUDGET,
    )
    .unwrap();
    render(&mut host, 600., 400.);
    assert!(host.focus(Some(id(15)), true));
    host.show_dialog(id(10)).unwrap();
    render(&mut host, 600., 400.);
    let input = request(
        &host,
        Pane::Leading,
        Content::new(id(70), Button::new("new")),
    );
    let mount = input.replacement.mount_identity();
    let before = format!("{host:?}");
    let rejected = host.replace_split_view_pane(input).unwrap_err();
    assert_eq!(rejected.error, SplitViewError::UnsupportedRestoreReveal);
    assert_eq!(rejected.input.replacement.mount_identity(), mount);
    assert_eq!(format!("{host:?}"), before);
}

#[test]
fn public_split_content_exposes_the_original_divider_as_a_splitter() {
    let mut host = ContentHost::try_new_prepared(
        split(
            Content::new(id(10), Button::new("leading")),
            Content::new(id(20), Button::new("trailing")),
        ),
        BUDGET,
    )
    .unwrap();
    render(&mut host, 600.0, 300.0);
    let node = host
        .semantics()
        .into_iter()
        .find(|node| node.id == id(4))
        .expect("visible original splitter");
    assert_eq!(node.semantics.role, ContentRole::Splitter);
    assert_eq!(host.semantic_value_range(id(4), None), Some(0.0..=1.0));
    assert!(
        host.accessibility_action(id(4), None, AccessibilityAction::SetValue("0.625".into()))
            .consumed
    );
    assert_eq!(
        host.get(id(1))
            .unwrap()
            .component::<SplitViewContent>()
            .unwrap()
            .config()
            .unwrap()
            .ratio,
        0.625
    );
    render(&mut host, 0.0, 0.0);
    assert!(host.semantics().iter().all(|node| node.id != id(4)));
    assert!(host.split_view_expected(id(1)).is_some());
}

#[test]
fn splitter_non_binary_minimum_domain_accepts_decimal_endpoint_through_strict_scroll_route() {
    let split = Content::new(
        id(2),
        SplitViewContent::try_new(SplitViewConstruction {
            config: SplitViewConfig {
                min_pane: 10.0,
                ..SplitViewConfig::default()
            },
            slots: SplitViewSlotIds {
                leading: id(3),
                trailing: id(4),
                divider: id(5),
            },
            leading: Content::new(id(6), Button::new("leading")),
            trailing: Content::new(id(7), Button::new("trailing")),
            budget: BUDGET,
        })
        .unwrap(),
    );
    let root = Content::new(id(1), ScrollContent::new(ScrollArea::default(), split));
    let mut host = ContentHost::try_new_prepared(root, BUDGET).unwrap();
    render(&mut host, 111.0, 80.0);

    assert_eq!(host.semantic_value_range(id(5), None), Some(0.1..=0.9));
    assert!(
        host.accessibility_action(id(5), None, AccessibilityAction::SetValue("0.1".into()))
            .consumed
    );
    let lower = host
        .semantics()
        .into_iter()
        .find(|node| node.id == id(5))
        .unwrap();
    assert_eq!(lower.semantics.value.as_deref(), Some("0.1"));
    let lower_ratio = host
        .get(id(1))
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .children()[0]
        .component::<SplitViewContent>()
        .unwrap()
        .config()
        .unwrap()
        .ratio;
    assert!((lower_ratio - 0.1).abs() < f32::EPSILON);
    let lower_value = lower.semantics.value.unwrap().parse::<f64>().unwrap();
    assert!(
        host.semantic_value_range(id(5), None)
            .unwrap()
            .contains(&lower_value)
    );

    #[cfg(target_os = "macos")]
    {
        host.set_window_active(true);
        let native = tekes_ui_platform_macos::content_accessibility::content_nodes(&host);
        let node = native
            .iter()
            .find(|node| node.key.component == host.semantic_mount(id(5)).unwrap())
            .unwrap();
        assert_eq!(
            node.role,
            tekes_ui_platform_macos::AccessibilityRole::Splitter(
                tekes_ui_platform_macos::AccessibilityRange::new(0.1, 0.9).unwrap()
            )
        );
        assert_eq!(node.value.as_deref(), Some("0.1"));
    }

    let lower_endpoint = host.accessibility_action(id(5), None, AccessibilityAction::Decrement);
    assert!(lower_endpoint.consumed);
    assert!(!lower_endpoint.changed);
    let before_invalid = host
        .get(id(1))
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .scroll
        .offset;
    let before_ratio = host
        .get(id(1))
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .children()[0]
        .component::<SplitViewContent>()
        .unwrap()
        .config()
        .unwrap()
        .ratio;
    let before_focus = host.focused();
    for invalid in ["1.0", "NaN"] {
        assert!(
            !host
                .accessibility_action(id(5), None, AccessibilityAction::SetValue(invalid.into()))
                .consumed
        );
    }
    assert!(
        !host
            .accessibility_action(id(5), Some(99), AccessibilityAction::Increment)
            .consumed
    );
    assert_eq!(
        host.get(id(1))
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .scroll
            .offset,
        before_invalid
    );
    assert_eq!(
        host.get(id(1))
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .children()[0]
            .component::<SplitViewContent>()
            .unwrap()
            .config()
            .unwrap()
            .ratio,
        before_ratio
    );
    assert_eq!(host.focused(), before_focus);

    let upper_response =
        host.accessibility_action(id(5), None, AccessibilityAction::SetValue("0.9".into()));
    assert!(upper_response.consumed);
    let upper = host
        .semantics()
        .into_iter()
        .find(|node| node.id == id(5))
        .unwrap();
    assert_eq!(upper.semantics.value.as_deref(), Some("0.9"));
    let upper_ratio = host
        .get(id(1))
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .children()[0]
        .component::<SplitViewContent>()
        .unwrap()
        .config()
        .unwrap()
        .ratio;
    assert!((upper_ratio - 0.9).abs() < f32::EPSILON);
    assert!(
        host.semantic_value_range(id(5), None)
            .unwrap()
            .contains(&upper.semantics.value.unwrap().parse::<f64>().unwrap())
    );
    let upper_endpoint = host.accessibility_action(id(5), None, AccessibilityAction::Increment);
    assert!(upper_endpoint.consumed);
    assert!(!upper_endpoint.changed);

    let clamped = Content::new(
        id(12),
        SplitViewContent::try_new(SplitViewConstruction {
            config: SplitViewConfig {
                ratio: 0.01,
                min_pane: 10.0,
                ..SplitViewConfig::default()
            },
            slots: SplitViewSlotIds {
                leading: id(13),
                trailing: id(14),
                divider: id(15),
            },
            leading: Content::new(id(16), Button::new("leading")),
            trailing: Content::new(id(17), Button::new("trailing")),
            budget: BUDGET,
        })
        .unwrap(),
    );
    let mut clamped_host = ContentHost::try_new_prepared(clamped, BUDGET).unwrap();
    render(&mut clamped_host, 101.0, 80.0);
    assert_eq!(
        clamped_host
            .semantics()
            .into_iter()
            .find(|node| node.id == id(15))
            .unwrap()
            .semantics
            .value
            .as_deref(),
        Some("0.1")
    );
    let response = clamped_host.accessibility_action(
        id(15),
        None,
        AccessibilityAction::SetValue("0.1".into()),
    );
    assert!(response.consumed);
    assert!(response.changed);
    assert!(
        (clamped_host
            .get(id(12))
            .unwrap()
            .component::<SplitViewContent>()
            .unwrap()
            .config()
            .unwrap()
            .ratio
            - 0.1)
            .abs()
            < f32::EPSILON
    );
}
