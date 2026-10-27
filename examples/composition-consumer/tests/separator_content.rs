//! Public Separator consumer; CoreText/Scene API evidence only, no GPU/window.
#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
#[path = "support/separator_fixture.rs"]
mod separator_content;
use tekes_ui_controls::{composition::*, extended::Axis};
use tekes_ui_scene::{Primitive, Rect, Scene};
use tekes_ui_tokens::Theme;
fn marks(scene: &Scene) -> Vec<(Rect, Rect)> {
    let mut clips = vec![Some(Rect::new(0., 0., 1000., 1000.))];
    let mut out = vec![];
    for p in scene.primitives() {
        match p {
            Primitive::PushClip(r) => clips.push(
                clips
                    .last()
                    .copied()
                    .flatten()
                    .and_then(|c| c.intersection(r)),
            ),
            Primitive::PopClip => {
                clips.pop();
            }
            Primitive::Quad(q) => {
                if let Some(c) = clips.last().copied().flatten()
                    && let Some(v) = q.rect.intersection(&c)
                {
                    out.push((q.rect, v));
                }
            }
            _ => {}
        }
    }
    out
}
#[test]
fn ordinary_public_tree_has_original_passive_separator() {
    let host = separator_content::build(Axis::Horizontal).unwrap();
    assert!(
        host.get(separator_content::DIVIDER)
            .unwrap()
            .component::<SeparatorContent>()
            .is_some()
    );
}
#[test]
fn tiny_allocations_and_sub_one_backing_never_paint_outside_the_allocated_leaf() {
    for axis in [Axis::Horizontal, Axis::Vertical] {
        for backing in [0.5, 1., 2.] {
            let frame = if axis == Axis::Horizontal {
                Rect::new(20., 20., 40., 0.125)
            } else {
                Rect::new(20., 20., 0.125, 40.)
            };
            let mut h = ContentHost::new(Content::new(ComponentId(1), SeparatorContent::new(axis)))
                .unwrap();
            let mut text = coretext::Text::new(1.);
            text.backing = backing;
            let mut scene = Scene::new();
            h.render(
                &mut scene,
                &mut text,
                frame,
                Rect::new(0., 0., 100., 100.),
                Theme::LIGHT.into(),
            )
            .unwrap();
            let marks = marks(&scene);
            assert_eq!(marks.len(), 1);
            let (_, visible) = marks[0];
            assert_eq!(
                visible.intersection(&frame),
                Some(visible),
                "real decorative ink overflow: axis={axis:?}, backing={backing}, frame={frame:?}, visible={visible:?}"
            );
        }
    }
}

