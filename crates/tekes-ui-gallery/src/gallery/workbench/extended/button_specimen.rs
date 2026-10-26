//! Gallery placement/resource adapter; the original public Button/ContentHost
//! exclusively own actions, focus, capture, key timing, availability and AX.
use tekes_ui_controls::{
    ButtonTextRenderer, ControlKey,
    composition::*,
    layout_view::{ButtonComponentLayout, ComponentEvent, IconRenderer},
    layout::MeasuredChild,
};
use tekes_ui_scene::{Point, Primitive, Rect, Scene, Size};
use tekes_ui_tokens::{Palette, proposal};
use std::{cell::RefCell, rc::Rc};
const OWNER: ComponentId = ComponentId(1);
const ICON_SLOT: ComponentId = ComponentId(2);

fn has_area(rect: Rect) -> bool {
    rect.size.width > 0. && rect.size.height > 0.
}

#[derive(Clone, Copy, Debug)]
struct IconPaint {
    frame: Rect,
    clip: Rect,
    tint: tekes_ui_tokens::Rgba,
}

#[derive(Clone, Copy, Debug)]
enum IconPaintState {
    Ready(IconPaint),
    MissingThemeInk,
    Clipped,
}

#[derive(Clone, Debug, Default)]
struct PassiveArtworkSlot {
    paint: Rc<RefCell<Option<IconPaintState>>>,
}

