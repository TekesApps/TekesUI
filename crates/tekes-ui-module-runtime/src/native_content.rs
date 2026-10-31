//! Native callback/query projection onto the child-owned original ContentHost.
//! Snapshots fence identity and geometry; they do not own editor or focus state.
use crate::{
    NativeViewOptions, ProcessComponent,
    component::{Session, content as wire},
    container_semantics::RemoteSemantics,
};
use anyhow::{Context, Result, ensure};
use std::time::Duration;
use tekes_ui_platform_macos as native;
use tekes_ui_scene::Rect;

#[derive(Clone, Debug, PartialEq)]
struct Stamp {
    session: Session,
    view: u64,
    viewport: u64,
    frame: Rect,
    clip: Rect,
    curve: Option<crate::component::curve::QueryStamp>,
}

/// A read-only result from one original child, not a locally editable replica.
/// Retain this with event-time native callbacks. A new owner/viewport/session
/// must never be substituted for an old callback.
#[derive(Clone, Debug)]
pub struct ProcessContentSnapshot {
    stamp: Stamp,
    reply: wire::Reply,
    text: Option<native::TextInputState>,
}
impl ProcessContentSnapshot {
    pub fn reply(&self) -> &wire::Reply {
        &self.reply
    }
    pub(crate) fn semantic_attachment(&self) -> (Session, u64) {
        (self.stamp.session.clone(), self.stamp.view)
    }
    pub(crate) fn belongs_to(&self, session: &Session, view: u64) -> bool {
        self.stamp.session == *session && self.stamp.view == view
    }
    /// The immutable projection's Curve identity, when it was produced from an
    /// exact verified frame. This is diagnostic identity only; mutable calls
    /// still require the live `ProcessContentBridge` and child admission.
    pub fn curve_stamp(&self) -> Option<&crate::component::curve::QueryStamp> {
        self.stamp.curve.as_ref()
    }
    pub(crate) fn from_curve(
        process: &ProcessComponent,
        view: u64,
        viewport: u64,
        frame: Rect,
        clip: Rect,
        curve: crate::component::curve::QueryStamp,
        reply: wire::Reply,
    ) -> Result<Self> {
        ensure!(
            curve.view == view
                && curve.viewport == viewport
                && reply.view == view
                && reply.viewport == viewport,
            "stale Curve Content projection"
        );
        let stamp = Stamp {
            session: process.session().clone(),
            view,
            viewport,
            frame,
            clip,
            curve: Some(curve),
        };
        let text = project(&stamp, &reply)?;
        Ok(Self { stamp, reply, text })
    }
    pub(crate) fn native_binding(&self) -> Option<Vec<u8>> {
        self.reply.focused?;
        // Length-prefix the original scope and append the child identity as
        // bytes. Nested containers grow linearly without hashing identities.
        let scope = serde_json::to_vec(&(
            &self.stamp.session,
            self.stamp.view,
            self.reply.focused,
            self.reply.focus_epoch,
            self.reply.text_binding.is_some(),
        ))
        .ok()?;
        let mut binding = (scope.len() as u64).to_le_bytes().to_vec();
        binding.extend(scope);
        if let Some(child) = &self.reply.text_binding {
            binding.extend(child);
        }
        Some(binding)
    }
    /// Already in parent content-view logical points. Never multiply by backing
    /// scale or reshape the child text when passing this to Window.
    pub fn text_input_state(&self) -> Option<&native::TextInputState> {
        self.text.as_ref()
    }
}

