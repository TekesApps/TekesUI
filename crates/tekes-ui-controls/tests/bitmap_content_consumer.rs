#![cfg(target_os = "macos")]
use std::rc::Rc;
use tekes_ui_controls::ControlKey;
use tekes_ui_controls::Insets;
use tekes_ui_controls::composition::{
    Button, ComponentId, Content, ContentEvent, ContentHost, ImageAlignment, ImageContent,
    ImageFit, ImageLoadOutcome, ImageSemantics, PanelContent, StackContent, TextFieldContent,
};
use tekes_ui_controls::extended::{Axis, Stack};
use tekes_ui_controls::{ButtonTextRenderer, TextSurfaceHost, TextSurfaceTextRenderer};
use tekes_ui_render::Renderer;
use tekes_ui_scene::{GlyphAtlas, Point, Primitive, Rect, Scene, Size};
use tekes_ui_text::{
    Paragraph, ShapeOptions, Shaper, TextStyle,
    coretext::{CoreTextFonts, CoreTextShaper},
};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal};

struct Text {
    shaper: CoreTextShaper,
}
impl Text {
    fn new() -> Self {
        Self {
            shaper: CoreTextShaper::new(CoreTextFonts::new()),
        }
    }
}
impl ButtonTextRenderer for Text {
    fn text_surface_host(&mut self) -> Option<&mut dyn TextSurfaceHost> {
        Some(self)
    }
    fn measure(&mut self, text: &str, role: proposal::TextRole, width: f32) -> Size {
        let paragraph = self.shape_surface(text, role, Some(width));
        Size::new(paragraph.width as f32, paragraph.height as f32)
    }
    fn draw(
        &mut self,
        _scene: &mut Scene,
        _text: &str,
        _role: proposal::TextRole,
        _rect: Rect,
        _align: tekes_ui_controls::ButtonTextAlign,
        _ink: Rgba,
    ) {
    }
}
impl TextSurfaceHost for Text {
    fn shape_label(
        &mut self,
        text: &str,
        role: proposal::TextRole,
        width: f32,
        options: tekes_ui_text::LabelOptions,
    ) -> Option<Rc<tekes_ui_text::LabelLayout>> {
        let style = role.style();
        Some(Rc::new(
            self.shaper
                .shape_label(
                    text,
                    &TextStyle::system(style.size).with_weight(style.weight),
                    &ShapeOptions::with_width(width as f64),
                    options,
                )
                .with_min_line_height(style.line_height as f64),
        ))
    }
    fn shape_surface(
        &mut self,
        text: &str,
        role: proposal::TextRole,
        width: Option<f32>,
    ) -> Rc<Paragraph> {
        Rc::new(self.shaper.shape(
            text,
            &TextStyle::system(role.style().size).with_weight(role.style().weight),
            &ShapeOptions {
                width: width.map(f64::from),
                line_spacing: 0.0,
            },
        ))
    }
}
impl TextSurfaceTextRenderer for Text {
    fn push_visible(
        &mut self,
        _scene: &mut Scene,
        _paragraph: &Paragraph,
        _origin: tekes_ui_scene::Point,
        _color: Rgba,
        _clip: Rect,
    ) {
    }
}

fn render(host: &mut ContentHost, renderer: &mut Renderer, frame: Rect, clip: Rect) -> Vec<u8> {
    let mut text = Text::new();
    let mut scene = Scene::new();
    host.render(
        &mut scene,
        &mut text,
        frame,
        clip,
        Palette::from(Theme::default()),
    )
    .unwrap();
    renderer
        .render_to_rgba8(
            &scene,
            &mut GlyphAtlas::default(),
            (frame.size.width as u32, frame.size.height as u32),
            1.0,
            Rgba::gray(0.125),
        )
        .unwrap()
}