impl Component for PassiveArtworkSlot {
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        let role = proposal::TextRole::UiBody;
        let scale = text.resolved_font_size(role) / role.style().size;
        let scale = if scale.is_finite() && scale > 0. { scale } else { 1. };
        let side = proposal::metric("size", "size-icon-standard")
            .expect("regular icon DS token")
            * scale;
        MeasuredChild::new(Size::new(
            side.min(available.width.max(0.)).max(0.),
            side.min(available.height.max(0.)).max(0.),
        ))
    }

    fn paint(
        &mut self,
        _scene: &mut Scene,
        _text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        let mut state = self.paint.borrow_mut();
        *state = if !has_area(frame) || !frame.intersection(&clip).is_some_and(has_area) {
            Some(IconPaintState::Clipped)
        } else if let Some(tint) = theme.icon_foreground {
            Some(IconPaintState::Ready(IconPaint { frame, clip, tint }))
        } else {
            Some(IconPaintState::MissingThemeInk)
        };
    }

    fn semantic_hidden(&self) -> bool {
        true
    }

    fn focusable(&self) -> bool {
        false
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ArtworkPaintError {
    MissingSlotPaint,
    MissingActionIconInk,
    ProviderProducedNoArtwork,
}

fn is_drawable_icon_output(primitive: &Primitive) -> bool {
    match primitive {
        Primitive::Quad(quad) => {
            has_area(quad.rect)
                && (quad.fill.a > 0.
                    || (quad.border_width > 0. && quad.border_color.a > 0.))
        }
        Primitive::Image(image) => {
            has_area(image.rect)
                && image.opacity.is_finite()
                && image.opacity > 0.
        }
        Primitive::TransformedImage(image) => {
            let bounds = image.bounds();
            has_area(image.quad.rect)
                && has_area(bounds)
                && image.quad.opacity.is_finite()
                && image.quad.opacity > 0.
        }
        _ => false,
    }
}

pub(crate) struct ButtonSpecimen {
    host: ContentHost,
    icon_paint: Rc<RefCell<Option<IconPaintState>>>,
}
impl Default for ButtonSpecimen {
    fn default() -> Self {
        Self::new("Create item")
    }
}
impl ButtonSpecimen {
    pub fn new(name: &str) -> Self {
        let icon_paint = Rc::new(RefCell::new(None));
        Self {
            host: ContentHost::new(Content::new(
                OWNER,
                Button::new(Content::new(
                    ICON_SLOT,
                    PassiveArtworkSlot {
                        paint: Rc::clone(&icon_paint),
                    },
                ))
                .with_accessible_name(name),
            ))
            .expect("named original Button"),
            icon_paint,
        }
    }
    pub fn button(&self) -> &Button {
        self.host.get(OWNER).unwrap().component::<Button>().unwrap()
    }
    pub fn button_mut(&mut self) -> &mut Button {
        self.host
            .get_mut(OWNER)
            .unwrap()
            .component_mut::<Button>()
            .unwrap()
    }
    pub fn name(&self) -> &str {
        self.button().accessible_name()
    }
    pub fn measure(text: &mut dyn ButtonTextRenderer) -> Size {
        Button::new(Content::new(ICON_SLOT, PassiveArtworkSlot::default()))
            .with_accessible_name("Icon")
            .measure(text, Size::new(f32::MAX, f32::MAX))
            .size
    }
    pub fn layout(&self) -> ButtonComponentLayout {
        self.button()
            .button
            .layout()
            .map_or(ButtonComponentLayout::default(), |l| {
                ButtonComponentLayout {
                    frame: l.frame,
                    visible: l.visible,
                    content: l.content,
                }
            })
    }
    #[cfg(test)]
    pub fn icon_frame(&self) -> Rect {
        match *self.icon_paint.borrow() {
            Some(IconPaintState::Ready(paint)) => paint.frame,
            _ => Rect::default(),
        }
    }
    /// The passive slot is measured, arranged, clipped and theme-resolved by
    /// the original Button/ContentHost. The existing renderer is invoked only
    /// after that host paint releases the Gallery text borrow.
    pub(super) fn draw_icon(
        &self,
        scene: &mut Scene,
        renderer: &mut impl IconRenderer,
    ) -> Result<(), ArtworkPaintError> {
        let layout = self.layout();
        let Some(visible) = layout.visible else {
            return Ok(());
        };
        if !has_area(visible) {
            return Ok(());
        }
        // ContentHost may skip paint for a child whose own arranged frame is
        // fully clipped while the outer Button remains visible. Query the
        // original mounted slot layout before treating absent paint data as a
        // failed paint.
        let slot_visible = self
            .host
            .get(ICON_SLOT)
            .and_then(Content::visible)
            .filter(|rect| has_area(*rect));
        if slot_visible.is_none() {
            return Ok(());
        }
        let paint = match *self.icon_paint.borrow() {
            Some(IconPaintState::Ready(paint)) => paint,
            Some(IconPaintState::Clipped) => return Ok(()),
            Some(IconPaintState::MissingThemeInk) => return Err(ArtworkPaintError::MissingActionIconInk),
            None => return Err(ArtworkPaintError::MissingSlotPaint),
        };
        let Some(clip) = visible.intersection(&paint.clip).filter(|clip| has_area(*clip)) else {
            return Ok(());
        };
        let mut artwork = Scene::new();
        artwork.push_clip(clip);
        let before_provider = artwork.primitives().len();
        renderer.draw_icon(&mut artwork, "plus", paint.frame, paint.tint);
        let has_artwork = artwork.primitives()[before_provider..]
            .iter()
            // This is a provider capability check. A valid plus may have
            // artwork entirely in the whitespace removed by this clip.
            .any(is_drawable_icon_output);
        artwork.pop_clip();
        if !has_artwork {
            return Err(ArtworkPaintError::ProviderProducedNoArtwork);
        }
        if self.button().button.state.disabled {
            let opacity = proposal::metric("opacity", "alpha-wash")
                .expect("disabled Button image opacity token");
            for primitive in artwork.primitives() {
                let mut primitive = primitive.clone();
                match &mut primitive {
                    Primitive::Image(image) => image.opacity *= opacity,
                    Primitive::TransformedImage(image) => image.quad.opacity *= opacity,
                    _ => {}
                }
                scene.push(primitive);
            }
        } else {
            for primitive in artwork.primitives() {
                scene.push(primitive.clone());
            }
        }
        Ok(())
    }
    pub fn render(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Option<Rect>,
        palette: Palette,
    ) -> ButtonComponentLayout {
        *self.icon_paint.borrow_mut() = None;
        self.host
            .render(scene, text, frame, clip.unwrap_or(frame), palette)
            .expect("public Button render");
        self.layout()
    }
    pub fn set_window_active(&mut self, active: bool) {
        self.host.set_window_active(active)
    }
    pub fn focus(&mut self, visible: bool) {
        self.host.focus(Some(OWNER), visible);
    }
    pub fn blur(&mut self) {
        self.host.focus(None, false);
    }
    pub fn activate(&mut self) -> bool {
        self.host
            .accessibility_action(OWNER, None, AccessibilityAction::Press)
            .activated
    }
    pub fn key_event(&mut self, key: ControlKey, down: bool) -> ComponentEvent {
        let r = self.host.event(&ContentEvent::Key { key, down });
        ComponentEvent {
            consumed: r.consumed,
            changed: r.activated,
        }
    }
    pub fn key(&mut self, key: ControlKey, down: bool) -> bool {
        self.key_event(key, down).changed
    }
    pub fn pointer_down(&mut self, p: Point) -> bool {
        self.host.event(&ContentEvent::PointerDown(p)).consumed
    }
    pub fn pointer_move(&mut self, p: Point) {
        self.host.event(&ContentEvent::PointerMove(p));
    }
    pub fn pointer_up(&mut self, p: Point) -> bool {
        self.host.event(&ContentEvent::PointerUp(p)).activated
    }
    pub fn cancel(&mut self) {
        self.host.cancel();
    }
}

impl std::fmt::Debug for ButtonSpecimen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ButtonSpecimen")
            .field("name", &self.name())
            .field("layout", &self.layout())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tekes_ui_scene::{ImageId, ImageQuad, Primitive, TransformedImageQuad};

    struct ImageBacked(Rc<RefCell<Option<tekes_ui_tokens::Rgba>>>);
    impl IconRenderer for ImageBacked {
        fn draw_icon(&mut self, scene: &mut Scene, _: &str, frame: Rect, tint: tekes_ui_tokens::Rgba) {
            *self.0.borrow_mut() = Some(tint);
            scene.push_image(ImageQuad {
                rect: frame,
                image: ImageId(7),
                opacity: 0.2,
            });
            scene.push_transformed_image(TransformedImageQuad::new(
                ImageQuad {
                    rect: frame,
                    image: ImageId(8),
                    opacity: 0.2,
                },
                0.25,
            ));
        }
    }

    struct EmptyProvider;
    impl IconRenderer for EmptyProvider {
        fn draw_icon(&mut self, _: &mut Scene, _: &str, _: Rect, _: tekes_ui_tokens::Rgba) {}
    }

    struct TransparentProvider;
    impl IconRenderer for TransparentProvider {
        fn draw_icon(&mut self, scene: &mut Scene, _: &str, _: Rect, _: tekes_ui_tokens::Rgba) {
            scene.push_quad(tekes_ui_scene::Quad::new(
                Rect::default(),
                tekes_ui_tokens::Rgba::new(1., 1., 1., 0.),
            ));
        }
    }

    struct PlusArtwork;
    impl IconRenderer for PlusArtwork {
        fn draw_icon(&mut self, scene: &mut Scene, _: &str, frame: Rect, _: tekes_ui_tokens::Rgba) {
            let ink = tekes_ui_tokens::Rgba::new(0., 0., 0., 1.);
            let center = frame.center();
            scene.push_quad(tekes_ui_scene::Quad::new(
                Rect::new(frame.origin.x, center.y - 1., frame.size.width, 2.),
                ink,
            ));
            scene.push_quad(tekes_ui_scene::Quad::new(
                Rect::new(center.x - 1., frame.origin.y, 2., frame.size.height),
                ink,
            ));
        }
    }

    #[test]
    fn passive_slot_reports_intrinsic_icon_size_and_clamps_negative_space() {
        let mut text = crate::gallery::text::TextEngine::new();
        let mut slot = PassiveArtworkSlot::default();
        let role = proposal::TextRole::UiBody;
        let scale = text.resolved_font_size(role) / role.style().size;
        let natural = proposal::metric("size", "size-icon-standard").unwrap() * scale;
        assert_eq!(
            slot.measure_intrinsic(&mut text, Size::new(100., 100.)).size,
            Size::new(natural, natural)
        );
        assert_eq!(
            slot.measure(&mut text, Size::new(-3., -7.)).size,
            Size::default()
        );
    }

    #[test]
    fn image_backed_provider_remains_supported_by_button_placement() {
        let mut text = crate::gallery::text::TextEngine::new();
        let mut view = ButtonSpecimen::new("Create item");
        let bounds = Rect::new(20., 30., 48., 48.);
        view.render(
            &mut Scene::new(),
            &mut text,
            bounds,
            Some(bounds),
            Palette::from(tekes_ui_tokens::Theme::LIGHT),
        );
        let mut scene = Scene::new();
        let tint = Rc::new(RefCell::new(None));
        view.draw_icon(&mut scene, &mut ImageBacked(Rc::clone(&tint))).unwrap();
        assert!(scene.primitives().iter().any(|primitive| matches!(
            primitive,
            Primitive::Image(image)
                if image.image == ImageId(7)
                    && image.rect == view.icon_frame()
                    && image.opacity == 0.2
        )));
        assert!(scene.primitives().iter().any(|primitive| matches!(
            primitive,
            Primitive::TransformedImage(image)
                if image.quad.image == ImageId(8) && image.quad.opacity == 0.2
        )));
        let expected = match *view.icon_paint.borrow() {
            Some(IconPaintState::Ready(paint)) => paint.tint,
            _ => panic!("Button action child must resolve icon tint"),
        };
        assert_eq!(*tint.borrow(), Some(expected));

        view.button_mut().button.state.disabled = true;
        view.render(
            &mut Scene::new(),
            &mut text,
            bounds,
            Some(bounds),
            Palette::from(tekes_ui_tokens::Theme::LIGHT),
        );
        let mut disabled_scene = Scene::new();
        view.draw_icon(
            &mut disabled_scene,
            &mut ImageBacked(Rc::clone(&tint)),
        ).unwrap();
        let disabled = match *view.icon_paint.borrow() {
            Some(IconPaintState::Ready(paint)) => paint.tint,
            _ => panic!("disabled Button action child must resolve icon tint"),
        };
        assert_eq!(*tint.borrow(), Some(disabled));
        assert!(disabled_scene.primitives().iter().any(|primitive| matches!(
            primitive,
            Primitive::Image(image)
                if image.image == ImageId(7)
                    && image.opacity == 0.2 * proposal::metric("opacity", "alpha-wash").unwrap()
        )));
        assert!(disabled_scene.primitives().iter().any(|primitive| matches!(
            primitive,
            Primitive::TransformedImage(image)
                if image.quad.image == ImageId(8)
                    && image.quad.opacity == 0.2 * proposal::metric("opacity", "alpha-wash").unwrap()
        )));
    }

    #[test]
    fn empty_provider_is_reported_as_failure() {
        let mut text = crate::gallery::text::TextEngine::new();
        let mut view = ButtonSpecimen::new("Create item");
        let bounds = Rect::new(20., 30., 48., 48.);
        view.render(
            &mut Scene::new(),
            &mut text,
            bounds,
            Some(bounds),
            Palette::from(tekes_ui_tokens::Theme::LIGHT),
        );
        assert_eq!(
            view.draw_icon(&mut Scene::new(), &mut EmptyProvider),
            Err(ArtworkPaintError::ProviderProducedNoArtwork)
        );
    }

    #[test]
    fn transparent_zero_area_provider_output_is_rejected() {
        let mut text = crate::gallery::text::TextEngine::new();
        let mut view = ButtonSpecimen::new("Create item");
        let bounds = Rect::new(20., 30., 48., 48.);
        view.render(
            &mut Scene::new(),
            &mut text,
            bounds,
            Some(bounds),
            Palette::from(tekes_ui_tokens::Theme::LIGHT),
        );
        assert_eq!(
            view.draw_icon(&mut Scene::new(), &mut TransparentProvider),
            Err(ArtworkPaintError::ProviderProducedNoArtwork)
        );
    }

    #[test]
    fn visible_button_with_fully_clipped_slot_is_not_missing_slot_paint() {
        let mut text = crate::gallery::text::TextEngine::new();
        let mut view = ButtonSpecimen::new("Create item");
        let bounds = Rect::new(20., 30., 160., 60.);
        let palette = Palette::from(tekes_ui_tokens::Theme::LIGHT);
        view.render(&mut Scene::new(), &mut text, bounds, Some(bounds), palette);
        let slot_frame = view.host.get(ICON_SLOT).unwrap().frame();
        assert!(view.layout().visible.is_some());

        // The Button remains visible in the left edge, while its centered
        // artwork slot receives no visible layout and therefore no paint call.
        let clip = Rect::new(bounds.origin.x, bounds.origin.y, 5., bounds.size.height);
        assert!(slot_frame.intersection(&clip).is_none());
        view.render(&mut Scene::new(), &mut text, bounds, Some(clip), palette);
        assert!(view.layout().visible.is_some());
        assert!(view.host.get(ICON_SLOT).unwrap().visible().is_none());
        assert_eq!(
            view.draw_icon(&mut Scene::new(), &mut EmptyProvider),
            Ok(())
        );
    }

    #[test]
    fn partial_slot_clip_through_plus_whitespace_keeps_provider_capability() {
        let mut text = crate::gallery::text::TextEngine::new();
        let mut view = ButtonSpecimen::new("Create item");
        let bounds = Rect::new(20., 30., 48., 48.);
        let palette = Palette::from(tekes_ui_tokens::Theme::LIGHT);
        view.render(&mut Scene::new(), &mut text, bounds, Some(bounds), palette);
        let slot = view.host.get(ICON_SLOT).unwrap().frame();
        let clip = Rect::new(slot.origin.x, slot.origin.y, 3., 3.);
        assert!(slot.intersection(&clip).is_some());

        view.render(&mut Scene::new(), &mut text, bounds, Some(clip), palette);
        assert!(view.host.get(ICON_SLOT).unwrap().visible().is_some());
        assert_eq!(view.draw_icon(&mut Scene::new(), &mut PlusArtwork), Ok(()));
    }

    #[test]
    fn original_button_keeps_pointer_keyboard_ax_disabled_and_cancel_ownership() {
        let mut text = crate::gallery::text::TextEngine::new();
        let mut view = ButtonSpecimen::new("Create item");
        let bounds = Rect::new(20., 30., 48., 48.);
        let palette = Palette::from(tekes_ui_tokens::Theme::LIGHT);
        view.render(&mut Scene::new(), &mut text, bounds, Some(bounds), palette);

        for key in [ControlKey::Enter, ControlKey::Space] {
            view.focus(true);
            let before = view.button().activations;
            view.key_event(key, true);
            view.key_event(key, false);
            assert_eq!(view.button().activations, before + 1, "{key:?}");
        }
        let before_ax = view.button().activations;
        assert!(view.activate());
        assert_eq!(view.button().activations, before_ax + 1);

        let point = view.layout().visible.expect("visible Button").center();
        let before_pointer = view.button().activations;
        assert!(view.pointer_down(point));
        assert!(view.pointer_up(point));
        assert_eq!(view.button().activations, before_pointer + 1);
        assert!(view.pointer_down(point));
        view.cancel();
        assert!(!view.pointer_up(point));

        view.button_mut().button.state.disabled = true;
        view.render(&mut Scene::new(), &mut text, bounds, Some(bounds), palette);
        let disabled_count = view.button().activations;
        assert!(!view.activate());
        assert!(!view.key_event(ControlKey::Enter, true).changed);
        assert!(!view.pointer_down(point));
        assert_eq!(view.button().activations, disabled_count);
    }
}