use std::{cell::RefCell, rc::Rc};
use tekes_ui_controls::{
    ButtonTextRenderer, ControlKey, ReadingDocumentVersion, ScrollArea,
    extended::{Axis as LayoutAxis, Stack},
    layout::MeasuredChild,
    layout_view::SeparatorView,
};
use tekes_ui_scene::{Point, Size};
use tekes_ui_tokens::{Palette, proposal};
#[test]
fn logical_measurement_is_finite_clamped_and_independent_of_backing_and_typography() {
    let rule = proposal::metric("stroke", "stroke-rule").unwrap();
    assert_eq!(SeparatorContent::default().model.axis, Axis::Horizontal);
    for axis in [Axis::Horizontal, Axis::Vertical] {
        for backing in [0.5, 1., 2., 0., f32::NAN] {
            for type_scale in [1., 1.5] {
                let mut text = coretext::Text::new(type_scale);
                text.backing = backing;
                let mut s = SeparatorContent::new(axis);
                let size = s.measure(&mut text, Size::new(120., 80.)).size;
                assert_eq!(
                    size,
                    if axis == Axis::Horizontal {
                        Size::new(120., rule)
                    } else {
                        Size::new(rule, 80.)
                    }
                );
                assert_eq!(
                    s.measure(&mut text, Size::new(0.125, 0.125)).size,
                    Size::new(0.125, 0.125)
                );
                assert_eq!(
                    s.measure(&mut text, Size::new(f32::INFINITY, f32::NAN))
                        .size,
                    Size::default()
                );
                assert_eq!(
                    s.measure(&mut text, Size::new(-1., -2.)).size,
                    Size::default()
                );
            }
        }
    }
}
#[test]
fn ordinary_adapter_uses_exact_original_palette_scale_and_view_geometry() {
    for theme in Theme::ALL {
        for brand in 0..3 {
            for axis in [Axis::Horizontal, Axis::Vertical] {
                for backing in [0.5, 1., 1.5, 2., 0., f32::NAN] {
                    let palette = Palette::new(theme, brand).unwrap();
                    let frame = Rect::new(12.25, 20.5, 120., 80.);
                    let clip = Rect::new(18., 16., 90., 100.);
                    let mut h = ContentHost::new(
                        Content::new(ComponentId(1), SeparatorContent::new(axis))
                            .with_surface("code")
                            .unwrap(),
                    )
                    .unwrap();
                    h.set_inherited_surface("sidebar").unwrap();
                    let mut t = coretext::Text::new(1.);
                    t.backing = backing;
                    let mut actual = Scene::new();
                    h.render(&mut actual, &mut t, frame, clip, palette).unwrap();
                    let mut view = SeparatorView::default();
                    let mut expected = Scene::new();
                    let layout = view.render_at_scale(
                        &mut expected,
                        frame,
                        axis,
                        Some(clip),
                        palette,
                        backing,
                    );
                    let quad = |s: &Scene| {
                        s.primitives()
                            .iter()
                            .filter_map(|p| {
                                if let Primitive::Quad(q) = p {
                                    Some(*q)
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>()
                    };
                    assert_eq!(quad(&actual), quad(&expected));
                    assert_eq!(quad(&actual)[0].fill, palette.color("separator").unwrap());
                    assert_eq!(
                        h.get(ComponentId(1))
                            .unwrap()
                            .component::<SeparatorContent>()
                            .unwrap()
                            .layout(),
                        layout
                    );
                }
            }
        }
    }
}
#[test]
fn decorative_leaf_has_no_focus_action_ax_or_editor_services() {
    let mut h = separator_content::build(Axis::Horizontal).unwrap();
    h.set_action_events(true);
    let mut t = coretext::Text::new(1.);
    let frame = Rect::new(0., 0., 240., 120.);
    h.render(&mut Scene::new(), &mut t, frame, frame, Theme::LIGHT.into())
        .unwrap();
    let id = separator_content::DIVIDER;
    assert!(!h.semantics().iter().any(|n| n.id == id));
    assert!(!h.focus_targets().iter().any(|(target, _)| *target == id));
    assert!(!h.focus(Some(id), true));
    assert!(
        !h.accessibility_action(id, None, AccessibilityAction::Press)
            .activated
    );
    assert!(h.text_input_snapshot().is_none());
    let line = h
        .get_mut(id)
        .unwrap()
        .component_mut::<SeparatorContent>()
        .unwrap();
    assert!(!line.interactive());
    assert!(!line.focusable());
    assert!(!line.requires_text_services());
    for event in [
        ContentEvent::PointerDown(Point::new(4., 4.)),
        ContentEvent::Key {
            key: ControlKey::Enter,
            down: true,
        },
        ContentEvent::Scroll {
            position: Point::new(4., 4.),
            delta: Point::new(0., 5.),
            precise: true,
            begin: true,
            end: true,
            momentum: false,
            time: 1.,
        },
    ] {
        assert_eq!(line.event(&event), ContentResponse::default());
    }
    assert!(h.drain_action_records().is_empty());
}
#[derive(Debug)]
struct RouteProbe {
    children: Vec<Content>,
    events: Rc<RefCell<Vec<&'static str>>>,
}
impl Component for RouteProbe {
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        MeasuredChild::new(a)
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        self.children[0].arrange(t, f, c);
    }
    fn interactive(&self) -> bool {
        true
    }
    fn event(&mut self, e: &ContentEvent) -> ContentResponse {
        let name = match e {
            ContentEvent::PointerDown(_) => "down",
            ContentEvent::PointerUp(_) => "up",
            ContentEvent::Scroll { .. } => "wheel",
            _ => return ContentResponse::default(),
        };
        self.events.borrow_mut().push(name);
        ContentResponse {
            consumed: true,
            capture_pointer: matches!(e, ContentEvent::PointerDown(_)),
            ..Default::default()
        }
    }
}
#[test]
fn original_parent_routing_is_identical_with_passive_separator_or_empty_slot() {
    let mut results = vec![];
    for separator in [false, true] {
        let seen = Rc::new(RefCell::new(vec![]));
        let child = if separator {
            Content::new(ComponentId(901), SeparatorContent::default())
        } else {
            Content::new(ComponentId(901), TextContent::new(""))
        };
        let mut h = ContentHost::new(Content::new(
            ComponentId(900),
            RouteProbe {
                children: vec![child],
                events: seen.clone(),
            },
        ))
        .unwrap();
        h.set_action_events(true);
        let mut t = coretext::Text::new(1.);
        let frame = Rect::new(0., 0., 100., 80.);
        h.render(&mut Scene::new(), &mut t, frame, frame, Theme::LIGHT.into())
            .unwrap();
        let p = frame.center();
        assert!(h.event(&ContentEvent::PointerDown(p)).capture_pointer);
        assert_eq!(h.pointer_capture().unwrap().0, ComponentId(900));
        h.event(&ContentEvent::PointerUp(p));
        assert!(
            h.event(&ContentEvent::Scroll {
                position: p,
                delta: Point::new(0., 10.),
                precise: true,
                begin: true,
                end: true,
                momentum: false,
                time: 1.
            })
            .consumed
        );
        assert!(h.drain_action_records().is_empty());
        results.push(seen.borrow().clone());
    }
    assert_eq!(results[0], vec!["down", "up", "wheel"]);
    assert_eq!(results[0], results[1]);
}

const EDITOR: ComponentId = ComponentId(54101);
const SCROLL: ComponentId = ComponentId(54102);
const DOC: ComponentId = ComponentId(54103);
const DIV_ROW: ComponentId = ComponentId(54300);
const SEP: ComponentId = ComponentId(54301);
#[derive(Debug)]
struct Reserved {
    children: Vec<Content>,
    extent: f32,
}
impl Component for Reserved {
    fn children(&self) -> &[Content] {
        &self.children
    }
    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(a.width, self.extent.min(a.height)))
    }
    fn arrange(&mut self, t: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        self.children[0].arrange(t, f, c);
    }
}
const CHECKED_SPLIT: ComponentId = ComponentId(56000);
const CHECKED_LEAF: ComponentId = ComponentId(56004);
fn reader(
    checked_separator: bool,
) -> (
    ContentHost,
    coretext::Text,
    ReadingDocumentVersion,
    Vec<ComponentId>,
) {
    let items = (0..20).map(|i| ComponentId(55000 + i)).collect::<Vec<_>>();
    let children=items.iter().enumerate().map(|(i,id)|Content::new(*id,TextContent::new(format!("Section {i}: original reading content 中文; stable section identity and original geometry.")))).collect();
    let history = Content::new(
        SCROLL,
        ScrollContent::new(
            ScrollArea::default(),
            Content::new(
                DOC,
                StackContent {
                    model: Stack {
                        axis: LayoutAxis::Vertical,
                        gap: 2.,
                    },
                    flexible_child: None,
                    children,
                },
            ),
        ),
    );
    let mut editor = TextFieldContent::new("Composer", "draft");
    editor.submit_policy = TextSubmitPolicy::Enter;
    let root = Content::new(
        ComponentId(54100),
        StackContent {
            model: Stack {
                axis: LayoutAxis::Vertical,
                gap: 4.,
            },
            flexible_child: Some(SCROLL),
            children: vec![Content::new(EDITOR, editor), history],
        },
    );
    let root = if checked_separator {
        Content::new(
            CHECKED_SPLIT,
            SplitViewContent::try_new(SplitViewConstruction {
                config: SplitViewConfig::default(),
                slots: SplitViewSlotIds {
                    leading: ComponentId(56001),
                    trailing: ComponentId(56002),
                    divider: ComponentId(56003),
                },
                leading: Content::new(CHECKED_LEAF, TextContent::new("Section boundary")),
                trailing: root,
                budget: 64 * 1024 * 1024,
            })
            .unwrap(),
        )
    } else {
        root
    };
    let mut h = ContentHost::new(root).unwrap();
    h.set_action_events(true);
    let (version, ()) = h
        .set_scroll_reading_document(SCROLL, "separator-sections", 1, items.clone(), |_| ())
        .unwrap();
    let mut text = coretext::Text::new(1.);
    text.services = true;
    text.reduced_motion = true;
    reader_paint(&mut h, &mut text, 360., Theme::LIGHT);
    (h, text, version, items)
}
fn reader_paint(h: &mut ContentHost, t: &mut coretext::Text, width: f32, theme: Theme) {
    let f = Rect::new(0., 0., width, 280.);
    t.atlas.begin_frame();
    h.render(&mut Scene::new(), t, f, f, theme.into()).unwrap();
}
fn scroll(h: &ContentHost) -> &ScrollContent {
    h.get(SCROLL).unwrap().component().unwrap()
}
fn wheel(h: &mut ContentHost, delta: f32, time: f64) {
    let p = h.get(SCROLL).unwrap().frame().center();
    assert!(
        h.event(&ContentEvent::Scroll {
            position: p,
            delta: Point::new(0., delta),
            precise: true,
            begin: true,
            end: true,
            momentum: false,
            time
        })
        .consumed
    );
}
#[test]
fn versioned_insert_extent_remove_and_reflow_keep_original_anchor_and_composer_ime() {
    let (mut h, mut text, mut version, items) = reader(false);
    h.get_mut(SCROLL)
        .unwrap()
        .component_mut::<ScrollContent>()
        .unwrap()
        .follow_latest();
    reader_paint(&mut h, &mut text, 360., Theme::LIGHT);
    wheel(&mut h, -120., 1.);
    reader_paint(&mut h, &mut text, 360., Theme::LIGHT);
    assert!(!scroll(&h).is_following_latest());
    let anchor = *items
        .iter()
        .find(|id| h.get(**id).unwrap().visible().is_some())
        .unwrap();
    let y = h.get(anchor).unwrap().frame().min_y();
    let mount = h.semantic_mount(anchor);
    assert!(h.focus(Some(EDITOR), true));
    h.event(&ContentEvent::Preedit {
        text: "ni".into(),
        selection: 2..2,
        clauses: std::iter::once(0..2).collect(),
        replacement: None,
    });
    reader_paint(&mut h, &mut text, 360., Theme::LIGHT);
    let original = h.text_input_snapshot().unwrap();
    assert!(original.marked.is_some());
    let editor_mount = h.semantic_mount(EDITOR);
    let mut width = 360.;
    let mut old_editor_frame = h.get(EDITOR).unwrap().frame();
    let mut old_snapshot = original.clone();
    for stage in 0..4 {
        let next = version.revision() + 1;
        let (v, ()) = h
            .update_scroll_reading_document(
                SCROLL,
                &version,
                next,
                items.clone(),
                |h| match stage {
                    0 => {
                        h.insert(
                            DOC,
                            Content::new(
                                DIV_ROW,
                                Reserved {
                                    extent: 1.,
                                    children: vec![Content::new(SEP, SeparatorContent::default())],
                                },
                            ),
                        )
                        .unwrap();
                        let mut order = items.clone();
                        order.insert(1, DIV_ROW);
                        h.reorder(DOC, &order).unwrap();
                    }
                    1 => {
                        h.get_mut(DIV_ROW)
                            .unwrap()
                            .component_mut::<Reserved>()
                            .unwrap()
                            .extent = 31.
                    }
                    2 => {
                        h.remove(DIV_ROW).unwrap();
                    }
                    _ => {}
                },
            )
            .unwrap();
        version = v;
        if stage == 3 {
            width = 280.;
        }
        reader_paint(
            &mut h,
            &mut text,
            width,
            if stage % 2 == 0 {
                Theme::DARK
            } else {
                Theme::LIGHT
            },
        );
        let result = scroll(&h).reading_result().unwrap();
        assert_eq!(result.version, version);
        assert_eq!(result.anchor, ContentReadingAnchorUpdate::Preserved(anchor));
        assert!(!result.clamped);
        assert!((h.get(anchor).unwrap().frame().min_y() - y).abs() <= 1.);
        assert_eq!(h.semantic_mount(anchor), mount);
        let input = h.text_input_snapshot().unwrap();
        let frame = h.get(EDITOR).unwrap().frame();
        let dx = frame.min_x() - old_editor_frame.min_x();
        let dy = frame.min_y() - old_editor_frame.min_y();
        let translate =
            |r: Rect| Rect::new(r.min_x() + dx, r.min_y() + dy, r.size.width, r.size.height);
        assert_eq!(input.binding, original.binding);
        assert_eq!(input.text, original.text);
        assert_eq!(input.marked, original.marked);
        assert_eq!(input.selection, original.selection);
        assert_eq!(input.caret_index, original.caret_index);
        assert_eq!(input.caret, old_snapshot.caret.map(translate));
        assert_eq!(
            input.carets,
            old_snapshot
                .carets
                .iter()
                .map(|(i, r)| (*i, translate(*r)))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            input.clip,
            old_snapshot.clip.map(|r| Rect::new(
                r.min_x() + dx,
                r.min_y() + dy,
                r.size.width + frame.size.width - old_editor_frame.size.width,
                r.size.height
            ))
        );
        assert_eq!(h.focused(), Some(EDITOR));
        assert_eq!(h.semantic_mount(EDITOR), editor_mount);
        assert!(h.drain_action_records().is_empty());
        let gate = h.event(&ContentEvent::Key {
            key: ControlKey::Enter,
            down: true,
        });
        assert!(
            !gate.activated,
            "the original marked Composer still rejects submit"
        );
        h.event(&ContentEvent::Key {
            key: ControlKey::Enter,
            down: false,
        });
        assert!(h.drain_action_records().is_empty());
        let after_gate = h.text_input_snapshot().unwrap();
        assert_eq!(after_gate.marked, original.marked);
        assert_eq!(after_gate.binding, original.binding);
        assert_eq!(after_gate.selection, original.selection);
        old_snapshot = input;
        old_editor_frame = frame;
    }
    // Fresh original upward intent wins a same-tick business document update.
    h.get_mut(SCROLL)
        .unwrap()
        .component_mut::<ScrollContent>()
        .unwrap()
        .follow_latest();
    reader_paint(&mut h, &mut text, width, Theme::LIGHT);
    wheel(&mut h, -30., 2.);
    let (v, ()) = h
        .update_scroll_reading_document(
            SCROLL,
            &version,
            version.revision() + 1,
            items.clone(),
            |_| (),
        )
        .unwrap();
    version = v;
    reader_paint(&mut h, &mut text, width, Theme::LIGHT);
    assert!(scroll(&h).reading_result().unwrap().user_input_won);
    assert!(!scroll(&h).is_following_latest());
    h.get_mut(SCROLL)
        .unwrap()
        .component_mut::<ScrollContent>()
        .unwrap()
        .follow_latest();
    reader_paint(&mut h, &mut text, width, Theme::LIGHT);
    assert_eq!(
        scroll(&h).reading_result().unwrap().anchor,
        ContentReadingAnchorUpdate::FollowingLatest
    );
    assert_eq!(scroll(&h).reading_result().unwrap().version, version);
    assert!(h.drain_action_records().is_empty());
}