#[test]
fn public_content_consumer_fits_clips_and_preserves_transparent_pixels() {
    let mut renderer = Renderer::new_headless()
        .expect("headless adapter required for bitmap offscreen acceptance");
    let lease = renderer
        .register_bitmap_image(&[255, 0, 0, 255, 0, 0, 255, 255], 2, 1)
        .unwrap();
    let image = ImageContent::ready(
        lease,
        Size::new(2.0, 1.0),
        ImageSemantics::Alternative("Quarterly revenue chart".into()),
    )
    .unwrap()
    .with_fit(ImageFit::Cover)
    .with_alignment(ImageAlignment::new(1.0, 0.5))
    .unwrap();
    let mut host = ContentHost::new(Content::new(ComponentId(1), image)).unwrap();
    let semantics = host.semantics();
    assert_eq!(semantics.len(), 1);
    assert_eq!(
        semantics[0].semantics.role,
        tekes_ui_controls::composition::ContentRole::Image
    );
    assert_eq!(semantics[0].semantics.name, "Quarterly revenue chart");
    let pixels = render(
        &mut host,
        &mut renderer,
        Rect::new(0.0, 0.0, 4.0, 4.0),
        Rect::new(1.0, 1.0, 2.0, 2.0),
    );
    let px = |x: usize, y: usize| &pixels[(y * 4 + x) * 4..(y * 4 + x + 1) * 4];
    assert!(
        px(1, 1)[2] > 200 && px(1, 1)[0] < 40 && px(1, 1)[3] == 255,
        "cover alignment selects the blue source half: {:?}",
        px(1, 1)
    );
    assert_eq!(
        px(0, 1),
        &[32, 32, 32, 255],
        "the original ContentHost clip excludes pixels outside the image viewport"
    );
    assert_eq!(px(3, 1), &[32, 32, 32, 255]);
    assert_eq!(px(1, 0), &[32, 32, 32, 255]);
    let transparent = renderer.register_bitmap_image(&[0, 0, 0, 0], 1, 1).unwrap();
    let mut clear_host = ContentHost::new(Content::new(
        ComponentId(4),
        ImageContent::ready(transparent, Size::new(1.0, 1.0), ImageSemantics::Decorative).unwrap(),
    ))
    .unwrap();
    assert!(
        clear_host.semantics().is_empty(),
        "decorative images do not add a semantic node"
    );
    let clear = render(
        &mut clear_host,
        &mut renderer,
        Rect::new(0.0, 0.0, 2.0, 2.0),
        Rect::new(0.0, 0.0, 2.0, 2.0),
    );
    assert!(
        clear
            .chunks_exact(4)
            .all(|pixel| pixel == [32, 32, 32, 255]),
        "transparent premultiplied pixels preserve the target"
    );
}

#[test]
fn image_child_keeps_the_original_button_as_the_only_action_owner() {
    let image = Content::new(
        ComponentId(2),
        ImageContent::loading(Size::new(24.0, 16.0), ImageSemantics::Decorative).unwrap(),
    );
    let button = Button::new(image).with_accessible_name("Open image");
    let mut host = ContentHost::new(Content::new(ComponentId(1), button)).unwrap();
    let mut text = Text::new();
    let mut scene = Scene::new();
    host.render(
        &mut scene,
        &mut text,
        Rect::new(0.0, 0.0, 96.0, 48.0),
        Rect::new(0.0, 0.0, 96.0, 48.0),
        Palette::from(Theme::default()),
    )
    .unwrap();
    let semantics = host.semantics();
    assert_eq!(semantics.len(), 1);
    assert_eq!(semantics[0].semantics.name, "Open image");
    host.event(&ContentEvent::PointerDown(Point::new(48.0, 24.0)));
    host.event(&ContentEvent::PointerUp(Point::new(48.0, 24.0)));
    assert_eq!(
        host.get(ComponentId(1))
            .unwrap()
            .component::<Button>()
            .unwrap()
            .activations,
        1
    );
    assert_eq!(host.focused(), Some(ComponentId(1)));
    host.event(&ContentEvent::Key {
        key: ControlKey::Enter,
        down: true,
    });
    host.event(&ContentEvent::Key {
        key: ControlKey::Enter,
        down: false,
    });
    assert_eq!(
        host.get(ComponentId(1))
            .unwrap()
            .component::<Button>()
            .unwrap()
            .activations,
        2,
        "the original Button owns exactly one keyboard activation in addition to pointer activation"
    );
}

