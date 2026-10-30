//! Independent Issue #63 generated-Surface variant QA candidate.
//! Copy to the composition-consumer test tree only after the Command geometry fix;
//! run with fresh TEKESUI_ISSUE63_GPU_OUT, TEKESUI_ISSUE63_THEME_ID and
//! TEKESUI_ISSUE63_SOURCE_SHA256 values. This candidate is not execution evidence.
#![cfg(target_os = "macos")]

#[path = "support/coretext.rs"]
mod coretext;

use std::{fs, path::PathBuf, time::Duration};

use tekes_ui_composition_consumer::navigation_content::{
    self, BACKGROUND_ACTION, BREADCRUMB, COMMAND, COMMAND_QUERY, COMMAND_TRIGGER, COMPOSER,
    FIRST_PANEL, ROOT, SECOND_PANEL, TABS,
};
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, TextSurfaceHost, TextSurfaceTextRenderer,
    composition::{CommandPaletteContent, ContentEvent, ContentHost, TextFieldContent},
};
use tekes_ui_scene::{Point, Primitive, Rect, Scene, Size};
use tekes_ui_text::model::Paragraph;
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal, surface_palette::resolve_surface};

#[derive(Clone, Debug)]
struct DrawRecord {
    text: String,
    role: String,
    frame: Rect,
    ink: Rgba,
}

#[derive(Clone, Debug)]
struct SurfaceDrawRecord {
    origin: Point,
    width: f64,
    height: f64,
    clip: Rect,
    glyph_count: usize,
    ink: Rgba,
}

struct RecordingText {
    inner: coretext::Text,
    draws: Vec<DrawRecord>,
    surface_draws: Vec<SurfaceDrawRecord>,
}

impl RecordingText {
    fn new(scale: f32, backing: f32) -> Self {
        let mut inner = coretext::Text::new(scale);
        inner.services = true;
        inner.reduced_motion = true;
        inner.now = Duration::ZERO;
        inner.backing = backing;
        Self {
            inner,
            draws: Vec::new(),
            surface_draws: Vec::new(),
        }
    }

    fn clear_records(&mut self) {
        self.draws.clear();
        self.surface_draws.clear();
    }
}

impl ButtonTextRenderer for RecordingText {
    fn measure(&mut self, text: &str, role: proposal::TextRole, max_width: f32) -> Size {
        self.inner.measure(text, role, max_width)
    }
    fn text_surface_host(&mut self) -> Option<&mut dyn TextSurfaceHost> {
        self.inner.services.then_some(self)
    }
    fn resolved_font_size(&mut self, role: proposal::TextRole) -> f32 {
        self.inner.resolved_font_size(role)
    }
    fn motion_context(&self) -> Option<tekes_ui_controls::ControlMotionContext> {
        self.inner.motion_context()
    }
    fn request_animation_frame(&mut self) {
        self.inner.request_animation_frame();
    }
    fn backing_scale(&self) -> f32 {
        self.inner.backing_scale()
    }
    fn draw(
        &mut self,
        scene: &mut Scene,
        text: &str,
        role: proposal::TextRole,
        frame: Rect,
        align: ButtonTextAlign,
        ink: Rgba,
    ) {
        self.draws.push(DrawRecord {
            text: text.to_owned(),
            role: format!("{role:?}"),
            frame,
            ink,
        });
        self.inner.draw(scene, text, role, frame, align, ink);
    }
}

impl TextSurfaceTextRenderer for RecordingText {
    fn push_visible(
        &mut self,
        scene: &mut Scene,
        paragraph: &Paragraph,
        origin: Point,
        color: Rgba,
        clip: Rect,
    ) {
        self.surface_draws.push(SurfaceDrawRecord {
            origin,
            width: paragraph.width,
            height: paragraph.height,
            clip,
            glyph_count: paragraph.glyph_count(),
            ink: color,
        });
        self.inner
            .push_visible(scene, paragraph, origin, color, clip);
    }
}

