//! Pending red/green Scene-order tests for original Disclosure/Content paint.
//! Uses real SVG resources and CoreText; no framebuffer/desktop/VO acceptance.
#![cfg(all(target_os = "macos", feature = "svg-icons"))]
#[path = "support/coretext.rs"]
mod coretext;
use std::time::Duration;
use tekes_ui_controls::{
    ButtonFramePolicy,
    composition::*,
    extended::{Axis, Stack},
    paint::ButtonVariant,
};
use tekes_ui_scene::{Primitive, Quad, Rect, Scene};
use tekes_ui_tokens::{Theme, proposal};
const DISCLOSURE: ComponentId = ComponentId(1);
const TRIGGER: ComponentId = ComponentId(2);
const BODY: ComponentId = ComponentId(4);
const BODY_BUTTON: ComponentId = ComponentId(5);
const FRAME: Rect = Rect::new(24., 24., 260., 180.);
const VIEWPORT: Rect = Rect::new(0., 0., 340., 260.);

fn fixture_with_body(root_button: bool) -> ContentHost {
    let mut button = Button::new("Opaque original body button");
    button.variant = ButtonVariant::Primary;
    button.button.frame_policy = ButtonFramePolicy::Exact;
    // A real public body container makes the body's focus predicate recursive:
    // the Stack owns no focus, its original exact Button descendant does.
    let body_button = Content::new(BODY_BUTTON, button);
    let body = if root_button {
        body_button
    } else {
        Content::new(
            BODY,
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: 0.,
                },
                flexible_child: None,
                children: vec![body_button],
            },
        )
    };
    let mut host = ContentHost::new(Content::new(
        DISCLOSURE,
        DisclosureContent::new(
            TRIGGER,
            Content::new(ComponentId(3), TextContent::new("Disclosure")),
            body,
            true,
        ),
    ))
    .unwrap();
    host.set_action_events(true);
    host
}
fn fixture() -> ContentHost {
    fixture_with_body(false)
}
fn text() -> coretext::Text {
    let mut text = coretext::Text::new(1.);
    text.services = true;
    text.backing = 1.;
    text.reduced_motion = false;
    text
}
fn paint(
    host: &mut ContentHost,
    text: &mut coretext::Text,
    gpu: &mut tekes_ui_render::Renderer,
    cache: &mut SvgIconCache,
) -> Scene {
    let mut scene = Scene::new();
    text.atlas.begin_frame();
    let mut icons = cache.renderer(gpu, text.backing);
    host.render(
        &mut scene,
        &mut WithIconRenderer::new(text, &mut icons),
        FRAME,
        VIEWPORT,
        Theme::LIGHT.into(),
    )
    .unwrap();
    scene
}
#[derive(Debug)]
struct PaintedQuad {
    index: usize,
    quad: Quad,
    clip: Rect,
}
fn quads(scene: &Scene) -> Vec<PaintedQuad> {
    let mut clips = vec![Some(VIEWPORT)];
    let mut quads = Vec::new();
    for (index, primitive) in scene.primitives().iter().enumerate() {
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
            Primitive::Quad(quad) => {
                if let Some(clip) = clips.last().copied().flatten() {
                    quads.push(PaintedQuad {
                        index,
                        quad: *quad,
                        clip,
                    });
                }
            }
            _ => {}
        }
    }
    assert_eq!(clips.len(), 1);
    quads
}
fn focus_quad(quads: &[PaintedQuad], frame: Rect) -> &PaintedQuad {
    let width = proposal::metric("stroke", "c-focus-width").unwrap();
    let gap = proposal::metric("spacing", "c-focus-offset").unwrap();
    quads
        .iter()
        .find(|p| {
            p.quad.rect == frame.inset(-(gap + width))
                && p.quad.fill.a == 0.
                && p.quad.border_width == width
                && p.quad.border_color.a > 0.
        })
        .expect("original runtime must emit its real outsetting focus-border quad")
}
fn opaque_quad(quads: &[PaintedQuad], frame: Rect) -> &PaintedQuad {
    quads
        .iter()
        .find(|p| p.quad.rect == frame && p.quad.fill.a >= 0.999)
        .expect("fixture must contain an actual opaque original control ground")
}
fn assert_header_above_body(host: &ContentHost, scene: &Scene) {
    let header = host.get(TRIGGER).unwrap().frame();
    let body = host.get(BODY_BUTTON).unwrap().frame();
    let bounds =
        tekes_ui_controls::extended::ControlFrame::new(header, Some(VIEWPORT)).focus_bounds(1.);
    assert_eq!(
        body.min_y(),
        bounds.max_y(),
        "body uses the original fixed focus-clearance extent"
    );
    let quads = quads(scene);
    let focus = focus_quad(&quads, header);
    let ground = opaque_quad(&quads, body);
    let width = proposal::metric("stroke", "c-focus-width").unwrap();
    let gap = proposal::metric("spacing", "c-focus-offset").unwrap();
    // The original lower stroke stays visible in the wide ancestor viewport.
    // The fixed inset keeps actual body ground out of that stroke. Its ancestor
    // clip also retains the allocated inset for the body's own focus decoration.
    // The original focused-branch-last ordering remains independent below.
    let lower_stroke = Rect::new(header.center().x - 4., header.max_y() + gap, 8., width);
    assert_eq!(focus.clip.intersection(&lower_stroke), Some(lower_stroke));
    let block = host
        .get(DISCLOSURE)
        .unwrap()
        .component::<DisclosureContent>()
        .unwrap()
        .body_layout();
    assert_eq!(ground.clip, block.visible_clip.unwrap());
    assert!(ground.quad.rect.intersection(&bounds).is_none());
    assert!(
        ground.index < focus.index,
        "the original header focus stroke must paint after the original opaque body; focus={focus:?}, ground={ground:?}"
    );
    assert_eq!(host.focused(), Some(TRIGGER));
}
#[test]
fn keyboard_header_ring_is_above_adjacent_opaque_standard_button_body() {
    let mut host = fixture();
    let mut text = text();
    let mut gpu = tekes_ui_render::Renderer::new_headless().unwrap();
    let mut cache = SvgIconCache::default();
    paint(&mut host, &mut text, &mut gpu, &mut cache);
    assert!(host.focus(Some(TRIGGER), true));
    let mount = host.semantic_mount(TRIGGER);
    let scene = paint(&mut host, &mut text, &mut gpu, &mut cache);
    assert_header_above_body(&host, &scene);
    assert_eq!(host.semantic_mount(TRIGGER), mount);
    assert!(host.drain_action_records().is_empty());
}
#[test]
fn keyboard_header_ring_is_above_original_inert_closing_retained_body() {
    let mut host = fixture();
    let mut text = text();
    let mut gpu = tekes_ui_render::Renderer::new_headless().unwrap();
    let mut cache = SvgIconCache::default();
    paint(&mut host, &mut text, &mut gpu, &mut cache);
    assert!(host.focus(Some(TRIGGER), true));
    assert!(
        host.get_mut(DISCLOSURE)
            .unwrap()
            .component_mut::<DisclosureContent>()
            .unwrap()
            .set_expanded(false)
    );
    paint(&mut host, &mut text, &mut gpu, &mut cache);
    text.now = Duration::from_millis(30);
    let scene = paint(&mut host, &mut text, &mut gpu, &mut cache);
    let reveal = host
        .get(DISCLOSURE)
        .unwrap()
        .component::<DisclosureContent>()
        .unwrap()
        .body_layout();
    assert!(reveal.progress > 0. && reveal.progress < 1.);
    assert!(!reveal.interactive);
    assert!(
        !host
            .focus_targets()
            .iter()
            .any(|(id, _)| *id == BODY_BUTTON)
    );
    assert!(!host.semantics().iter().any(|node| node.id == BODY_BUTTON));
    assert_header_above_body(&host, &scene);
    assert!(host.drain_action_records().is_empty());
}
#[test]
fn original_nested_body_focus_ring_stays_above_header_ground() {
    for root_button in [false, true] {
        let mut host = fixture_with_body(root_button);
        let mut text = text();
        let mut gpu = tekes_ui_render::Renderer::new_headless().unwrap();
        let mut cache = SvgIconCache::default();
        paint(&mut host, &mut text, &mut gpu, &mut cache);
        let body = host.get(BODY_BUTTON).unwrap().frame();
        let header = host.get(TRIGGER).unwrap().frame();
        let gap_point =
            tekes_ui_scene::Point::new(body.center().x, (header.max_y() + body.min_y()) * 0.5);
        assert!(!body.contains(gap_point) && !header.contains(gap_point));
        assert!(
            !host
                .event(&ContentEvent::PointerDown(gap_point))
                .capture_pointer
        );
        assert!(!host.event(&ContentEvent::PointerUp(gap_point)).activated);
        assert!(
            host.drain_action_records().is_empty(),
            "allocated focus clearance is not an action hit target"
        );
        assert_eq!(
            host.semantics()
                .iter()
                .find(|n| n.id == BODY_BUTTON)
                .unwrap()
                .frame,
            body
        );
        assert!(host.focus(Some(BODY_BUTTON), true));
        let mount = host.semantic_mount(BODY_BUTTON);
        let scene = paint(&mut host, &mut text, &mut gpu, &mut cache);
        let quads = quads(&scene);
        let focus = focus_quad(&quads, body);
        let ground = opaque_quad(&quads, header);
        let width = proposal::metric("stroke", "c-focus-width").unwrap();
        let gap = proposal::metric("spacing", "c-focus-offset").unwrap();
        // Full straight upper stroke, between the original rounded corners.
        let upper_stroke = Rect::new(
            focus.quad.rect.min_x() + focus.quad.corner_radii.top_left,
            body.min_y() - gap - width,
            focus.quad.rect.size.width
                - focus.quad.corner_radii.top_left
                - focus.quad.corner_radii.top_right,
            width,
        );
        let block = host
            .get(DISCLOSURE)
            .unwrap()
            .component::<DisclosureContent>()
            .unwrap()
            .body_layout();
        assert_eq!(
            block.visible_clip.unwrap().intersection(&upper_stroke),
            Some(upper_stroke)
        );
        assert_eq!(
            focus.clip.intersection(&upper_stroke),
            Some(upper_stroke),
            "original body-root/descendant focus stroke must use its allocated ancestor block, root_button={root_button}"
        );
        for x in [focus.quad.rect.min_x(), focus.quad.rect.max_x() - width] {
            let side_stroke = Rect::new(x, body.center().y - 2., width, 4.);
            assert_eq!(focus.clip.intersection(&side_stroke), Some(side_stroke));
        }
        assert_eq!(ground.clip.intersection(&upper_stroke), Some(upper_stroke));
        assert!(ground.index < focus.index);
        assert_eq!(host.focused(), Some(BODY_BUTTON));
        assert_eq!(host.semantic_mount(BODY_BUTTON), mount);
        assert_eq!(
            host.semantics()
                .iter()
                .find(|n| n.id == BODY_BUTTON)
                .unwrap()
                .frame,
            body
        );
        assert!(host.drain_action_records().is_empty());
    }
}