#[test]
fn bitmap_local_curve_contract_composes_in_real_panel_stack_and_keeps_button_owner() {
    let mut renderer =
        Renderer::new_headless().expect("headless adapter required for bitmap curve composition");
    let cover_lease = renderer
        .register_bitmap_image(&[255, 0, 0, 255, 180, 0, 0, 255], 2, 1)
        .unwrap();
    let cover_id = cover_lease.id();
    let cover = ImageContent::ready(
        cover_lease,
        Size::new(320.0, 100.0),
        ImageSemantics::Alternative("Cover preview".into()),
    )
    .unwrap()
    .with_fit(ImageFit::Cover);

    let contain_lease = renderer
        .register_bitmap_image(&[0, 0, 255, 255, 0, 80, 220, 255], 2, 1)
        .unwrap();
    let contain_id = contain_lease.id();
    let contain = ImageContent::ready(
        contain_lease,
        Size::new(16.0, 8.0),
        ImageSemantics::Decorative,
    )
    .unwrap()
    .with_fit(ImageFit::Contain);
    let button =
        Button::new(Content::new(ComponentId(9103), contain)).with_accessible_name("Open preview");
    let field = TextFieldContent::read_only_content(
        "Notes",
        "Image inside the original curved Panel",
        tekes_ui_tokens::proposal::TextRole::UiBody,
    );
    let stack = Content::new(
        ComponentId(9102),
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 4.0,
            },
            children: vec![
                Content::new(ComponentId(9104), cover),
                Content::new(ComponentId(9105), button),
                Content::new(ComponentId(9106), field),
            ],
            flexible_child: None,
        },
    );
    let mut panel = PanelContent::new(stack).expect("Local ImageContent is valid in Panel");
    panel.set_padding(Some(Insets::default())).unwrap();
    let mut host = ContentHost::new(Content::new(ComponentId(9101), panel)).unwrap();

    let mut text = Text::new();
    let frame = Rect::new(0.0, 0.0, 160.0, 160.0);
    let mut scene = Scene::new();
    host.render(
        &mut scene,
        &mut text,
        frame,
        frame,
        Palette::from(Theme::default()),
    )
    .unwrap();

    let panel_geometry = host
        .get(ComponentId(9101))
        .and_then(|node| node.component::<PanelContent>())
        .and_then(PanelContent::geometry)
        .cloned()
        .expect("the original Host publishes the arranged Panel geometry");
    let panel_outer = panel_geometry.outer;
    let panel_shape = panel_geometry.shape;
    let scene_panel_clips: Vec<_> = scene
        .primitives()
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::PushShapeClip(key) => scene.clip_shape(key).ok(),
            _ => None,
        })
        .collect();
    eprintln!(
        "Issue 117 R3 geometry diagnostic: panel={panel_geometry:?}; required_clip_geometry={}; clip_shape_count={}; scene={scene:#?}",
        scene.required_clip_geometry(),
        scene.clip_shape_count(),
    );
    assert_eq!(scene.required_clip_geometry(), 1);
    assert!(scene.clip_shape_count() > 0);
    assert_eq!(
        panel_shape.rect(),
        panel_outer,
        "the public Panel geometry owns its shape bounds"
    );
    assert!(
        panel_shape.radii().top_left > 0.0,
        "the arranged original Panel is rounded: {panel_geometry:?}"
    );
    assert!(
        scene_panel_clips.contains(&panel_shape),
        "the original Host carries the exact arranged Panel shape into the child Scene; panel={panel_geometry:?}; scene_clips={scene_panel_clips:?}; scene={scene:#?}"
    );

    let controlled_images: Vec<_> = scene
        .primitives()
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::ControlledTransformedImage { quad, .. } => Some(quad),
            _ => None,
        })
        .collect();
    assert_eq!(controlled_images.len(), 2);
    let cover_quad = controlled_images
        .iter()
        .find(|quad| quad.quad.image == cover_id)
        .expect("cover lease remains attached to the real Scene primitive");
    assert_ne!(cover_quad.uv, Rect::new(0.0, 0.0, 1.0, 1.0));
    let contain_quad = controlled_images
        .iter()
        .find(|quad| quad.quad.image == contain_id)
        .expect("contained image remains a controlled Scene primitive");
    assert_eq!(contain_quad.uv, Rect::new(0.0, 0.0, 1.0, 1.0));

    let cover_destination = cover_quad.quad.rect;
    let contain_destination = contain_quad.quad.rect;
    let cover_frame = host.get(ComponentId(9104)).unwrap().frame();
    let contain_frame = host.get(ComponentId(9103)).unwrap().frame();
    let render_bounds = frame;
    let cover_visible = cover_destination
        .intersection(&render_bounds)
        .and_then(|visible| visible.intersection(&panel_outer))
        .expect("the actual Cover destination intersects the current render and Panel geometry");
    let contain_visible = contain_destination
        .intersection(&render_bounds)
        .and_then(|visible| visible.intersection(&panel_outer))
        .expect("the actual Contain destination intersects the current render and Panel geometry");
    let cover_probe = cover_visible.center();
    let contain_probe = contain_visible.center();
    assert_eq!(
        cover_destination, cover_frame,
        "Cover uses its actual arranged image frame"
    );
    assert!(
        contain_frame.contains(contain_probe),
        "Contain probe is inside its actual image frame: frame={contain_frame:?}, destination={contain_destination:?}"
    );
    assert_eq!(
        panel_shape.contains([f64::from(cover_probe.x), f64::from(cover_probe.y)]),
        Ok(true)
    );
    assert_eq!(
        panel_shape.contains([f64::from(contain_probe.x), f64::from(contain_probe.y)]),
        Ok(true)
    );

    let corner_search = panel_shape.radii().top_left.ceil() as i32 + 1;
    let corner_x0 = ((panel_outer.min_x() - frame.min_x()).floor() as i32).max(0);
    let corner_y0 = ((panel_outer.min_y() - frame.min_y()).floor() as i32).max(0);
    let mut rounded_corner_probe = None;
    'corner: for y in 0..=corner_search {
        for x in 0..=corner_search {
            let point = Point::new(
                frame.min_x() + corner_x0 as f32 + x as f32 + 0.5,
                frame.min_y() + corner_y0 as f32 + y as f32 + 0.5,
            );
            if frame.contains(point)
                && panel_outer.contains(point)
                && cover_destination.contains(point)
                && panel_shape.contains([f64::from(point.x), f64::from(point.y)]) == Ok(false)
            {
                rounded_corner_probe = Some(point);
                break 'corner;
            }
        }
    }
    let rounded_corner_probe = rounded_corner_probe.expect(
        "the actual Cover destination must expose a pixel-center inside the Panel bounds but outside its rounded shape",
    );

    let pixels = renderer
        .render_to_rgba8(
            &scene,
            &mut GlyphAtlas::default(),
            (frame.size.width as u32, frame.size.height as u32),
            1.0,
            Rgba::gray(0.125),
        )
        .unwrap();
    let sample_pixel = |point: Point| -> [u8; 4] {
        assert!(
            frame.contains(point),
            "pixel sample must lie inside render bounds: {point:?} / {frame:?}"
        );
        let x = (point.x - frame.min_x()).floor() as usize;
        let y = (point.y - frame.min_y()).floor() as usize;
        let width = frame.size.width as usize;
        let index = (y * width + x) * 4;
        pixels[index..index + 4].try_into().unwrap()
    };
    let cover_center = sample_pixel(cover_probe);
    let contain_center = sample_pixel(contain_probe);
    let rounded_corner_pixel = sample_pixel(rounded_corner_probe);
    let semantics = host.semantics();
    let field_semantic = semantics
        .iter()
        .find(|node| node.id == ComponentId(9106))
        .expect("the original read-only TextField keeps its own semantic node");
    assert!(semantics.iter().any(|node| {
        node.semantics.role == tekes_ui_controls::composition::ContentRole::Image
            && node.semantics.name == "Cover preview"
    }));
    eprintln!(
        "Issue 117 R3 render diagnostic: panel={panel_geometry:?}; cover_frame={cover_frame:?}; cover_destination={cover_destination:?}; cover_uv={:?}; contain_frame={contain_frame:?}; contain_destination={contain_destination:?}; contain_uv={:?}; probes={{cover:{cover_probe:?}, contain:{contain_probe:?}, rounded_corner:{rounded_corner_probe:?}}}; pixels={{cover:{cover_center:?}, contain:{contain_center:?}, rounded_corner:{rounded_corner_pixel:?}}}; field_semantic={field_semantic:?}; whole_scene={scene:#?}",
        cover_quad.uv, contain_quad.uv,
    );
    assert!(
        cover_center[0] > 200
            && cover_center[1] < 40
            && cover_center[2] < 40
            && cover_center[3] == 255,
        "the actual Cover destination paints its red source inside the arranged Panel; destination={cover_destination:?}, probe={cover_probe:?}, pixel={cover_center:?}, panel={panel_geometry:?}"
    );
    assert!(
        rounded_corner_pixel[0] <= 64
            && rounded_corner_pixel[1] <= 64
            && rounded_corner_pixel[2] <= 64
            && rounded_corner_pixel[3] == 255,
        "the actual outside-curve pixel stays in the clear/dark-shadow background color domain, with no red Cover contribution; probe={rounded_corner_probe:?}, destination={cover_destination:?}, pixel={rounded_corner_pixel:?}, panel={panel_geometry:?}"
    );
    assert!(
        contain_center[2] > 200 && contain_center[0] < 40 && contain_center[3] == 255,
        "the actual Contain destination paints the decorative image inside its original Button; destination={contain_destination:?}, probe={contain_probe:?}, pixel={contain_center:?}"
    );

    let action_nodes: Vec<_> = semantics
        .iter()
        .filter(|node| node.semantics.role == tekes_ui_controls::composition::ContentRole::Button)
        .collect();
    assert_eq!(
        action_nodes.len(),
        1,
        "ImageContent adds no second action owner"
    );
    assert_eq!(action_nodes[0].semantics.name, "Open preview");
    assert_eq!(
        field_semantic.semantics.role,
        tekes_ui_controls::composition::ContentRole::TextInput
    );
    assert_eq!(field_semantic.semantics.name, "Notes");
    assert_eq!(
        field_semantic.semantics.value.as_deref(),
        Some("Image inside the original curved Panel")
    );
    assert!(
        !semantics.iter().any(|node| {
            node.semantics.role == tekes_ui_controls::composition::ContentRole::Image
                && node.semantics.name.is_empty()
        }),
        "the contain image is decorative and stays hidden from accessibility"
    );

    let (button_id, button_frame) = host
        .focus_targets()
        .into_iter()
        .find(|(id, _)| *id == ComponentId(9105))
        .expect("the original Button remains the only focusable action in this fixture");
    let center = button_frame.center();
    host.event(&ContentEvent::PointerDown(center));
    host.event(&ContentEvent::PointerUp(center));
    assert_eq!(host.focused(), Some(button_id));
    assert_eq!(
        host.get(button_id)
            .unwrap()
            .component::<Button>()
            .unwrap()
            .activations,
        1,
        "the original Button owns one activation; ImageContent adds no action owner"
    );
}