/// Borrow the process only for a synchronous native delivery/query. The caller
/// supplies its current acknowledged allocation, not a separate text transform.
/// Timeouts return errors; inputs are never retried or replayed here.
pub struct ProcessContentBridge<'a> {
    process: &'a ProcessComponent,
    stamp: Stamp,
    timeout: Duration,
    host_semantics: Option<(&'a RemoteSemantics, Rect, Rect)>,
}
impl<'a> ProcessContentBridge<'a> {
    /// Preserve the original ordinary-consumer constructor. Curve identity is
    /// supplied only by the existing ProcessContainer owner below.
    pub fn new(
        process: &'a ProcessComponent,
        view: &NativeViewOptions,
        viewport: u64,
        frame: Rect,
        clip: Rect,
        timeout: Duration,
    ) -> Result<Self> {
        Self::new_with_curve(process, view, viewport, frame, clip, None, timeout)
    }
    pub(crate) fn new_with_curve(
        process: &'a ProcessComponent,
        view: &NativeViewOptions,
        viewport: u64,
        frame: Rect,
        clip: Rect,
        curve: Option<crate::component::curve::QueryStamp>,
        timeout: Duration,
    ) -> Result<Self> {
        ensure!(
            process.window_identity() == Some(view.window)
                && view.window != 0
                && view.view != 0
                && viewport != 0,
            "invalid native Content attachment"
        );
        let capabilities = process
            .view_capabilities()
            .ok_or_else(|| anyhow::anyhow!("native Content unavailable"))?;
        ensure!(
            capabilities.keyboard
                || capabilities.text_ime
                || capabilities.content_pointer
                || capabilities.scroll
                || capabilities.accessibility,
            "native Content not negotiated"
        );
        ensure!(
            valid_rect(frame)
                && valid_rect(clip)
                && frame.intersection(&clip).is_some_and(|r| !r.is_empty()),
            "invalid native Content allocation"
        );
        ensure!(
            !timeout.is_zero() && timeout <= Duration::from_secs(10),
            "invalid Content query timeout"
        );
        Ok(Self {
            process,
            stamp: Stamp {
                session: process.session().clone(),
                view: view.view,
                viewport,
                frame,
                clip,
                curve,
            },
            timeout,
            host_semantics: None,
        })
    }
    /// Bind collection AX to the owning ProcessContainer's current semantic
    /// projection and final host clip. The borrow keeps that owner unchanged
    /// for this bridge's lifetime.
    pub(crate) fn with_host_semantics(
        mut self,
        semantics: &'a RemoteSemantics,
        frame: Rect,
        clip: Rect,
    ) -> Self {
        self.host_semantics = Some((semantics, frame, clip));
        self
    }
    pub fn snapshot(&self) -> Result<ProcessContentSnapshot> {
        if let Some(stamp) = &self.stamp.curve {
            return self.curve_query(crate::component::curve::ReadOnlyQueryWire::SemanticTree {
                stamp: stamp.clone(),
            });
        }
        self.send(None, None, 0, None, wire::Input::Snapshot)
    }
    /// Query original semantic owners without acquiring focus or dirtying paint.
    pub fn semantics(&self) -> Result<ProcessContentSnapshot> {
        if let Some(stamp) = &self.stamp.curve {
            return self.curve_query(crate::component::curve::ReadOnlyQueryWire::SemanticTree {
                stamp: stamp.clone(),
            });
        }
        self.send(None, None, 0, None, wire::Input::Semantics)
    }
    /// The target must come from this exact stamped semantic query. The child
    /// validates current active scope/mount/part and runs its original AX path.
    pub fn accessibility_action(
        &self,
        previous: &ProcessContentSnapshot,
        target: wire::Target,
        part: Option<usize>,
        action: wire::AccessibilityAction,
    ) -> Result<ProcessContentSnapshot> {
        let nodes = previous
            .reply
            .semantics
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("semantic query snapshot required"))?;
        let node = nodes
            .iter()
            .find(|n| {
                n.target == target && part.is_none_or(|i| n.parts.iter().any(|p| p.index == i))
            })
            .ok_or_else(|| anyhow::anyhow!("target not in original semantic snapshot"))?;
        let binding = if let Some(part) = part {
            node.parts
                .iter()
                .find(|p| p.index == part)
                .and_then(|p| p.text_layout.as_ref())
                .map(|s| s.binding.clone())
        } else {
            node.text_binding.clone()
        };
        self.request(
            previous,
            wire::Input::Accessibility {
                target,
                part,
                binding,
                action,
            },
        )
    }
    /// Internal re-projection after allocation acknowledgement, not an input
    /// retry. Public retained callbacks still require their exact old stamp.
    pub(crate) fn refresh_allocation(
        &self,
        previous: &ProcessContentSnapshot,
    ) -> Result<ProcessContentSnapshot> {
        if previous.stamp == self.stamp {
            return Ok(previous.clone());
        }
        ensure!(
            previous.stamp.session == self.stamp.session && previous.stamp.view == self.stamp.view,
            "stale native Content session/View"
        );
        let current = self.snapshot()?;
        ensure!(
            previous.reply.focused == current.reply.focused
                && previous.reply.focus_epoch == current.reply.focus_epoch
                && previous.reply.text_binding == current.reply.text_binding
                && previous.reply.captured == current.reply.captured,
            "changed Content lease during allocation"
        );
        Ok(current)
    }
    /// Read the original declared reader and mounted items. This neither focuses
    /// the reader nor consumes a pending paint invalidation.
    pub fn reading_state(
        &self,
        previous: &ProcessContentSnapshot,
        reader: u64,
    ) -> Result<ProcessContentSnapshot> {
        if let Some(stamp) = &self.stamp.curve {
            ensure!(
                previous.stamp == self.stamp,
                "stale native Content snapshot"
            );
            let reader_mount = previous
                .reply
                .semantics
                .as_deref()
                .and_then(|nodes| nodes.iter().find(|node| node.target.id == reader))
                .map(|node| node.target.mount)
                .ok_or_else(|| {
                    anyhow::anyhow!("reader is absent from original semantic snapshot")
                })?;
            return self.curve_query(crate::component::curve::ReadOnlyQueryWire::ReadingState {
                stamp: stamp.clone(),
                reader_id: reader,
                reader_mount,
            });
        }
        self.request(previous, wire::Input::ReadingState { reader })
    }
    /// Retain the exact returned document/item identities with asynchronous work.
    /// The child rejects replacement, stale revision and inactive overlay scope.
    pub fn reveal_reading_item(
        &self,
        previous: &ProcessContentSnapshot,
        item: wire::Target,
        options: wire::RevealOptions,
    ) -> Result<ProcessContentSnapshot> {
        // Keep the reading identity attached to its original process stamp.
        // A separate data-only state from another child is not a callback lease.
        let state = previous
            .reply
            .reading
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("reading query snapshot required"))?;
        self.request(
            previous,
            wire::Input::RevealReadingItem {
                reader: state.reader,
                document: state.document.clone(),
                item,
                options,
            },
        )
    }
    /// Explicit focus intent uses original targets from the child. Composition
    /// transfer is accepted/rejected by the original ContentHost.
    pub fn focus(
        &self,
        previous: &ProcessContentSnapshot,
        target: Option<wire::Target>,
        visible: bool,
    ) -> Result<ProcessContentSnapshot> {
        self.request(previous, wire::Input::Focus { target, visible })
    }
    pub fn request(
        &self,
        previous: &ProcessContentSnapshot,
        input: wire::Input,
    ) -> Result<ProcessContentSnapshot> {
        ensure!(
            previous.stamp == self.stamp,
            "stale native Content snapshot"
        );
        if let wire::Input::Accessibility {
            target,
            part,
            action,
            ..
        } = &input
        {
            let snapshot_role = previous
                .reply
                .semantics
                .as_deref()
                .and_then(|nodes| nodes.iter().find(|node| node.target == *target))
                .and_then(|node| match part {
                    Some(index) => node
                        .parts
                        .iter()
                        .find(|candidate| candidate.index == *index)
                        .map(|candidate| &candidate.semantics.role),
                    None => Some(&node.semantics.role),
                });
            let snapshot_collection = snapshot_role.is_some_and(is_collection_role);
            let current_admission =
                self.host_semantics
                    .as_ref()
                    .and_then(|(semantics, frame, clip)| {
                        semantics.collection_action_admission(*target, *part, *frame, *clip, action)
                    });
            if snapshot_collection || current_admission.is_some() {
                ensure!(
                    current_admission == Some(true),
                    "collection AX requires a current visible owner Host projection"
                );
            }
        }
        ensure!(
            self.stamp.curve.is_none()
                || !matches!(
                    &input,
                    wire::Input::Snapshot
                        | wire::Input::Semantics
                        | wire::Input::ReadingState { .. }
                ),
            "Curve read-only queries use the stamped query lane"
        );
        let owner = if matches!(input, wire::Input::Pointer(_)) {
            previous
                .reply
                .captured
                .map(|c| c.target)
                .or(previous.reply.focused)
        } else {
            previous.reply.focused
        };
        let capture = matches!(input, wire::Input::Pointer(_))
            .then_some(previous.reply.captured)
            .flatten();
        self.send(
            owner,
            capture,
            previous.reply.focus_epoch,
            previous.reply.text_binding.clone(),
            input,
        )
    }
    /// Translate the original host pointer data into child logical coordinates.
    /// The child decides hit, focus, capture, consumption and bubbling.
    pub fn pointer(
        &self,
        previous: &ProcessContentSnapshot,
        event: &tekes_ui_controls::composition::PointerEvent,
    ) -> Result<ProcessContentSnapshot> {
        self.request(previous, pointer_input(event, self.stamp.frame)?)
    }

    /// Project one original parent-space wheel into this acknowledged child.
    /// Delta/phase/time remain unchanged; no focus, capture or scroll model lives here.
    pub fn scroll(
        &self,
        previous: &ProcessContentSnapshot,
        mut event: wire::ScrollEvent,
    ) -> Result<ProcessContentSnapshot> {
        event = scroll_to_child(event, self.stamp.frame)?;
        self.request(previous, wire::Input::Scroll(event))
    }
    /// Text callbacks and key provenance are forwarded once, without editing,
    /// synthesizing submit, or guessing which input-method key was consumed.
    /// Scroll uses the original parent-to-child allocation; AX/lifecycle remain
    /// separate contracts. Unsupported
    /// events return None rather than pretending to consume the background.
    pub fn event(
        &self,
        previous: &ProcessContentSnapshot,
        event: &native::Event,
    ) -> Result<Option<ProcessContentSnapshot>> {
        if let native::Event::Mouse(mouse) = event {
            return self.pointer(previous, &native_pointer(mouse)).map(Some);
        }
        let Some(input) = native_input(event) else {
            return Ok(None);
        };
        match input {
            wire::Input::Scroll(scroll) => self.scroll(previous, scroll).map(Some),
            input => self.request(previous, input).map(Some),
        }
    }
    fn send(
        &self,
        owner: Option<wire::Target>,
        capture: Option<wire::Capture>,
        focus_epoch: u64,
        text_binding: Option<Vec<u8>>,
        input: wire::Input,
    ) -> Result<ProcessContentSnapshot> {
        let request = wire::Request {
            view: self.stamp.view,
            viewport: self.stamp.viewport,
            owner,
            focus_epoch,
            text_binding,
            capture,
            input,
        };
        let reply = if let Some(stamp) = &self.stamp.curve {
            self.process.curve_content_request(
                &self.stamp.session,
                stamp.clone(),
                request,
                self.timeout,
            )?
        } else {
            self.process
                .content_request(&self.stamp.session, request, self.timeout)?
        };
        let text = project(&self.stamp, &reply)?;
        Ok(ProcessContentSnapshot {
            stamp: self.stamp.clone(),
            reply,
            text,
        })
    }
    fn curve_query(
        &self,
        query: crate::component::curve::ReadOnlyQueryWire,
    ) -> Result<ProcessContentSnapshot> {
        let stamp = self
            .stamp
            .curve
            .as_ref()
            .context("Curve query requires a frame stamp")?
            .clone();
        let reply = self.process.curve_content_query(
            &self.stamp.session,
            stamp.clone(),
            query,
            self.timeout,
        )?;
        ProcessContentSnapshot::from_curve(
            self.process,
            self.stamp.view,
            self.stamp.viewport,
            self.stamp.frame,
            self.stamp.clip,
            stamp,
            reply,
        )
    }
}