impl TextSurfaceHost for RecordingText {
    fn caret_visible(&self) -> bool {
        self.inner.caret_visible()
    }
    fn shape_surface(
        &mut self,
        text: &str,
        role: proposal::TextRole,
        width: Option<f32>,
    ) -> std::rc::Rc<Paragraph> {
        self.inner.shape_surface(text, role, width)
    }
    fn shape_label(
        &mut self,
        text: &str,
        role: proposal::TextRole,
        width: f32,
        options: tekes_ui_text::LabelOptions,
    ) -> Option<std::rc::Rc<tekes_ui_text::LabelLayout>> {
        self.inner.shape_label(text, role, width, options)
    }
    fn type_scale(&self) -> f32 {
        self.inner.type_scale()
    }
}

fn rect(rect: Rect) -> serde_json::Value {
    serde_json::json!([
        rect.origin.x,
        rect.origin.y,
        rect.size.width,
        rect.size.height
    ])
}

fn rgba(color: Rgba) -> serde_json::Value {
    serde_json::json!([color.r, color.g, color.b, color.a])
}

fn inside(outer: Rect, inner: Rect) -> bool {
    inner.min_x() >= outer.min_x()
        && inner.min_y() >= outer.min_y()
        && inner.max_x() <= outer.max_x()
        && inner.max_y() <= outer.max_y()
}

fn intersects(a: Rect, b: Rect) -> bool {
    a.min_x() < b.max_x() && b.min_x() < a.max_x() && a.min_y() < b.max_y() && b.min_y() < a.max_y()
}

fn draw_json(draw: &DrawRecord) -> serde_json::Value {
    serde_json::json!({
        "text": draw.text, "role": draw.role, "renderer_frame_pt": rect(draw.frame),
        "ink": rgba(draw.ink),
    })
}

fn surface_draw_json(draw: &SurfaceDrawRecord) -> serde_json::Value {
    serde_json::json!({
        "paragraph_origin_pt": [draw.origin.x, draw.origin.y],
        "paragraph_bounds_pt": [draw.origin.x, draw.origin.y, draw.width, draw.height],
        "clip_pt": rect(draw.clip), "shaped_glyph_count": draw.glyph_count,
        "ink": rgba(draw.ink),
    })
}

fn semantic_json(host: &ContentHost) -> serde_json::Value {
    serde_json::Value::Array(
        host.semantics()
            .into_iter()
            .map(|node| {
                serde_json::json!({
                    "component_id": node.id.0, "parent": node.parent.map(|id| id.0),
                    "frame_pt": rect(node.frame), "visible_pt": node.visible.map(rect),
                    "role": format!("{:?}", node.semantics.role), "name": node.semantics.name,
                    "selected": node.semantics.selected, "expanded": node.semantics.expanded,
                    "parts": node.parts.into_iter().map(|part| serde_json::json!({
                        "index": part.index, "frame_pt": rect(part.frame), "focused": part.focused,
                        "role": format!("{:?}", part.semantics.role), "name": part.semantics.name,
                        "selected": part.semantics.selected, "disabled": part.semantics.disabled,
                    })).collect::<Vec<_>>(),
                })
            })
            .collect(),
    )
}

fn content_frame(host: &ContentHost, id: tekes_ui_controls::composition::ComponentId) -> Rect {
    host.get(id)
        .unwrap_or_else(|| panic!("missing mounted Content {id:?}"))
        .frame()
}

fn save_gpu(
    renderer: &mut tekes_ui_render::Renderer,
    destination: (&std::path::Path, &str),
    scene: &Scene,
    text: &mut RecordingText,
    canvas: Rect,
    backing: f32,
    palette: Palette,
) {
    let (output, filename) = destination;
    assert!(
        !output.join(filename).exists(),
        "refusing to overwrite {filename}"
    );
    let width = (canvas.size.width * backing) as u32;
    let height = (canvas.size.height * backing) as u32;
    let clear = resolve_surface(palette, "canvas", "content", "rest")
        .expect("canonical canvas content surface")
        .background;
    let pixels = renderer
        .render_to_rgba8(
            scene,
            &mut text.inner.atlas,
            (width, height),
            backing,
            clear,
        )
        .expect("actual headless Metal render");
    assert_eq!(pixels.len(), (width * height * 4) as usize);
    assert!(
        pixels.chunks_exact(4).any(|pixel| pixel != &pixels[..4]),
        "blank GPU image"
    );
    resvg::tiny_skia::Pixmap::from_vec(
        pixels,
        resvg::tiny_skia::IntSize::from_wh(width, height).unwrap(),
    )
    .unwrap()
    .save_png(output.join(filename))
    .expect("write untouched GPU pixels");
}