#[test]
fn completion_tokens_reject_stale_and_cross_component_sources() {
    let mut a = ImageContent::loading(Size::new(12.0, 8.0), ImageSemantics::Decorative).unwrap();
    let mut b = ImageContent::loading(
        Size::new(12.0, 8.0),
        ImageSemantics::Alternative("Preview".into()),
    )
    .unwrap();
    let first = a.begin_load().unwrap();
    let second = a.begin_load().unwrap();
    assert!(
        !a.complete(
            first,
            tekes_ui_controls::composition::ImageLoadOutcome::Missing
        )
        .unwrap()
    );
    assert!(
        !b.complete(
            second.clone(),
            tekes_ui_controls::composition::ImageLoadOutcome::Missing
        )
        .unwrap()
    );
    assert!(
        a.complete(
            second.clone(),
            tekes_ui_controls::composition::ImageLoadOutcome::Missing
        )
        .unwrap()
    );
    assert!(
        !a.complete(
            second,
            tekes_ui_controls::composition::ImageLoadOutcome::Failed("late".into())
        )
        .unwrap()
    );
}

#[test]
fn two_content_owners_and_old_scene_share_the_original_registration_until_last_release() {
    let mut renderer =
        Renderer::new_headless().expect("headless adapter required for lease lifetime");
    let lease = renderer
        .register_bitmap_image(&[0, 255, 0, 255], 1, 1)
        .unwrap();
    let first = Content::new(
        ComponentId(2),
        ImageContent::ready(
            lease.clone(),
            Size::new(1.0, 1.0),
            ImageSemantics::Alternative("First view".into()),
        )
        .unwrap(),
    );
    let second = Content::new(
        ComponentId(3),
        ImageContent::ready(
            lease.clone(),
            Size::new(1.0, 1.0),
            ImageSemantics::Alternative("Second view".into()),
        )
        .unwrap(),
    );
    let root = Content::new(
        ComponentId(1),
        StackContent {
            model: Stack::new(Axis::Vertical),
            children: vec![first, second],
            flexible_child: None,
        },
    );
    let mut host = ContentHost::new(root).unwrap();
    let mut text = Text::new();
    let mut shared_scene = Scene::new();
    host.render(
        &mut shared_scene,
        &mut text,
        Rect::new(0.0, 0.0, 16.0, 32.0),
        Rect::new(0.0, 0.0, 16.0, 32.0),
        Palette::from(Theme::default()),
    )
    .unwrap();
    assert_eq!(
        shared_scene
            .primitives()
            .iter()
            .filter(|p| matches!(
                p,
                tekes_ui_scene::Primitive::ControlledTransformedImage { .. }
            ))
            .count(),
        2
    );
    drop(lease);

    host.get_mut(ComponentId(1))
        .unwrap()
        .component_mut::<StackContent>()
        .unwrap()
        .children
        .remove(0);
    let mut remaining_scene = Scene::new();
    host.render(
        &mut remaining_scene,
        &mut text,
        Rect::new(0.0, 0.0, 16.0, 32.0),
        Rect::new(0.0, 0.0, 16.0, 32.0),
        Palette::from(Theme::default()),
    )
    .unwrap();
    assert_eq!(
        remaining_scene
            .primitives()
            .iter()
            .filter(|p| matches!(
                p,
                tekes_ui_scene::Primitive::ControlledTransformedImage { .. }
            ))
            .count(),
        1
    );
    assert_eq!(
        renderer.image_count(),
        1,
        "the remaining content and retained old Scene share one registration"
    );

    shared_scene.clear();
    host.get_mut(ComponentId(1))
        .unwrap()
        .component_mut::<StackContent>()
        .unwrap()
        .children
        .clear();
    let mut empty_scene = Scene::new();
    host.render(
        &mut empty_scene,
        &mut text,
        Rect::new(0.0, 0.0, 16.0, 32.0),
        Rect::new(0.0, 0.0, 16.0, 32.0),
        Palette::from(Theme::default()),
    )
    .unwrap();
    drop(remaining_scene);
    let _ = renderer
        .render_to_rgba8(
            &empty_scene,
            &mut GlyphAtlas::default(),
            (16, 32),
            1.0,
            Rgba::gray(0.0),
        )
        .unwrap();
    assert_eq!(
        renderer.image_count(),
        0,
        "the original retirement queue releases after the final Scene lease disappears"
    );
}