// Observes actual CoreText/GlyphAtlas output from the original body draw. This
// wrapper borrows/delegates the existing text service; it owns no layout/editor.
struct GlyphTrace {
    core: coretext::Text,
    body_ranges: Vec<std::ops::Range<usize>>,
}
const BODY_COPY: &str = "Body glyph 中文";
impl tekes_ui_controls::ButtonTextRenderer for GlyphTrace {
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> tekes_ui_scene::Size {
        self.core.measure(s, r, w)
    }
    fn draw(
        &mut self,
        scene: &mut Scene,
        s: &str,
        r: proposal::TextRole,
        frame: Rect,
        align: tekes_ui_controls::ButtonTextAlign,
        ink: tekes_ui_tokens::Rgba,
    ) {
        let start = scene.primitives().len();
        self.core.draw(scene, s, r, frame, align, ink);
        if s == BODY_COPY {
            self.body_ranges.push(start..scene.primitives().len());
        }
    }
    fn resolved_font_size(&mut self, r: proposal::TextRole) -> f32 {
        self.core.resolved_font_size(r)
    }
    fn backing_scale(&self) -> f32 {
        self.core.backing
    }
    fn text_surface_host(&mut self) -> Option<&mut dyn tekes_ui_controls::TextSurfaceHost> {
        self.core.text_surface_host()
    }
    fn motion_context(&self) -> Option<tekes_ui_controls::ControlMotionContext> {
        self.core.motion_context()
    }
    fn request_animation_frame(&mut self) {
        self.core.request_animation_frame();
    }
}
fn original_body_ink(scene: &Scene, trace: &GlyphTrace) -> Vec<tekes_ui_scene::Point> {
    let backing = trace.core.backing;
    let atlas = &trace.core.atlas;
    let side = atlas.page_size() as usize;
    let mut clips = vec![Some(VIEWPORT)];
    let mut points = Vec::new();
    for (index, primitive) in scene.primitives().iter().enumerate() {
        match primitive {
            Primitive::PushClip(rect) => clips.push(
                clips
                    .last()
                    .copied()
                    .flatten()
                    .and_then(|c| c.intersection(rect)),
            ),
            Primitive::PopClip => {
                assert!(clips.len() > 1);
                clips.pop();
            }
            Primitive::Glyphs(run) if trace.body_ranges.iter().any(|r| r.contains(&index)) => {
                let Some(clip) = clips.last().copied().flatten() else {
                    continue;
                };
                for positioned in &run.glyphs {
                    let g = positioned.glyph;
                    let page = atlas.page_pixels(g.page).unwrap();
                    let left = (positioned.origin.x * backing).round() as i32 + g.left;
                    let top = (positioned.origin.y * backing).round() as i32 + g.top;
                    for y in 0..g.height {
                        for x in 0..g.width {
                            let alpha =
                                page[((g.y + y) as usize * side + (g.x + x) as usize) * 4 + 3];
                            if alpha == 0 {
                                continue;
                            }
                            let p = tekes_ui_scene::Point::new(
                                (left + x as i32) as f32 / backing + 0.5 / backing,
                                (top + y as i32) as f32 / backing + 0.5 / backing,
                            );
                            if clip.contains(p) {
                                points.push(p);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    assert_eq!(clips.len(), 1);
    points
}
#[test]
fn actual_body_glyph_ink_clears_header_focus_bounds_at_real_backing_and_type_scales() {
    for backing in [1., 2.] {
        for type_scale in [1., 1.5] {
            let mut host = ContentHost::new(Content::new(
                DISCLOSURE,
                DisclosureContent::new(
                    TRIGGER,
                    Content::new(ComponentId(3), TextContent::new("Disclosure")),
                    Content::new(BODY, TextContent::new(BODY_COPY)),
                    true,
                ),
            ))
            .unwrap();
            let mut trace = GlyphTrace {
                core: text(),
                body_ranges: vec![],
            };
            trace.core.backing = backing;
            trace.core.scale = type_scale;
            let mut gpu = tekes_ui_render::Renderer::new_headless().unwrap();
            let mut cache = SvgIconCache::default();
            let mut draw = |host: &mut ContentHost, trace: &mut GlyphTrace| {
                trace.core.atlas.begin_frame();
                trace.body_ranges.clear();
                let mut scene = Scene::new();
                let mut icons = cache.renderer(&mut gpu, backing);
                host.render(
                    &mut scene,
                    &mut WithIconRenderer::new(trace, &mut icons),
                    FRAME,
                    VIEWPORT,
                    Theme::LIGHT.into(),
                )
                .unwrap();
                scene
            };
            draw(&mut host, &mut trace);
            let before = host.get(BODY).unwrap().frame();
            assert!(host.focus(Some(TRIGGER), true));
            let scene = draw(&mut host, &mut trace);
            assert_eq!(
                host.get(BODY).unwrap().frame(),
                before,
                "keyboard focus cannot move the document"
            );
            let header = host.get(TRIGGER).unwrap().frame();
            let focus = tekes_ui_controls::extended::ControlFrame::new(header, Some(VIEWPORT))
                .focus_bounds(backing);
            let ink = original_body_ink(&scene, &trace);
            assert!(!ink.is_empty(), "must observe real body atlas coverage");
            let overlaps = ink.iter().filter(|p| p.y < focus.max_y()).count();
            assert_eq!(
                overlaps, 0,
                "actual body ink enters original header focus clearance: backing={backing} type={type_scale}, header={header:?}, focus={focus:?}, body={before:?}"
            );
            assert!(before.min_y() >= focus.max_y());
        }
    }
}

#[test]
fn clearance_is_focus_independent_and_part_of_one_reversible_reveal_block() {
    use tekes_ui_controls::extended::ControlFrame;
    for backing in [1., 2.] {
        for scale in [1., 1.5] {
            for reduced in [false, true] {
                let mut host = fixture();
                let mut text = text();
                text.backing = backing;
                text.scale = scale;
                text.reduced_motion = reduced;
                let mut gpu = tekes_ui_render::Renderer::new_headless().unwrap();
                let mut cache = SvgIconCache::default();
                paint(&mut host, &mut text, &mut gpu, &mut cache);
                let header = host.get(TRIGGER).unwrap().frame();
                let child = host.get(BODY).unwrap().frame();
                let bounds = ControlFrame::new(header, Some(VIEWPORT)).focus_bounds(backing);
                let clearance = bounds.max_y() - header.max_y();
                assert_eq!(child.min_y(), bounds.max_y());
                assert!(clearance > 0.);
                let mount = host.semantic_mount(BODY_BUTTON);
                let trigger_mount = host.semantic_mount(TRIGGER);
                let initial = host
                    .get(DISCLOSURE)
                    .unwrap()
                    .component::<DisclosureContent>()
                    .unwrap()
                    .body_layout();
                assert_eq!(initial.body_rect.min_y(), header.max_y());
                assert!(
                    (initial.body_rect.size.height - (clearance + child.size.height)).abs()
                        < 0.0001
                );
                assert!((initial.occupied_height - initial.body_rect.size.height).abs() < 0.0001);
                for focused in [BODY_BUTTON, TRIGGER] {
                    assert!(host.focus(Some(focused), true));
                    paint(&mut host, &mut text, &mut gpu, &mut cache);
                    assert_eq!(host.get(TRIGGER).unwrap().frame(), header);
                    assert_eq!(host.get(BODY).unwrap().frame(), child);
                    assert_eq!(
                        host.focus_targets()
                            .iter()
                            .map(|(id, _)| *id)
                            .collect::<Vec<_>>(),
                        vec![TRIGGER, BODY_BUTTON]
                    );
                    assert_eq!(
                        host.semantics()
                            .iter()
                            .filter(|node| node.id == TRIGGER || node.id == BODY_BUTTON)
                            .map(|node| node.id)
                            .collect::<Vec<_>>(),
                        vec![TRIGGER, BODY_BUTTON]
                    );
                }
                host.get_mut(DISCLOSURE)
                    .unwrap()
                    .component_mut::<DisclosureContent>()
                    .unwrap()
                    .set_expanded(false);
                paint(&mut host, &mut text, &mut gpu, &mut cache);
                text.now = Duration::from_millis(30);
                paint(&mut host, &mut text, &mut gpu, &mut cache);
                let closing = host
                    .get(DISCLOSURE)
                    .unwrap()
                    .component::<DisclosureContent>()
                    .unwrap()
                    .body_layout();
                assert!(!closing.interactive);
                assert!(closing.occupied_height <= closing.body_rect.size.height);
                assert_eq!(host.get(BODY).unwrap().frame(), child);
                assert!(
                    !host
                        .focus_targets()
                        .iter()
                        .any(|(id, _)| *id == BODY_BUTTON)
                );
                assert!(
                    closing.visible_clip.is_none_or(|clip| clip.max_y()
                        <= host.get(DISCLOSURE).unwrap().frame().max_y() + 0.0001)
                );
                if reduced {
                    assert_eq!(closing.occupied_height, 0.);
                } else {
                    assert!(closing.progress > 0. && closing.progress < 1.);
                }
                // Reverse the same original transition before the closing endpoint.
                host.get_mut(DISCLOSURE)
                    .unwrap()
                    .component_mut::<DisclosureContent>()
                    .unwrap()
                    .set_expanded(true);
                text.now = Duration::from_millis(45);
                paint(&mut host, &mut text, &mut gpu, &mut cache);
                text.now = Duration::from_millis(500);
                paint(&mut host, &mut text, &mut gpu, &mut cache);
                assert_eq!(host.semantic_mount(BODY_BUTTON), mount);
                assert_eq!(host.semantic_mount(TRIGGER), trigger_mount);
                assert_eq!(host.get(BODY).unwrap().frame(), child);
                host.get_mut(DISCLOSURE)
                    .unwrap()
                    .component_mut::<DisclosureContent>()
                    .unwrap()
                    .set_expanded(false);
                paint(&mut host, &mut text, &mut gpu, &mut cache);
                text.now = Duration::from_millis(1000);
                paint(&mut host, &mut text, &mut gpu, &mut cache);
                let closed = host
                    .get(DISCLOSURE)
                    .unwrap()
                    .component::<DisclosureContent>()
                    .unwrap()
                    .body_layout();
                assert_eq!(closed.occupied_height, 0.);
                assert!(closed.visible_clip.is_none());
                // A root host may keep its caller-allocated FRAME height. The
                // original component's requested child extent must still shrink
                // to the header alone; do not confuse allocation with occupancy.
                let measured = host
                    .get_mut(DISCLOSURE)
                    .unwrap()
                    .component_mut::<DisclosureContent>()
                    .unwrap()
                    .measure(&mut text, FRAME.size);
                assert_eq!(
                    measured.size.height, header.size.height,
                    "no residual inset in the original measured block after full collapse"
                );
                assert!(host.drain_action_records().is_empty());
            }
        }
    }
}
#[test]
fn reveal_block_and_child_clip_stay_inside_the_actual_height_allocation() {
    let mut host = fixture();
    let mut text = text();
    text.reduced_motion = true;
    let mut gpu = tekes_ui_render::Renderer::new_headless().unwrap();
    let mut cache = SvgIconCache::default();
    paint(&mut host, &mut text, &mut gpu, &mut cache);
    let header = host.get(TRIGGER).unwrap().frame();
    let clearance = tekes_ui_controls::extended::ControlFrame::new(header, None)
        .focus_bounds(text.backing)
        .max_y()
        - header.max_y();
    let short = Rect::new(
        FRAME.min_x(),
        FRAME.min_y(),
        FRAME.size.width,
        header.size.height + clearance + 2.,
    );
    let mut scene = Scene::new();
    text.atlas.begin_frame();
    let mut icons = cache.renderer(&mut gpu, text.backing);
    host.render(
        &mut scene,
        &mut WithIconRenderer::new(&mut text, &mut icons),
        short,
        VIEWPORT,
        Theme::LIGHT.into(),
    )
    .unwrap();
    let block = host
        .get(DISCLOSURE)
        .unwrap()
        .component::<DisclosureContent>()
        .unwrap()
        .body_layout();
    assert!(block.occupied_height <= short.size.height - header.size.height);
    assert!(
        block
            .visible_clip
            .is_none_or(|clip| clip.max_y() <= short.max_y())
    );
    assert!(
        host.get(BODY)
            .unwrap()
            .visible()
            .is_none_or(|clip| clip.max_y() <= short.max_y())
    );
}
