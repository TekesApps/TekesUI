//! Actions are collected at the original delivery boundary, never by polling
//! control state or replaying a callback during painting/focus changes.
use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentActionKind {
    Activate,
    /// An editor's configured submission intent, not business delivery.
    Submit,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentActionSource {
    Pointer,
    Keyboard,
    /// The semantic action entry point; this alone does not prove VoiceOver.
    Accessibility,
    Programmatic,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContentAction {
    pub target: ComponentId,
    pub kind: ContentActionKind,
    pub source: ContentActionSource,
}
/// Owned data captured by the original component at delivery time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContentActionPayload {
    /// Original toolbar command captured before menu-close focus cleanup.
    ToolbarCommand(u64),
    MenuSelection(MenuSelection),
    /// Owned controlled Disclosure request captured at original delivery.
    DisclosureExpansion(DisclosureIntent),
}
/// Item identity in one immutable model revision. Explicit values provide a
/// stable path; legacy positional menus retain index, label and model revision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuSelection {
    pub(super) index: usize,
    pub(super) label: String,
    pub(super) model_revision: u64,
    pub(super) value_path: Option<Vec<String>>,
    pub(super) context_request: Option<ContextMenuRequest>,
}
impl MenuSelection {
    pub fn context_request(&self) -> Option<ContextMenuRequest> {
        self.context_request
    }
    pub fn index(&self) -> usize {
        self.index
    }
    pub fn label(&self) -> &str {
        &self.label
    }
    pub fn model_revision(&self) -> u64 {
        self.model_revision
    }
    pub fn value_path(&self) -> Option<&[String]> {
        self.value_path.as_deref()
    }
}
/// Rich view of the same once-only queue used by `drain_actions`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentActionRecord {
    pub action: ContentAction,
    /// Opaque original Content mount identity, not a business identifier.
    pub mount: u64,
    pub payload: Option<ContentActionPayload>,
}
#[derive(Debug)]
pub(super) struct PendingAction {
    pub(super) action: ContentAction,
    pub(super) mount: u64,
    pub(super) payload: Option<ContentActionPayload>,
}
static NEXT_MOUNT: AtomicU64 = AtomicU64::new(1);
pub(super) fn next_mount() -> u64 {
    try_next_mount().expect("content mount identity exhausted")
}
/// Same allocator and successful sequence as Content::new/lifecycle.
pub(super) fn try_next_mount() -> Result<u64, CompositionError> {
    #[cfg(test)]
    if FAIL_MOUNT.with(|flag| flag.get()) {
        return Err(CompositionError::MountExhausted);
    }
    NEXT_MOUNT
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
        .map_err(|_| CompositionError::MountExhausted)
}
#[cfg(test)]
mod mount_capacity_test {
    thread_local! { pub(crate) static FAIL_MOUNT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }
}
#[cfg(test)]
pub(super) use mount_capacity_test::FAIL_MOUNT;
/// Finite tickets from the original allocator. No ambient identity fallback.
pub(super) struct MountRange {
    next: u64,
    end: u64,
}
impl MountRange {
    pub(super) fn empty() -> Self {
        Self { next: 0, end: 0 }
    }
    pub(super) fn take(&mut self) -> u64 {
        assert!(self.next < self.end, "staged mount count violated");
        let n = self.next;
        self.next += 1;
        n
    }
    pub(super) fn exhausted(&self) -> bool {
        self.next == self.end
    }
}
pub(super) fn reserve_mounts(count: usize) -> Option<MountRange> {
    let count = u64::try_from(count).ok()?;
    {
        let counter = &NEXT_MOUNT;
        if count == 0 {
            let n = counter.load(Ordering::Relaxed);
            return Some(MountRange { next: n, end: n });
        }
        let start = counter
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
                n.checked_add(count)
            })
            .ok()?;
        Some(MountRange {
            next: start,
            end: start + count,
        })
    }
}
impl ContentEvent {
    pub(super) fn action_source(&self) -> ContentActionSource {
        match self {
            Self::Pointer(_)
            | Self::PointerDown(_)
            | Self::PointerUp(_)
            | Self::PointerMove(_)
            | Self::Scroll { .. } => ContentActionSource::Pointer,
            Self::KeyInput(_)
            | Self::Key { .. }
            | Self::TextKeyStart { .. }
            | Self::TextKeyEnd { .. }
            | Self::Commit { .. }
            | Self::Preedit { .. }
            | Self::Unmark
            | Self::TextCommand(_) => ContentActionSource::Keyboard,
            Self::Cancel => ContentActionSource::Programmatic,
        }
    }
}
impl Content {
    pub(super) fn record_actions(
        &self,
        result: &ContentResponse,
        source: ContentActionSource,
        allowed: bool,
        pending: &mut Option<Vec<PendingAction>>,
    ) {
        if allowed && let Some(pending) = pending {
            pending.extend(self.component.action_kinds(result).into_iter().map(|kind| {
                PendingAction {
                    action: ContentAction {
                        target: self.id,
                        kind,
                        source,
                    },
                    mount: self.mount,
                    payload: self.component.action_payload(kind),
                }
            }));
        }
    }
    pub(super) fn deliver_actions(
        &mut self,
        event: &ContentEvent,
        overlay: bool,
        pending: &mut Option<Vec<PendingAction>>,
    ) -> ContentResponse {
        let allowed =
            pending.is_some() && self.component.active() && !self.component.semantics().disabled;
        let result = if overlay {
            event.deliver_overlay(&mut *self.component)
        } else {
            match event.routing_event().as_ref() {
                ContentEvent::PointerMove(point) | ContentEvent::PointerUp(point) => {
                    let inside = self.curve_contains(*point);
                    self.component.pointer_event_in_region(event, inside)
                }
                _ => event.deliver(&mut *self.component),
            }
        };
        self.record_actions(&result, event.action_source(), allowed, pending);
        result
    }
}
impl ContentHost {
    /// Opt into typed action collection before dispatch. Legacy hosts do not
    /// accumulate an unconsumed queue. Re-enabling is idempotent; disabling
    /// explicitly discards pending records. This never replays control counters.
    pub fn set_action_events(&mut self, enabled: bool) {
        if enabled {
            self.actions.get_or_insert_default();
        } else {
            self.actions = None;
        }
    }
    /// Take pending actions in actual component-delivery order, then each
    /// component's declared kind order. The returned Vec owns its data: handlers
    /// may redraw, focus, remove/reinsert or dispatch another event without a
    /// borrowed queue/callback. Empty drains never replay previous actions.
    /// Undrained records belonging to an unmounted/replaced instance
    /// are discarded, including a retained instance remounted under the same ID.
    /// Capture consumer-owned business context when handling the originating
    /// event; IDs/source never encode a Session/destination.
    pub fn drain_actions(&mut self) -> Vec<ContentAction> {
        self.drain_action_records()
            .into_iter()
            .map(|record| record.action)
            .collect()
    }
    /// Same queue with original mount and owned delivery data. Later model or
    /// selection changes cannot rewrite it. Unmount discards undrained records;
    /// either drain consumes the queue, so switching drains cannot replay it.
    pub fn drain_action_records(&mut self) -> Vec<ContentActionRecord> {
        self.actions
            .as_mut()
            .map(std::mem::take)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|pending| {
                self.root
                    .find(pending.action.target)
                    .filter(|content| {
                        content.mount == pending.mount
                            && empty_state::authorized_action(&self.root, pending.action.target)
                            && notice::authorized_action(&self.root, pending.action.target)
                            && pending
                                .payload
                                .as_ref()
                                .is_none_or(|payload| self.context_menu_action_current(payload))
                            && pending
                                .payload
                                .as_ref()
                                .is_none_or(|p| content.component.action_payload_current(p))
                    })
                    .map(|_| ContentActionRecord {
                        action: pending.action,
                        mount: pending.mount,
                        payload: pending.payload,
                    })
            })
            .collect()
    }
}