fn layout_frames(frames: impl IntoIterator<Item = (String, Rect)>) -> serde_json::Value {
    serde_json::Value::Array(
        frames
            .into_iter()
            .map(|(name, frame)| serde_json::json!({"content":name,"frame_pt":rect(frame)}))
            .collect(),
    )
}

fn assert_draws_inside(draws: &[DrawRecord], clip: Rect) {
    for draw in draws {
        assert!(
            inside(clip, draw.frame),
            "text draw frame for {:?} escapes the actual canvas clip: {:?}",
            draw.text,
            draw.frame
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Component {
    Breadcrumb,
    Tabs,
    Command,
}

impl Component {
    fn slug(self) -> &'static str {
        match self {
            Self::Breadcrumb => "breadcrumb",
            Self::Tabs => "tabs",
            Self::Command => "command",
        }
    }
    fn name(self) -> &'static str {
        match self {
            Self::Breadcrumb => "BreadcrumbContent",
            Self::Tabs => "TabsContent",
            Self::Command => "CommandPaletteContent",
        }
    }
}

#[derive(Clone, Copy)]
struct Case {
    component: Component,
    theme: Theme,
    width: f32,
    type_scale: f32,
    backing: f32,
    surface: &'static str,
    tag: &'static str,
}

fn mode_name(theme: Theme) -> &'static str {
    match (theme.appearance, theme.contrast) {
        (tekes_ui_tokens::Appearance::Light, tekes_ui_tokens::Contrast::Standard) => "light",
        (tekes_ui_tokens::Appearance::Dark, tekes_ui_tokens::Contrast::Standard) => "dark",
        (tekes_ui_tokens::Appearance::Light, tekes_ui_tokens::Contrast::Increased) => "light-ic",
        (tekes_ui_tokens::Appearance::Dark, tekes_ui_tokens::Contrast::Increased) => "dark-ic",
    }
}

fn planned_cases() -> Vec<Case> {
    let mut v = Vec::new();
    for theme in Theme::ALL {
        for component in [Component::Breadcrumb, Component::Tabs, Component::Command] {
            let (width, scale, surface) = if component == Component::Command {
                (520., 1., "sidebar")
            } else {
                (320., 1.5, "canvas")
            };
            v.push(Case {
                component,
                theme,
                width,
                type_scale: scale,
                backing: 2.,
                surface,
                tag: "base-2x",
            });
        }
    }
    for component in [Component::Breadcrumb, Component::Tabs, Component::Command] {
        let (width, scale, surface) = if component == Component::Command {
            (520., 1., "sidebar")
        } else {
            (320., 1.5, "canvas")
        };
        v.push(Case {
            component,
            theme: Theme::LIGHT,
            width,
            type_scale: scale,
            backing: 1.,
            surface,
            tag: "base-1x",
        });
    }
    v.push(Case {
        component: Component::Breadcrumb,
        theme: Theme::LIGHT,
        width: 240.,
        type_scale: 1.5,
        backing: 2.,
        surface: "canvas",
        tag: "narrow-large-type",
    });
    v.push(Case {
        component: Component::Tabs,
        theme: Theme::LIGHT,
        width: 240.,
        type_scale: 1.5,
        backing: 2.,
        surface: "panel",
        tag: "narrow-large-type",
    });
    v.push(Case {
        component: Component::Command,
        theme: Theme::LIGHT,
        width: 360.,
        type_scale: 1.5,
        backing: 2.,
        surface: "canvas",
        tag: "narrow-large-type",
    });
    v
}

