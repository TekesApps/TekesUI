//! Opt-in observations of the original live component. Reading never renders,
//! drains actions, advances motion, or changes focus. Lookup values and emitted
//! quads are separate evidence: a lookup does not prove pixel consumption.
use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InspectionProperty {
    pub name: &'static str,
    pub value: String,
}
impl InspectionProperty {
    pub fn new(name: &'static str, value: impl std::fmt::Display) -> Self {
        Self {
            name,
            value: value.to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct InspectionGeometry {
    pub frame: Rect,
    pub rectangular_clip: Rect,
    pub visible: Option<Rect>,
    /// A rectangular intersection is not the complete curve clip.
    pub has_curve_geometry: bool,
}

#[derive(Clone, Debug)]
pub struct InspectionPaint {
    pub theme: ContentTheme,
    pub tokens: tekes_ui_tokens::audit::TokenObservation,
    /// Actual emitted quads, before GPU composition. Text/images are not sampled.
    pub quads: Vec<tekes_ui_scene::Quad>,
    pub includes_owned_children: bool,
}

#[derive(Clone, Debug)]
pub struct InspectionSnapshot {
    pub id: ComponentId,
    pub mount: u64,
    pub semantics: ContentSemantics,
    pub focused: bool,
    /// None means layout is absent, failed, or invalidated since the last render.
    pub geometry: Option<InspectionGeometry>,
    /// None means the component has not implemented the optional property hook.
    pub properties: Option<Vec<InspectionProperty>>,
    /// Historical paint evidence; it is not recomputed from current model state.
    pub last_paint: Option<InspectionPaint>,
}

impl ContentHost {
    /// Enable recording only for an explicitly inspected mounted node. Disabling
    /// drops the cached observation. This configuration requests normal repaint.
    pub fn set_inspection_enabled(&mut self, id: ComponentId, enabled: bool) -> bool {
        let Some(node) = self.root.find_mut(id) else {
            return false;
        };
        if node.inspection_enabled == enabled {
            return false;
        }
        node.inspection_enabled = enabled;
        if !enabled {
            node.inspection_paint = None;
        }
        self.dirty = true;
        true
    }

    pub fn inspection_snapshot(&self, id: ComponentId) -> Option<InspectionSnapshot> {
        let node = self.root.find(id)?;
        let layout_current = self.last_layout.is_some()
            && !self.dirty
            && !self.reveal_clock.pending
            && self.reveal_clock.stamp().ok() == self.last_render_stamp;
        Some(InspectionSnapshot {
            id,
            mount: node.mount,
            semantics: node.component.semantics(),
            focused: self.focused == Some(id),
            geometry: layout_current.then(|| InspectionGeometry {
                frame: node.frame,
                rectangular_clip: node.clip,
                visible: node.visible(),
                has_curve_geometry: node.curve_geometry.is_some(),
            }),
            properties: node.component.inspection_properties(),
            last_paint: node.inspection_paint.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tekes_ui_scene::Primitive;

    struct Text;
    impl ButtonTextRenderer for Text {
        fn measure(&mut self, _: &str, role: proposal::TextRole, width: f32) -> Size {
            Size::new(60_f32.min(width), role.style().line_height)
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            _: &str,
            _: proposal::TextRole,
            _: Rect,
            _: ButtonTextAlign,
            _: tekes_ui_tokens::Rgba,
        ) {
        }
    }
    const ID: ComponentId = ComponentId(72);
    const FRAME: Rect = Rect::new(0., 0., 180., 80.);
    fn button_host() -> ContentHost {
        let mut button = Button::new("Apply");
        button.variant = ButtonVariant::Primary;
        ContentHost::new(Content::new(ID, button)).unwrap()
    }
    fn render(host: &mut ContentHost, theme: tekes_ui_tokens::Theme) -> Scene {
        let mut scene = Scene::new();
        host.render(&mut scene, &mut Text, FRAME, FRAME, Palette::from(theme))
            .unwrap();
        scene
    }

    #[test]
    fn inspection_observes_original_paint_without_repainting_or_relabeling_stale_state() {
        let mut host = button_host();
        render(&mut host, tekes_ui_tokens::Theme::LIGHT);
        assert!(host.inspection_snapshot(ID).unwrap().last_paint.is_none());
        assert!(host.set_inspection_enabled(ID, true));
        let scene = render(&mut host, tekes_ui_tokens::Theme::DARK);
        let snapshot = host.inspection_snapshot(ID).unwrap();
        assert!(snapshot.geometry.is_some());
        let paint = snapshot.last_paint.unwrap();
        let emitted: Vec<_> = scene
            .primitives()
            .iter()
            .filter_map(|p| match p {
                Primitive::Quad(q) | Primitive::KeyedQuad { quad: q, .. } => Some(*q),
                _ => None,
            })
            .collect();
        assert!(!paint.quads.is_empty());
        assert_eq!(paint.quads, emitted);
        assert_eq!(paint.theme.palette.theme, tekes_ui_tokens::Theme::DARK);
        assert!(!paint.tokens.colors.is_empty());
        for read in &paint.tokens.colors {
            assert_eq!(read.palette.color(&read.role), Some(read.value));
        }
        host.get_mut(ID)
            .unwrap()
            .component_mut::<Button>()
            .unwrap()
            .label = "Changed".into();
        let changed = host.inspection_snapshot(ID).unwrap();
        assert!(changed.geometry.is_none());
        assert!(
            changed
                .properties
                .unwrap()
                .iter()
                .any(|p| p.name == "label" && p.value == "Changed")
        );
        assert_eq!(changed.last_paint.unwrap().quads, paint.quads);
        host.set_inspection_enabled(ID, false);
        assert!(host.inspection_snapshot(ID).unwrap().last_paint.is_none());
    }

    #[test]
    fn inspection_reads_do_not_release_pressed_button_or_consume_activation() {
        let mut host = button_host();
        render(&mut host, tekes_ui_tokens::Theme::LIGHT);
        let point = host.get(ID).unwrap().frame().center();
        host.event(&ContentEvent::PointerDown(point));
        assert!(
            host.get(ID)
                .unwrap()
                .component::<Button>()
                .unwrap()
                .button
                .state
                .pressed
        );
        for _ in 0..3 {
            assert!(host.inspection_snapshot(ID).unwrap().focused);
        }
        assert!(
            host.get(ID)
                .unwrap()
                .component::<Button>()
                .unwrap()
                .button
                .state
                .pressed
        );
        host.event(&ContentEvent::PointerUp(point));
        assert_eq!(
            host.get(ID)
                .unwrap()
                .component::<Button>()
                .unwrap()
                .activations,
            1
        );
    }

    #[test]
    fn inspection_reused_id_does_not_inherit_retired_mount_observations() {
        let root = ComponentId(71);
        let mut host = ContentHost::new(Content::new(
            root,
            ListContent::new("Items", vec![Content::new(ID, Button::new("Old"))]),
        ))
        .unwrap();
        host.set_inspection_enabled(ID, true);
        render(&mut host, tekes_ui_tokens::Theme::LIGHT);
        let old = host.inspection_snapshot(ID).unwrap();
        assert!(old.last_paint.is_some());
        host.remove(ID).unwrap();
        assert!(host.inspection_snapshot(ID).is_none());
        host.insert(root, Content::new(ID, Button::new("New")))
            .unwrap();
        let new = host.inspection_snapshot(ID).unwrap();
        assert_ne!(old.mount, new.mount);
        assert!(new.geometry.is_none());
        assert!(new.last_paint.is_none());
    }
}
