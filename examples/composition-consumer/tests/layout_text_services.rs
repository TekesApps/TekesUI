//! Generic layout-driven pane; real retained TextFieldContent, public APIs only.
use std::rc::Rc;
use tekes_ui_composition_consumer::layout_activation::{FIELD, PANE, Pane, fixture};
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ReadingController, TextSurfaceHost,
    TextSurfaceTextRenderer, composition::*,
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_text::{
    Paragraph, ShapeOptions, Shaper, TextStyle,
    coretext::{CoreTextFonts, CoreTextShaper},
};
use tekes_ui_tokens::{Rgba, Theme, proposal};
struct Text {
    enabled: bool,
    shaper: CoreTextShaper,
    widths: Vec<Option<f32>>,
}
impl Text {
    fn new(enabled: bool) -> Self {
        Self {
            enabled,
            shaper: CoreTextShaper::new(CoreTextFonts::new()),
            widths: Vec::new(),
        }
    }
}
impl ButtonTextRenderer for Text {
    fn text_surface_host(&mut self) -> Option<&mut dyn TextSurfaceHost> {
        if self.enabled { Some(self) } else { None }
    }
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        assert!(w > 0., "inactive pane must not measure zero-width text");
        Size::new(
            (s.chars().count() as f32 * 7.).min(w),
            r.style().line_height,
        )
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
impl TextSurfaceHost for Text {
    fn shape_surface(&mut self, s: &str, r: proposal::TextRole, w: Option<f32>) -> Rc<Paragraph> {
        assert!(w.is_none_or(|w| w > 0.), "zero-width editor shaping");
        self.widths.push(w);
        Rc::new(self.shaper.shape(
            s,
            &TextStyle::system(r.style().size).with_weight(r.style().weight),
            &ShapeOptions {
                width: w.map(f64::from),
                line_spacing: 0.,
            },
        ))
    }
}
impl TextSurfaceTextRenderer for Text {
    fn push_visible(&mut self, _: &mut Scene, _: &Paragraph, _: Point, _: Rgba, _: Rect) {}
}
fn render(h: &mut ContentHost, t: &mut Text, f: Rect) -> (Result<(), CompositionError>, Scene) {
    let mut scene = Scene::new();
    let result = h.render(&mut scene, t, f, f, Theme::LIGHT.into());
    (result, scene)
}
fn positive() -> Rect {
    Rect::new(0., 0., 400., 300.)
}
#[test]
fn layout_arrange_activation_requires_the_same_service_as_always_active_fields() {
    let (mut h, paints, arranges) = fixture(false, false);
    let mut t = Text::new(false);
    let mut always = ContentHost::new(Content::new(
        FIELD,
        TextFieldContent::new("always active", "retained"),
    ))
    .unwrap();
    assert_eq!(
        render(&mut always, &mut t, positive()).0,
        Err(CompositionError::MissingTextServices(FIELD))
    );
    assert!(
        render(&mut h, &mut t, Rect::new(0., 0., 0., 300.))
            .0
            .is_ok()
    );
    let (result, scene) = render(&mut h, &mut t, positive());
    assert_eq!(result, Err(CompositionError::MissingTextServices(FIELD)));
    assert_eq!(paints.get(), 0, "no invalid subtree paint");
    assert!(scene.primitives().is_empty());
    assert_eq!(arranges.get(), 2);
    assert!(
        h.get(PANE)
            .unwrap()
            .component::<Pane>()
            .unwrap()
            .layout_active(),
        "application layout mutation is retained,not rolled back"
    );
}
#[test]
fn layout_measure_activation_is_checked_before_arrange() {
    let (mut h, paints, arranges) = fixture(true, false);
    let mut t = Text::new(false);
    assert_eq!(
        render(&mut h, &mut t, positive()).0,
        Err(CompositionError::MissingTextServices(FIELD))
    );
    assert_eq!(arranges.get(), 0);
    assert_eq!(paints.get(), 0);
}
#[test]
fn a_valid_backend_can_recover_after_failure_without_replacing_or_focusing_editor() {
    for readonly in [false, true] {
        let (mut h, paints, _) = fixture(false, readonly);
        let mut t = Text::new(false);
        let before = h
            .get(FIELD)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap();
        let baseline = before.surface.input.external_baseline();
        let version = before
            .reading
            .as_ref()
            .and_then(ReadingController::document_version)
            .cloned();
        let value = before.surface.input.text.clone();
        assert_eq!(
            render(&mut h, &mut t, positive()).0,
            Err(CompositionError::MissingTextServices(FIELD))
        );
        assert_eq!(paints.get(), 0);
        t.enabled = true;
        render(&mut h, &mut t, positive()).0.unwrap();
        let after = h
            .get(FIELD)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap();
        assert_eq!(after.surface.input.external_baseline(), baseline);
        assert_eq!(after.surface.input.text, value);
        assert_eq!(
            after
                .reading
                .as_ref()
                .and_then(ReadingController::document_version),
            version.as_ref()
        );
        assert!(after.surface.paragraph.is_some());
        assert_eq!(h.focused(), None);
        assert!(paints.get() > 0);
        assert!(h.focus(Some(FIELD), true));
        assert_eq!(h.focused(), Some(FIELD));
        h.focus(None, false);
    }
}
#[test]
fn inactive_zero_width_or_height_does_not_shape_and_valid_reopen_retains_attachment() {
    for readonly in [false, true] {
        let (mut h, _, _) = fixture(false, readonly);
        let mut t = Text::new(true);
        for f in [Rect::new(0., 0., 0., 300.), Rect::new(0., 0., 400., 0.)] {
            render(&mut h, &mut t, f).0.unwrap();
            assert!(t.widths.is_empty());
            assert!(h.text_surface(FIELD).is_none());
        }
        let baseline = h
            .get(FIELD)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap()
            .surface
            .input
            .external_baseline();
        render(&mut h, &mut t, positive()).0.unwrap();
        assert!(h.text_surface(FIELD).unwrap().paragraph.is_some());
        assert_eq!(h.focused(), None);
        t.widths.clear();
        render(&mut h, &mut t, Rect::new(0., 0., 0., 300.))
            .0
            .unwrap();
        assert!(t.widths.is_empty());
        assert!(h.text_surface(FIELD).is_none());
        t.enabled = false;
        render(&mut h, &mut t, Rect::new(0., 0., 0., 300.))
            .0
            .unwrap();
        t.enabled = true;
        render(&mut h, &mut t, positive()).0.unwrap();
        assert_eq!(
            h.text_surface(FIELD).unwrap().input.external_baseline(),
            baseline
        );
        assert_eq!(h.focused(), None);
    }
}

#[test]
fn legacy_label_only_renderers_do_not_acquire_a_new_text_surface_requirement() {
    let mut host = ContentHost::new(Content::new(
        ComponentId(9920),
        TextContent::new("ordinary label"),
    ))
    .unwrap();
    render(&mut host, &mut Text::new(false), positive())
        .0
        .unwrap();
}