fn render_case(
    renderer: &mut tekes_ui_render::Renderer,
    output: &std::path::Path,
    case: Case,
    theme_id: &str,
    source_sha: &str,
) -> serde_json::Value {
    let palette = Palette::for_theme_definition(case.theme, theme_id).unwrap_or_else(|| {
        panic!(
            "compiled ThemeDefinition {theme_id:?} unavailable for {}",
            mode_name(case.theme)
        )
    });
    let palette_source = format!("{:?}", palette.source());
    let height = if case.component == Component::Command {
        480.
    } else {
        420.
    };
    let canvas = Rect::new(0., 0., case.width + 24., height);
    let frame = Rect::new(12., 12., case.width, height - 24.);
    let clip = canvas;
    let mut text = RecordingText::new(case.type_scale, case.backing);
    let mut scene = Scene::new();
    text.inner.atlas.begin_frame();
    let mut extra = serde_json::json!({});
    let mut frames = Vec::<(String, Rect)>::new();
    let mut assertions = serde_json::json!({});
    match case.component {
        Component::Breadcrumb => {
            let mut host = navigation_content::breadcrumb(&["Home", "Library", "Current"])
                .expect("public breadcrumb fixture");
            host.get_mut(BREADCRUMB)
                .unwrap()
                .set_surface(Some(case.surface))
                .unwrap();
            host.render(&mut scene, &mut text, frame, clip, palette)
                .expect("Breadcrumb ContentHost render");
            let owner = content_frame(&host, BREADCRUMB);
            let part = host
                .get(BREADCRUMB)
                .unwrap()
                .component::<tekes_ui_controls::composition::BreadcrumbContent>()
                .unwrap();
            let buttons: Vec<Rect> = part
                .view
                .layout()
                .items
                .iter()
                .map(|item| item.frame())
                .collect();
            assert_eq!(buttons.len(), 3);
            for (label, role) in [
                ("Home", "rest"),
                ("Library", "rest"),
                ("Current", "selected-disabled"),
            ] {
                let draw = text
                    .draws
                    .iter()
                    .find(|d| d.text == label)
                    .expect("breadcrumb label draw");
                let colors = resolve_surface(palette, case.surface, "action-quiet", role).unwrap();
                assert_eq!(
                    draw.ink, colors.text,
                    "generated breadcrumb role/ink mismatch"
                );
                assert!(inside(owner, draw.frame) && inside(clip, draw.frame));
            }
            let slash: Vec<_> = text.draws.iter().filter(|d| d.text == "/").collect();
            assert!(!slash.is_empty());
            for d in &slash {
                assert!(inside(clip, d.frame));
                assert!(
                    buttons[..2].iter().all(|b| !intersects(d.frame, *b)),
                    "slash overlaps ancestor Button background"
                );
            }
            frames.extend([
                ("BreadcrumbContent".into(), owner),
                ("root".into(), content_frame(&host, ROOT)),
            ]);
            assertions = serde_json::json!({"three_original_path_buttons":true,"slash_outside_ancestor_button_frames":true});
            extra = serde_json::json!({"breadcrumb_item_button_frames_pt":buttons.iter().copied().map(rect).collect::<Vec<_>>()});
            extra["surface_roles"] = serde_json::json!({
                "ancestor": format!("{}/action-quiet/rest", case.surface),
                "current": format!("{}/action-quiet/selected-disabled", case.surface),
                "separator": format!("{}/content/rest", case.surface)
            });
            extra["semantic_nodes"] = semantic_json(&host);
        }
        Component::Tabs => {
            let mut host = navigation_content::tabs().expect("public tabs fixture");
            host.get_mut(TABS)
                .unwrap()
                .set_surface(Some(case.surface))
                .unwrap();
            host.render(&mut scene, &mut text, frame, clip, palette)
                .expect("Tabs ContentHost render");
            let tabs_frame = content_frame(&host, TABS);
            let panel = content_frame(&host, FIRST_PANEL);
            let field_frame = content_frame(&host, COMPOSER);
            let node = host
                .semantics()
                .into_iter()
                .find(|n| n.id == TABS)
                .expect("TabList semantics");
            assert_eq!(node.parts.len(), 2);
            let selected =
                resolve_surface(palette, case.surface, "action-quiet", "selected").unwrap();
            let rest = resolve_surface(palette, case.surface, "action-quiet", "rest").unwrap();
            for (part, colors) in [(&node.parts[0], selected), (&node.parts[1], rest)] {
                // Semantic parts publish ButtonLayout::visible, while Scene keeps
                // the original quad. Compare the same ancestor-clipped geometry.
                assert!(scene.primitives().iter().any(|p|matches!(p,Primitive::Quad(q) if q.rect.intersection(&clip)==Some(part.frame) && q.fill==colors.background)),"tab Button missing actual generated role background");
            }
            for (label, ink) in [
                ("Overview", selected.text),
                ("Welcome", selected.text),
                ("Settings", rest.text),
            ] {
                assert!(
                    text.draws.iter().any(|d| d.text == label && d.ink == ink),
                    "missing real tab label/role {label}"
                );
            }
            let field = host
                .get(COMPOSER)
                .unwrap()
                .component::<TextFieldContent>()
                .unwrap();
            let colors = tekes_ui_tokens::surface_palette::resolve_surface_with_options(
                palette,
                case.surface,
                "input",
                if field.surface.input.disabled {
                    "disabled"
                } else {
                    "rest"
                },
                tekes_ui_tokens::surface_palette::SurfaceOptions {
                    error: field.surface.error,
                    read_only: field.surface.input.read_only,
                },
            )
            .expect("original TextField surface colors");
            assert!(scene.primitives().iter().any(|p|matches!(p,Primitive::Quad(q) if q.rect==field.surface.outer&&q.fill==colors.background)),"TextField panel missing generated input background");
            assert!(
                text.surface_draws
                    .iter()
                    .any(|d| d.glyph_count > 0 && d.ink == colors.text),
                "TextField lacks actual CoreText glyph ink"
            );
            frames.extend([
                ("TabsContent".into(), tabs_frame),
                ("selected panel".into(), panel),
                ("TextFieldContent".into(), field_frame),
                ("inactive panel".into(), content_frame(&host, SECOND_PANEL)),
                ("root".into(), content_frame(&host, ROOT)),
            ]);
            assertions = serde_json::json!({"selected_and_rest_tab_quads_match_generated_roles":true,"original_textfield_surface_and_coretext_ink":true,"natural_two_line_label_layout_recorded":true});
            extra = serde_json::json!({"semantic_nodes":semantic_json(&host),"tab_item_semantic_frames_pt":node.parts.iter().map(|p|rect(p.frame)).collect::<Vec<_>>(),"natural_two_line_header_height_pt":tabs_frame.size.height-panel.size.height,
                "surface_roles":{"selected_tab":format!("{}/action-quiet/selected",case.surface),"rest_tab":format!("{}/action-quiet/rest",case.surface),"textfield_panel":format!("{}/input/rest",case.surface)}});
        }
        Component::Command => {
            let mut host =
                navigation_content::command_palette().expect("public command palette fixture");
            host.get_mut(COMMAND)
                .unwrap()
                .set_surface(Some(case.surface))
                .unwrap();
            assert!(host.focus(Some(COMMAND), true), "public trigger focus");
            host.render(&mut scene, &mut text, frame, clip, palette)
                .expect("closed command layout");
            assert!(
                host.get_mut(COMMAND)
                    .unwrap()
                    .component_mut::<CommandPaletteContent>()
                    .unwrap()
                    .open(),
                "public CommandPaletteContent::open"
            );
            scene = Scene::new();
            text.inner.atlas.begin_frame();
            host.render(&mut scene, &mut text, frame, clip, palette)
                .expect("opened command layout");
            assert_eq!(host.focused(), Some(COMMAND_QUERY));
            assert!(
                host.event(&ContentEvent::Commit {
                    text: "e".into(),
                    replacement: None
                })
                .changed,
                "public query commit e"
            );
            text.clear_records();
            scene = Scene::new();
            text.inner.atlas.begin_frame();
            host.render(&mut scene, &mut text, frame, clip, palette)
                .expect("filtered command layout");
            let command = host
                .get(COMMAND)
                .unwrap()
                .component::<CommandPaletteContent>()
                .unwrap();
            assert!(command.is_open());
            assert_eq!(command.model.combo.query, "e");
            let popup = command.options().layout().expect("actual filtered popup");
            let visible = popup.visible.expect("popup visible clip");
            assert!(inside(clip, popup.popup) && inside(clip, visible));
            let query_frame = content_frame(&host, COMMAND_QUERY);
            let query = host
                .get(COMMAND_QUERY)
                .unwrap()
                .component::<TextFieldContent>()
                .unwrap();
            let qcolors = tekes_ui_tokens::surface_palette::resolve_surface_with_options(
                palette,
                "popover",
                "input",
                if query.surface.input.disabled {
                    "disabled"
                } else {
                    "rest"
                },
                tekes_ui_tokens::surface_palette::SurfaceOptions {
                    error: query.surface.error,
                    read_only: query.surface.input.read_only,
                },
            )
            .expect("default query popover input roles");
            assert!(scene.primitives().iter().any(|p|matches!(p,Primitive::Quad(q) if q.rect==query.surface.outer&&q.fill==qcolors.background)),"query input background must be generated popover role");
            let popup_colors =
                resolve_surface(palette, "popover", "surface-frame", "rest").unwrap();
            assert!(scene.primitives().iter().any(|p|matches!(p,Primitive::Quad(q) if q.rect==popup.popup&&q.fill==popup_colors.background)),"popup frame must use generated popover role");
            let trigger = resolve_surface(palette, case.surface, "action-primary", "rest").unwrap();
            assert!(
                scene
                    .primitives()
                    .iter()
                    .any(|p| matches!(p,Primitive::Quad(q) if q.fill==trigger.background)),
                "command trigger must use its declared Surface role"
            );
            let parts = host
                .semantics()
                .into_iter()
                .find(|n| n.id == COMMAND)
                .unwrap()
                .parts;
            assert_eq!(
                parts.len(),
                3,
                "query e should expose the existing three candidates"
            );
            assert!(parts.iter().all(|p| inside(visible, p.frame)));
            let background = content_frame(&host, BACKGROUND_ACTION);
            let query_label = text
                .draws
                .iter()
                .rev()
                .find(|d| d.text == "Filter commands")
                .expect("last actual query label draw");
            assert!(
                inside(query_frame, query_label.frame),
                "actual external query label must lie inside its TextFieldContent allocation"
            );
            assert!(text.draws.iter().any(|d| d.text == "Commands"));
            assert!(
                text.surface_draws
                    .iter()
                    .any(|d| d.glyph_count > 0 && d.ink == qcolors.text),
                "query must produce real CoreText TextSurface ink"
            );
            for name in ["Open File", "Open Folder", "Preferences"] {
                assert!(
                    text.draws.iter().any(|d| d.text == name),
                    "missing filtered menu label {name}"
                );
            }
            frames.extend([
                (
                    "CommandPaletteContent".into(),
                    content_frame(&host, COMMAND),
                ),
                ("trigger".into(), content_frame(&host, COMMAND_TRIGGER)),
                ("query TextFieldContent".into(), query_frame),
                ("background action".into(), background),
                ("root".into(), content_frame(&host, ROOT)),
            ]);
            assertions = serde_json::json!({
                "opened_and_committed_via_public_api":true,
                "query":"e",
                "three_original_candidates_visible":true,
                "actual_query_label_inside_query_content":true,
                "query_surface_overlaps_background_action":intersects(query.surface.outer,background),
                "actual_query_label_overlaps_background_action":intersects(query_label.frame,background),
                "real_query_textsurface_ink":true
            });
            extra = serde_json::json!({"semantic_nodes":semantic_json(&host),
                "surface_roles":{"trigger":format!("{}/action-primary/rest",case.surface),"query":"popover/input/rest","popup":"popover/surface-frame/rest"},
                "popup_layout":{"anchor_pt":rect(popup.anchor),"frame_pt":rect(popup.popup),"visible_clip_pt":rect(visible),"item_frames_pt":popup.items.iter().map(|i|rect(i.frame)).collect::<Vec<_>>(),"active_item":popup.active_item}});
        }
    }
    assert_draws_inside(&text.draws, clip);
    let scale_tag = case.type_scale.to_string().replace('.', "_");
    let filename = format!(
        "issue63-{theme_id}-{}-{}-{}-w{}-text{}-b{}-{}.png",
        mode_name(case.theme),
        case.component.slug(),
        case.surface,
        case.width as u32,
        scale_tag,
        case.backing as u32,
        case.tag
    );
    save_gpu(
        renderer,
        (output, &filename),
        &scene,
        &mut text,
        canvas,
        case.backing,
        palette,
    );
    let mut result = serde_json::json!({
        "file":filename,"component":case.component.name(),"mode":mode_name(case.theme),"theme_id":theme_id,
        "surface":case.surface,"width_pt":case.width,"type_scale":case.type_scale,"backing_scale":case.backing,"tag":case.tag,
        "canvas_pt":rect(canvas),"host_frame_pt":rect(frame),"clip_pt":rect(clip),"content_frames":layout_frames(frames),
        "draw_calls":text.draws.iter().map(draw_json).collect::<Vec<_>>(),"text_surface_draw_calls":text.surface_draws.iter().map(surface_draw_json).collect::<Vec<_>>(),
        "assertions":assertions,"palette_source_identity":palette_source,"source_sha256":source_sha,"scene_primitive_count":scene.primitives().len(),"actual_gpu_required":true
    });
    if let (Some(dst), Some(src)) = (result.as_object_mut(), extra.as_object()) {
        for (k, v) in src {
            dst.insert(k.clone(), v.clone());
        }
    }
    result
}