fn scroll_to_child(mut event: wire::ScrollEvent, frame: Rect) -> Result<wire::ScrollEvent> {
    ensure!(event.is_valid(), "invalid native Content scroll");
    event.position[0] -= frame.min_x();
    event.position[1] -= frame.min_y();
    ensure!(
        event.is_valid(),
        "overflowing native Content scroll projection"
    );
    Ok(event)
}

fn native_pointer(mouse: &native::MouseEvent) -> tekes_ui_controls::composition::PointerEvent {
    use tekes_ui_controls::composition::{
        InputModifiers, PointerButton as B, PointerEvent, PointerPhase as P,
    };
    let (phase, button, count) = match mouse.kind {
        native::MouseEventKind::Down {
            button,
            click_count,
        } => (P::Down, Some(button), click_count),
        native::MouseEventKind::Up {
            button,
            click_count,
        } => (P::Up, Some(button), click_count),
        native::MouseEventKind::Dragged { button } => (P::Drag, Some(button), 0),
        native::MouseEventKind::Moved => (P::Move, None, 0),
        native::MouseEventKind::Entered => (P::Enter, None, 0),
        native::MouseEventKind::Exited => (P::Exit, None, 0),
    };
    PointerEvent {
        position: tekes_ui_scene::Point::new(mouse.position.x as f32, mouse.position.y as f32),
        phase,
        button: button.map(|b| match b {
            native::MouseButton::Left => B::Primary,
            native::MouseButton::Right => B::Secondary,
            native::MouseButton::Middle => B::Middle,
            native::MouseButton::Other(n) => B::Other(n),
        }),
        modifiers: InputModifiers {
            shift: mouse.modifiers.shift,
            control: mouse.modifiers.control,
            alt: mouse.modifiers.alt,
            command: mouse.modifiers.command,
            caps_lock: mouse.modifiers.caps_lock,
            function: mouse.modifiers.function,
        },
        click_count: count,
    }
}

fn is_collection_role(role: &wire::SemanticRole) -> bool {
    matches!(
        role,
        wire::SemanticRole::TreeItem { .. }
            | wire::SemanticRole::Table
            | wire::SemanticRole::TableColumn { .. }
            | wire::SemanticRole::TableColumnHeader { .. }
            | wire::SemanticRole::TableRow { .. }
            | wire::SemanticRole::TableCell { .. }
    )
}

fn valid_rect(r: Rect) -> bool {
    [
        r.min_x(),
        r.min_y(),
        r.size.width,
        r.size.height,
        r.max_x(),
        r.max_y(),
    ]
    .iter()
    .all(|v| v.is_finite())
        && r.size.width >= 0.
        && r.size.height >= 0.
}
fn rect(v: [f32; 4]) -> Result<Rect> {
    let r = Rect::new(v[0], v[1], v[2], v[3]);
    ensure!(valid_rect(r), "invalid child Content geometry");
    Ok(r)
}
fn boundary(text: &str, index: usize) -> bool {
    let mut count = 0;
    if index == 0 {
        return true;
    }
    for ch in text.chars() {
        count += ch.len_utf16();
        if count == index {
            return true;
        }
        if count > index {
            return false;
        }
    }
    false
}
fn project(stamp: &Stamp, reply: &wire::Reply) -> Result<Option<native::TextInputState>> {
    ensure!(
        reply.view == stamp.view && reply.viewport == stamp.viewport,
        "stale native Content reply"
    );
    let Some(text) = &reply.text else {
        return Ok(None);
    };
    ensure!(
        Some(text.owner) == reply.focused,
        "mismatched native Content text owner"
    );
    let valid_range = |r: &std::ops::Range<usize>| {
        r.start <= r.end && boundary(&text.text, r.start) && boundary(&text.text, r.end)
    };
    ensure!(
        boundary(&text.text, text.caret_index)
            && valid_range(&text.selection)
            && text.marked.as_ref().is_none_or(valid_range),
        "invalid child UTF-16 text range"
    );
    let origin = stamp.frame.origin;
    let child_clip = rect(text.clip)?.offset(origin.x, origin.y);
    let clip = stamp
        .frame
        .intersection(&stamp.clip)
        .and_then(|r| r.intersection(&child_clip));
    let translated = |v: [f32; 4]| -> Result<Option<(native::Point, native::Size)>> {
        let child = rect(v)?.offset(origin.x, origin.y);
        let visible = clip
            .and_then(|clip| child.intersection(&clip))
            .filter(|r| r.size.height > 0.);
        Ok(visible.map(|r| {
            (
                native::Point {
                    x: r.min_x().into(),
                    y: r.min_y().into(),
                },
                native::Size {
                    width: r.size.width.into(),
                    height: r.size.height.into(),
                },
            )
        }))
    };
    let (caret_origin, caret_size) = translated(text.caret)?.unwrap_or_default();
    let mut carets = Vec::new();
    for (index, geometry) in &text.carets {
        ensure!(
            boundary(&text.text, *index),
            "invalid child UTF-16 caret index"
        );
        if let Some((p, s)) = translated(*geometry)? {
            carets.push((*index, p, s))
        }
    }
    Ok(Some(native::TextInputState {
        text: text.text.clone(),
        caret_index: text.caret_index,
        selection: text.selection.clone(),
        marked: text.marked.clone(),
        read_only: text.read_only,
        caret_origin,
        caret_size,
        carets,
    }))
}

