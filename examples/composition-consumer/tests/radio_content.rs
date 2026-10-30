#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{
    ControlKey, RadioGroupView, SelectionEvent, composition::*, extended::RadioGroup,
};
use tekes_ui_scene::{Primitive, Rect, Scene};
use tekes_ui_tokens::{Palette, Theme};
const RADIO: ComponentId = ComponentId(109);
fn labels() -> Vec<String> {
    vec!["Same".into(), "".into(), "Same".into()]
}
fn host() -> ContentHost {
    let mut h = ContentHost::new(Content::new(
        RADIO,
        RadioGroupContent::new("Choice", labels()),
    ))
    .unwrap();
    h.set_action_events(true);
    h
}
fn render(h: &mut ContentHost, t: &mut coretext::Text, clip: Rect) {
    h.render(
        &mut Scene::new(),
        t,
        Rect::new(0., 0., 300., 160.),
        clip,
        Theme::LIGHT.into(),
    )
    .unwrap();
}
fn get(h: &ContentHost) -> &RadioGroupContent {
    h.get(RADIO).unwrap().component().unwrap()
}
fn edit(h: &mut ContentHost) -> &mut RadioGroupContent {
    h.get_mut(RADIO).unwrap().component_mut().unwrap()
}
fn parts(h: &ContentHost) -> Vec<SemanticPart> {
    h.semantics()
        .into_iter()
        .find(|n| n.id == RADIO)
        .unwrap()
        .parts
}
fn key(h: &mut ContentHost, k: ControlKey, down: bool) -> ContentResponse {
    h.event(&ContentEvent::Key { key: k, down })
}
fn intent(h: &mut ContentHost) -> RadioSelectionIntent {
    let r = h.drain_action_records();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].action.kind, ContentActionKind::Activate);
    match r[0].payload.clone().unwrap() {
        ContentActionPayload::RadioSelection(i) => i,
        _ => panic!("radio intent"),
    }
}
#[test]
fn controlled_none_some_noop_and_invalid_preserve_original_gesture_and_delivery() {
    let mut h = host();
    let mut t = coretext::Text::new(1.);
    let clip = Rect::new(0., 0., 300., 160.);
    render(&mut h, &mut t, clip);
    let p = parts(&h)[2].frame.center();
    h.event(&ContentEvent::PointerDown(p));
    let focus = get(&h).view().focused();
    edit(&mut h).set_selected(None).unwrap();
    assert_eq!(get(&h).view().focused(), focus);
    assert!(!h.event(&ContentEvent::PointerUp(p)).activated);
    assert_eq!(get(&h).model().selected, None);
    assert!(h.drain_action_records().is_empty());
    h.event(&ContentEvent::PointerDown(p));
    edit(&mut h).set_selected(Some(1)).unwrap();
    assert!(!h.event(&ContentEvent::PointerUp(p)).activated);
    assert_eq!(get(&h).model().selected, Some(1));
    h.event(&ContentEvent::PointerDown(p));
    edit(&mut h).set_selected(Some(1)).unwrap();
    assert!(h.event(&ContentEvent::PointerUp(p)).activated);
    assert_eq!(intent(&mut h).index(), 2);
    edit(&mut h).set_selected(None).unwrap();
    h.event(&ContentEvent::PointerDown(p));
    assert!(edit(&mut h).set_selected(Some(99)).is_err());
    assert!(edit(&mut h).set_item_enabled(99, false).is_err());
    assert!(h.event(&ContentEvent::PointerUp(p)).activated);
    assert_eq!(intent(&mut h).index(), 2);
    edit(&mut h).set_item_enabled(1, false).unwrap();
    edit(&mut h).set_selected(None).unwrap();
    render(&mut h, &mut t, clip);
    h.event(&ContentEvent::PointerDown(p));
    assert!(edit(&mut h).set_selected(Some(1)).is_err());
    assert!(h.event(&ContentEvent::PointerUp(p)).activated);
    assert_eq!(intent(&mut h).index(), 2);
    // Value updates preserve the original delivery index, without re-reading value.
    let first = parts(&h)[0].index;
    h.accessibility_action(RADIO, Some(first), AccessibilityAction::Press);
    edit(&mut h).set_selected(Some(2)).unwrap();
    h.event(&ContentEvent::Cancel);
    assert_eq!(intent(&mut h).index(), 0);
    assert!(h.drain_action_records().is_empty());
}
#[test]
fn original_roving_focus_differs_from_selection_and_space_uses_key_down() {
    let mut h = host();
    let mut t = coretext::Text::new(1.);
    let f = Rect::new(0., 0., 300., 160.);
    render(&mut h, &mut t, f);
    let mut m = RadioGroup::new(3);
    let mut v = RadioGroupView::new();
    v.render(
        &mut Scene::new(),
        &mut t,
        &m,
        f,
        Some(f),
        Theme::LIGHT.into(),
        &["Same", "", "Same"],
    );
    let second = parts(&h)[1].index;
    assert!(
        h.accessibility_action(RADIO, Some(second), AccessibilityAction::Focus)
            .consumed
    );
    v.focus(&m, Some(1), true);
    assert_eq!(h.focused(), Some(RADIO));
    assert_eq!(get(&h).view().focused(), Some(1));
    assert_eq!(get(&h).model().selected, Some(0));
    assert!(h.drain_action_records().is_empty());
    for (k, down) in [
        (ControlKey::Space, true),
        (ControlKey::Space, false),
        (ControlKey::Right, true),
        (ControlKey::Right, false),
        (ControlKey::Right, true),
        (ControlKey::End, true),
        (ControlKey::Home, true),
    ] {
        let original = v.key(k, down, &mut m);
        let r = key(&mut h, k, down);
        assert_eq!(r.consumed, original != SelectionEvent::Ignored);
        assert_eq!(r.activated, original == SelectionEvent::Changed);
        assert_eq!(get(&h).model(), &m);
        assert_eq!(get(&h).view().focused(), v.focused());
        let records = h.drain_action_records();
        assert_eq!(
            records.len(),
            usize::from(original == SelectionEvent::Changed)
        );
    }
    edit(&mut h).set_item_enabled(1, false).unwrap();
    m.set_enabled(1, false);
    key(&mut h, ControlKey::Home, true);
    v.key(ControlKey::Home, true, &mut m);
    key(&mut h, ControlKey::Right, true);
    v.key(ControlKey::Right, true, &mut m);
    assert_eq!(get(&h).model(), &m);
    assert_eq!(get(&h).view().focused(), v.focused());
}
#[test]
fn per_part_clip_activity_availability_aba_and_pending_queue_are_checked() {
    let mut h = host();
    let mut t = coretext::Text::new(1.);
    let f = Rect::new(0., 0., 300., 160.);
    render(&mut h, &mut t, f);
    let old = parts(&h);
    let p = old[2].index;
    h.accessibility_action(RADIO, Some(p), AccessibilityAction::Press);
    let sibling = old[0].index;
    edit(&mut h).set_item_enabled(1, false).unwrap();
    assert_eq!(intent(&mut h).index(), 2); // unrelated availability retains target delivery
    assert_eq!(parts(&h)[0].index, sibling);
    h.accessibility_action(RADIO, Some(sibling), AccessibilityAction::Press);
    render(&mut h, &mut t, Rect::new(0., 0., 300., 35.));
    render(&mut h, &mut t, f);
    assert!(h.drain_action_records().len() == 1); // first remained continuously visible
    assert!(
        !h.accessibility_action(RADIO, Some(p), AccessibilityAction::Press)
            .consumed
    );
    let fresh = parts(&h).last().unwrap().index;
    assert_ne!(fresh, p);
    h.accessibility_action(RADIO, Some(fresh), AccessibilityAction::Press);
    edit(&mut h).set_item_enabled(2, false).unwrap();
    edit(&mut h).set_item_enabled(2, true).unwrap();
    render(&mut h, &mut t, f);
    assert!(h.drain_action_records().is_empty());
    assert!(
        !h.accessibility_action(RADIO, Some(fresh), AccessibilityAction::Focus)
            .consumed
    );
    let token = parts(&h).last().unwrap().index;
    h.accessibility_action(RADIO, Some(token), AccessibilityAction::Press);
    edit(&mut h).set_disabled(true);
    edit(&mut h).set_disabled(false);
    render(&mut h, &mut t, f);
    assert!(h.drain_action_records().is_empty());
    assert!(
        !h.accessibility_action(RADIO, Some(token), AccessibilityAction::Press)
            .consumed
    );
    let token = parts(&h).last().unwrap().index;
    h.set_window_active(false);
    h.set_window_active(true);
    render(&mut h, &mut t, f);
    assert!(
        !h.accessibility_action(RADIO, Some(token), AccessibilityAction::Focus)
            .consumed
    );
    for action in [
        AccessibilityAction::Press,
        AccessibilityAction::SetValue("2".into()),
        AccessibilityAction::SetSelection(0..1),
        AccessibilityAction::ReplaceSelectedText("x".into()),
    ] {
        assert!(!h.accessibility_action(RADIO, None, action).consumed);
    }
}
#[test]
fn empty_duplicate_labels_all_disabled_and_model_fallback_keep_old_apis() {
    let mut h = host();
    let mut t = coretext::Text::new(1.);
    let f = Rect::new(0., 0., 300., 160.);
    render(&mut h, &mut t, f);
    assert_eq!(parts(&h).len(), 3);
    for i in 0..3 {
        edit(&mut h).set_item_enabled(i, false).unwrap();
    }
    assert_eq!(get(&h).model().selected, None);
    render(&mut h, &mut t, f);
    assert_eq!(parts(&h).len(), 3);
    assert!(parts(&h).iter().all(|part| part.semantics.disabled));
    for part in parts(&h) {
        assert!(
            !h.accessibility_action(RADIO, Some(part.index), AccessibilityAction::Press)
                .consumed
        );
    }
    assert!(!h.focus(Some(RADIO), true));
    let mut empty =
        ContentHost::new(Content::new(RADIO, RadioGroupContent::new("", vec![]))).unwrap();
    render(&mut empty, &mut t, f);
    assert!(parts(&empty).is_empty());
    let mut m = RadioGroup::new(3);
    assert!(m.select(2));
    m.set_enabled(2, false);
    assert_eq!(m.selected, Some(0));
    assert!(m.set_selected(None).unwrap());
    assert!(!m.set_selected(None).unwrap());
    assert!(m.set_selected(Some(2)).is_err());
    assert!(RadioGroupContent::from_original("", m, RadioGroupView::new(), vec![]).is_err());
}
fn reference(gpu_out: Option<std::path::PathBuf>) {
    let mut gpu = gpu_out.as_ref().map(|p| {
        assert!(!p.exists());
        std::fs::create_dir_all(p).unwrap();
        tekes_ui_render::Renderer::new_headless().unwrap()
    });
    let mut receipt = vec![];
    for (mode, theme) in Theme::ALL.into_iter().enumerate() {
        for surface in ["canvas", "sidebar", "toolbar"] {
            for phase in ["rest", "focus", "error", "disabled", "pressed"] {
                let mut t = coretext::Text::new(1.5);
                t.backing = 2.;
                t.atlas.begin_frame();
                let palette = if std::env::var_os("TEKESUI_THEME_LOCK").is_some() {
                    Palette::for_theme_definition(theme, "radio-external-inputs")
                        .expect("real external compiled Theme")
                } else {
                    Palette::from(theme)
                };
                let f = Rect::new(12., 12., 280., 180.);
                let clip = Rect::new(16., 16., 250., 140.);
                let strings = vec![
                    "Long wrapped 中文 radio choice paragraph for first-line alignment".to_string(),
                    "".into(),
                    "Last radio choice".into(),
                ];
                let refs: Vec<_> = strings.iter().map(String::as_str).collect();
                let mut h = ContentHost::new(Content::new(
                    RADIO,
                    RadioGroupContent::new("Choice", strings.clone()),
                ))
                .unwrap();
                h.set_inherited_surface(surface).unwrap();
                h.render(&mut Scene::new(), &mut t, f, clip, palette)
                    .unwrap();
                let mut v = RadioGroupView::new();
                v.set_surface(surface).unwrap();
                let m = RadioGroup::new(3);
                v.render(&mut Scene::new(), &mut t, &m, f, Some(clip), palette, &refs);
                match phase {
                    "focus" => {
                        let token = parts(&h)[1].index;
                        h.accessibility_action(RADIO, Some(token), AccessibilityAction::Focus);
                        v.focus(&m, Some(1), true);
                    }
                    "error" => {
                        edit(&mut h).set_error(true);
                        v.set_error(true);
                    }
                    "disabled" => {
                        edit(&mut h).set_disabled(true);
                        v.set_disabled(true);
                    }
                    "pressed" => {
                        let point = v.layout().rows[0].frame().center();
                        h.event(&ContentEvent::PointerDown(point));
                        v.pointer_down(point, &m);
                    }
                    _ => {}
                }
                let mut actual = Scene::new();
                h.render(&mut actual, &mut t, f, clip, palette).unwrap();
                let mut expected = Scene::new();
                expected.push_clip(clip);
                let l = v.render(&mut expected, &mut t, &m, f, Some(clip), palette, &refs);
                expected.pop_clip();
                assert_eq!(get(&h).view().layout(), &l);
                assert_eq!(get(&h).view().focused(), v.focused());
                assert_eq!(
                    format!("{:?}", actual.primitives()),
                    format!("{:?}", expected.primitives())
                );
                if let Some(gpu) = gpu.as_mut() {
                    let background = tekes_ui_tokens::surface_palette::resolve_surface(
                        palette, surface, "content", "rest",
                    )
                    .unwrap()
                    .background;
                    let a = gpu
                        .render_to_rgba8(&actual, &mut t.atlas, (640, 420), 2., background)
                        .unwrap();
                    let e = gpu
                        .render_to_rgba8(&expected, &mut t.atlas, (640, 420), 2., background)
                        .unwrap();
                    assert_eq!(a, e, "same original paint on GPU");
                    assert!(a.chunks_exact(4).any(|p| p != &a[..4]));
                    let file = format!("{mode}-{surface}-{phase}.png");
                    resvg::tiny_skia::Pixmap::from_vec(
                        a,
                        resvg::tiny_skia::IntSize::from_wh(640, 420).unwrap(),
                    )
                    .unwrap()
                    .save_png(gpu_out.as_ref().unwrap().join(&file))
                    .unwrap();
                    receipt.push(serde_json::json!({"mode":mode,"surface":surface,"phase":phase,"file":file,"rgba_difference":0,"scale":1.5,"backing_scale":2.,"external":std::env::var_os("TEKESUI_THEME_LOCK").is_some()}));
                }
                assert!(
                    actual
                        .primitives()
                        .iter()
                        .any(|p| matches!(p, Primitive::Glyphs(_)))
                );
            }
        }
    }
    if let Some(out) = gpu_out {
        std::fs::write(
            out.join("receipt.json"),
            serde_json::to_vec_pretty(&receipt).unwrap(),
        )
        .unwrap();
    }
}
#[test]
fn original_view_geometry_paint_theme_surface_and_real_clip_match() {
    reference(None);
}
#[test]
#[ignore = "explicit actual GPU evidence"]
fn radio_on_actual_gpu() {
    reference(Some(
        std::env::var_os("TEKESUI_RADIO_GPU_OUT").unwrap().into(),
    ));
}
#[test]
fn same_host_composer_marked_binding_reading_and_remount_stay_with_original_owners() {
    use tekes_ui_composition_consumer::scroll_updates::{COMPOSER, ROOT, SCROLLER, Scenario};
    let mut s = Scenario::reading_document().unwrap();
    let version = s.attach_reading("radio-neighbor", 1).unwrap();
    s.host
        .insert(
            ROOT,
            Content::new(RADIO, RadioGroupContent::new("Choice", labels())),
        )
        .unwrap();
    s.host
        .reorder(
            ROOT,
            &[
                RADIO,
                SCROLLER,
                tekes_ui_composition_consumer::scroll_updates::LATEST,
                COMPOSER,
            ],
        )
        .unwrap();
    let mut t = coretext::Text::new(1.);
    t.services = true;
    let f = Rect::new(0., 0., 500., 700.);
    render(&mut s.host, &mut t, f); // render helper deliberately retains a bounded original viewport
    assert!(s.host.focus(Some(COMPOSER), true));
    s.host
        .event_with_text(
            &ContentEvent::Preedit {
                text: "ni".into(),
                selection: 0..2,
                clauses: std::iter::once(0..2).collect(),
                replacement: None,
            },
            &mut t,
        )
        .unwrap();
    let before = s.host.text_input_snapshot().unwrap();
    let binding = s.host.focused_text_binding();
    let epoch = s.host.focus_epoch();
    let mount = s.host.semantic_mount(COMPOSER);
    let keys = s.item_keys();
    let old = parts(&s.host)[1].index;
    for action in [AccessibilityAction::Focus, AccessibilityAction::Press] {
        assert!(
            !s.host
                .accessibility_action(RADIO, Some(old), action)
                .consumed
        );
    }
    edit(&mut s.host).set_selected(None).unwrap();
    edit(&mut s.host).set_item_enabled(1, false).unwrap();
    edit(&mut s.host).set_item_enabled(1, true).unwrap();
    edit(&mut s.host).set_disabled(true);
    edit(&mut s.host).set_disabled(false);
    render(&mut s.host, &mut t, f);
    assert_eq!(s.host.focused_text_binding(), binding);
    assert_eq!(s.host.focus_epoch(), epoch);
    assert_eq!(s.host.semantic_mount(COMPOSER), mount);
    let after = s.host.text_input_snapshot().unwrap();
    assert_eq!(
        (after.text, after.marked, after.selection),
        (before.text, before.marked, before.selection)
    );
    assert_eq!(s.item_keys(), keys);
    assert_eq!(
        s.host
            .get(SCROLLER)
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .reading_document_version(),
        Some(&version)
    );
    assert!(s.host.drain_action_records().is_empty());
    s.host
        .event_with_text(&ContentEvent::Unmark, &mut t)
        .unwrap();
    let token = parts(&s.host)[1].index;
    assert!(
        s.host
            .accessibility_action(RADIO, Some(token), AccessibilityAction::Press)
            .activated
    );
    let removed = s.host.remove(RADIO).unwrap();
    s.host.insert(ROOT, removed).unwrap();
    render(&mut s.host, &mut t, f);
    assert!(s.host.drain_action_records().is_empty());
    assert!(
        !s.host
            .accessibility_action(RADIO, Some(token), AccessibilityAction::Press)
            .consumed
    );
    let fresh = parts(&s.host)[0].index;
    assert!(
        s.host
            .accessibility_action(RADIO, Some(fresh), AccessibilityAction::Press)
            .activated
    );
}

