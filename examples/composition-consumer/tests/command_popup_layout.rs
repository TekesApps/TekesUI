//! Regression for the unchanged public command fixture's floating query region.
use std::rc::Rc;
use tekes_ui_composition_consumer::navigation_content::{self, COMMAND, COMMAND_QUERY};
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, TextSurfaceHost, TextSurfaceTextRenderer,
    composition::{CommandPaletteContent, ContentHost, TextFieldContent},
};
use tekes_ui_scene::{Point, Primitive, Rect, Scene, Size};
use tekes_ui_text::{
    coretext::{CoreTextFonts, CoreTextShaper},
    model::{Paragraph, ShapeOptions},
    style::TextStyle,
    traits::Shaper,
};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Draw {
    label: String,
    frame: Rect,
    role: proposal::TextRole,
    opaque_before: Vec<Rect>,
}
struct Text {
    shaper: CoreTextShaper,
    draws: Vec<Draw>,
}
impl Text {
    fn new() -> Self {
        Self {
            shaper: CoreTextShaper::new(CoreTextFonts::new()),
            draws: vec![],
        }
    }
}
impl ButtonTextRenderer for Text {
    fn text_surface_host(&mut self) -> Option<&mut dyn TextSurfaceHost> {
        Some(self)
    }
    fn measure(&mut self, text: &str, role: proposal::TextRole, width: f32) -> Size {
        if width <= 0. {
            return Size::default();
        }
        let p = self.shape_surface(text, role, Some(width));
        Size::new(p.width as f32, p.height as f32)
    }
    fn draw(
        &mut self,
        scene: &mut Scene,
        text: &str,
        role: proposal::TextRole,
        frame: Rect,
        _: ButtonTextAlign,
        _: Rgba,
    ) {
        let opaque_before = scene
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Quad(q) if q.fill.a >= 0.999 => Some(q.rect),
                _ => None,
            })
            .collect();
        self.draws.push(Draw {
            label: text.into(),
            frame,
            role,
            opaque_before,
        });
    }
}
impl TextSurfaceHost for Text {
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
                line_spacing: 0.,
            },
        ))
    }
}
impl TextSurfaceTextRenderer for Text {
    fn push_visible(&mut self, _: &mut Scene, _: &Paragraph, _: Point, _: Rgba, _: Rect) {}
}
fn opened() -> (ContentHost, Text) {
    let mut host = navigation_content::command_palette().unwrap();
    host.set_inherited_surface("sidebar").unwrap();
    let frame = Rect::new(12., 12., 520., 480.);
    host.render(
        &mut Scene::new(),
        &mut Text::new(),
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .unwrap();
    host.focus(Some(COMMAND), true);
    assert!(
        host.get_mut(COMMAND)
            .unwrap()
            .component_mut::<CommandPaletteContent>()
            .unwrap()
            .open()
    );
    let mut text = Text::new();
    host.render(
        &mut Scene::new(),
        &mut text,
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .unwrap();
    (host, text)
}
#[test]
fn floating_query_uses_popup_width_instead_of_trigger_width() {
    let (host, mut text) = opened();
    let label = text
        .draws
        .iter()
        .rfind(|draw| draw.label == "Filter commands")
        .unwrap();
    let (label_frame, role) = (label.frame, label.role);
    let unwrapped = text.measure("Filter commands", role, f32::MAX);
    assert!(
        label_frame.size.height <= unwrapped.height.max(role.style().line_height) + 0.01,
        "query metadata wrapped at the trigger width: {label_frame:?} vs {unwrapped:?}"
    );
    let popup = host
        .get(COMMAND)
        .unwrap()
        .component::<CommandPaletteContent>()
        .unwrap()
        .options()
        .layout()
        .unwrap()
        .popup;
    assert!(
        label_frame.min_x() >= popup.min_x() && label_frame.max_x() <= popup.max_x(),
        "query and menu do not share floating width"
    );
}
#[test]
fn floating_query_paints_its_opaque_surface_before_label_and_editor() {
    let (host, text) = opened();
    let label = text
        .draws
        .iter()
        .rfind(|draw| draw.label == "Filter commands")
        .unwrap();
    let editor = host
        .get(COMMAND_QUERY)
        .unwrap()
        .component::<TextFieldContent>()
        .unwrap()
        .surface
        .outer;
    let covers = |outer: &Rect, inner: Rect| {
        outer.min_x() <= inner.min_x()
            && outer.min_y() <= inner.min_y()
            && outer.max_x() >= inner.max_x()
            && outer.max_y() >= inner.max_y()
    };
    assert!(
        label
            .opaque_before
            .iter()
            .any(|rect| covers(rect, label.frame) && covers(rect, editor)),
        "query label/editor painted without an opaque shared floating surface; background sibling text remains visible"
    );
}

#[test]
fn header_only_empty_query_keeps_original_editor_and_clamps_to_viewport() {
    use tekes_ui_controls::composition::{ContentEvent, ContentRole};
    let (mut host, _) = opened();
    let binding = host.native_key_binding().expect("original query binding");
    assert!(!binding.is_empty());
    assert!(
        host.event(&ContentEvent::Preedit {
            text: "no matching command".into(),
            selection: 1..3,
            clauses: std::iter::once(0..19).collect(),
            replacement: None,
        })
        .changed
    );
    assert!(host.is_composing());
    let frame = Rect::new(12., 12., 160., 160.);
    let mut text = Text::new();
    host.render(
        &mut Scene::new(),
        &mut text,
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .unwrap();
    assert!(host.is_composing());
    assert_eq!(
        host.native_key_binding().as_deref(),
        Some(binding.as_slice())
    );
    let command = host
        .get(COMMAND)
        .unwrap()
        .component::<CommandPaletteContent>()
        .unwrap();
    assert!(command.model.combo.filtered().is_empty());
    let menu = command
        .options()
        .layout()
        .expect("query-only floating surface");
    assert!(menu.items.is_empty());
    assert!(menu.popup.min_x() >= frame.min_x() && menu.popup.max_x() <= frame.max_x());
    assert!(menu.popup.min_y() >= frame.min_y() && menu.popup.max_y() <= frame.max_y());
    assert!(host.semantics().iter().any(|node| node.id == COMMAND_QUERY));
    assert!(
        host.semantics()
            .iter()
            .flat_map(|node| &node.parts)
            .all(|part| part.semantics.role != ContentRole::MenuItem)
    );
    let label = text
        .draws
        .iter()
        .rfind(|draw| draw.label == "Filter commands")
        .unwrap();
    assert!(
        label
            .opaque_before
            .iter()
            .any(|surface| surface.min_x() <= label.frame.min_x()
                && surface.max_x() >= label.frame.max_x()
                && surface.min_y() <= label.frame.min_y()
                && surface.max_y() >= label.frame.max_y())
    );
    assert!(host.event(&ContentEvent::Unmark).changed);
    assert!(!host.is_composing());
    let previous_len = host
        .get(COMMAND_QUERY)
        .unwrap()
        .component::<TextFieldContent>()
        .unwrap()
        .surface
        .input
        .len();
    assert!(
        host.event(&ContentEvent::Commit {
            text: "still no matching command".into(),
            replacement: Some(0..previous_len),
        })
        .changed,
        "query-only popup must remain editable"
    );
    host.render(
        &mut Scene::new(),
        &mut Text::new(),
        frame,
        frame,
        Theme::LIGHT.into(),
    )
    .unwrap();
    assert_eq!(
        host.get(COMMAND_QUERY)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap()
            .surface
            .input
            .text,
        "still no matching command"
    );
    assert_eq!(
        host.native_key_binding().as_deref(),
        Some(binding.as_slice())
    );
    assert!(
        host.get(COMMAND)
            .unwrap()
            .component::<CommandPaletteContent>()
            .unwrap()
            .model
            .combo
            .filtered()
            .is_empty()
    );
    assert!(
        host.semantics()
            .iter()
            .flat_map(|node| &node.parts)
            .all(|part| part.semantics.role != ContentRole::MenuItem)
    );
    assert!(
        host.event(&ContentEvent::Key {
            key: tekes_ui_controls::ControlKey::Escape,
            down: true
        })
        .consumed
    );
    assert!(
        !host
            .get(COMMAND)
            .unwrap()
            .component::<CommandPaletteContent>()
            .unwrap()
            .is_open()
    );
    assert_eq!(host.focused(), Some(COMMAND));
    assert!(host.semantics().iter().all(|node| node.id != COMMAND_QUERY));
}

#[test]
fn custom_trigger_raw_frame_clip_preserves_visibility_and_availability() {
    use tekes_ui_controls::{extended::CommandPalette, layout_view::CommandPaletteView};
    let mut view = CommandPaletteView::default();
    let mut model = CommandPalette::default();
    let mut text = Text::new();
    let frame = Rect::new(12., 12., 120., 40.);
    let content = Size::new(48., 16.);
    let palette = Theme::LIGHT.into();
    let layout = view.render_trigger_content(
        &mut Scene::new(), &mut text, &mut model, content, (frame, None), palette,
    );
    assert_eq!(layout.visible, Some(layout.frame));
    let point = layout.visible.unwrap().center();
    assert!(view.pointer_down_trigger(point));
    assert!(view.pointer_up_trigger(point, &mut model));
    assert!(!view.pointer_up_trigger(point, &mut model));
    model.combo.open = false;
    let clipped = view.render_trigger_content(
        &mut Scene::new(), &mut text, &mut model, content,
        (frame, Some(Rect::default())), palette,
    );
    assert_eq!(clipped.visible, None);
    assert!(!view.pointer_down_trigger(point));
    let clip = Rect::new(12., 12., 40., 40.);
    let partial = view.render_trigger_content(
        &mut Scene::new(), &mut text, &mut model, content, (frame, Some(clip)), palette,
    );
    assert_eq!(partial.visible, partial.frame.intersection(&clip));
    let point = partial.visible.unwrap().center();
    assert!(view.pointer_down_trigger(point));
    model.combo.disabled = true;
    view.render_trigger_content(
        &mut Scene::new(), &mut text, &mut model, content, (frame, None), palette,
    );
    assert!(!view.pointer_down_trigger(point));
    assert!(!view.pointer_up_trigger(point, &mut model));
    assert!(!model.combo.open);
    model.combo.disabled = false;
    view.set_window_active(false, &mut model);
    view.render_trigger_content(
        &mut Scene::new(), &mut text, &mut model, content, (frame, None), palette,
    );
    assert!(!view.pointer_down_trigger(point));
    assert!(!view.pointer_up_trigger(point, &mut model));
    view.set_window_active(true, &mut model);
    view.render_trigger_content(
        &mut Scene::new(), &mut text, &mut model, content, (frame, None), palette,
    );
    assert!(view.pointer_down_trigger(point));
    assert!(view.pointer_up_trigger(point, &mut model));
}
