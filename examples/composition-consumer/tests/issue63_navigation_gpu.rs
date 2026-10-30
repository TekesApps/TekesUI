//! Three-frame Issue #63 public navigation Content GPU calibration.
//! Copy to examples/composition-consumer/tests/issue63_navigation_gpu.rs and
//! run only with a fresh TEKESUI_ISSUE63_GPU_OUT directory.
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
    output: &std::path::Path,
    filename: &str,
    scene: &Scene,
    text: &mut RecordingText,
    canvas: Rect,
    backing: f32,
    palette: Palette,
) {
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

#[test]
#[ignore = "explicit Issue #63 three-frame actual-GPU calibration; set TEKESUI_ISSUE63_GPU_OUT to a fresh directory"]
fn public_breadcrumb_tabs_and_open_command_palette_use_generated_surfaces() {
    let output = PathBuf::from(
        std::env::var_os("TEKESUI_ISSUE63_GPU_OUT")
            .expect("fresh output directory required; prior evidence must remain untouched"),
    );
    fs::create_dir(&output).expect("refuse to overwrite existing evidence directory");
    let mut renderer =
        tekes_ui_render::Renderer::new_headless().expect("actual headless Metal renderer required");
    let palette = Palette::new(Theme::LIGHT, 0).expect("compiled consumer default palette");
    let palette_source = format!("{:?}", palette.source());
    let mut captures = Vec::new();

    // Breadcrumb: the public consumer fixture, 320pt available width, 1.5x
    // typography and 2x backing. All three path labels remain original Content.
    {
        let canvas = Rect::new(0., 0., 344., 344.);
        let frame = Rect::new(12., 12., 320., 320.);
        let clip = canvas;
        let mut host = navigation_content::breadcrumb(&["Home", "Library", "Current"])
            .expect("public breadcrumb fixture");
        host.get_mut(BREADCRUMB)
            .unwrap()
            .set_surface(Some("canvas"))
            .unwrap();
        let mut text = RecordingText::new(1.5, 2.);
        let mut scene = Scene::new();
        text.inner.atlas.begin_frame();
        host.render(&mut scene, &mut text, frame, clip, palette)
            .expect("breadcrumb ContentHost render");
        let breadcrumb_frame = content_frame(&host, BREADCRUMB);
        let breadcrumb = host
            .get(BREADCRUMB)
            .unwrap()
            .component::<tekes_ui_controls::composition::BreadcrumbContent>()
            .unwrap();
        let item_frames: Vec<_> = breadcrumb
            .view
            .layout()
            .items
            .iter()
            .map(|item| item.frame())
            .collect();
        assert_eq!(item_frames.len(), 3);
        let current_ink = resolve_surface(palette, "canvas", "action-quiet", "selected-disabled")
            .unwrap()
            .text;
        let ancestor_ink = resolve_surface(palette, "canvas", "action-quiet", "rest")
            .unwrap()
            .text;
        let separator_ink = resolve_surface(palette, "canvas", "content", "rest")
            .unwrap()
            .text;
        for label in ["Home", "Library"] {
            let draw = text
                .draws
                .iter()
                .find(|draw| draw.text == label)
                .expect("ancestor label draw");
            assert_eq!(draw.ink, ancestor_ink);
            assert!(
                inside(breadcrumb_frame, draw.frame),
                "ancestor label draw exceeded measured Breadcrumb Content frame"
            );
        }
        let current = text
            .draws
            .iter()
            .find(|draw| draw.text == "Current")
            .expect("current label draw");
        assert_eq!(current.ink, current_ink);
        assert!(
            inside(breadcrumb_frame, current.frame),
            "current label draw exceeded measured Breadcrumb Content frame"
        );
        let separators: Vec<_> = text.draws.iter().filter(|draw| draw.text == "/").collect();
        assert!(
            !separators.is_empty(),
            "passive slash separators must be drawn"
        );
        for slash in separators {
            assert_eq!(slash.ink, separator_ink);
            assert!(
                item_frames[..2]
                    .iter()
                    .all(|button| !intersects(slash.frame, *button)),
                "passive slash falls inside ancestor Button background/hit frame"
            );
            assert!(inside(clip, slash.frame));
        }
        assert_draws_inside(&text.draws, clip);
        let filename = "issue63-light-canvas-breadcrumb-w320-text1_5-b2.png";
        save_gpu(
            &mut renderer,
            &output,
            filename,
            &scene,
            &mut text,
            canvas,
            2.,
            palette,
        );
        captures.push(serde_json::json!({
            "file":filename,"component":"BreadcrumbContent","component_id":BREADCRUMB.0,
            "canvas_pt":rect(canvas),"host_frame_pt":rect(frame),"clip_pt":rect(clip),
            "type_scale":1.5,"backing_scale":2,"surface":"canvas",
            "semantic_nodes":semantic_json(&host),
            "content_frames":layout_frames([("BreadcrumbContent".into(),breadcrumb_frame),
                ("root".into(),content_frame(&host,ROOT))]),
            "breadcrumb_item_button_frames_pt":item_frames.iter().copied().map(rect).collect::<Vec<_>>(),
            "slash_draw_frames_pt":text.draws.iter().filter(|draw|draw.text=="/").map(|draw|rect(draw.frame)).collect::<Vec<_>>(),
            "draw_calls":text.draws.iter().map(draw_json).collect::<Vec<_>>(),
            "surface_draw_calls":text.surface_draws.iter().map(surface_draw_json).collect::<Vec<_>>(),
            "resolved_roles":{"ancestor":"canvas/action-quiet/rest","current":"canvas/action-quiet/selected-disabled","separator":"canvas/content/rest",
                "ancestor_ink":rgba(ancestor_ink),"current_ink":rgba(current_ink),"separator_ink":rgba(separator_ink)},
            "palette_source_identity":palette_source,"palette_source_hash":"46027fd69397bcc1c00406f33fe84f1091c0505353a0e8377205603eaea33f48",
            "scene_primitive_count":scene.primitives().len(),"actual_gpu":true
        }));
    }

    // Tabs: the public fixture has the actual two-line Overview/Welcome label
    // and an original TextFieldContent panel. Its natural multi-line height is
    // recorded as-is, not treated as a clipping defect.
    {
        let canvas = Rect::new(0., 0., 344., 420.);
        let frame = Rect::new(12., 12., 320., 396.);
        let clip = canvas;
        let mut host = navigation_content::tabs().expect("public tabs fixture");
        host.get_mut(TABS)
            .unwrap()
            .set_surface(Some("canvas"))
            .unwrap();
        let mut text = RecordingText::new(1.5, 2.);
        let mut scene = Scene::new();
        text.inner.atlas.begin_frame();
        host.render(&mut scene, &mut text, frame, clip, palette)
            .expect("tabs ContentHost render");
        let tabs_frame = content_frame(&host, TABS);
        let panel_frame = content_frame(&host, FIRST_PANEL);
        let composer_frame = content_frame(&host, COMPOSER);
        let nodes = host.semantics();
        let tabs_semantics = nodes
            .iter()
            .find(|node| node.id == TABS)
            .expect("TabList semantics");
        assert_eq!(tabs_semantics.parts.len(), 2);
        let selected_ink = resolve_surface(palette, "canvas", "action-quiet", "selected")
            .unwrap()
            .text;
        let unselected_ink = resolve_surface(palette, "canvas", "action-quiet", "rest")
            .unwrap()
            .text;
        let selected_tab = resolve_surface(palette, "canvas", "action-quiet", "selected")
            .expect("generated selected tab package");
        let unselected_tab = resolve_surface(palette, "canvas", "action-quiet", "rest")
            .expect("generated unselected tab package");
        for (part, expected) in [
            (&tabs_semantics.parts[0], selected_tab),
            (&tabs_semantics.parts[1], unselected_tab),
        ] {
            assert!(scene.primitives().iter().any(|primitive| matches!(primitive,
                Primitive::Quad(quad) if quad.rect == part.frame && quad.fill == expected.background)),
                "original tab Button must paint its generated selected/rest background at the semantic item frame");
        }
        for label in ["Overview", "Welcome"] {
            assert!(
                text.draws
                    .iter()
                    .any(|draw| draw.text == label && draw.ink == selected_ink),
                "selected real two-line label {label} must use generated selected text role"
            );
        }
        assert!(
            text.draws
                .iter()
                .any(|draw| draw.text == "Settings" && draw.ink == unselected_ink)
        );
        let field = host
            .get(COMPOSER)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap();
        let field_colors = field
            .surface
            .surface_colors(palette, "canvas")
            .expect("original input role");
        assert!(scene.primitives().iter().any(|primitive|matches!(primitive,Primitive::Quad(q) if q.rect==field.surface.outer && q.fill==field_colors.background)),
            "original TextField panel must paint generated input background");
        assert!(
            text.surface_draws
                .iter()
                .any(|draw| draw.glyph_count > 0 && draw.ink == field_colors.text),
            "original TextField must draw real CoreText glyphs"
        );
        assert_draws_inside(&text.draws, clip);
        let filename = "issue63-light-canvas-tabs-w320-text1_5-b2.png";
        save_gpu(
            &mut renderer,
            &output,
            filename,
            &scene,
            &mut text,
            canvas,
            2.,
            palette,
        );
        captures.push(serde_json::json!({
            "file":filename,"component":"TabsContent","component_id":TABS.0,
            "canvas_pt":rect(canvas),"host_frame_pt":rect(frame),"clip_pt":rect(clip),
            "type_scale":1.5,"backing_scale":2,"surface":"canvas","selected_index":0,
            "semantic_nodes":semantic_json(&host),
            "content_frames":layout_frames([("TabsContent".into(),tabs_frame),
                ("selected TextField panel".into(),panel_frame),("COMPOSER TextFieldContent".into(),composer_frame),
                ("inactive retained panel".into(),content_frame(&host,SECOND_PANEL)),("root".into(),content_frame(&host,ROOT))]),
            "tab_item_semantic_frames_pt":tabs_semantics.parts.iter().map(|part|rect(part.frame)).collect::<Vec<_>>(),
            "draw_calls":text.draws.iter().map(draw_json).collect::<Vec<_>>(),
            "text_field_surface_draw_calls":text.surface_draws.iter().map(surface_draw_json).collect::<Vec<_>>(),
            "resolved_roles":{"selected_tab":"canvas/action-quiet/selected","unselected_tab":"canvas/action-quiet/rest","panel_input":"canvas/input/rest",
                "selected_tab_background":rgba(selected_tab.background),"selected_tab_ink":rgba(selected_ink),
                "unselected_tab_background":rgba(unselected_tab.background),"unselected_tab_ink":rgba(unselected_ink),
                "input_background":rgba(field_colors.background),"input_foreground":rgba(field_colors.text)},
            "natural_two_line_header_height_pt":tabs_frame.size.height-(panel_frame.size.height),
            "palette_source_identity":palette_source,"palette_source_hash":"46027fd69397bcc1c00406f33fe84f1091c0505353a0e8377205603eaea33f48",
            "scene_primitive_count":scene.primitives().len(),"actual_gpu":true
        }));
    }

    // Command palette: set a public Content Surface for the original trigger,
    // open via CommandPaletteContent::open, then let ContentHost paint the real
    // editable query and filtered popup in their independent popover Surface.
    {
        let canvas = Rect::new(0., 0., 544., 480.);
        let frame = Rect::new(12., 12., 520., 456.);
        let clip = canvas;
        let mut host =
            navigation_content::command_palette().expect("public command palette fixture");
        host.get_mut(COMMAND)
            .unwrap()
            .set_surface(Some("sidebar"))
            .unwrap();
        assert!(
            host.focus(Some(COMMAND), true),
            "trigger owner must accept public focus"
        );
        let mut text = RecordingText::new(1., 2.);
        let mut closed = Scene::new();
        text.inner.atlas.begin_frame();
        host.render(&mut closed, &mut text, frame, clip, palette)
            .expect("closed palette layout");
        assert!(
            host.get_mut(COMMAND)
                .unwrap()
                .component_mut::<CommandPaletteContent>()
                .unwrap()
                .open(),
            "public CommandPaletteContent::open must activate original query and popup"
        );
        let mut opened = Scene::new();
        text.inner.atlas.begin_frame();
        host.render(&mut opened, &mut text, frame, clip, palette)
            .expect("activate original query focus");
        assert_eq!(host.focused(), Some(COMMAND_QUERY));
        assert!(
            host.event(&ContentEvent::Commit {
                text: "e".into(),
                replacement: None,
            })
            .changed,
            "public commit must edit the original query"
        );
        text.clear_records();
        let mut scene = Scene::new();
        text.inner.atlas.begin_frame();
        host.render(&mut scene, &mut text, frame, clip, palette)
            .expect("open palette ContentHost render");
        let command = host
            .get(COMMAND)
            .unwrap()
            .component::<CommandPaletteContent>()
            .unwrap();
        assert!(command.is_open());
        assert_eq!(command.model.combo.query, "e");
        let popup = command
            .options()
            .layout()
            .expect("actual filtered-menu popup layout");
        assert!(popup.visible.is_some_and(|visible| !visible.is_empty()));
        assert!(
            inside(clip, popup.popup),
            "actual popup must fit in canvas, including focus/elevation margins"
        );
        let query_frame = content_frame(&host, COMMAND_QUERY);
        let query = host
            .get(COMMAND_QUERY)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap();
        let query_colors = query
            .surface
            .surface_colors(palette, "popover")
            .expect("query independent popover input roles");
        assert!(scene.primitives().iter().any(|primitive|matches!(primitive,Primitive::Quad(q) if q.rect==query.surface.outer && q.fill==query_colors.background)),
            "original query must paint with its default popover Surface");
        let popup_colors = resolve_surface(palette, "popover", "surface-frame", "rest").unwrap();
        assert!(scene.primitives().iter().any(|primitive|matches!(primitive,Primitive::Quad(q) if q.rect==popup.popup && q.fill==popup_colors.background)),
            "original filtered popup must paint with default popover Surface");
        let trigger_colors = resolve_surface(palette, "sidebar", "action-primary", "rest").unwrap();
        assert!(scene.primitives().iter().any(|primitive|matches!(primitive,Primitive::Quad(q) if q.fill==trigger_colors.background)),
            "original command trigger must inherit the explicitly declared sidebar Surface");
        let menu_items = host
            .semantics()
            .into_iter()
            .find(|node| node.id == COMMAND)
            .unwrap()
            .parts;
        assert_eq!(
            menu_items.len(),
            3,
            "open palette exposes three original menu items"
        );
        assert!(menu_items.iter().all(|part| {
            popup
                .visible
                .is_some_and(|visible| inside(visible, part.frame))
        }));
        assert!(
            text.draws.iter().any(|draw| draw.text == "Commands"),
            "original trigger TextContent draw missing"
        );
        assert!(
            text.surface_draws
                .iter()
                .any(|draw| draw.glyph_count > 0 && draw.ink == query_colors.text),
            "query must draw through CoreText TextSurface renderer"
        );
        assert!(
            text.draws.iter().any(|draw| draw.text == "Open File")
                && text.draws.iter().any(|draw| draw.text == "Open Folder")
                && text.draws.iter().any(|draw| draw.text == "Preferences"),
            "actual popup labels missing from text renderer"
        );
        assert_draws_inside(&text.draws, clip);
        let filename = "issue63-light-sidebar-command-palette-open-w520-text1-b2.png";
        save_gpu(
            &mut renderer,
            &output,
            filename,
            &scene,
            &mut text,
            canvas,
            2.,
            palette,
        );
        captures.push(serde_json::json!({
            "file":filename,"component":"CommandPaletteContent","component_id":COMMAND.0,
            "canvas_pt":rect(canvas),"host_frame_pt":rect(frame),"clip_pt":rect(clip),
            "type_scale":1.,"backing_scale":2,"open_via_public_component_api":true,"public_query_commit":"e",
            "trigger_surface":"sidebar (explicit Content declaration)","query_surface":"popover (default overlay child context)","popup_surface":"popover (original filtered-menu default)",
            "semantic_nodes":semantic_json(&host),
            "content_frames":layout_frames([("CommandPaletteContent".into(),content_frame(&host,COMMAND)),
                ("original trigger Content".into(),content_frame(&host,COMMAND_TRIGGER)),
                ("original TextField query Content".into(),query_frame),
                ("background action".into(),content_frame(&host,BACKGROUND_ACTION)),("root".into(),content_frame(&host,ROOT))]),
            "popup_layout":{"anchor_pt":rect(popup.anchor),"frame_pt":rect(popup.popup),"visible_clip_pt":popup.visible.map(rect),
                "item_frames_pt":popup.items.iter().map(|item|rect(item.frame)).collect::<Vec<_>>(),"active_item":popup.active_item},
            "draw_calls":text.draws.iter().map(draw_json).collect::<Vec<_>>(),
            "text_surface_draw_calls":text.surface_draws.iter().map(surface_draw_json).collect::<Vec<_>>(),
            "resolved_roles":{"trigger":"sidebar/action-primary/rest","query":"popover/input/rest","popup":"popover/surface-frame/rest",
                "trigger_background":rgba(trigger_colors.background),"trigger_foreground":rgba(trigger_colors.text),
                "query_background":rgba(query_colors.background),"query_foreground":rgba(query_colors.text),
                "popup_background":rgba(popup_colors.background)},
            "palette_source_identity":palette_source,"palette_source_hash":"46027fd69397bcc1c00406f33fe84f1091c0505353a0e8377205603eaea33f48",
            "scene_primitive_count":scene.primitives().len(),"actual_gpu":true
        }));
    }

    assert_eq!(captures.len(), 3);
    fs::write(output.join("receipt.json"), serde_json::to_vec_pretty(&serde_json::json!({
        "issue":63,
        "scope":"three public consumer Content GPU calibrations only: Breadcrumb 320pt/type1.5@2x, Tabs 320pt/type1.5@2x, open CommandPalette 520pt/type1@2x",
        "theme":{"mode":"light","palette_source_identity":palette_source,"palette_source_hash":"46027fd69397bcc1c00406f33fe84f1091c0505353a0e8377205603eaea33f48"},
        "renderer":"actual headless Metal using examples/composition-consumer/tests/support/coretext.rs GlyphAtlas and TextSurface services",
        "reduced_motion":true,"frame_count":captures.len(),"complete_gold7":false,"native_interaction":false,
        "scope_exclusions":["not all Gallery or all new Surface combinations","not native physical interaction","not complete Gold7 or full application acceptance"],
        "captures":captures
    })).unwrap()).expect("write receipt after all frame assertions and captures");
}