#[test]
#[ignore = "explicit Issue #63 18-case generated-Surface variants candidate; parent executes after Command geometry repair"]
fn public_navigation_content_generated_surface_variants() {
    let out = PathBuf::from(
        std::env::var_os("TEKESUI_ISSUE63_GPU_OUT").expect("fresh output directory required"),
    );
    fs::create_dir(&out).expect("refuse existing evidence directory");
    let theme_id = std::env::var("TEKESUI_ISSUE63_THEME_ID")
        .expect("parent-selected compiled non-default Theme ID required");
    let source_sha = std::env::var("TEKESUI_ISSUE63_SOURCE_SHA256")
        .expect("parent-provided source SHA-256 required");
    assert_eq!(source_sha.len(), 64);
    assert!(
        source_sha.bytes().all(|b| b.is_ascii_hexdigit()),
        "source SHA must be 64 hex chars"
    );
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../design/theme/themes.json");
    let definitions: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest).expect("live theme manifest read"))
            .expect("theme manifest JSON");
    assert!(
        definitions
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["id"] == theme_id && theme_id != "default"),
        "Theme ID must be a real non-default live themes.json entry"
    );
    let mut plan = planned_cases();
    assert_eq!(plan.len(), 18, "full plan must remain exactly 18 frames");
    let filter = std::env::var("TEKESUI_ISSUE63_GPU_CASE_FILTER").ok();
    let selected: Vec<String> = filter
        .as_deref()
        .map(|s| {
            s.split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_else(|| vec!["breadcrumb".into(), "tabs".into(), "command".into()]);
    assert!(!selected.is_empty());
    assert!(
        selected
            .iter()
            .all(|s| ["breadcrumb", "tabs", "command"].contains(&s.as_str())),
        "filter must contain component slugs"
    );
    plan.retain(|c| selected.contains(&c.component.slug().to_owned()));
    assert!(!plan.is_empty());
    let mut renderer =
        tekes_ui_render::Renderer::new_headless().expect("actual headless Metal renderer required");
    let captures: Vec<_> = plan
        .iter()
        .map(|c| render_case(&mut renderer, &out, *c, &theme_id, &source_sha))
        .collect();
    fs::write(out.join("receipt.json"),serde_json::to_vec_pretty(&serde_json::json!({
        "issue":63,"qa_origin":"variant candidate authored without execution; receipt emitted only after selected GPU captures complete",
        "full_plan":{"frame_count":18,"modes":["light","dark","light-ic","dark-ic"],"theme_id":theme_id,"backing_scales":[1,2],"components":["breadcrumb","tabs","command"],"case_families":["three components x four modes at 2x","three Light cases at 1x","Light narrow/high-type cases at 2x"]},
        "selected_plan":{"frame_count":captures.len(),"components":selected,"filter":filter,"filtered":captures.len()!=18},
        "theme_manifest_path":manifest,"source_sha256":source_sha,"renderer":"actual headless Metal plus CoreText GlyphAtlas/TextSurface services",
        "reduced_motion":true,"complete_gold7":false,"native_interaction":false,
        "scope_exclusions":["not a full Gallery matrix","not all theme definitions","not native interaction or complete Gold7 acceptance"],"captures":captures
    })).unwrap()).expect("write receipt after selected captures pass");
}