#[test]
fn natural_text_fractional_layout_keeps_original_view_effective_clip() {
    fn quad_clips(scene: &Scene, viewport: Rect) -> Vec<Rect> {
        let mut clips = vec![viewport];
        let mut result = Vec::new();
        for primitive in scene.primitives() {
            match primitive {
                Primitive::PushClip(rect) => {
                    clips.push(clips.last().unwrap().intersection(rect).unwrap_or_default())
                }
                Primitive::PopClip => {
                    clips.pop();
                }
                Primitive::Quad(_) => result.push(*clips.last().unwrap()),
                _ => {}
            }
        }
        result
    }
    for axis in [Axis::Horizontal, Axis::Vertical] {
        for backing in [1., 2.] {
            let mut host = separator_content::build(axis).unwrap();
            let mut text = coretext::Text::new(1.);
            text.backing = backing;
            // Text's original natural advance locates the vertical leaf;
            // the fractional host origin also exercises horizontal placement.
            let viewport = Rect::new(16., 28.125, 440., 100.);
            host.render(
                &mut Scene::new(),
                &mut text,
                viewport,
                viewport,
                Palette::from(Theme::LIGHT),
            )
            .unwrap();
            let frame = host.get(separator_content::DIVIDER).unwrap().frame();
            let short_origin = if axis == Axis::Horizontal {
                frame.min_y()
            } else {
                frame.min_x()
            };
            assert!(
                short_origin.fract().abs() > 0.001,
                "natural fractional placement required: {frame:?}"
            );
            let mut actual = Scene::new();
            host.get_mut(separator_content::DIVIDER)
                .unwrap()
                .component_mut::<SeparatorContent>()
                .unwrap()
                .paint(
                    &mut actual,
                    &mut text,
                    frame,
                    viewport,
                    Palette::from(Theme::LIGHT).into(),
                );
            let mut expected = Scene::new();
            SeparatorView::default().render_at_scale(
                &mut expected,
                frame,
                axis,
                Some(viewport),
                Palette::from(Theme::LIGHT),
                backing,
            );
            let actual_clips = quad_clips(&actual, viewport);
            let expected_clips = quad_clips(&expected, viewport);
            assert_eq!(actual_clips.len(), 1);
            assert_eq!(
                actual_clips, expected_clips,
                "normal allocation must retain original View clip/quantization: axis={axis:?}, backing={backing}, frame={frame:?}"
            );
            assert_eq!(marks(&actual), marks(&expected));
        }
    }
}