/// Window adapters publish one original Host query, without branching by component.
pub fn content_text_input_state(
    host: &tekes_ui_controls::composition::ContentHost,
) -> Option<native::TextInputState> {
    let state = host.text_input_snapshot()?;
    let valid_range = |r: &std::ops::Range<usize>| {
        r.start <= r.end && boundary(&state.text, r.start) && boundary(&state.text, r.end)
    };
    if !boundary(&state.text, state.caret_index)
        || !valid_range(&state.selection)
        || state.marked.as_ref().is_some_and(|r| !valid_range(r))
    {
        return None;
    }
    let geometry = |r: Rect| -> Option<(native::Point, native::Size)> {
        if !valid_rect(r) {
            return None;
        }
        let clip = state.clip.filter(|clip| valid_rect(*clip))?;
        let r = r.intersection(&clip)?;
        if r.size.height <= 0. {
            return None;
        }
        Some((
            native::Point {
                x: r.min_x().into(),
                y: r.min_y().into(),
            },
            native::Size {
                width: r.size.width.into(),
                height: r.size.height.into(),
            },
        ))
    };
    let (caret_origin, caret_size) = state.caret.and_then(geometry).unwrap_or_default();
    let mut carets = Vec::new();
    for (index, r) in state.carets {
        if !boundary(&state.text, index) {
            return None;
        }
        if let Some((p, s)) = geometry(r) {
            carets.push((index, p, s));
        }
    }
    Some(native::TextInputState {
        text: state.text,
        caret_index: state.caret_index,
        selection: state.selection,
        marked: state.marked,
        read_only: state.read_only,
        caret_origin,
        caret_size,
        carets,
    })
}
/// One native-to-Content conversion for the whole window, not each group/process.
pub fn content_event(
    event: &native::Event,
) -> Option<tekes_ui_controls::composition::ContentEvent> {
    if let native::Event::Mouse(mouse) = event {
        let pointer = native_pointer(mouse);
        return pointer
            .position
            .x
            .is_finite()
            .then_some(())
            .and_then(|_| pointer.position.y.is_finite().then_some(()))
            .map(|_| tekes_ui_controls::composition::ContentEvent::Pointer(pointer));
    }
    if let native::Event::Key(key) = event
        && key.kind == native::KeyEventKind::Down
        && key.modifiers.command
        && !key.modifiers.control
        && !key.modifiers.alt
    {
        let command = match key
            .characters_ignoring_modifiers
            .to_ascii_lowercase()
            .as_str()
        {
            "a" => Some("selectAll:"),
            "z" if key.modifiers.shift => Some("redo:"),
            "z" => Some("undo:"),
            _ => None,
        };
        if let Some(command) = command {
            return Some(tekes_ui_controls::composition::ContentEvent::TextCommand(
                command.into(),
            ));
        }
    }
    native_input(event).and_then(|input| crate::content_input::event(&input))
}

/// One window-level adapter for original local/remote Content, including the
/// native clipboard capability. Components retain all editing/key state.
/// Hosts may handle their own product/window commands before calling this.
pub fn dispatch_content_window_event(
    host: &mut tekes_ui_controls::composition::ContentHost,
    event: &native::Event,
    window: &native::Window,
) -> Option<tekes_ui_controls::composition::ContentResponse> {
    use tekes_ui_controls::composition::{ContentEvent as E, ContentResponse as R};
    host.set_window_active(window.is_active());
    if let native::Event::Accessibility { target, action } = event {
        return Some(native::content_accessibility::content_action(
            host,
            *target,
            action.clone(),
        ));
    }
    let command = match event {
        native::Event::TextCommand(command)
            if matches!(command.as_str(), "copy:" | "cut:" | "paste:") =>
        {
            Some(command.as_str())
        }
        native::Event::Key(key)
            if key.kind == native::KeyEventKind::Down
                && key.modifiers.command
                && !key.modifiers.control
                && !key.modifiers.alt =>
        {
            match key.characters_ignoring_modifiers.as_str() {
                "c" | "C" => Some("copy:"),
                "x" | "X" => Some("cut:"),
                "v" | "V" => Some("paste:"),
                _ => None,
            }
        }
        _ => None,
    };
    if let Some(command) = command {
        let Some(snapshot) = host.claim_clipboard_input() else {
            return Some(R::consumed(false));
        };
        if snapshot.selection.start > snapshot.selection.end
            || !boundary(&snapshot.text, snapshot.selection.start)
            || !boundary(&snapshot.text, snapshot.selection.end)
        {
            return Some(R::consumed(false));
        }
        if command == "paste:" {
            return Some(if !snapshot.read_only {
                window
                    .clipboard_text()
                    .map(|text| {
                        host.event(&E::Commit {
                            text,
                            replacement: None,
                        })
                    })
                    .unwrap_or_else(|| R::consumed(false))
            } else {
                R::consumed(false)
            });
        }
        if snapshot.selection.is_empty() || (command == "cut:" && snapshot.read_only) {
            return Some(R::consumed(false));
        }
        let utf16: Vec<_> = snapshot.text.encode_utf16().collect();
        let selected = String::from_utf16(&utf16[snapshot.selection.clone()]).ok()?;
        window.set_clipboard_text(&selected);
        return Some(if command == "cut:" {
            host.event(&E::Commit {
                text: String::new(),
                replacement: Some(snapshot.selection),
            })
        } else {
            R::consumed(false)
        });
    }
    content_event(event).map(|event| host.event(&event))
}