#[derive(Debug)]
struct Gate {
    active: bool,
    stack: StackContent,
}
impl Component for Gate {
    fn active(&self) -> bool {
        self.active
    }
    fn children(&self) -> &[Content] {
        &self.stack.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.stack.children)
    }
    fn measure(
        &mut self,
        t: &mut dyn tekes_ui_controls::ButtonTextRenderer,
        s: tekes_ui_scene::Size,
    ) -> tekes_ui_controls::layout::MeasuredChild {
        self.stack.measure(t, s)
    }
    fn arrange(&mut self, t: &mut dyn tekes_ui_controls::ButtonTextRenderer, f: Rect, c: Rect) {
        self.stack.arrange(t, f, c)
    }
}
#[test]
fn actual_inactive_ancestor_hidden_restore_and_invalid_surface_do_not_revive_authority() {
    use tekes_ui_controls::extended::{Axis, Stack};
    let gate = ComponentId(700);
    let mut h = ContentHost::new(Content::new(
        gate,
        Gate {
            active: true,
            stack: StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: 0.,
                },
                children: vec![Content::new(
                    RADIO,
                    RadioGroupContent::new("Choice", labels()),
                )],
                flexible_child: None,
            },
        },
    ))
    .unwrap();
    h.set_action_events(true);
    let mut t = coretext::Text::new(1.);
    let f = Rect::new(0., 0., 300., 160.);
    render(&mut h, &mut t, f);
    let old = parts(&h)[1].index;
    h.accessibility_action(RADIO, Some(old), AccessibilityAction::Press);
    h.get_mut(gate)
        .unwrap()
        .component_mut::<Gate>()
        .unwrap()
        .active = false;
    h.event(&ContentEvent::Cancel);
    assert!(h.drain_action_records().is_empty());
    h.get_mut(gate)
        .unwrap()
        .component_mut::<Gate>()
        .unwrap()
        .active = true;
    render(&mut h, &mut t, f);
    assert!(
        !h.accessibility_action(RADIO, Some(old), AccessibilityAction::Focus)
            .consumed
    );
    let old = parts(&h)[0].index;
    h.accessibility_action(RADIO, Some(old), AccessibilityAction::Press);
    render(&mut h, &mut t, Rect::new(900., 900., 20., 20.));
    render(&mut h, &mut t, f);
    assert!(h.drain_action_records().is_empty());
    assert!(
        !h.accessibility_action(RADIO, Some(old), AccessibilityAction::Press)
            .consumed
    );
    let p = parts(&h)[2].frame.center();
    let tokens = parts(&h);
    h.event(&ContentEvent::PointerDown(p));
    assert!(h.set_inherited_surface("invalid-surface").is_err());
    assert_eq!(
        parts(&h).iter().map(|p| p.index).collect::<Vec<_>>(),
        tokens.iter().map(|p| p.index).collect::<Vec<_>>()
    );
    assert!(h.event(&ContentEvent::PointerUp(p)).activated);
    assert_eq!(intent(&mut h).index(), 2);
}
#[test]
fn scaled_narrow_wide_wrapped_layout_and_focus_ring_match_original_view() {
    for scale in [1., 1.5, 2.] {
        for width in [72., 360.] {
            let mut t = coretext::Text::new(scale);
            let f = Rect::new(8., 8., width, 400.);
            let clip = Rect::new(0., 0., 500., 500.);
            let labels = vec![
                "中文 first line and wrapped paragraph original geometry".to_string(),
                "Second".into(),
            ];
            let refs: Vec<_> = labels.iter().map(String::as_str).collect();
            let mut h = ContentHost::new(Content::new(
                RADIO,
                RadioGroupContent::new("Choice", labels.clone()),
            ))
            .unwrap();
            h.render(&mut Scene::new(), &mut t, f, clip, Theme::LIGHT.into())
                .unwrap();
            let m = RadioGroup::new(2);
            let mut v = RadioGroupView::new();
            let layout = v.render(
                &mut Scene::new(),
                &mut t,
                &m,
                f,
                Some(clip),
                Theme::LIGHT.into(),
                &refs,
            );
            assert_eq!(get(&h).view().layout(), &layout);
            let token = parts(&h)[0].index;
            h.accessibility_action(RADIO, Some(token), AccessibilityAction::Focus);
            v.focus(&m, Some(0), true);
            let mut a = Scene::new();
            h.render(&mut a, &mut t, f, clip, Theme::LIGHT.into())
                .unwrap();
            let mut b = Scene::new();
            b.push_clip(clip);
            v.render(
                &mut b,
                &mut t,
                &m,
                f,
                Some(clip),
                Theme::LIGHT.into(),
                &refs,
            );
            b.pop_clip();
            assert_eq!(
                format!("{:?}", a.primitives()),
                format!("{:?}", b.primitives())
            );
        }
    }
}