#[test]
fn host_unmount_and_remount_rejects_a_completion_from_the_old_mount() {
    let mut renderer =
        Renderer::new_headless().expect("headless adapter required for lease lifetime");
    let old_mount_lease = renderer
        .register_bitmap_image(&[0, 0, 255, 255], 1, 1)
        .unwrap();
    let child_id = ComponentId(22);
    let root_id = ComponentId(21);
    let image = ImageContent::loading(Size::new(1.0, 1.0), ImageSemantics::Decorative).unwrap();
    let root = Content::new(
        root_id,
        StackContent {
            model: Stack::new(Axis::Vertical),
            children: vec![Content::new(child_id, image)],
            flexible_child: None,
        },
    );
    let mut host = ContentHost::new(root).unwrap();
    let old_request = host
        .get_mut(child_id)
        .unwrap()
        .component_mut::<ImageContent>()
        .unwrap()
        .begin_load()
        .unwrap();
    let returned = host.remove(child_id).unwrap();
    host.insert(root_id, returned).unwrap();
    assert!(
        !host
            .get_mut(child_id)
            .unwrap()
            .component_mut::<ImageContent>()
            .unwrap()
            .complete(
                old_request,
                ImageLoadOutcome::Ready {
                    lease: old_mount_lease,
                    logical_size: Size::new(1.0, 1.0),
                }
            )
            .unwrap(),
        "an old mount's Ready completion cannot publish into the remounted component"
    );
    assert_eq!(
        host.get(child_id)
            .unwrap()
            .component::<ImageContent>()
            .unwrap()
            .status(),
        tekes_ui_controls::composition::ImageLoadStatus::Loading
    );
}