fn key(k: &native::KeyEvent) -> wire::KeyEvent {
    wire::KeyEvent {
        key: match k.key_code {
            36 | 76 => Some(wire::Key::Enter),
            49 => Some(wire::Key::Space),
            48 => Some(wire::Key::Tab {
                reverse: k.modifiers.shift,
            }),
            53 => Some(wire::Key::Escape),
            123 => Some(wire::Key::Left),
            124 => Some(wire::Key::Right),
            125 => Some(wire::Key::Down),
            126 => Some(wire::Key::Up),
            115 => Some(wire::Key::Home),
            119 => Some(wire::Key::End),
            _ => None,
        },
        down: k.kind == native::KeyEventKind::Down,
        repeat: k.is_repeat,
        modifiers: wire::Modifiers {
            shift: k.modifiers.shift,
            control: k.modifiers.control,
            alt: k.modifiers.alt,
            command: k.modifiers.command,
            caps_lock: k.modifiers.caps_lock,
            function: k.modifiers.function,
        },
        native_code: k.key_code,
        characters: k.characters.clone(),
        characters_ignoring_modifiers: k.characters_ignoring_modifiers.clone(),
    }
}
pub(crate) fn content_key(k: &tekes_ui_controls::composition::ContentKeyEvent) -> wire::KeyEvent {
    use tekes_ui_controls::ControlKey as K;
    wire::KeyEvent {
        key: k.key.map(|key| match key {
            K::Enter => wire::Key::Enter,
            K::Space => wire::Key::Space,
            K::Left => wire::Key::Left,
            K::Right => wire::Key::Right,
            K::Up => wire::Key::Up,
            K::Down => wire::Key::Down,
            K::Home => wire::Key::Home,
            K::End => wire::Key::End,
            K::Escape => wire::Key::Escape,
            K::Tab { reverse } => wire::Key::Tab { reverse },
        }),
        down: k.down,
        repeat: k.repeat,
        modifiers: wire::Modifiers {
            shift: k.modifiers.shift,
            control: k.modifiers.control,
            alt: k.modifiers.alt,
            command: k.modifiers.command,
            caps_lock: k.modifiers.caps_lock,
            function: k.modifiers.function,
        },
        native_code: k.native_code,
        characters: k.characters.clone(),
        characters_ignoring_modifiers: k.characters_ignoring_modifiers.clone(),
    }
}
pub(crate) fn pointer_input(
    event: &tekes_ui_controls::composition::PointerEvent,
    frame: Rect,
) -> Result<wire::Input> {
    use tekes_ui_controls::composition::{PointerButton as B, PointerPhase as P};
    ensure!(
        event.position.x.is_finite() && event.position.y.is_finite(),
        "invalid native pointer position"
    );
    Ok(wire::Input::Pointer(wire::PointerEvent {
        position: [
            event.position.x - frame.min_x(),
            event.position.y - frame.min_y(),
        ],
        phase: match event.phase {
            P::Down => wire::PointerPhase::Down,
            P::Up => wire::PointerPhase::Up,
            P::Move => wire::PointerPhase::Move,
            P::Drag => wire::PointerPhase::Drag,
            P::Enter => wire::PointerPhase::Enter,
            P::Exit => wire::PointerPhase::Exit,
        },
        button: event.button.map(|b| match b {
            B::Primary => wire::PointerButton::Primary,
            B::Secondary => wire::PointerButton::Secondary,
            B::Middle => wire::PointerButton::Middle,
            B::Other(n) => wire::PointerButton::Other(n),
        }),
        modifiers: wire::Modifiers {
            shift: event.modifiers.shift,
            control: event.modifiers.control,
            alt: event.modifiers.alt,
            command: event.modifiers.command,
            caps_lock: event.modifiers.caps_lock,
            function: event.modifiers.function,
        },
        click_count: event.click_count,
    }))
}
pub(crate) fn content_input(
    event: &tekes_ui_controls::composition::ContentEvent,
) -> Option<wire::Input> {
    use tekes_ui_controls::composition::{ContentEvent as E, ContentKeyEvent};
    Some(match event {
        E::KeyInput(key) => wire::Input::Key(content_key(key)),
        E::Key { key, down } => wire::Input::Key(content_key(&ContentKeyEvent {
            key: Some(*key),
            down: *down,
            repeat: false,
            modifiers: Default::default(),
            native_code: 0,
            characters: String::new(),
            characters_ignoring_modifiers: String::new(),
        })),
        E::Scroll {
            position,
            delta,
            precise,
            begin,
            end,
            momentum,
            time,
        } => {
            let value = wire::ScrollEvent {
                position: [position.x, position.y],
                delta: [delta.x, delta.y],
                precise: *precise,
                begin: *begin,
                end: *end,
                momentum: *momentum,
                time: *time,
            };
            if !value.is_valid() {
                return None;
            }
            wire::Input::Scroll(value)
        }
        E::Commit { text, replacement } => wire::Input::Commit {
            text: text.clone(),
            replacement: replacement.clone(),
        },
        E::Preedit {
            text,
            selection,
            clauses,
            replacement,
        } => wire::Input::Preedit {
            text: text.clone(),
            selection: selection.clone(),
            clauses: clauses.clone(),
            replacement: replacement.clone(),
        },
        E::Unmark => wire::Input::Unmark,
        E::TextCommand(command) => wire::Input::TextCommand(command.clone()),
        E::TextKeyStart { id, key, composing } => wire::Input::TextKeyStart {
            id: *id,
            key: content_key(key),
            composing: *composing,
        },
        E::TextKeyEnd {
            id,
            input_context_consumed,
        } => wire::Input::TextKeyEnd {
            id: *id,
            input_context_consumed: *input_context_consumed,
        },
        _ => return None,
    })
}
fn native_input(event: &native::Event) -> Option<wire::Input> {
    Some(match event {
        native::Event::Scroll(s) => wire::Input::Scroll(wire::ScrollEvent {
            position: [s.position.x as f32, s.position.y as f32],
            delta: [-(s.delta.x as f32), -(s.delta.y as f32)],
            precise: s.precise,
            begin: matches!(
                s.phase,
                native::ScrollPhase::Began | native::ScrollPhase::MayBegin
            ),
            end: matches!(
                s.phase,
                native::ScrollPhase::Ended | native::ScrollPhase::Cancelled
            ),
            momentum: !matches!(s.momentum_phase, native::ScrollPhase::None),
            time: s.timestamp,
        }),
        native::Event::Key(k) => wire::Input::Key(key(k)),
        native::Event::TextKey(native::TextKeyEvent::Started(context)) => {
            wire::Input::TextKeyStart {
                id: context.id,
                key: key(&context.key),
                composing: context.composing_at_start,
            }
        }
        native::Event::TextKey(native::TextKeyEvent::Finished {
            context,
            disposition,
            ..
        }) => wire::Input::TextKeyEnd {
            id: context.id,
            input_context_consumed: *disposition
                == native::TextKeyDisposition::InputContextConsumed,
        },
        native::Event::TextCommit { text, replacement } => wire::Input::Commit {
            text: text.clone(),
            replacement: replacement.clone(),
        },
        native::Event::TextPreedit {
            text,
            selection,
            clauses,
            replacement,
        } => wire::Input::Preedit {
            text: text.clone(),
            selection: selection.clone(),
            clauses: clauses.clone(),
            replacement: replacement.clone(),
        },
        native::Event::TextUnmark => wire::Input::Unmark,
        native::Event::TextCommand(command) => wire::Input::TextCommand(command.clone()),
        _ => return None,
    })
}