#[test]
fn recovery_does_not_admit_pointer_before_successful_layout_reexposure() {
    let mut h = host();
    let mut t = coretext::Text::new(1.);
    let f = Rect::new(0., 0., 300., 160.);
    render(&mut h, &mut t, f);
    let p = parts(&h)[2].frame.center();
    edit(&mut h).set_selected(None).unwrap();
    for mode in ["window", "group", "part"] {
        match mode {
            "window" => {
                h.set_window_active(false);
                h.set_window_active(true);
            }
            "group" => {
                edit(&mut h).set_disabled(true);
                edit(&mut h).set_disabled(false);
            }
            _ => {
                edit(&mut h).set_item_enabled(2, false).unwrap();
                render(&mut h, &mut t, f);
                edit(&mut h).set_item_enabled(2, true).unwrap();
            }
        }
        assert!(!h.event(&ContentEvent::PointerDown(p)).capture_pointer);
        assert!(!h.event(&ContentEvent::PointerUp(p)).activated);
        assert_eq!(get(&h).model().selected, None);
        assert!(h.drain_action_records().is_empty());
        render(&mut h, &mut t, f);
        assert!(h.event(&ContentEvent::PointerDown(p)).capture_pointer);
        assert!(h.event(&ContentEvent::PointerUp(p)).activated);
        assert_eq!(intent(&mut h).index(), 2);
        edit(&mut h).set_selected(None).unwrap();
    }
}
