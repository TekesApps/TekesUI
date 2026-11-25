//! Shared leading decorative icon + flexible wrapping text row composition (#370).
use super::*;
use crate::ButtonTextRenderer;
use tekes_ui_scene::{Rect, Size};
use tekes_ui_tokens::Spacing;

/// Shared horizontal `IconContent` + flexible `TextContent` row anatomy with
/// token-sourced gap (`Spacing::Snug` = 6 pt), preserving caller-owned IDs and mounts.
#[derive(Debug)]
pub struct IconTextContent {
    row: StackContent,
    icon_id: ComponentId,
    text_id: ComponentId,
    arranged: bool,
}

impl IconTextContent {
    /// Semantic horizontal spacing between the leading decorative icon and text (`Spacing::Snug` = 6 pt).
    pub const GAP: f32 = Spacing::Snug.value();

    /// Compose an existing `icon` and `text` `Content` node into a shared horizontal
    /// row with `Spacing::Snug` gap. The leading `icon` retains layout, paint,
    /// surface/theme inheritance, and mount identity while suppressing redundant AX semantics.
    pub fn new(mut icon: Content, text: Content) -> Self {
        icon.suppress_semantics();
        let icon_id = icon.id;
        let text_id = text.id;
        Self {
            row: StackContent::horizontal(Self::GAP, vec![icon, text]).with_flexible_child(text_id),
            icon_id,
            text_id,
            arranged: false,
        }
    }

    /// Construct a standard canonical [`IconContent`] paired with plain [`TextContent`],
    /// preserving the caller's explicit child `ComponentId`s.
    pub fn icon_label(
        icon_id: ComponentId,
        canonical_icon: &str,
        text_id: ComponentId,
        text: impl Into<String>,
    ) -> Result<Self, IconRenderError> {
        Self::icon_label_with_extensions(icon_id, canonical_icon, text_id, text, &[])
    }

    /// Construct a standard [`IconContent`] paired with plain [`TextContent`], resolving
    /// core glyphs first and falling back to `extensions`.
    pub fn icon_label_with_extensions(
        icon_id: ComponentId,
        icon_name: &str,
        text_id: ComponentId,
        text: impl Into<String>,
        extensions: &[tekes_ui_icons::Glyph],
    ) -> Result<Self, IconRenderError> {
        Ok(Self::new(
            Content::new(
                icon_id,
                IconContent::new_with_extensions(icon_name, extensions)?,
            ),
            Content::new(text_id, TextContent::new(text)),
        ))
    }

    /// Horizontal spacing between the leading icon and text label (`Spacing::Snug`).
    pub fn gap(&self) -> f32 {
        Self::GAP
    }

    /// Stable `ComponentId` of the leading decorative icon child.
    pub fn icon_id(&self) -> ComponentId {
        self.icon_id
    }

    /// Stable `ComponentId` of the flexible text child.
    pub fn text_id(&self) -> ComponentId {
        self.text_id
    }
}

impl Component for IconTextContent {
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
        if self.row.children().len() != 2 {
            return Err(CompositionError::InvalidSlotCount);
        }
        Ok(())
    }

    fn children(&self) -> &[Content] {
        self.row.children()
    }

    fn children_mut(&mut self) -> Option<&mut Vec<Content>> {
        Some(&mut self.row.children)
    }

    fn lifecycle(&mut self, event: Lifecycle) {
        if event == Lifecycle::Unmounted {
            self.arranged = false;
        }
    }

    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        self.arranged = false;
        self.row.measure(text, available)
    }

    fn measure_intrinsic(
        &mut self,
        text: &mut dyn ButtonTextRenderer,
        available: Size,
    ) -> MeasuredChild {
        self.row.measure_intrinsic(text, available)
    }

    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        self.row.arrange(text, frame, clip);
        self.arranged = frame.size.width > 0. && frame.size.height > 0.;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ButtonTextAlign, layout_view::IconRenderer};
    use std::rc::Rc;
    use tekes_ui_scene::{ImageId, Scene};
    use tekes_ui_tokens::{Palette, Rgba, Theme};

    struct WrapText;
    impl ButtonTextRenderer for WrapText {
        fn measure(&mut self, value: &str, role: proposal::TextRole, width: f32) -> Size {
            let char_w = 7.;
            let natural = value.chars().count() as f32 * char_w;
            let cols = (width.max(char_w) / char_w).floor().max(1.);
            let explicit_lines = value.split('\n').collect::<Vec<_>>();
            let wrapped_lines = explicit_lines
                .iter()
                .map(|line| {
                    let chars = line.chars().count().max(1) as f32;
                    (chars / cols).ceil().max(1.)
                })
                .sum::<f32>();
            Size::new(
                natural.min(width.max(0.)),
                wrapped_lines * role.style().line_height,
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

    struct Icons;
    impl IconRenderer for Icons {
        fn draw_icon(&mut self, _: &mut Scene, _: &str, _: Rect, _: Rgba) {}
        fn prepare_icon(
            &mut self,
            _: &str,
            _: f32,
            _: f32,
            _: Rgba,
        ) -> Result<IconImage, IconRenderError> {
            Ok(IconImage::new(ImageId(1), Rc::new(())))
        }
    }

    #[test]
    fn icon_text_wraps_narrow_text_without_overlap_and_preserves_mounts() {
        const ROOT: ComponentId = ComponentId(700);
        const ICON: ComponentId = ComponentId(701);
        const TEXT: ComponentId = ComponentId(702);

        let row = IconTextContent::icon_label(
            ICON,
            "tekes-settings",
            TEXT,
            "Long descriptive item label that wraps cleanly",
        )
        .unwrap();
        assert_eq!(row.gap(), Spacing::Snug.value());
        assert_eq!(row.icon_id(), ICON);
        assert_eq!(row.text_id(), TEXT);

        let mut host = ContentHost::new(Content::new(ROOT, row)).unwrap();
        let mut text = WrapText;
        let mut icons = Icons;
        let mut renderer = WithIconRenderer::new(&mut text, &mut icons);

        let wide = Rect::new(0., 0., 400., 200.);
        host.render(
            &mut Scene::new(),
            &mut renderer,
            wide,
            wide,
            Theme::LIGHT.into(),
        )
        .unwrap();
        let wide_icon = host.get(ICON).unwrap().frame();
        let wide_text = host.get(TEXT).unwrap().frame();
        assert!(wide_icon.max_x() + Spacing::Snug.value() <= wide_text.min_x() + 0.01);
        let initial_mount = host.semantic_mount(TEXT).unwrap();

        // Decorative leading icon stays suppressed from semantics while text retains identity.
        let sem = host.semantics();
        assert!(sem.iter().all(|n| n.id != ICON));
        assert!(sem.iter().any(|n| n.id == TEXT));

        // Narrow allocation forces text wrapping within remaining horizontal budget.
        let narrow = Rect::new(0., 0., 110., 200.);
        host.set_inherited_surface("panel").unwrap();
        host.render(
            &mut Scene::new(),
            &mut renderer,
            narrow,
            narrow,
            Palette::from(Theme::DARK),
        )
        .unwrap();
        let narrow_icon = host.get(ICON).unwrap().frame();
        let narrow_text = host.get(TEXT).unwrap().frame();
        assert!(narrow_icon.max_x() + Spacing::Snug.value() <= narrow_text.min_x() + 0.01);
        assert!(narrow_text.max_x() <= narrow.max_x() + 0.01);
        assert!(narrow_text.size.height > wide_text.size.height);
        assert_eq!(host.semantic_mount(TEXT), Some(initial_mount));
    }
}