#[cfg(test)]
mod freshness_tests {
    use super::*;
    use std::{cell::Cell, rc::Rc};
    #[derive(Debug)]
    struct Producer {
        revision: Rc<Cell<u64>>,
        filter: bool,
    }
    impl Component for Producer {
        fn measure(&mut self, _: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
            MeasuredChild::new(available)
        }
        fn semantics(&self) -> ContentSemantics {
            ContentSemantics::new(ContentRole::Button, "Produce")
        }
        fn accessibility_action(
            &mut self,
            _: Option<usize>,
            action: &AccessibilityAction,
        ) -> ContentResponse {
            ContentResponse {
                activated: matches!(action, AccessibilityAction::Press),
                ..Default::default()
            }
        }
        fn action_payload(&self, _: ContentActionKind) -> Option<ContentActionPayload> {
            Some(ContentActionPayload::MenuSelection(MenuSelection {
                index: 0,
                label: "item".into(),
                model_revision: self.revision.get(),
                value_path: None,
                context_request: None,
            }))
        }
        fn action_payload_current(&self, payload: &ContentActionPayload) -> bool {
            let ContentActionPayload::MenuSelection(value) = payload else {
                return false;
            };
            !self.filter || value.model_revision() == self.revision.get()
        }
    }
    #[test]
    fn stale_payload_is_filtered_at_original_once_only_drain() {
        for filter in [false, true] {
            let revision = Rc::new(Cell::new(1));
            let mut h = ContentHost::new(Content::new(
                ComponentId(740),
                Producer {
                    revision: revision.clone(),
                    filter,
                },
            ))
            .unwrap();
            h.set_action_events(true);
            h.accessibility_action(ComponentId(740), None, AccessibilityAction::Press);
            revision.set(2);
            assert_eq!(h.drain_action_records().len(), usize::from(!filter));
            assert!(h.drain_actions().is_empty());
            assert_eq!(revision.get(), 2);
        }
    }
    #[test]
    fn ordinary_owners_default_to_accepting_payloads() {
        let owner = TextContent::new("ordinary");
        let payload = ContentActionPayload::MenuSelection(MenuSelection {
            index: 0,
            label: "item".into(),
            model_revision: 1,
            value_path: None,
            context_request: None,
        });
        assert!(owner.action_payload_current(&payload));
    }
}