#[test]
fn ready_source_swap_preserves_old_pixels_while_loading_or_failed_and_missing_releases_it() {
    let mut renderer = Renderer::new_headless()
        .expect("headless adapter required for bitmap offscreen acceptance");
    let red = renderer
        .register_bitmap_image(&[255, 0, 0, 255], 1, 1)
        .unwrap();
    let mut host = ContentHost::new(Content::new(
        ComponentId(31),
        ImageContent::ready(
            red,
            Size::new(1.0, 1.0),
            ImageSemantics::Alternative("Status preview".into()),
        )
        .unwrap()
        .with_fit(ImageFit::Cover),
    ))
    .unwrap();
    let frame = Rect::new(0.0, 0.0, 8.0, 4.0);
    let red_pixels = render(&mut host, &mut renderer, frame, frame);
    assert!(red_pixels[(2 * 8 + 4) * 4] > 200);

    let request_a = host
        .get_mut(ComponentId(31))
        .unwrap()
        .component_mut::<ImageContent>()
        .unwrap()
        .begin_load()
        .unwrap();
    let request_b = host
        .get_mut(ComponentId(31))
        .unwrap()
        .component_mut::<ImageContent>()
        .unwrap()
        .begin_load()
        .unwrap();
    let green = renderer
        .register_bitmap_image(&[0, 255, 0, 255], 1, 1)
        .unwrap();
    assert!(
        !host
            .get_mut(ComponentId(31))
            .unwrap()
            .component_mut::<ImageContent>()
            .unwrap()
            .complete(
                request_a.clone(),
                ImageLoadOutcome::Ready {
                    lease: green.clone(),
                    logical_size: Size::new(1.0, 1.0)
                }
            )
            .unwrap(),
        "late A is rejected after the request changed to B"
    );
    let request_a_again = host
        .get_mut(ComponentId(31))
        .unwrap()
        .component_mut::<ImageContent>()
        .unwrap()
        .begin_load()
        .unwrap();
    assert!(
        !host
            .get_mut(ComponentId(31))
            .unwrap()
            .component_mut::<ImageContent>()
            .unwrap()
            .complete(
                request_b,
                ImageLoadOutcome::Ready {
                    lease: green,
                    logical_size: Size::new(1.0, 1.0)
                }
            )
            .unwrap(),
        "late B is rejected after the request changed back to A"
    );
    assert_eq!(
        host.get(ComponentId(31))
            .unwrap()
            .component::<ImageContent>()
            .unwrap()
            .status(),
        tekes_ui_controls::composition::ImageLoadStatus::Loading
    );
    let loading_pixels = render(&mut host, &mut renderer, frame, frame);
    assert!(
        loading_pixels[(2 * 8 + 4) * 4] > 200 && loading_pixels[(2 * 8 + 4) * 4 + 1] < 40,
        "the previous red pixels remain visible through stale completions"
    );

    assert!(
        host.get_mut(ComponentId(31))
            .unwrap()
            .component_mut::<ImageContent>()
            .unwrap()
            .complete(
                request_a_again,
                ImageLoadOutcome::Failed("offline source".into())
            )
            .unwrap()
    );
    assert_eq!(
        host.get(ComponentId(31))
            .unwrap()
            .component::<ImageContent>()
            .unwrap()
            .status(),
        tekes_ui_controls::composition::ImageLoadStatus::Failed
    );
    let failed_pixels = render(&mut host, &mut renderer, frame, frame);
    assert!(
        failed_pixels[(2 * 8 + 4) * 4] > 200 && failed_pixels[(2 * 8 + 4) * 4 + 1] < 40,
        "a failed replacement preserves the previous ready image"
    );

    let missing_request = host
        .get_mut(ComponentId(31))
        .unwrap()
        .component_mut::<ImageContent>()
        .unwrap()
        .begin_load()
        .unwrap();
    assert!(
        host.get_mut(ComponentId(31))
            .unwrap()
            .component_mut::<ImageContent>()
            .unwrap()
            .complete(missing_request, ImageLoadOutcome::Missing)
            .unwrap()
    );
    assert_eq!(
        host.get(ComponentId(31))
            .unwrap()
            .component::<ImageContent>()
            .unwrap()
            .status(),
        tekes_ui_controls::composition::ImageLoadStatus::Missing
    );
    let blank = render(&mut host, &mut renderer, frame, frame);
    assert!(
        blank
            .chunks_exact(4)
            .all(|pixel| pixel == [32, 32, 32, 255])
    );
    let _ = render(&mut host, &mut renderer, frame, frame);
    assert_eq!(
        renderer.image_count(),
        0,
        "Missing releases the component's old registration through the original retirement queue"
    );
}