// Test replies enter the actual receiver/container publication and failure paths.
#[cfg(test)]
pub(crate) fn semantic_test_snapshot(nodes: Vec<wire::SemanticNode>) -> ProcessContentSnapshot {
    let reply = serde_json::from_value(serde_json::json!({
        "view":1,"viewport":1,"consumed":true,"changed":true,
        "capture_pointer":false,"activated":false,"bubbles":true,
        "pointer_focus_requested":false,"key":null,"focused":null,
        "focus_epoch":0,"text_binding":null,"captured":null,"focus_targets":[],
        "composing":false,"text":null,"semantics":nodes
    }))
    .unwrap();
    ProcessContentSnapshot {
        stamp: Stamp {
            session: Session {
                module: "combo-contract".into(),
                instance: 1,
                host_epoch: 1,
            },
            view: 1,
            viewport: 1,
            curve: None,
            frame: Rect::new(0., 0., 200., 80.),
            clip: Rect::new(0., 0., 200., 80.),
        },
        reply,
        text: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_scroll_projection_keeps_units_phases_and_original_document_sign() {
        let input = native::ScrollEvent {
            timestamp: 123.5,
            delta: native::Vec2 { x: -2., y: 16. },
            precise: true,
            phase: native::ScrollPhase::Began,
            momentum_phase: native::ScrollPhase::Changed,
            inverted: true,
            position: native::Point { x: 120.5, y: 75.25 },
            modifiers: native::Modifiers::NONE,
        };
        let Some(wire::Input::Scroll(mapped)) = native_input(&native::Event::Scroll(input)) else {
            panic!("scroll projection");
        };
        assert_eq!(mapped.delta, [2., -16.]);
        assert!(mapped.precise && mapped.begin && mapped.momentum && !mapped.end);
        assert_eq!(mapped.time, 123.5);
        let projected = scroll_to_child(mapped, Rect::new(100., 60., 200., 100.)).unwrap();
        assert_eq!(projected.position, [20.5, 15.25]);
        assert_eq!(projected.delta, mapped.delta);
        let event = content_event(&native::Event::Scroll(input)).unwrap();
        assert!(
            matches!(event,tekes_ui_controls::composition::ContentEvent::Scroll{position,delta,..} if position.x==120.5 && delta.y == -16.)
        );
        let native::Event::Scroll(mut ended) = native::Event::Scroll(input) else {
            unreachable!()
        };
        ended.phase = native::ScrollPhase::Cancelled;
        ended.momentum_phase = native::ScrollPhase::None;
        assert!(
            matches!(native_input(&native::Event::Scroll(ended)),Some(wire::Input::Scroll(value)) if value.end&&!value.begin&&!value.momentum)
        );
        for invalid in [f64::NAN, f64::INFINITY, f64::MAX] {
            let bad = native::ScrollEvent {
                delta: native::Vec2 { x: 0., y: invalid },
                ..input
            };
            assert!(content_event(&native::Event::Scroll(bad)).is_none());
        }
        assert!(
            scroll_to_child(
                wire::ScrollEvent {
                    position: [f32::MAX, 0.],
                    ..mapped
                },
                Rect::new(-f32::MAX, 0., 20., 20.)
            )
            .is_err()
        );
    }
    #[test]
    fn native_pointer_metadata_keeps_buttons_counts_and_modifiers() {
        use tekes_ui_controls::composition::{PointerButton, PointerPhase};
        let modifiers = native::Modifiers {
            shift: true,
            control: true,
            alt: true,
            command: true,
            caps_lock: true,
            function: true,
        };
        for (button, expected) in [
            (native::MouseButton::Left, PointerButton::Primary),
            (native::MouseButton::Right, PointerButton::Secondary),
            (native::MouseButton::Middle, PointerButton::Middle),
            (native::MouseButton::Other(7), PointerButton::Other(7)),
        ] {
            let result = native_pointer(&native::MouseEvent {
                kind: native::MouseEventKind::Down {
                    button,
                    click_count: 3,
                },
                position: native::Point { x: 123.5, y: 47.25 },
                modifiers,
            });
            assert_eq!(result.phase, PointerPhase::Down);
            assert_eq!(result.button, Some(expected));
            assert_eq!(result.click_count, 3);
            assert!(
                result.modifiers.shift
                    && result.modifiers.control
                    && result.modifiers.alt
                    && result.modifiers.command
                    && result.modifiers.caps_lock
                    && result.modifiers.function
            );
            assert_eq!(result.position, tekes_ui_scene::Point::new(123.5, 47.25));
        }
        let exit = native_pointer(&native::MouseEvent {
            kind: native::MouseEventKind::Exited,
            position: native::Point::default(),
            modifiers: native::Modifiers::NONE,
        });
        assert_eq!(exit.phase, PointerPhase::Exit);
        assert_eq!(exit.button, None);
        assert_eq!(exit.click_count, 0);
    }
    #[derive(Debug)]
    struct QueryOnly(tekes_ui_controls::composition::ContentTextInput);
    impl tekes_ui_controls::composition::Component for QueryOnly {
        fn measure(
            &mut self,
            _: &mut dyn tekes_ui_controls::ButtonTextRenderer,
            _: tekes_ui_scene::Size,
        ) -> tekes_ui_controls::layout::MeasuredChild {
            tekes_ui_controls::layout::MeasuredChild::new(tekes_ui_scene::Size::new(100., 60.))
        }
        fn focusable(&self) -> bool {
            true
        }
        fn text_input_snapshot(&self) -> Option<tekes_ui_controls::composition::ContentTextInput> {
            Some(self.0.clone())
        }
        fn native_key_binding(&self) -> Option<Vec<u8>> {
            Some(self.0.binding.clone())
        }
    }
    #[test]
    fn original_query_without_local_editor_validates_utf16_clip_and_lifecycle() {
        use tekes_ui_controls::composition::{ComponentId, Content, ContentHost, ContentTextInput};
        let id = ComponentId(51);
        let mut host = ContentHost::new(Content::new(
            id,
            QueryOnly(ContentTextInput {
                binding: vec![1],
                text: "A😀B".into(),
                caret_index: 3,
                selection: 1..3,
                marked: Some(1..3),
                read_only: false,
                caret: Some(Rect::new(10., 5., 1., 20.)),
                clip: Some(Rect::new(0., 10., 100., 10.)),
                carets: vec![(0, Rect::new(1., 5., 1., 20.))],
            }),
        ))
        .unwrap();
        host.focus(Some(id), true);
        assert!(host.focused_text().is_none());
        let state = content_text_input_state(&host).unwrap();
        assert_eq!(state.selection, 1..3);
        assert_eq!(state.caret_origin.y, 10.);
        assert_eq!(state.caret_size.height, 10.);
        assert_eq!(state.carets.len(), 1);
        host.get_mut(id)
            .unwrap()
            .component_mut::<QueryOnly>()
            .unwrap()
            .0
            .clip = None;
        let state = content_text_input_state(&host).unwrap();
        assert_eq!(state.marked, Some(1..3));
        assert_eq!(state.caret_size.height, 0.);
        assert!(state.carets.is_empty());
        host.get_mut(id)
            .unwrap()
            .component_mut::<QueryOnly>()
            .unwrap()
            .0
            .selection = 2..3;
        assert!(
            content_text_input_state(&host).is_none(),
            "surrogate midpoint accepted"
        );
        host.get_mut(id)
            .unwrap()
            .component_mut::<QueryOnly>()
            .unwrap()
            .0
            .selection = 1..3;
        host.set_window_active(false);
        assert!(content_text_input_state(&host).is_none());
    }
    #[test]
    fn clipboard_claim_uses_original_utf16_snapshot_and_rejects_duplicate_or_stale_owner() {
        use tekes_ui_controls::composition::*;
        for scenario in 0..6 {
            let id = ComponentId(51);
            let mut host = ContentHost::new(Content::new(
                id,
                QueryOnly(ContentTextInput {
                    binding: vec![1],
                    text: "A😀B".into(),
                    caret_index: 3,
                    selection: 1..3,
                    marked: None,
                    read_only: scenario == 5,
                    caret: None,
                    clip: None,
                    carets: vec![],
                }),
            ))
            .unwrap();
            host.focus(Some(id), true);
            host.event(&ContentEvent::TextKeyStart {
                id: 1,
                composing: false,
                key: ContentKeyEvent {
                    key: None,
                    down: true,
                    repeat: false,
                    native_code: 8,
                    modifiers: InputModifiers {
                        command: true,
                        ..Default::default()
                    },
                    characters: "c".into(),
                    characters_ignoring_modifiers: "c".into(),
                },
            });
            match scenario {
                1 => {
                    host.get_mut(id)
                        .unwrap()
                        .component_mut::<QueryOnly>()
                        .unwrap()
                        .0
                        .binding = vec![2]
                }
                2 => {
                    host.focus(None, false);
                    host.focus(Some(id), true);
                }
                3 => {
                    host.get_mut(id)
                        .unwrap()
                        .component_mut::<QueryOnly>()
                        .unwrap()
                        .0
                        .marked = Some(1..3)
                }
                4 => host.set_window_active(false),
                _ => {}
            }
            let claimed = host.claim_clipboard_input();
            assert_eq!(
                claimed.is_some(),
                scenario == 0 || scenario == 5,
                "scenario {scenario}"
            );
            if let Some(snapshot) = claimed {
                assert_eq!(snapshot.selection, 1..3);
                assert_eq!(snapshot.text, "A😀B");
                assert!(
                    host.claim_clipboard_input().is_none(),
                    "clipboard command replayed"
                );
                let response = host.event(&ContentEvent::TextKeyEnd {
                    id: 1,
                    input_context_consumed: false,
                });
                assert_eq!(
                    response.text_key.unwrap().disposition,
                    TextKeyDisposition::EditHandled
                );
                assert!(!response.activated);
            }
        }
    }
    #[test]
    fn window_input_rejects_nonfinite_and_overflowing_pointer_geometry() {
        for x in [f64::NAN, f64::INFINITY, f64::MAX] {
            assert!(
                content_event(&native::Event::Mouse(native::MouseEvent {
                    kind: native::MouseEventKind::Moved,
                    position: native::Point { x, y: 0. },
                    modifiers: native::Modifiers::NONE,
                }))
                .is_none()
            );
        }
    }
    fn fixture() -> (Stamp, wire::Reply) {
        let owner = wire::Target { id: 2, mount: 17 };
        (
            Stamp {
                session: Session {
                    module: "text".into(),
                    instance: 1,
                    host_epoch: 9,
                },
                view: 7,
                viewport: 3,
                frame: Rect::new(100., 60., 200., 100.),
                clip: Rect::new(110., 65., 180., 80.),
                curve: None,
            },
            wire::Reply {
                view: 7,
                viewport: 3,
                consumed: false,
                changed: false,
                capture_pointer: false,
                activated: false,
                bubbles: false,
                pointer_focus_requested: false,
                accessibility_focus_requested: false,
                key: None,
                overlay: None,
                reading_scroll_source: None,
                reading: None,
                semantics: None,
                animation_frame_requested: false,
                focused: Some(owner),
                focus_epoch: 1,
                text_binding: Some(vec![9, 7]),
                captured: None,
                focus_targets: vec![owner],
                composing: true,
                text: Some(wire::TextState {
                    owner,
                    text: "A😀B".into(),
                    caret_index: 3,
                    selection: 1..3,
                    marked: Some(1..3),
                    read_only: false,
                    caret: [35., 10., 1., 18.],
                    clip: [10., 5., 180., 80.],
                    carets: vec![(1, [20., 10., 1., 18.]), (3, [35., 10., 1., 18.])],
                }),
            },
        )
    }
    #[test]
    fn native_text_reuses_utf16_and_original_shaped_geometry_in_parent_points() {
        let (stamp, reply) = fixture();
        let state = project(&stamp, &reply).unwrap().unwrap();
        assert_eq!(state.text, "A😀B");
        assert_eq!(state.selection, 1..3);
        assert_eq!(state.marked, Some(1..3));
        assert_eq!(state.caret_index, 3);
        assert_eq!(state.caret_origin, native::Point { x: 135., y: 70. });
        let (range, p, s) = state.first_rect_for_range(1..3).unwrap();
        assert_eq!(range, 1..3);
        assert_eq!(p, native::Point { x: 120., y: 70. });
        assert_eq!(s.width, 15.);
        assert!(state.first_rect_for_range(2..3).is_none());
    }
    #[test]
    fn native_text_clip_never_publishes_offscreen_candidate_carets() {
        let (mut stamp, reply) = fixture();
        stamp.clip = Rect::new(132., 70., 20., 10.);
        let state = project(&stamp, &reply).unwrap().unwrap();
        assert_eq!(state.carets.len(), 1);
        assert_eq!(state.caret_size.height, 10.);
        assert!(state.first_rect_for_range(1..1).is_none());
        stamp.clip = Rect::new(250., 70., 20., 10.);
        let state = project(&stamp, &reply).unwrap().unwrap();
        assert!(state.carets.is_empty());
        assert_eq!(state.caret_size.height, 0.);
        assert!(state.first_rect_for_range(3..3).is_none());
    }
    #[test]
    fn malformed_or_stale_child_text_is_not_a_native_query_answer() {
        let (stamp, reply) = fixture();
        for mutate in [0, 1, 2, 3, 4, 5] {
            let mut reply = reply.clone();
            match mutate {
                0 => reply.viewport += 1,
                1 => reply.focused = None,
                2 => reply.text.as_mut().unwrap().selection = 2..3,
                3 => reply.text.as_mut().unwrap().caret[0] = f32::NAN,
                4 => reply.text.as_mut().unwrap().carets[0].0 = 2,
                _ => reply.text.as_mut().unwrap().marked = Some(0..99),
            }
            assert!(project(&stamp, &reply).is_err(), "case {mutate}");
        }
    }
    #[test]
    fn native_callback_translation_retains_replacement_and_key_provenance() {
        let event = native::Event::TextPreedit {
            text: "ni".into(),
            selection: 2..2,
            clauses: std::iter::once(0..2).collect(),
            replacement: Some(1..3),
        };
        assert_eq!(
            native_input(&event),
            Some(wire::Input::Preedit {
                text: "ni".into(),
                selection: 2..2,
                clauses: std::iter::once(0..2).collect(),
                replacement: Some(1..3)
            })
        );
        let context = native::TextKeyContext {
            id: 19,
            key: native::KeyEvent {
                kind: native::KeyEventKind::Down,
                key_code: 48,
                characters: "\t".into(),
                characters_ignoring_modifiers: "\t".into(),
                modifiers: native::Modifiers {
                    shift: true,
                    function: true,
                    ..native::Modifiers::NONE
                },
                is_repeat: true,
            },
            composing_at_start: true,
        };
        let Some(wire::Input::TextKeyStart { id, key, composing }) = native_input(
            &native::Event::TextKey(native::TextKeyEvent::Started(context.clone())),
        ) else {
            panic!("start")
        };
        assert_eq!(id, 19);
        assert!(composing && key.repeat && key.modifiers.function);
        assert_eq!(key.key, Some(wire::Key::Tab { reverse: true }));
        assert_eq!(
            native_input(&native::Event::TextKey(native::TextKeyEvent::Finished {
                context,
                input_context_handled: Some(true),
                disposition: native::TextKeyDisposition::InputContextConsumed,
                text_callbacks: 0,
                command_callbacks: 0
            })),
            Some(wire::Input::TextKeyEnd {
                id: 19,
                input_context_consumed: true
            })
        );
        assert!(native_input(&native::Event::Redraw).is_none());
    }
    #[test]
    fn progress_producer_wire_receiver_keeps_domain_identity_and_revokes_invalid_range() {
        use tekes_ui_controls::composition::{
            ComponentId, Content, ContentHost, ContentRole, ProgressContent,
        };
        use tekes_ui_controls::{ButtonTextAlign, ButtonTextRenderer};
        use tekes_ui_scene::{Scene, Size};
        use tekes_ui_tokens::{Rgba, Theme, proposal};
        struct Text;
        impl ButtonTextRenderer for Text {
            fn measure(&mut self, s: &str, _: proposal::TextRole, width: f32) -> Size {
                Size::new((s.len() as f32 * 8.).min(width), 20.)
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
        let (stamp, mut reply) = fixture();
        let frame = Rect::new(0., 0., 100., 12.);
        let mut text = Text;
        let mut host = ContentHost::new(Content::new(
            ComponentId(71),
            ProgressContent::new("Transfer progress", 0.45),
        ))
        .unwrap();
        let mut remote = crate::container_semantics::RemoteSemantics::default();
        let mut previous_index = None;
        for unknown in [false, true, false] {
            host.get_mut(ComponentId(71))
                .unwrap()
                .component_mut::<ProgressContent>()
                .unwrap()
                .model
                .indeterminate = unknown;
            host.render(
                &mut Scene::new(),
                &mut text,
                frame,
                frame,
                Theme::LIGHT.into(),
            )
            .unwrap();
            reply.semantics = Some(crate::content_accessibility::snapshot(&host).unwrap());
            // Serialize the complete producer reply before entering the real receiver.
            let encoded = serde_json::to_vec(&reply).unwrap();
            let received = serde_json::from_slice(&encoded).unwrap();
            remote
                .update(ProcessContentSnapshot {
                    stamp: stamp.clone(),
                    reply: received,
                    text: None,
                })
                .unwrap();
            let parts = remote.parts(frame, frame, false);
            assert_eq!(parts.len(), 1);
            let part = &parts[0];
            assert_eq!(part.semantics.role, ContentRole::ProgressIndicator);
            assert_eq!(part.semantics.name, "Transfer progress");
            assert_eq!(
                part.semantics.value.as_deref(),
                Some(if unknown { "进度未知" } else { "45%" })
            );
            assert_eq!(
                remote.value_range(part.index),
                (!unknown).then_some(0.0..=100.0)
            );
            if let Some(index) = previous_index {
                assert_eq!(part.index, index);
            }
            previous_index = Some(part.index);
            assert!(!part.focused);
        }
        reply.semantics.as_mut().unwrap()[0].value_range = Some([0., f64::INFINITY]);
        assert!(
            remote
                .update(ProcessContentSnapshot {
                    stamp,
                    reply,
                    text: None
                })
                .is_err()
        );
        assert!(remote.parts(frame, frame, false).is_empty());
        assert!(remote.value_range(previous_index.unwrap()).is_none());
    }

    #[test]
    fn matched_image_snapshot_reaches_remote_parts_and_invalid_alt_withdraws_prior_projection() {
        let (stamp, mut reply) = fixture();
        reply.focused = None;
        reply.text = None;
        reply.text_binding = None;
        reply.composing = false;
        reply.focus_targets.clear();
        let image = |id, name| {
            serde_json::from_value::<wire::SemanticNode>(serde_json::json!({
            "target":{"id":id,"mount":id+100},"parent":null,
            "frame":[0,0,20,10],"visible":[0,0,20,10],"focused":false,
            "semantics":{"role":"Image","name":name,"value":null,"disabled":false,
                "selected":false,"expanded":null,"invalid":false,"required":false,"description":null},
            "parts":[],"value_range":null,"text":null
        })).unwrap()
        };
        reply.semantics = Some(vec![image(11, "Informative image"), image(12, "")]);
        let mut remote = crate::container_semantics::RemoteSemantics::default();
        remote
            .update(ProcessContentSnapshot {
                stamp: stamp.clone(),
                reply: reply.clone(),
                text: None,
            })
            .unwrap();
        let frame = Rect::new(0., 0., 100., 100.);
        let parts = remote.parts(frame, frame, false);
        assert_eq!(parts.len(), 2);
        assert!(
            parts
                .iter()
                .all(|p| p.semantics.role == tekes_ui_controls::composition::ContentRole::Image)
        );
        assert_eq!(parts[0].semantics.name, "Informative image");
        assert!(parts[1].semantics.name.is_empty());
        reply.semantics.as_mut().unwrap()[1].semantics.name = " \n".into();
        assert!(
            remote
                .update(ProcessContentSnapshot {
                    stamp,
                    reply,
                    text: None
                })
                .is_err()
        );
        assert!(remote.parts(frame, frame, false).is_empty());
    }
}
