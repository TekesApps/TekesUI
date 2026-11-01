//! Controlled-window fixture. Default mode injects no input; explicit M6 IME
//! mode uses public programmatic AppKit selftests, never a physical IME claim.
//! Raw native events and operator input provenance remain separate.
#[cfg(target_os = "macos")]
#[path = "../tests/support/coretext.rs"]
mod coretext;
#[cfg(target_os = "macos")]
#[path = "../tests/support/viewer_fixture.rs"]
mod fixture;
#[cfg(target_os = "macos")]
#[path = "../tests/support/native_key.rs"]
mod native_key;
#[cfg(target_os = "macos")]
#[path = "../tests/support/native_pointer.rs"]
mod native_pointer;
// Native-object probe only; no shared Host/AX implementation or replacement UI.
#[cfg(target_os = "macos")]
mod stale_ax {
    use objc2::{Message, msg_send, rc::Retained, runtime::AnyObject};
    use objc2_app_kit::NSApplication;
    use objc2_foundation::{MainThreadMarker, NSArray, NSString};
    use std::{
        path::PathBuf,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
            mpsc,
        },
        thread::JoinHandle,
        time::{Duration, Instant},
    };
    use tekes_ui_platform_macos::{WakeResult, Window, WindowWake};

    pub struct Peer {
        view: Retained<AnyObject>,
        zoom: Retained<AnyObject>,
    }
    fn named(node: &AnyObject, label: &str, matches: &mut Vec<Retained<AnyObject>>) {
        let children: Retained<NSArray<AnyObject>> =
            unsafe { msg_send![node, accessibilityChildren] };
        for child in children.iter() {
            let name: Retained<NSString> = unsafe { msg_send![&*child, accessibilityLabel] };
            if name.to_string() == label {
                matches.push(child.retain());
            }
            named(&child, label, matches);
        }
    }
    impl Peer {
        pub fn retain(window: &Window) -> Self {
            let mtm = MainThreadMarker::new().expect("native AX witness stays on main thread");
            let windows = NSApplication::sharedApplication(mtm).windows();
            let matches: Vec<_> = windows
                .iter()
                .filter(|w| {
                    w.title().to_string() == "Media Viewer Consumer 204"
                        && w.windowNumber() == window.native_window_number()
                })
                .collect();
            assert_eq!(matches.len(), 1, "unique actual owned native window");
            let view: Retained<AnyObject> = matches[0].contentView().unwrap().into();
            let mut matches = Vec::new();
            named(&view, "Zoom in", &mut matches);
            assert_eq!(matches.len(), 1, "unique current published native Zoom");
            Self {
                view,
                zoom: matches.remove(0),
            }
        }
        pub fn identity(&self) -> String {
            format!("view={:p} zoom={:p}", &*self.view, &*self.zoom)
        }
        pub fn live(&self) -> bool {
            unsafe { msg_send![&*self.zoom, isAccessibilityElement] }
        }
        pub fn press(&self) -> bool {
            unsafe { msg_send![&*self.zoom, accessibilityPerformPress] }
        }
    }
    #[derive(Debug)]
    pub enum Request {
        Retained,
        Check,
        ObserverStopped,
        TimedOut,
    }
    pub struct Channel {
        receiver: mpsc::Receiver<Request>,
        path: PathBuf,
        stop: Arc<AtomicBool>,
        worker: Option<JoinHandle<Result<(), String>>>,
    }
    impl Channel {
        pub fn start(path: PathBuf, wake: WindowWake) -> Self {
            let (sender, receiver) = mpsc::channel();
            let stop = Arc::new(AtomicBool::new(false));
            let stopping = stop.clone();
            let cleanup_path = path.clone();
            let worker = std::thread::spawn(move || {
                let result = (|| -> Result<(), String> {
                    let end = Instant::now() + Duration::from_secs(90);
                    for (file, request) in [
                        ("MEDIA_M6_EXTERNAL_RETAINED", Request::Retained),
                        ("MEDIA_M6_STALE_CHECK_REQUEST", Request::Check),
                        ("MEDIA_M6_OBSERVER_STOPPED", Request::ObserverStopped),
                    ] {
                        let request = loop {
                            if stopping.load(Ordering::Relaxed) {
                                return Ok(());
                            }
                            if Instant::now() >= end {
                                break Request::TimedOut;
                            }
                            if path.join(file).try_exists().map_err(|error| {
                                format!("probe channel metadata {file}: {error}")
                            })? {
                                break request;
                            }
                            std::thread::sleep(Duration::from_millis(20));
                        };
                        let timed_out = matches!(request, Request::TimedOut);
                        sender
                            .send(request)
                            .map_err(|error| format!("probe channel send {file}: {error}"))?;
                        match wake.wake() {
                            WakeResult::Queued | WakeResult::Coalesced => {}
                            WakeResult::Closed => {
                                if stopping.load(Ordering::Relaxed) {
                                    return Ok(());
                                }
                                return Err(format!(
                                    "probe channel wake {file}: window closed before completion"
                                ));
                            }
                        }
                        if timed_out {
                            return Ok(());
                        }
                    }
                    Ok(())
                })();
                if let Err(error) = &result {
                    // Preserve the first worker failure even if a closed window cannot wake UI.
                    if let Err(record_error) =
                        std::fs::write(path.join("MEDIA_M6_CHANNEL_ERROR"), error)
                    {
                        return Err(format!(
                            "{error}; recording channel failure: {record_error}"
                        ));
                    }
                }
                result
            });
            Self {
                receiver,
                path: cleanup_path,
                stop,
                worker: Some(worker),
            }
        }
        fn join_worker(&mut self) -> Result<(), String> {
            let Some(worker) = self.worker.take() else {
                return Ok(());
            };
            worker.join().map_err(|payload| {
                let detail = payload
                    .downcast_ref::<String>()
                    .map(String::as_str)
                    .or_else(|| payload.downcast_ref::<&str>().copied())
                    .unwrap_or("non-string panic payload");
                format!("probe channel worker panic: {detail}")
            })?
        }
        pub fn next(&mut self) -> Option<Request> {
            match self.receiver.try_recv() {
                Ok(request) => Some(request),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => {
                    let error = self
                        .join_worker()
                        .err()
                        .unwrap_or_else(|| "probe channel disconnected before completion".into());
                    panic!("{error}")
                }
            }
        }
    }
    impl Drop for Channel {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
            // Cleanup must not replace an existing UI/probe failure with a second panic.
            let result = self.join_worker();
            let receipt = format!(
                "stop_requested=true ui_unwinding={} join={result:?}\n",
                std::thread::panicking()
            );
            if let Err(error) = std::fs::write(self.path.join("MEDIA_M6_CHANNEL_CLEANUP"), receipt)
            {
                eprintln!("probe channel cleanup receipt: {error}");
            }
            if let Err(error) = result {
                eprintln!("{error}");
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn main() {
    panic!("macOS native fixture required");
}
#[cfg(target_os = "macos")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;
    use tekes_ui_controls::composition::*;
    use tekes_ui_platform_macos::{
        self as p, App, AppConfig, Event, MenuBar, Window, WindowConfig, WindowHandler,
        content_accessibility::ContentAccessibilitySnapshot,
    };
    use tekes_ui_render::Renderer;
    use tekes_ui_scene::{Rect, Scene};
    use tekes_ui_tokens::{Palette, Theme};
    const IME_EDITOR: ComponentId = ComponentId(20470);
    const IME_BODY: ComponentId = ComponentId(20471);
    const IME_EDITOR_LABEL: &str = "M6 composition editor";
    fn ime_modal_fixture(gpu: &mut Renderer) -> ContentHost {
        let mut tree = fixture::modal_fixture(gpu);
        // A profile: the original Form and MediaPanel are siblings within the
        // original Dialog. Its existing inset reserves the complete focus ring;
        // the editor no longer inherits the MediaPanel's zero-padding clip.
        let slots = tree
            .get_mut(fixture::DIALOG)
            .unwrap()
            .component_mut::<DialogContent>()
            .unwrap()
            .children_mut()
            .unwrap();
        let panel = slots.remove(1);
        slots.insert(
            1,
            Content::new(
                IME_BODY,
                StackContent {
                    model: tekes_ui_controls::extended::Stack::new(
                        tekes_ui_controls::extended::Axis::Vertical,
                    ),
                    flexible_child: Some(fixture::ROOT),
                    children: vec![
                        Content::new(IME_EDITOR, TextFieldContent::new(IME_EDITOR_LABEL, "draft")),
                        panel,
                    ],
                },
            ),
        );
        tree
    }
    // Reads the same original controls; no get_mut/drain or copied editor state.
    #[derive(Clone, Debug, PartialEq)]
    struct ImeWitness {
        buttons: Vec<usize>,
        focused: Option<ComponentId>,
        dialog_open: bool,
        transform: Option<MediaViewerTransform>,
        image_status: String,
    }
    fn ime_witness(tree: &ContentHost) -> ImeWitness {
        ImeWitness {
            buttons: fixture::COMMANDS
                .iter()
                .map(|(id, _, _)| *id)
                .chain([fixture::CLOSE, fixture::RETRY, fixture::INVOKER])
                .map(|id| {
                    tree.get(id)
                        .unwrap()
                        .component::<Button>()
                        .unwrap()
                        .activations
                })
                .collect(),
            focused: tree.focused(),
            dialog_open: tree
                .get(fixture::DIALOG)
                .unwrap()
                .component::<DialogContent>()
                .unwrap()
                .is_open(),
            transform: tree
                .get(fixture::VIEW)
                .unwrap()
                .component::<MediaViewerContent>()
                .unwrap()
                .transform(),
            image_status: format!(
                "{:?}",
                tree.get(fixture::VIEW)
                    .unwrap()
                    .component::<MediaViewerContent>()
                    .unwrap()
                    .image_status()
            ),
        }
    }
    #[derive(Debug, PartialEq)]
    enum StalePhase {
        Publication,
        ExternalRetain,
        Dismissal,
        StaleRequest,
        Checked,
        Complete,
    }
    struct StaleProbe {
        path: std::path::PathBuf,
        phase: StalePhase,
        peer: Option<stale_ax::Peer>,
        channel: Option<stale_ax::Channel>,
        host_identity: Option<usize>,
        dismissed: Option<ImeWitness>,
    }
    impl StaleProbe {
        fn new(path: std::path::PathBuf) -> Self {
            std::fs::create_dir_all(&path).expect("owned probe channel");
            assert!(
                std::fs::read_dir(&path).unwrap().next().is_none(),
                "fresh probe channel required"
            );
            Self {
                path,
                phase: StalePhase::Publication,
                peer: None,
                channel: None,
                host_identity: None,
                dismissed: None,
            }
        }
        fn emit(&self, name: &str, window: &Window) {
            let file = self.path.join(name);
            assert!(!file.exists(), "phase receipt must not be overwritten");
            let temporary = self.path.join(format!("{name}.tmp"));
            std::fs::write(
                &temporary,
                format!(
                    "pid={}\nwindow_number={}\nhost={:x}\n{}\n",
                    std::process::id(),
                    window.native_window_number(),
                    self.host_identity.unwrap(),
                    self.peer.as_ref().unwrap().identity()
                ),
            )
            .expect("phase receipt write");
            std::fs::rename(temporary, file).expect("atomic phase publication");
        }
        fn current_caller(tree: &ContentHost, window: &Window) {
            assert!(
                window.is_active() && tree.is_window_active(),
                "same live active window required"
            );
            assert_eq!(tree.focused(), Some(fixture::INVOKER));
            let key = p::content_accessibility::content_key(tree, fixture::INVOKER, None).unwrap();
            let publication = window.content_accessibility_publication().unwrap();
            assert!(
                publication.is_current() && publication.owner(key).is_some(),
                "live caller publication required"
            );
        }
        fn requests(
            &mut self,
            tree: &ContentHost,
            window: &Window,
            log: &mut std::fs::File,
        ) -> bool {
            while let Some(request) = self.channel.as_mut().and_then(stale_ax::Channel::next) {
                assert_eq!(
                    self.host_identity,
                    Some(tree as *const ContentHost as usize)
                );
                match request {
                    stale_ax::Request::Retained => {
                        assert_eq!(self.phase, StalePhase::ExternalRetain);
                        assert!(window.is_active() && tree.is_window_active());
                        assert!(ime_witness(tree).dialog_open);
                        let ack =
                            std::fs::read_to_string(self.path.join("MEDIA_M6_EXTERNAL_RETAINED"))
                                .unwrap();
                        writeln!(
                            log,
                            "m6_stale_external_retained host={tree:p} receipt={ack:?}"
                        )
                        .unwrap();
                        self.phase = StalePhase::Dismissal;
                        // Existing public AppKit input path, after external retain only.
                        p::selftest::run_script(vec![(
                            0,
                            p::selftest::Action::KeyCharacters {
                                key_code: 53,
                                chars: "\u{1b}",
                            },
                        )]);
                    }
                    stale_ax::Request::Check => {
                        assert_eq!(self.phase, StalePhase::StaleRequest);
                        Self::current_caller(tree, window);
                        let before = ime_witness(tree);
                        assert_eq!(self.dismissed.as_ref(), Some(&before));
                        let peer = self.peer.as_ref().unwrap();
                        let live = peer.live();
                        let accepted = peer.press();
                        let after = ime_witness(tree);
                        writeln!(log, "m6_stale_post_external host={tree:p} {} live={live} press={accepted} before={before:?} after={after:?}", peer.identity()).unwrap();
                        assert!(
                            !live && !accepted,
                            "retained native scoped Zoom must refuse"
                        );
                        assert_eq!(before, after);
                        self.phase = StalePhase::Checked;
                        self.emit("MEDIA_M6_STALE_CHECKED_READY", window);
                    }
                    stale_ax::Request::ObserverStopped => {
                        assert_eq!(self.phase, StalePhase::Checked);
                        Self::current_caller(tree, window);
                        assert_eq!(self.dismissed.as_ref(), Some(&ime_witness(tree)));
                        let receipt =
                            std::fs::read_to_string(self.path.join("MEDIA_M6_OBSERVER_STOPPED"))
                                .unwrap();
                        writeln!(log, "m6_stale_observer_stopped host={tree:p} receipt={receipt:?} final={:?}", ime_witness(tree)).unwrap();
                        self.phase = StalePhase::Complete;
                        self.emit("MEDIA_M6_COMPLETED", window);
                        writeln!(log, "M6_RETAINED_NATIVE_SCOPE_CHECK_COMPLETED external_AXError_requires_separate_review=true").unwrap();
                        log.flush().unwrap();
                        return true;
                    }
                    stale_ax::Request::TimedOut => panic!("retained AX channel deadline reached"),
                }
            }
            false
        }
        fn published(&mut self, tree: &ContentHost, window: &Window, log: &mut std::fs::File) {
            if self.phase == StalePhase::Publication {
                let key = p::content_accessibility::content_key(tree, fixture::COMMANDS[1].0, None);
                let publication = window.content_accessibility_publication();
                let ready = window.is_active()
                    && tree.is_window_active()
                    && key.is_some_and(|key| {
                        publication
                            .as_ref()
                            .is_some_and(|p| p.is_current() && p.owner(key).is_some())
                    });
                if !ready {
                    return;
                }
                assert!(ime_witness(tree).dialog_open);
                self.host_identity = Some(tree as *const ContentHost as usize);
                let peer = stale_ax::Peer::retain(window);
                assert!(peer.live(), "original current native Zoom must start live");
                writeln!(
                    log,
                    "m6_stale_modal_ready host={tree:p} key={key:?} {} state={:?}",
                    peer.identity(),
                    ime_witness(tree)
                )
                .unwrap();
                self.peer = Some(peer);
                self.channel = Some(stale_ax::Channel::start(
                    self.path.clone(),
                    window.wake_proxy(),
                ));
                self.phase = StalePhase::ExternalRetain;
                self.emit("MEDIA_M6_MODAL_READY", window);
            } else if self.phase == StalePhase::Dismissal && !ime_witness(tree).dialog_open {
                Self::current_caller(tree, window);
                let before = ime_witness(tree);
                let peer = self.peer.as_ref().unwrap();
                let live = peer.live();
                let accepted = peer.press();
                let after = ime_witness(tree);
                writeln!(log, "m6_stale_dismissed host={tree:p} {} live={live} press={accepted} before={before:?} after={after:?}", peer.identity()).unwrap();
                assert!(!live && !accepted);
                assert_eq!(before, after);
                self.dismissed = Some(after);
                self.phase = StalePhase::StaleRequest;
                self.emit("MEDIA_M6_DISMISSED_READY", window);
            }
        }
    }
    struct Host {
        tree: Option<ContentHost>,
        gpu: Option<Renderer>,
        text: coretext::Text,
        actions: fixture::Actions,
        log: std::fs::File,
        external_display_id: u32,
        stale_probe: Option<StaleProbe>,
        ime_mode: bool,
        ime_script_started: bool,
        ime_before: Option<(u64, ImeWitness)>,
        ime_completed: usize,
        ime_positive_before: Option<ImeWitness>,
        ime_positive_done: bool,
    }
    impl Host {
        // Same published UTF-16/caret mapping as composition-consumer Host::text_state.
        fn text_state(s: &tekes_ui_controls::TextSurface) -> p::TextInputState {
            let r = s.caret();
            let origin = s.origin();
            p::TextInputState {
                caret_index: s.input.caret,
                text: s.input.text.clone(),
                selection: s.input.selection(),
                marked: s.input.marked.clone(),
                caret_origin: p::Point {
                    x: r.min_x() as f64,
                    y: r.min_y() as f64,
                },
                caret_size: p::Size {
                    width: r.size.width as f64,
                    height: r.size.height as f64,
                },
                read_only: s.input.read_only,
                carets: s
                    .paragraph
                    .as_ref()
                    .map(|paragraph| {
                        paragraph
                            .lines
                            .iter()
                            .filter(|line| {
                                origin.y + (line.top + line.height) as f32 >= s.inner.min_y()
                                    && origin.y + line.top as f32 <= s.inner.max_y()
                            })
                            .flat_map(|line| {
                                line.carets.iter().map(move |(index, x)| {
                                    (
                                        *index,
                                        p::Point {
                                            x: f64::from(origin.x) + x,
                                            y: f64::from(origin.y) + line.top,
                                        },
                                        p::Size {
                                            width: f64::from(tekes_ui_tokens::Stroke::Rule.value()),
                                            height: line.height,
                                        },
                                    )
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
            }
        }
        // Read-only diagnostics for the original pointer/focus admission. These
        // observations are not a second hit-test or focus implementation.
        fn write_pointer_witness(
            tree: &ContentHost,
            window: &Window,
            point: tekes_ui_scene::Point,
            phase: &str,
            log: &mut std::fs::File,
        ) {
            let geometry = tree.geometry_snapshot();
            let editor_key = p::content_accessibility::content_key(tree, IME_EDITOR, None);
            let publication = window.content_accessibility_publication();
            let editor_published = editor_key.is_some_and(|key| {
                publication
                    .as_ref()
                    .is_some_and(|publication| publication.owner(key).is_some())
            });
            let scope_focus_targets = tree.focus_targets();
            let editor_focus_target = scope_focus_targets.iter().find(|(id, _)| *id == IME_EDITOR);
            let dialog_open = tree
                .get(fixture::DIALOG)
                .unwrap()
                .component::<DialogContent>()
                .unwrap()
                .is_open();
            writeln!(log,
                "m6_pointer_{phase} point={point:?} window_active={} host_active={} focused={:?} needs_render={} requires_curve={} geometry={:?} host_pointer_hit={:?} geometry_entry_hit={:?} editor_key={editor_key:?} editor_published={editor_published} editor_focus_target={editor_focus_target:?} scope_focus_targets={scope_focus_targets:?} dialog_open={dialog_open}",
                window.is_active(), tree.is_window_active(), tree.focused(), tree.needs_render(),
                tree.requires_curve_geometry(), geometry.as_ref().map(|g| (g.generation, g.is_current())),
                tree.pointer_hit_test(point), geometry.as_ref().and_then(|g| g.hit_test(point))).unwrap();
            // Actual A containment: Dialog -> IME_BODY -> [Editor, Panel].
            for id in [
                fixture::MODAL_ROOT,
                fixture::DIALOG,
                IME_BODY,
                fixture::ROOT,
                fixture::BODY,
                IME_EDITOR,
            ] {
                let node = tree.get(id).unwrap();
                writeln!(log,
                    "m6_pointer_node phase={phase} id={id:?} mount={} frame={:?} visible={:?} allocated_contains={} visible_contains={} curve_bounds={:?}",
                    node.mount_identity(), node.frame(), node.visible(), node.frame().contains(point),
                    node.visible().is_some_and(|rect| rect.contains(point)),
                    geometry.as_ref().map(|g| g.bounds(id, None, node.frame()))).unwrap();
            }
            let surface = &tree
                .get(IME_EDITOR)
                .unwrap()
                .component::<TextFieldContent>()
                .unwrap()
                .surface;
            let metrics = tekes_ui_controls::extended::FocusRingMetrics::new(window.scale() as f32);
            let full_ring = metrics.bounds(surface.outer);
            let dialog = tree.get(fixture::DIALOG).unwrap();
            let modal = dialog
                .component::<DialogContent>()
                .unwrap()
                .semantic_frame(dialog.frame());
            let dialog_clip = dialog.visible().and_then(|clip| modal.intersection(&clip));
            let full_ring_in_dialog =
                dialog_clip.and_then(|clip| full_ring.intersection(&clip)) == Some(full_ring);
            writeln!(log,
                "m6_legal_containment_a phase={phase} backing_scale={} metrics={metrics:?} full_ring={full_ring:?} dialog_modal={modal:?} dialog_clip={dialog_clip:?} full_ring_in_dialog={full_ring_in_dialog} editor_parent={IME_BODY:?} panel_sibling={:?}",
                window.scale(), fixture::ROOT).unwrap();
            writeln!(log,
                "m6_pointer_surface phase={phase} outer={:?} inner={:?} visible={:?} contains_visible={} disabled={} read_only={} marked={:?}",
                surface.outer, surface.inner, surface.visible_frame(), surface.contains_visible(point),
                surface.input.disabled, surface.input.read_only, surface.input.marked).unwrap();
        }
        fn start_ime_script(tree: &ContentHost) {
            use p::selftest::{Action as A, run_script_published_state};
            let zoom = p::content_accessibility::content_key(tree, fixture::COMMANDS[1].0, None)
                .expect("current original Zoom key");
            let mut steps = vec![
                (0, A::AXPressKey { key: zoom }),
                (100, A::AXPressKey { key: zoom }),
                (200, A::AXPressKey { key: zoom }),
                (300, A::AXPressKey { key: zoom }),
                (
                    400,
                    A::ClickPublished {
                        label: IME_EDITOR_LABEL,
                        secondary: false,
                    },
                ),
                (
                    500,
                    A::ExpectText {
                        text: "draft".into(),
                        marked: false,
                    },
                ),
            ];
            // Reset only after the previous native End asserted no global effect.
            // Key-local text/mark/caret changes are legal; native replacement
            // re-establishes an explicit UTF-16 baseline without copying state.
            for (index, (key_code, chars)) in [(48, "\t"), (124, "\u{f703}"), (53, "\u{1b}")]
                .into_iter()
                .enumerate()
            {
                let t = 600 + index as u64 * 800;
                steps.extend([
                    (t, A::Unmark),
                    (t + 100, A::TextCommand("selectAll:")),
                    (t + 200, A::Text("draft".into())),
                    (
                        t + 300,
                        A::ExpectTextState {
                            text: "draft".into(),
                            selection: 5..5,
                            marked: None,
                            query: 0..5,
                            actual: 0..5,
                        },
                    ),
                    (
                        t + 400,
                        A::PreeditRange {
                            text: "拼音".into(),
                            selection: 2..2,
                            replacement: Some(5..5),
                        },
                    ),
                    (
                        t + 500,
                        A::ExpectTextState {
                            text: "draft拼音".into(),
                            selection: 7..7,
                            marked: Some(5..7),
                            query: 0..7,
                            actual: 0..7,
                        },
                    ),
                    (
                        t + 600,
                        A::ExpectText {
                            text: "draft拼音".into(),
                            marked: true,
                        },
                    ),
                    (t + 700, A::KeyCharacters { key_code, chars }),
                ]);
            }
            steps.extend([
                (3100, A::Unmark),
                (
                    3200,
                    A::ClickPublished {
                        label: "Media viewer",
                        secondary: false,
                    },
                ),
                (
                    3300,
                    A::KeyCharacters {
                        key_code: 124,
                        chars: "\u{f703}",
                    },
                ),
                (
                    3400,
                    A::KeyCharacters {
                        key_code: 53,
                        chars: "\u{1b}",
                    },
                ),
                (3500, A::ExpectNoText),
                (3600, A::Close),
            ]);
            run_script_published_state(steps, None);
        }
    }
    impl WindowHandler for Host {
        fn handle(&mut self, event: Event, window: &mut Window) {
            writeln!(self.log, "event={event:?}").unwrap();
            if matches!(&event, Event::Created) {
                writeln!(
                    self.log,
                    "screen={:?} screen_id={:?} placement={:?} window_frame={:?}",
                    window.screen_info(),
                    window.native_screen_id(),
                    window.placement_report(),
                    window.native_window_frame()
                )
                .unwrap();
                self.log.flush().unwrap();
                assert_eq!(
                    window.native_screen_id(),
                    Some(self.external_display_id),
                    "native window must start on the granted external display"
                );
                assert!(
                    window.screen_info().is_some_and(|screen| !screen.builtin),
                    "external display required"
                );
                assert!(window.set_text_key_events(true));
                let mut gpu = unsafe {
                    Renderer::new_for_metal_layer(
                        window.metal_layer(),
                        window.drawable_size(),
                        window.scale() as f32,
                    )
                }
                .unwrap();
                let mut tree = if self.ime_mode {
                    ime_modal_fixture(&mut gpu)
                } else if std::env::var_os("MEDIA_VIEWER_MODAL").is_some() {
                    fixture::modal_fixture(&mut gpu)
                } else {
                    fixture::fixture(&mut gpu)
                };
                if tree.get(fixture::DIALOG).is_some() {
                    let frame = Rect::new(
                        0.,
                        0.,
                        window.size().width as f32,
                        window.size().height as f32,
                    );
                    tree.render(
                        &mut Scene::new(),
                        &mut self.text,
                        frame,
                        frame,
                        Theme::LIGHT.into(),
                    )
                    .unwrap();
                    tree.focus(Some(fixture::INVOKER), true);
                    tree.show_dialog(fixture::DIALOG).unwrap();
                }
                self.tree = Some(tree);
                self.gpu = Some(gpu);
            }
            let Some(tree) = &mut self.tree else {
                return;
            };
            if self.ime_mode
                && let Event::TextKey(p::TextKeyEvent::Started(context)) = &event
                && context.composing_at_start
            {
                assert!(self.ime_before.is_none(), "nested native key frame");
                assert_eq!(
                    context.key.key_code,
                    [48, 124, 53][self.ime_completed],
                    "original Tab/Right/Escape sequence"
                );
                let witness = ime_witness(tree);
                assert_eq!(witness.focused, Some(IME_EDITOR));
                assert!(witness.dialog_open);
                assert!(
                    witness.transform.unwrap().relative_scale > 1.,
                    "Viewer zoom setup must be active; positive Right later verifies horizontal pan"
                );
                writeln!(
                    self.log,
                    "m6_ime_start id={} before={witness:?}",
                    context.id
                )
                .unwrap();
                self.ime_before = Some((context.id, witness));
            }
            // With Viewer focused there is no text input state, so the actual
            // AppKit route is ordinary Key Down/Up, not a TextKey frame.
            let ime_positive_key = if self.ime_mode
                && self.ime_completed == 3
                && tree.focused() == Some(fixture::VIEW)
                && let Event::Key(key) = &event
                && key.key_code == 124
            {
                if key.kind == p::KeyEventKind::Down {
                    assert!(!self.ime_positive_done);
                    assert!(self.ime_positive_before.is_none());
                    assert!(!key.is_repeat);
                    self.ime_positive_before = Some(ime_witness(tree));
                }
                Some(key.kind)
            } else {
                None
            };
            let ime_finish = if self.ime_mode {
                match &event {
                    Event::TextKey(p::TextKeyEvent::Finished { context, .. }) => Some(context.id),
                    _ => None,
                }
            } else {
                None
            };
            match event {
                Event::Wake
                    if self
                        .stale_probe
                        .as_mut()
                        .is_some_and(|probe| probe.requests(tree, window, &mut self.log)) =>
                {
                    window.set_content_accessibility_publication(None);
                    window.close();
                    return;
                }
                Event::Resized {
                    drawable_size,
                    scale,
                    ..
                } => self
                    .gpu
                    .as_mut()
                    .unwrap()
                    .resize(drawable_size, scale as f32),
                Event::Focus(active) => tree.set_window_active(active),
                Event::Mouse(mouse) => {
                    let point = tekes_ui_scene::Point::new(
                        mouse.position.x as f32,
                        mouse.position.y as f32,
                    );
                    let focus_before = tree.focused();
                    if self.ime_mode {
                        Self::write_pointer_witness(tree, window, point, "before", &mut self.log);
                    }
                    let response = tree
                        .event_with_text(&native_pointer::translate(mouse), &mut self.text)
                        .unwrap();
                    if self.ime_mode {
                        writeln!(self.log, "m6_pointer_response point={point:?} focus_before={focus_before:?} focus_after={:?} response={response:?}", tree.focused()).unwrap();
                        Self::write_pointer_witness(tree, window, point, "after", &mut self.log);
                    }
                }
                Event::Key(key) => {
                    tree.event_with_text(
                        &ContentEvent::KeyInput(native_key::key(key)),
                        &mut self.text,
                    )
                    .unwrap();
                }
                Event::TextKey(key) => {
                    let response = tree
                        .event_with_text(&native_key::translate(key), &mut self.text)
                        .unwrap();
                    if self.ime_mode {
                        writeln!(self.log, "m6_text_key_response={response:?}").unwrap();
                        if let Some(id) = ime_finish
                            && self.ime_before.is_some()
                        {
                            let outcome = response
                                .text_key
                                .expect("original Host key acknowledgement");
                            assert_eq!(outcome.id, id);
                            assert_eq!(outcome.target, Some(IME_EDITOR));
                            assert_eq!(outcome.disposition, TextKeyDisposition::InputConsumed);
                        }
                    }
                }
                Event::TextPreedit {
                    text,
                    selection,
                    clauses,
                    replacement,
                } if self.ime_mode => {
                    tree.event_with_text(
                        &ContentEvent::Preedit {
                            text,
                            selection,
                            clauses,
                            replacement,
                        },
                        &mut self.text,
                    )
                    .unwrap();
                }
                Event::TextCommit { text, replacement } if self.ime_mode => {
                    tree.event_with_text(
                        &ContentEvent::Commit { text, replacement },
                        &mut self.text,
                    )
                    .unwrap();
                }
                Event::TextUnmark if self.ime_mode => {
                    tree.event_with_text(&ContentEvent::Unmark, &mut self.text)
                        .unwrap();
                }
                Event::TextCommand(command) => {
                    tree.event_with_text(&ContentEvent::TextCommand(command), &mut self.text)
                        .unwrap();
                }
                Event::Accessibility { target, action } => {
                    let response = window
                        .content_accessibility_publication()
                        .and_then(|publication| {
                            let owner = publication.owner(target)?;
                            Some(publication.dispatch(owner, tree, target, action))
                        })
                        .unwrap_or_default();
                    writeln!(self.log, "ax_response={response:?}").unwrap();
                }
                Event::CloseRequested => {
                    if let Some(probe) = &self.stale_probe {
                        assert_eq!(
                            probe.phase,
                            StalePhase::Complete,
                            "probe must finish before own window close"
                        );
                    }
                    if self.ime_mode {
                        assert!(
                            self.ime_script_started,
                            "original script must start after publication"
                        );
                        assert!(self.ime_before.is_none());
                        assert!(
                            self.ime_positive_done,
                            "unmarked Viewer Right must pan 32pt"
                        );
                        assert_eq!(
                            self.ime_completed, 3,
                            "all three composing native key frames required"
                        );
                        assert!(
                            !tree
                                .get(fixture::DIALOG)
                                .unwrap()
                                .component::<DialogContent>()
                                .unwrap()
                                .is_open()
                        );
                        assert_eq!(tree.focused(), Some(fixture::INVOKER));
                        window.set_text_input_state(None);
                        writeln!(self.log, "M6_NATIVE_IME_PASS composing_keys=3 positive_right=32 positive_escape_restore=20461 physical_ime=false").unwrap();
                    }
                    tree.set_window_active(false);
                    window.set_content_accessibility_publication(None);
                    window.close();
                    return;
                }
                _ => {}
            }
            let (close, retry) = self.actions.dispatch(tree);
            writeln!(
                self.log,
                "close={close} retry={retry} focused={:?} transform={:?} changes={:?}",
                tree.focused(),
                fixture::viewer(tree).transform(),
                fixture::viewer(tree).drain_transform_changes()
            )
            .unwrap();
            if close > 0 && tree.get(fixture::DIALOG).is_some() {
                assert!(tree.close_dialog());
                writeln!(self.log, "modal_close_restored_focus={:?}", tree.focused()).unwrap();
            } else if close > 0 {
                tree.set_window_active(false);
                window.set_content_accessibility_publication(None);
                window.close();
                return;
            }
            if let Some(id) = ime_finish
                && let Some((started, before)) = self.ime_before.take()
            {
                assert_eq!(started, id, "native TextKey Start/End ID mismatch");
                let after = ime_witness(tree);
                assert_eq!(
                    before, after,
                    "composing native key must not escape to global controls"
                );
                self.ime_completed += 1;
                writeln!(self.log, "m6_ime_finish id={id} after={after:?}").unwrap();
            }
            if let Some(kind) = ime_positive_key {
                let mut expected = self
                    .ime_positive_before
                    .as_ref()
                    .expect("positive Right Down must precede Up")
                    .clone();
                expected.transform.as_mut().unwrap().translation.x -= 32.;
                let after = ime_witness(tree);
                assert_eq!(
                    expected, after,
                    "original Viewer Right Down pans exactly32pt; Up adds no changes"
                );
                writeln!(
                    self.log,
                    "m6_ime_positive_right kind={kind:?} after={after:?}"
                )
                .unwrap();
                if kind == p::KeyEventKind::Up {
                    self.ime_positive_before = None;
                    self.ime_positive_done = true;
                }
            }
            // Retry notifies this host once; it performs no I/O or implicit load.
            fixture::refresh_buttons(tree);
            fixture::refresh_status(tree);
            tree.set_window_active(window.is_active());
            self.text.backing = window.scale() as f32;
            self.text.atlas.begin_frame();
            let frame = Rect::new(
                0.,
                0.,
                window.size().width as f32,
                window.size().height as f32,
            );
            let mut scene = Scene::new();
            let palette = Palette::from(Theme::LIGHT);
            tree.render(&mut scene, &mut self.text, frame, frame, palette)
                .unwrap();
            let clear = tekes_ui_tokens::surface_palette::resolve_surface(
                palette, "canvas", "content", "rest",
            )
            .unwrap()
            .background;
            self.gpu
                .as_mut()
                .unwrap()
                .render(&scene, &mut self.text.atlas, clear)
                .unwrap();
            if window.is_active() {
                assert!(window.set_content_accessibility_snapshot(
                    ContentAccessibilitySnapshot::from_host(tree).unwrap()
                ));
            } else {
                window.set_content_accessibility_publication(None);
            }
            if self.ime_mode {
                window.set_text_input_state(tree.focused_text().map(Self::text_state));
                let zoom_key =
                    p::content_accessibility::content_key(tree, fixture::COMMANDS[1].0, None);
                let publication = window.content_accessibility_publication();
                let publication_current = publication.as_ref().is_some_and(|p| p.is_current());
                let zoom_published = zoom_key.is_some_and(|key| {
                    publication.as_ref().is_some_and(|p| p.owner(key).is_some())
                });
                let ready = window.is_active()
                    && tree.is_window_active()
                    && publication_current
                    && zoom_published;
                writeln!(self.log,
                    "m6_script_readiness window_active={} host_active={} zoom_key={zoom_key:?} publication_current={publication_current} zoom_published={zoom_published} started={}",
                    window.is_active(), tree.is_window_active(), self.ime_script_started).unwrap();
                if !self.ime_script_started && ready {
                    // Created can be background. Start once only after the actual
                    // active normal frame has published this original Zoom key.
                    self.ime_script_started = true;
                    writeln!(self.log, "m6_script_start count=1 zoom_key={zoom_key:?}").unwrap();
                    Self::start_ime_script(tree);
                }
            }
            if let Some(probe) = &mut self.stale_probe {
                probe.published(tree, window, &mut self.log);
            }
            writeln!(
                self.log,
                "viewport={:?} image={:?} semantics={:#?}",
                tree.get(fixture::VIEW).unwrap().frame(),
                tree.get(fixture::IMAGE).unwrap().frame(),
                tree.semantics()
            )
            .unwrap();
            self.log.flush().unwrap();
        }
    }
    let directory = std::path::PathBuf::from(
        std::env::var_os("MEDIA_VIEWER_OUTPUT").ok_or("set MEDIA_VIEWER_OUTPUT")?,
    );
    std::fs::create_dir_all(&directory)?;
    if std::fs::read_dir(&directory)?.next().is_some() {
        return Err("use a fresh raw-evidence directory".into());
    }
    let binding =
        std::env::var_os("MEDIA_VIEWER_BINDING").ok_or("set verified run-binding JSON")?;
    std::fs::copy(binding, directory.join("run-binding.json"))?;
    std::fs::write(
        directory.join("process.txt"),
        format!(
            "pid={}\ncurrent_exe={:?}\n",
            std::process::id(),
            std::env::current_exe()?
        ),
    )?;
    let mut log = std::fs::File::create(directory.join("native-events.txt"))?;
    writeln!(
        log,
        "input_source=UNATTESTED; operator must supply provenance; physical IME/VO/performance/pinch NOT_TESTED"
    )?;
    if std::env::var_os("MEDIA_VIEWER_M6_IME").is_some() {
        writeln!(
            log,
            "m6_input_source=PROGRAMMATIC_NATIVE_APPKIT_SELFTEST physical_ime=false cua=false"
        )?;
    }
    let stale_probe = std::env::var_os("MEDIA_VIEWER_M6_STALE_CHANNEL").map(|path| {
        assert!(std::env::var_os("MEDIA_VIEWER_MODAL").is_some());
        assert!(
            std::env::var_os("MEDIA_VIEWER_M6_IME").is_none(),
            "independent probe modes"
        );
        let path = std::path::PathBuf::from(path);
        assert_eq!(
            path,
            directory.join("m6-stale-channel"),
            "channel belongs to fresh owned raw directory"
        );
        StaleProbe::new(path)
    });
    let external_display_id: u32 = std::env::var("MEDIA_VIEWER_EXTERNAL_DISPLAY_ID")?.parse()?;
    App::new(AppConfig {
        name: Some("Media Viewer Consumer 204".into()),
        ..Default::default()
    })
    .run(
        WindowConfig {
            title: "Media Viewer Consumer 204".into(),
            size: p::Size {
                width: 480.,
                height: 720.,
            },
            placement: p::WindowPlacement::Screen(p::ScreenChoice::External),
            ..Default::default()
        },
        MenuBar::default(),
        Host {
            tree: None,
            gpu: None,
            text: coretext::Text::new(1.),
            actions: fixture::Actions::default(),
            log,
            external_display_id,
            stale_probe,
            ime_mode: std::env::var_os("MEDIA_VIEWER_M6_IME").is_some(),
            ime_script_started: false,
            ime_before: None,
            ime_completed: 0,
            ime_positive_before: None,
            ime_positive_done: false,
        },
    );
    Ok(())
}