#[test]
fn two_x_pixel_source_keeps_logical_size_when_constraints_change() {
    let mut renderer = Renderer::new_headless()
        .expect("headless adapter required for bitmap offscreen acceptance");
    let pixels = [0, 0, 255, 255].repeat(8); // 4 × 2 pixels at a 2 × 1 logical size.
    let lease = renderer.register_bitmap_image(&pixels, 4, 2).unwrap();
    let image = ImageContent::ready(
        lease,
        Size::new(2.0, 1.0),
        ImageSemantics::Alternative("Two-times preview".into()),
    )
    .unwrap()
    .with_fit(ImageFit::Cover);
    let mut host = ContentHost::new(Content::new(ComponentId(41), image)).unwrap();
    let large = Rect::new(0.0, 0.0, 8.0, 4.0);
    let first = render(&mut host, &mut renderer, large, large);
    assert!(first[(2 * 8 + 4) * 4 + 2] > 200);
    assert_eq!(
        host.get(ComponentId(41))
            .unwrap()
            .component::<ImageContent>()
            .unwrap()
            .logical_size(),
        Size::new(2.0, 1.0)
    );
    let smaller = Rect::new(0.0, 0.0, 4.0, 2.0);
    let second = render(&mut host, &mut renderer, smaller, smaller);
    assert!(second[(4 + 2) * 4 + 2] > 200);
    assert_eq!(
        host.get(ComponentId(41))
            .unwrap()
            .component::<ImageContent>()
            .unwrap()
            .logical_size(),
        Size::new(2.0, 1.0)
    );
}