#[cfg(test)]
fn issue102_counter_process(name: &str) -> bool {
    if std::env::var("TEKESUI_ISSUE102_COUNTER_CHILD").as_deref() == Ok(name) {
        return true;
    }
    let output = std::process::Command::new(std::env::current_exe().expect("test executable"))
        .args(["--exact", name, "--test-threads=1", "--nocapture"])
        .env("TEKESUI_ISSUE102_COUNTER_CHILD", name)
        .output()
        .expect("isolated original allocator process");
    eprint!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "isolated original allocator witness failed: {name}"
    );
    false
}

#[cfg(test)]
mod issue102_original_allocator_tests {
    use super::*;
    use crate::extended::{Axis, Pane, Stack};
    use std::cell::Cell;
    use std::rc::Rc;
    const BUDGET: usize = 64 * 1024 * 1024;
    fn id(n: u64) -> ComponentId {
        ComponentId(n)
    }
    struct Globals {
        mount: u64,
        attachment: u64,
    }
    impl Globals {
        fn save() -> Self {
            Self {
                mount: NEXT_MOUNT.load(Ordering::Relaxed),
                attachment: crate::reading::test_attachment_counter(None),
            }
        }
        fn restore(&self) {
            NEXT_MOUNT.store(self.mount, Ordering::Relaxed);
            crate::reading::test_attachment_counter(Some(self.attachment));
        }
    }
    impl Drop for Globals {
        fn drop(&mut self) {
            self.restore();
        }
    }
    fn stack(n: u64, children: Vec<Content>) -> Content {
        Content::new(
            id(n),
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
    fn fixture() -> ContentHost {
        let mut host = ContentHost::try_new_prepared(
            split(
                Content::new(
                    id(10),
                    ScrollContent::new(
                        ScrollArea::default(),
                        Content::new(id(11), TextContent::new("History")),
                    ),
                ),
                Content::new(id(20), Button::new("B")),
            ),
            BUDGET,
        )
        .unwrap();
        host.set_scroll_reading_document(id(10), "history", 7, vec![id(11)], |_| ())
            .unwrap();
        host
    }
    fn req(host: &ContentHost, replacement: Content) -> SplitViewReplacement {
        SplitViewReplacement {
            owner: id(1),
            expected: host.split_view_expected(id(1)).unwrap(),
            pane: Pane::Leading,
            replacement,
            budget: BUDGET,
        }
    }
    #[test]
    fn issue102_real_global_hidden_reader_max_refuses_and_returns_whole_input() {
        if !issue102_counter_process(
            "composition::actions::issue102_original_allocator_tests::issue102_real_global_hidden_reader_max_refuses_and_returns_whole_input",
        ) {
            return;
        }
        let globals = Globals::save();
        let mut host = fixture();
        let expected = host.split_view_expected(id(1)).unwrap();
        let mut config = host
            .get(id(1))
            .unwrap()
            .component::<SplitViewContent>()
            .unwrap()
            .config()
            .unwrap();
        config.collapsed = Some(Pane::Leading);
        host.update_split_view(id(1), expected, config, BUDGET)
            .unwrap();
        let input = req(&host, Content::new(id(30), Button::new("new")));
        let input_mount = input.replacement.mount;
        let before = format!("{host:?}");
        let mount = NEXT_MOUNT.load(Ordering::Relaxed);
        crate::reading::test_attachment_counter(Some(u64::MAX));
        let rejected = host.replace_split_view_pane(input).unwrap_err();
        assert_eq!(rejected.error, SplitViewError::ResourceCapacity);
        assert_eq!(rejected.input.replacement.mount, input_mount);
        assert_eq!(format!("{host:?}"), before);
        assert_eq!(NEXT_MOUNT.load(Ordering::Relaxed), mount);
        assert_eq!(crate::reading::test_attachment_counter(None), u64::MAX);
        eprintln!(
            "ISSUE102 real original NEXT_ATTACHMENT=MAX hidden-reader refusal; live/input/mount allocator unchanged"
        );
        globals.restore();
        drop(rejected.input);
        drop(host);
    }
    #[test]
    fn issue102_original_reader_hole_then_success_never_publishes_or_reuses_hole() {
        if !issue102_counter_process(
            "composition::actions::issue102_original_allocator_tests::issue102_original_reader_hole_then_success_never_publishes_or_reuses_hole",
        ) {
            return;
        }
        let globals = Globals::save();
        let mut host = fixture();
        let large = stack(
            30,
            (31..45)
                .map(|n| Content::new(id(n), Button::new("large")))
                .collect(),
        );
        let small = Content::new(id(50), Button::new("small"));
        let input = req(&host, large);
        let before = format!("{host:?}");
        let start = crate::reading::test_attachment_counter(None);
        NEXT_MOUNT.store(u64::MAX - 10, Ordering::Relaxed);
        let rejected = host.replace_split_view_pane(input).unwrap_err();
        assert_eq!(rejected.error, SplitViewError::ResourceCapacity);
        assert_eq!(format!("{host:?}"), before);
        assert_eq!(NEXT_MOUNT.load(Ordering::Relaxed), u64::MAX - 10);
        assert_eq!(crate::reading::test_attachment_counter(None), start + 1);
        // No reset between observations: a smaller preconstructed request fits.
        let input = req(&host, small);
        let old = host.replace_split_view_pane(input).unwrap();
        let version = old
            .component::<ScrollContent>()
            .unwrap()
            .reading_document_version()
            .unwrap();
        assert_eq!(version.attachment(), start + 1);
        assert_eq!(version.key(), "history");
        assert_eq!(version.revision(), 7);
        assert_eq!(crate::reading::test_attachment_counter(None), start + 2);
        assert_eq!(NEXT_MOUNT.load(Ordering::Relaxed), u64::MAX - 7);
        eprintln!(
            "ISSUE102 resource A success/B refusal: attachment hole={start}; successful returned reader={} (no reuse), no refusal prefix",
            version.attachment()
        );
        globals.restore();
        drop(rejected.input);
        drop(old);
        drop(host);
    }
    #[test]
    fn issue102_real_global_keep_at_both_max_and_constructor_plus_mount_costs() {
        if !issue102_counter_process(
            "composition::actions::issue102_original_allocator_tests::issue102_real_global_keep_at_both_max_and_constructor_plus_mount_costs",
        ) {
            return;
        }
        let globals = Globals::save();
        let mut host = fixture();
        let expected = host.split_view_expected(id(1)).unwrap();
        let config = host
            .get(id(1))
            .unwrap()
            .component::<SplitViewContent>()
            .unwrap()
            .config()
            .unwrap();
        NEXT_MOUNT.store(u64::MAX, Ordering::Relaxed);
        crate::reading::test_attachment_counter(Some(u64::MAX));
        let before = format!("{host:?}");
        assert!(
            !host
                .update_split_view(id(1), expected, config, BUDGET)
                .unwrap()
        );
        assert_eq!(format!("{host:?}"), before);
        assert_eq!(NEXT_MOUNT.load(Ordering::Relaxed), u64::MAX);
        assert_eq!(crate::reading::test_attachment_counter(None), u64::MAX);
        globals.restore();
        drop(host);
        let leading = Content::new(id(100), Button::new("A"));
        let trailing = Content::new(id(101), Button::new("B"));
        let input = SplitViewConstruction {
            config: SplitViewConfig::default(),
            slots: SplitViewSlotIds {
                leading: id(102),
                trailing: id(103),
                divider: id(104),
            },
            leading,
            trailing,
            budget: BUDGET,
        };
        NEXT_MOUNT.store(u64::MAX - 2, Ordering::Relaxed);
        let rejected = SplitViewContent::try_new(input).unwrap_err();
        assert_eq!(NEXT_MOUNT.load(Ordering::Relaxed), u64::MAX - 2);
        NEXT_MOUNT.store(u64::MAX - 3, Ordering::Relaxed);
        let split = SplitViewContent::try_new(*rejected.input).unwrap();
        assert_eq!(NEXT_MOUNT.load(Ordering::Relaxed), u64::MAX);
        globals.restore();
        let root = Content::new(id(105), split);
        let count = super::super::prepared::count(&root).unwrap();
        NEXT_MOUNT.store(u64::MAX - (count as u64 - 1), Ordering::Relaxed);
        let before = NEXT_MOUNT.load(Ordering::Relaxed);
        let rejected = ContentHost::try_new_prepared(root, BUDGET).unwrap_err();
        assert_eq!(NEXT_MOUNT.load(Ordering::Relaxed), before);
        globals.restore();
        drop(rejected.input);
        eprintln!(
            "ISSUE102 original allocator Keep MAX; internal constructors=3; full initial Mounted={count}; refusal no prefix"
        );
    }
    #[derive(Debug)]
    struct DynamicDrop(Rc<Cell<usize>>);
    impl Drop for DynamicDrop {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
            let _owned = Content::new(id(999), TextContent::new("ordinary dynamic Drop"));
        }
    }
    impl Component for DynamicDrop {
        fn measure(
            &mut self,
            _: &mut dyn ButtonTextRenderer,
            a: Size,
        ) -> crate::layout::MeasuredChild {
            crate::layout::MeasuredChild::new(a)
        }
    }
    #[test]
    fn issue102_earliest_missing_expected_budget_and_unknown_return_complete_input_without_sdk_drop()
     {
        if !issue102_counter_process(
            "composition::actions::issue102_original_allocator_tests::issue102_earliest_missing_expected_budget_and_unknown_return_complete_input_without_sdk_drop",
        ) {
            return;
        }
        let mut host = fixture();
        let drops = Rc::new(Cell::new(0));
        for gate in 0..4 {
            let mut input = req(&host, Content::new(id(30), DynamicDrop(drops.clone())));
            match gate {
                0 => input.owner = id(777),
                1 => input.expected.mount += 1,
                2 => input.budget = 0,
                _ => {}
            }
            let mount = NEXT_MOUNT.load(Ordering::Relaxed);
            let original = input.replacement.mount;
            let before = host.split_view_expected(id(1));
            let rejected = host.replace_split_view_pane(input).unwrap_err();
            assert_eq!(drops.get(), gate);
            assert_eq!(rejected.input.replacement.mount, original);
            assert_eq!(NEXT_MOUNT.load(Ordering::Relaxed), mount);
            assert_eq!(host.split_view_expected(id(1)), before);
            drop(rejected.input);
            assert_eq!(drops.get(), gate + 1);
        }
    }
    #[test]
    fn issue102_constructor_returns_both_owned_inputs_before_dynamic_drop() {
        if !issue102_counter_process(
            "composition::actions::issue102_original_allocator_tests::issue102_constructor_returns_both_owned_inputs_before_dynamic_drop",
        ) {
            return;
        }
        let drops = Rc::new(Cell::new(0));
        let leading = Content::new(id(70), DynamicDrop(drops.clone()));
        let trailing = Content::new(id(80), DynamicDrop(drops.clone()));
        let mounts = (leading.mount, trailing.mount);
        let counter = NEXT_MOUNT.load(Ordering::Relaxed);
        let input = SplitViewConstruction {
            config: SplitViewConfig {
                ratio: f32::NAN,
                ..Default::default()
            },
            slots: SplitViewSlotIds {
                leading: id(2),
                trailing: id(3),
                divider: id(4),
            },
            leading,
            trailing,
            budget: BUDGET,
        };
        let rejected = SplitViewContent::try_new(input).unwrap_err();
        assert_eq!(rejected.error, SplitViewError::InvalidConfig);
        assert_eq!(drops.get(), 0);
        assert_eq!(
            (rejected.input.leading.mount, rejected.input.trailing.mount),
            mounts
        );
        assert_eq!(
            (rejected.input.leading.id, rejected.input.trailing.id),
            (id(70), id(80))
        );
        assert_eq!(NEXT_MOUNT.load(Ordering::Relaxed), counter);
        drop(rejected.input);
        assert_eq!(drops.get(), 2);
        assert_eq!(NEXT_MOUNT.load(Ordering::Relaxed), counter + 2);
    }
    #[test]
    fn issue102_actual_reading_callback_earliest_guard_retains_dynamic_drop_input() {
        if !issue102_counter_process(
            "composition::actions::issue102_original_allocator_tests::issue102_actual_reading_callback_earliest_guard_retains_dynamic_drop_input",
        ) {
            return;
        }
        let mut host = fixture();
        let drops = Rc::new(Cell::new(0));
        let input = req(&host, Content::new(id(70), DynamicDrop(drops.clone())));
        let input_mount = input.replacement.mount;
        let (_, rejected) = host
            .set_scroll_reading_document(id(10), "history", 8, vec![id(11)], |host| {
                assert!(host.reading_transaction);
                let before = format!("{host:?}");
                let counter = NEXT_MOUNT.load(Ordering::Relaxed);
                let attachment = crate::reading::test_attachment_counter(None);
                let rejected = host.replace_split_view_pane(input).unwrap_err();
                assert_eq!(
                    rejected.error,
                    SplitViewError::Composition(CompositionError::ReadingTransactionInProgress)
                );
                assert_eq!(drops.get(), 0);
                assert_eq!(rejected.input.replacement.mount, input_mount);
                assert_eq!(format!("{host:?}"), before);
                assert_eq!(NEXT_MOUNT.load(Ordering::Relaxed), counter);
                assert_eq!(crate::reading::test_attachment_counter(None), attachment);
                rejected
            })
            .unwrap();
        assert_eq!(drops.get(), 0);
        drop(rejected.input);
        assert_eq!(drops.get(), 1);
    }
    #[derive(Debug)]
    struct FiniteDrop {
        drops: Rc<Cell<usize>>,
        hooks: Rc<Cell<usize>>,
    }
    impl Drop for FiniteDrop {
        fn drop(&mut self) {
            self.drops.set(self.drops.get() + 1);
            drop(Content::new(id(999), TextContent::new("caller Drop")));
        }
    }
    impl Component for FiniteDrop {
        fn table_transaction_compatibility(&self) -> TableTransactionCompatibility {
            TableTransactionCompatibility::StableHooks
        }
        fn prepared_effects_work(&self, n: usize) -> Result<usize, PreparedEffectsError> {
            PreparedEffects::local_work(n)
        }
        fn prepare_effects(
            &self,
            h: &[PreparedHook],
        ) -> Result<PreparedEffects, PreparedEffectsError> {
            Ok(PreparedEffects::local(self, h))
        }
        fn measure(
            &mut self,
            _: &mut dyn ButtonTextRenderer,
            a: Size,
        ) -> crate::layout::MeasuredChild {
            crate::layout::MeasuredChild::new(a)
        }
        fn lifecycle(&mut self, _: Lifecycle) {
            self.hooks.set(self.hooks.get() + 1);
        }
        fn event(&mut self, _: &ContentEvent) -> ContentResponse {
            self.hooks.set(self.hooks.get() + 1);
            ContentResponse::default()
        }
    }
    #[test]
    fn issue102_final_reserved_expected_and_owner_defense_preserves_live_and_input() {
        if !issue102_counter_process(
            "composition::actions::issue102_original_allocator_tests::issue102_final_reserved_expected_and_owner_defense_preserves_live_and_input",
        ) {
            return;
        }
        let mut host = fixture();
        let drops = Rc::new(Cell::new(0));
        let hooks = Rc::new(Cell::new(0));
        for stale_owner in [false, true] {
            let mut input = req(
                &host,
                Content::new(
                    id(70),
                    FiniteDrop {
                        drops: drops.clone(),
                        hooks: hooks.clone(),
                    },
                ),
            );
            let plan = host.prepare_split_view_replacement(&input).unwrap();
            if stale_owner {
                input.owner = id(777);
            } else {
                input.expected.mount += 1;
            }
            let owner = input.owner;
            let expected = input.expected;
            let input_mount = input.replacement.mount;
            let before = format!("{host:?}");
            let mount = NEXT_MOUNT.load(Ordering::Relaxed);
            let attachment = crate::reading::test_attachment_counter(None);
            let old_drops = drops.get();
            // Directly exercise the SAME final boundary after preparing a valid plan.
            // The synchronous public entry has exclusive Host ownership; no fake concurrent callback.
            let rejected = host.commit_split_view_replacement(input, plan).unwrap_err();
            assert_eq!(rejected.error, SplitViewError::StaleExpected);
            assert_eq!(format!("{host:?}"), before);
            assert_eq!(NEXT_MOUNT.load(Ordering::Relaxed), mount + 3);
            assert_eq!(
                crate::reading::test_attachment_counter(None),
                attachment + 1
            );
            assert_eq!(rejected.input.owner, owner);
            assert_eq!(rejected.input.expected, expected);
            assert_eq!(rejected.input.replacement.mount, input_mount);
            assert_eq!(drops.get(), old_drops);
            assert_eq!(hooks.get(), 0);
            drop(rejected.input);
            assert_eq!(drops.get(), old_drops + 1);
        }
    }
    #[test]
    fn issue102_original_capture_epoch_max_refuses_before_divider_effect_and_near_max_succeeds() {
        struct Text;
        impl ButtonTextRenderer for Text {
            fn measure(&mut self, _: &str, _: tekes_ui_tokens::proposal::TextRole, _: f32) -> Size {
                Size::new(20., 20.)
            }
            fn draw(
                &mut self,
                _: &mut Scene,
                _: &str,
                _: tekes_ui_tokens::proposal::TextRole,
                _: Rect,
                _: crate::ButtonTextAlign,
                _: tekes_ui_tokens::Rgba,
            ) {
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
        let frame = Rect::new(0., 0., 600., 300.);
        host.render_checked(
            &mut Scene::new(),
            &mut Text,
            frame,
            frame,
            tekes_ui_tokens::Theme::LIGHT.into(),
        )
        .unwrap();
        let point = host
            .get(id(1))
            .unwrap()
            .component::<SplitViewContent>()
            .unwrap()
            .layout()
            .unwrap()
            .divider
            .frame()
            .center();
        host.capture_epoch = u64::MAX;
        let before = format!("{:?}", host.root);
        let focus = host.focus_epoch;
        let response = host.event(&ContentEvent::PointerDown(point));
        // Ordinary event bookkeeping invalidates reveal input, even for an ignored event.
        // The MAX guard prevents ALL divider/model/focus/capture effects before acquisition.
        assert!(!response.changed);
        assert_eq!(format!("{:?}", host.root), before);
        assert_eq!(host.focus_epoch, focus);
        assert_eq!(host.capture_epoch, u64::MAX);
        assert!(host.pointer_capture().is_none());
        host.capture_epoch = u64::MAX - 1;
        host.event(&ContentEvent::PointerDown(point));
        assert_eq!(host.capture_epoch, u64::MAX);
        assert_eq!(host.pointer_capture().unwrap().0, id(4));
        host.cancel_pointer_capture();
        assert_eq!(host.capture_epoch, u64::MAX);
        let expected = host.split_view_expected(id(1)).unwrap();
        let mut config = expected.config;
        config.ratio = 0.3;
        host.update_split_view(id(1), expected, config, BUDGET)
            .unwrap();
        assert_eq!(host.capture_epoch, u64::MAX);
        let replacement = req(&host, Content::new(id(30), Button::new("new")));
        host.replace_split_view_pane(replacement).unwrap();
        assert_eq!(host.capture_epoch, u64::MAX);
    }
}

#[cfg(test)]
mod issue102_sequence_tests {
    use super::super::prepared::TreePlan;
    use super::*;
    use std::sync::atomic::Ordering;
    #[test]
    fn issue102_ordered_repeated_blur_help_and_lifecycle_use_successive_state() {
        let mut field = TextFieldContent::new("Composer", "");
        field.set_focus(true, false);
        field.event(&ContentEvent::Preedit {
            text: "中".into(),
            selection: 1..1,
            replacement: None,
            clauses: vec![],
        });
        field.surface.input.test_edit_epoch(u64::MAX - 1);
        let mut node = Content::new(ComponentId(1100), field).with_help("owned help");
        node.help_revision = u64::MAX - 1;
        let mut plan = TreePlan::default();
        for focused in [false, true, false] {
            plan.add(
                &node,
                &[],
                PreparedHook::retained(PreparedHookKind::Focus {
                    focused,
                    visible: false,
                }),
            )
            .unwrap();
        }
        plan.subtree(&node, &[], Lifecycle::Unmounted).unwrap();
        plan.subtree(&node, &[], Lifecycle::Mounted).unwrap();
        plan.subtree(&node, &[], Lifecycle::Unmounted).unwrap();
        plan.prepare(&node).unwrap();
        let mut attachments = crate::reading::reserve_attachments(plan.readers()).unwrap();
        let mut mounts = reserve_mounts(plan.mounts).unwrap();
        plan.bind(&mut attachments);
        plan.commit(&mut node, &mut mounts);
        let field = node.component::<TextFieldContent>().unwrap();
        assert_eq!(field.surface.input.edit_revision().1, u64::MAX);
        assert!(field.surface.input.marked.is_none());
        assert_eq!(node.help_revision, u64::MAX);
        assert!(node.help.is_none());
        assert!(mounts.exhausted());
        eprintln!(
            "ISSUE102 ordered repeated blur/Cancel/unmount: single legal unmark + help clear, no false second exhaustion"
        );
    }
    #[test]
    fn issue102_owner_capacity_refuses_before_hook_including_hidden_composition() {
        let mut field = TextFieldContent::new("Composer", "");
        field.set_focus(true, false);
        field.event(&ContentEvent::Preedit {
            text: "中".into(),
            selection: 1..1,
            replacement: None,
            clauses: vec![],
        });
        field.surface.input.marked = None;
        field.surface.input.test_edit_epoch(u64::MAX);
        let node = Content::new(ComponentId(1101), field);
        let before = format!(
            "{:?}",
            node.component::<TextFieldContent>().unwrap().surface.input
        );
        let mut plan = TreePlan::default();
        plan.subtree(&node, &[], Lifecycle::Unmounted).unwrap();
        assert_eq!(
            plan.prepare(&node),
            Err(PreparedEffectsError::CounterCapacity)
        );
        assert_eq!(
            format!(
                "{:?}",
                node.component::<TextFieldContent>().unwrap().surface.input
            ),
            before
        );
    }
    #[test]
    fn issue102_reader_whole_sequence_cancel_then_two_unmounts_matches_original_writes() {
        if !issue102_counter_process(
            "composition::actions::issue102_sequence_tests::issue102_reader_whole_sequence_cancel_then_two_unmounts_matches_original_writes",
        ) {
            return;
        }
        let saved = crate::reading::test_attachment_counter(None);
        let mut host = ContentHost::new(Content::new(
            ComponentId(1200),
            StackContent {
                model: crate::extended::Stack {
                    axis: crate::extended::Axis::Vertical,
                    gap: 0.,
                },
                flexible_child: None,
                children: vec![Content::new(
                    ComponentId(1201),
                    ScrollContent::new(
                        ScrollArea::default(),
                        Content::new(ComponentId(1202), TextContent::new("history")),
                    ),
                )],
            },
        ))
        .unwrap();
        host.set_scroll_reading_document(
            ComponentId(1201),
            "reader",
            9,
            vec![ComponentId(1202)],
            |_| (),
        )
        .unwrap();
        let mut node = host.remove(ComponentId(1201)).unwrap();
        let s = node.component_mut::<ScrollContent>().unwrap();
        s.scroll.overscroll = Point::new(0., 8.);
        let mut expected = s.reading.clone().unwrap();
        let mut scroll = s.scroll.clone();
        scroll.cancel();
        let start = crate::reading::test_attachment_counter(None);
        let mut plan = TreePlan::default();
        plan.add(&node, &[], PreparedHook::retained(PreparedHookKind::Cancel))
            .unwrap();
        plan.subtree(&node, &[], Lifecycle::Unmounted).unwrap();
        plan.subtree(&node, &[], Lifecycle::Mounted).unwrap();
        plan.subtree(&node, &[], Lifecycle::Unmounted).unwrap();
        plan.prepare(&node).unwrap();
        assert_eq!(plan.readers(), 2);
        assert_eq!(crate::reading::test_attachment_counter(None), start);
        for n in [start, start + 1] {
            let writes = super::super::content_reading::prepare_reader_writes(&expected, &scroll);
            super::super::content_reading::apply_reader_writes(
                &mut expected,
                writes,
                crate::ReadingDocumentVersion::from_reserved("reader".into(), 9, n),
            );
        }
        let mut attachments = crate::reading::reserve_attachments(2).unwrap();
        let mut mounts = reserve_mounts(plan.mounts).unwrap();
        plan.bind(&mut attachments);
        plan.commit(&mut node, &mut mounts);
        let actual = node.component::<ScrollContent>().unwrap();
        assert_eq!(
            format!("{:?}", actual.reading.as_ref().unwrap()),
            format!("{expected:?}")
        );
        assert_eq!(actual.scroll.overscroll, Point::default());
        assert_eq!(crate::reading::test_attachment_counter(None), start + 2);
        assert!(mounts.exhausted());
        drop(node);
        drop(host);
        crate::reading::test_attachment_counter(Some(saved));
        // This process lane runs only issue102 tests, --test-threads=1.
        assert!(NEXT_MOUNT.load(Ordering::Relaxed) > 0);
    }
}

#[cfg(test)]
mod issue102_nested_reader_tests {
    use super::super::prepared::TreePlan;
    use super::*;
    #[test]
    fn issue102_hidden_readers_and_resource_ticket_order_follow_actual_hook_sequence() {
        let name = "composition::actions::issue102_nested_reader_tests::issue102_hidden_readers_and_resource_ticket_order_follow_actual_hook_sequence";
        if !issue102_counter_process(name) {
            return;
        }
        let mut inner = ScrollContent::new(
            ScrollArea::default(),
            Content::new(ComponentId(1403), TextContent::new("inner")),
        );
        inner
            .initialize_reading_document("inner", 1, vec![ComponentId(1403)])
            .unwrap();
        let mut outer = ScrollContent::new(
            ScrollArea::default(),
            Content::new(ComponentId(1402), inner),
        );
        outer
            .initialize_reading_document("outer", 1, vec![ComponentId(1402)])
            .unwrap();
        let mut root = Content::new(ComponentId(1401), outer);
        let mut plan = TreePlan::default();
        // Simulate a captured descendant being prepared before its ancestor.
        plan.add(
            &root,
            &[0],
            PreparedHook::retained(PreparedHookKind::CancelPointer),
        )
        .unwrap();
        plan.subtree(&root, &[], Lifecycle::Unmounted).unwrap();
        plan.subtree(&root, &[], Lifecycle::Mounted).unwrap();
        plan.subtree(&root, &[], Lifecycle::Unmounted).unwrap();
        plan.prepare(&root).unwrap();
        assert_eq!(plan.readers(), 4);
        let before = format!("{root:?}");
        let mount = NEXT_MOUNT.load(Ordering::Relaxed);
        let saved = crate::reading::test_attachment_counter(Some(u64::MAX - 3));
        assert!(crate::reading::reserve_attachments(plan.readers()).is_err());
        assert_eq!(crate::reading::test_attachment_counter(None), u64::MAX - 3);
        assert_eq!(NEXT_MOUNT.load(Ordering::Relaxed), mount);
        assert_eq!(format!("{root:?}"), before);
        crate::reading::test_attachment_counter(Some(saved));
        let first = saved;
        let mut reading = crate::reading::reserve_attachments(4).unwrap();
        let mut mounts = reserve_mounts(plan.mounts).unwrap();
        plan.bind(&mut reading);
        plan.commit(&mut root, &mut mounts);
        plan.assert_consumed();
        let outer = root.component::<ScrollContent>().unwrap();
        let inner = outer.children()[0].component::<ScrollContent>().unwrap();
        assert_eq!(
            outer.reading_document_version().unwrap().attachment(),
            first + 2
        );
        assert_eq!(
            inner.reading_document_version().unwrap().attachment(),
            first + 3
        );
        assert!(mounts.exhausted());
        eprintln!(
            "ISSUE102 two hidden readers x two Unmounted: fresh IDs follow parent/child callback order, not node registration order"
        );
    }
}