#[test]
fn checked_separator_mount_and_retirement_preserve_reading_and_marked_composer() {
    use tekes_ui_controls::extended::Pane;
    let (mut h, mut text, version, items) = reader(true);
    h.get_mut(SCROLL)
        .unwrap()
        .component_mut::<ScrollContent>()
        .unwrap()
        .follow_latest();
    reader_paint(&mut h, &mut text, 360., Theme::LIGHT);
    wheel(&mut h, -120., 1.);
    reader_paint(&mut h, &mut text, 360., Theme::LIGHT);
    assert!(!scroll(&h).is_following_latest());
    let anchor = *items
        .iter()
        .find(|id| h.get(**id).unwrap().visible().is_some())
        .unwrap();
    let anchor_y = h.get(anchor).unwrap().frame().min_y();
    let anchor_mount = h.semantic_mount(anchor);
    assert!(h.focus(Some(EDITOR), true));
    h.event(&ContentEvent::Preedit {
        text: "ni".into(),
        selection: 2..2,
        clauses: std::iter::once(0..2).collect(),
        replacement: None,
    });
    reader_paint(&mut h, &mut text, 360., Theme::LIGHT);
    let original = h.text_input_snapshot().unwrap();
    let editor_mount = h.semantic_mount(EDITOR);
    assert!(original.marked.is_some());
    for separator in [true, false] {
        let expected = h.split_view_expected(CHECKED_SPLIT).unwrap();
        let replacement = if separator {
            Content::new(CHECKED_LEAF, SeparatorContent::default())
        } else {
            Content::new(CHECKED_LEAF, TextContent::new("Section boundary"))
        };
        // This public API executes the existing checked prepare/commit path;
        // its finite hooks must accept both the new and retiring passive leaf.
        let retired = h
            .replace_split_view_pane(SplitViewReplacement {
                owner: CHECKED_SPLIT,
                expected,
                pane: Pane::Leading,
                replacement,
                budget: 64 * 1024 * 1024,
            })
            .unwrap();
        if separator {
            assert!(retired.component::<TextContent>().is_some());
        } else {
            assert!(retired.component::<SeparatorContent>().is_some());
        }
        let current = h.split_view_expected(CHECKED_SPLIT).unwrap();
        assert_eq!(current.trailing_mount, expected.trailing_mount);
        assert_ne!(current.leading_mount, expected.leading_mount);
        assert_eq!(
            h.get(CHECKED_LEAF)
                .unwrap()
                .component::<SeparatorContent>()
                .is_some(),
            separator
        );
        reader_paint(
            &mut h,
            &mut text,
            360.,
            if separator { Theme::DARK } else { Theme::LIGHT },
        );
        let reading = scroll(&h).reading_result().unwrap();
        assert_eq!(reading.version, version);
        assert_eq!(
            reading.anchor,
            ContentReadingAnchorUpdate::Preserved(anchor)
        );
        assert!(!reading.clamped);
        assert!((h.get(anchor).unwrap().frame().min_y() - anchor_y).abs() <= 1.);
        assert_eq!(h.semantic_mount(anchor), anchor_mount);
        let input = h.text_input_snapshot().unwrap();
        assert_eq!(input.binding, original.binding);
        assert_eq!(input.text, original.text);
        assert_eq!(input.marked, original.marked);
        assert_eq!(input.selection, original.selection);
        assert_eq!(input.caret, original.caret);
        assert_eq!(input.clip, original.clip);
        assert_eq!(input.carets, original.carets);
        assert_eq!(h.focused(), Some(EDITOR));
        assert_eq!(h.semantic_mount(EDITOR), editor_mount);
        assert!(h.drain_action_records().is_empty());
        assert!(
            !h.event(&ContentEvent::Key {
                key: ControlKey::Enter,
                down: true
            })
            .activated
        );
        h.event(&ContentEvent::Key {
            key: ControlKey::Enter,
            down: false,
        });
        assert!(h.drain_action_records().is_empty());
    }
}
