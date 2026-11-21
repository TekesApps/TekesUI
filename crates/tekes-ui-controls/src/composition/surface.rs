//! Non-curved semantic Surface container for grouped cards and empty-state frames.
use super::*;
use crate::ButtonTextRenderer;
use crate::Insets;
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::Spacing;

/// Rectangular semantic Surface container with single owned padding and optional hairline frame.
///
/// Unlike [`PanelContent`], `SurfaceContent` does not enforce curved clip chains,
/// allowing standard rectangular grouping cards and framed callout containers.
#[derive(Debug)]
pub struct SurfaceContent {
    children: Vec<Content>,
    surface: &'static str,
    framed: bool,
    padding: Insets,
    max_width: Option<f32>,
    min_height: Option<f32>,
    backing_scale: f32,
    arranged: bool,
}

impl SurfaceContent {
    fn new_internal(child: Content, surface: &'static str, framed: bool) -> Self {
        let p = Spacing::Loose.value();
        Self {
            children: vec![child],
            surface,
            framed,
            padding: Insets::new(p, p, p, p),
            max_width: None,
            min_height: None,
            backing_scale: 1.,
            arranged: false,
        }
    }

    /// Elevated panel-surface container (`surface = "panel"`, hairline boundary).
    pub fn panel(child: Content) -> Self {
        Self::new_internal(child, "panel", true)
    }

    /// Canvas-surface container (`surface = "canvas"`, flat ground by default).
    pub fn canvas(child: Content) -> Self {
        Self::new_internal(child, "canvas", false)
    }

    /// Configure whether this container paints a decorative hairline boundary.
    pub fn with_border(mut self, framed: bool) -> Self {
        self.framed = framed;
        self
    }

    /// Override uniform/four-edge inner padding (defaults to `Spacing::Loose` = 16 pt).
    pub fn with_padding(mut self, padding: Insets) -> Self {
        self.padding = Insets::new(
            finite(padding.top),
            finite(padding.right),
            finite(padding.bottom),
            finite(padding.left),
        );
        self
    }

    /// Cap measured and arranged outer width to `max_width` (when smaller than available width).
    pub fn with_max_width(mut self, max_width: f32) -> Self {
        self.max_width = max_width.is_finite().then_some(max_width.max(0.));
        self
    }

    /// Enforce a minimum outer height.
    pub fn with_min_height(mut self, min_height: f32) -> Self {
        self.min_height = min_height.is_finite().then_some(min_height.max(0.));
        self
    }

    pub fn surface_name(&self) -> &'static str {
        self.surface
    }
}

impl Component for SurfaceContent {
    fn default_surface(&self) -> Option<&'static str> {
        Some(self.surface)
    }

    fn host_reveal_passthrough(&self) -> bool {
        self.arranged
    }

    fn curve_subtree_capability(&self) -> CurveSubtreeCapability {
        CurveSubtreeCapability::Recursive
    }

    fn has_stable_hooks(&self) -> bool {
        true
    }

    fn supports_prepared_lifecycle(&self) -> bool {
        true
    }

    fn ignores_pointer_actions(&self) -> bool {
        true
    }

    fn fixed_slots(&self) -> bool {
        true
    }

    fn validate(&self) -> Result<(), CompositionError> {
        if self.children.len() != 1 {
            Err(CompositionError::InvalidSlotCount)
        } else {
            Ok(())
        }
    }

    fn lifecycle(&mut self, event: Lifecycle) {
        if event == Lifecycle::Unmounted {
            self.arranged = false;
        }
    }

    fn children(&self) -> &[Content] {
        &self.children
    }

    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.children)
    }

    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        self.arranged = false;
        let outer_w = self
            .max_width
            .map_or(available.width, |limit| available.width.min(limit));
        let inner_w = (outer_w - self.padding.left - self.padding.right).max(0.);
        let inner_h = (available.height - self.padding.top - self.padding.bottom).max(0.);
        let child = self.children[0].measure(text, Size::new(inner_w, inner_h));
        let natural_h = child.size.height + self.padding.top + self.padding.bottom;
        let outer_h = self
            .min_height
            .map_or(natural_h, |min_h| natural_h.max(min_h));
        let mut measured = MeasuredChild::new(Size::new(outer_w, outer_h));
        measured.first_baseline = child.first_baseline.map(|b| b + self.padding.top);
        measured
    }

    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        self.backing_scale = text.backing_scale();
        let inner = Rect::new(
            frame.min_x() + self.padding.left,
            frame.min_y() + self.padding.top,
            (frame.size.width - self.padding.left - self.padding.right).max(0.),
            (frame.size.height - self.padding.top - self.padding.bottom).max(0.),
        );
        self.children[0].arrange(text, inner, clip);
        self.arranged = frame.size.width > 0. && frame.size.height > 0.;
    }

    fn paint(
        &mut self,
        scene: &mut Scene,
        _text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        scene.push_clip(clip);
        if self.framed {
            let _ = crate::paint::panel_on_surface_at_scale(
                scene,
                frame,
                theme.palette,
                theme.surface(),
                self.backing_scale,
            );
        } else {
            crate::paint::panel_background(
                scene,
                frame,
                theme.palette,
                theme.surface(),
                true,
                self.backing_scale,
            );
        }
        scene.pop_clip();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ButtonTextAlign;
    use tekes_ui_scene::Primitive;
    use tekes_ui_tokens::Theme;

    struct Text;
    impl ButtonTextRenderer for Text {
        fn measure(&mut self, value: &str, role: proposal::TextRole, width: f32) -> Size {
            Size::new(
                (value.chars().count() as f32 * 8.).min(width),
                role.style().line_height,
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

    #[test]
    fn surface_content_respects_padding_bounds_and_propagates_surface() {
        let card = Content::new(
            ComponentId(1),
            SurfaceContent::panel(Content::new(ComponentId(2), Button::new("Inspect"))),
        );
        let mut host = ContentHost::new(card).unwrap();
        let viewport = Rect::new(0., 0., 400., 300.);
        let mut scene = Scene::new();
        host.render(
            &mut scene,
            &mut Text,
            viewport,
            viewport,
            Theme::LIGHT.into(),
        )
        .unwrap();
        let outer = host.get(ComponentId(1)).unwrap().frame();
        let inner = host.get(ComponentId(2)).unwrap().frame();
        assert_eq!(inner.min_x() - outer.min_x(), Spacing::Loose.value());
        assert_eq!(outer.max_x() - inner.max_x(), Spacing::Loose.value());
        assert_eq!(inner.min_y() - outer.min_y(), Spacing::Loose.value());
        assert!(
            scene
                .primitives()
                .iter()
                .any(|p| matches!(p, Primitive::Quad(_)))
        );

        let capped = Content::new(
            ComponentId(10),
            StackContent {
                model: crate::extended::Stack::new(crate::extended::Axis::Vertical),
                flexible_child: None,
                children: vec![Content::new(
                    ComponentId(3),
                    SurfaceContent::canvas(Content::new(ComponentId(4), TextContent::new("Empty")))
                        .with_border(true)
                        .with_max_width(520.)
                        .with_min_height(164.),
                )],
            },
        );
        let mut host2 = ContentHost::new(capped).unwrap();
        let wide = Rect::new(0., 0., 800., 300.);
        host2
            .render(&mut Scene::new(), &mut Text, wide, wide, Theme::DARK.into())
            .unwrap();
        let card2 = host2.get(ComponentId(3)).unwrap().frame();
        assert_eq!(card2.size.width, 520.);
        assert!(card2.size.height >= 164.);
    }
}
