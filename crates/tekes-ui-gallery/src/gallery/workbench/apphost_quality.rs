//! Original quality references in the receiving ContentHost, not new control demos.
use crate::gallery::{catalog, pages::showcase};
use tekes_ui_controls::{ButtonTextRenderer, composition::*, layout::MeasuredChild};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{Rgba, Theme, proposal::TextRole};

pub(crate) const PREVIEW: ComponentId = ComponentId(100020);
const GROUP_ACTION: u64 = 100100;
pub(crate) const GROUPS: [(&str, &str, &str); 8] = [
    ("components-foundations", "foundations", "Layout specimens"),
    ("components-actions", "actions", "Action specimens"),
    ("components-forms", "forms", "Input specimens"),
    (
        "components-navigation",
        "navigation",
        "Navigation specimens",
    ),
    ("components-overlays", "overlays", "Overlay specimens"),
    ("components-feedback", "feedback", "Feedback specimens"),
    ("components-documents", "documents", "Document specimens"),
    ("components-data", "data", "Data specimens"),
];
pub(crate) fn category_for_slug(slug: &str) -> Option<&'static str> {
    GROUPS.iter().find(|g| g.0 == slug).map(|g| g.1)
}
pub(crate) fn destination(id: ComponentId) -> Option<&'static str> {
    GROUPS
        .get(id.0.checked_sub(GROUP_ACTION)? as usize)
        .map(|g| g.0)
}
pub(crate) fn index_host(entry: &'static super::Entry) -> anyhow::Result<ContentHost> {
    let mut children = vec![super::story_heading(100001, entry.title())];
    children.extend(
        GROUPS
            .iter()
            .enumerate()
            .map(|(i, g)| Content::new(ComponentId(GROUP_ACTION + i as u64), Button::new(g.2))),
    );
    let mut host = ContentHost::new(Content::new(
        ComponentId(100000),
        ListContent::scrollable("Cross-component consistency", ComponentId(100002), children),
    ))
    .map_err(|e| anyhow::anyhow!("quality index: {e:?}"))?;
    host.set_action_events(true);
    Ok(host)
}

struct HostText<'a>(&'a mut dyn ButtonTextRenderer);
impl HostText<'_> {
    fn paint(
        &mut self,
        scene: &mut Scene,
        slot: Rect,
        value: &str,
        role: TextRole,
        ink: Rgba,
        wrapped: bool,
    ) {
        let host = self
            .0
            .text_surface_host()
            .expect("quality text services checked by Host");
        let paragraph = host.shape_surface(value, role, wrapped.then_some(slot.size.width));
        let origin = if wrapped {
            Point::new(slot.min_x().round(), slot.min_y().round())
        } else {
            Point::new(
                slot.min_x().round(),
                (slot.center().y - paragraph.height as f32 / 2.).round(),
            )
        };
        scene.push_clip(slot);
        host.push_visible(scene, &paragraph, origin, ink, slot);
        scene.pop_clip();
    }
}
impl showcase::ShowcaseText for HostText<'_> {
    fn proposal_label(
        &mut self,
        value: &str,
        role: TextRole,
        width: Option<f32>,
    ) -> std::rc::Rc<tekes_ui_text::Paragraph> {
        self.0
            .text_surface_host()
            .expect("quality text services checked by Host")
            .shape_surface(value, role, width)
    }
    fn proposal_selected_in(
        &mut self,
        scene: &mut Scene,
        theme: Theme,
        slot: Rect,
        value: &str,
        role: TextRole,
        wrapped: bool,
    ) {
        let host = self
            .0
            .text_surface_host()
            .expect("quality text services checked by Host");
        let paragraph = host.shape_surface(value, role, wrapped.then_some(slot.size.width));
        let origin = if wrapped {
            slot.origin
        } else {
            crate::gallery::text::label_origin(slot, &paragraph, crate::gallery::text::HAlign::Left)
        };
        scene.push_clip(slot);
        for rect in
            paragraph.selection_rects(0..paragraph.utf16_len, slot.offset(-origin.x, -origin.y))
        {
            scene.push_quad(tekes_ui_scene::Quad::new(
                rect.offset(origin.x, origin.y),
                tekes_ui_tokens::proposal::render_color("text-selection", theme).unwrap(),
            ));
        }
        host.push_visible(
            scene,
            &paragraph,
            origin,
            tekes_ui_tokens::proposal::render_color("selected-text", theme).unwrap(),
            slot,
        );
        scene.pop_clip();
    }
    fn label(
        &mut self,
        scene: &mut Scene,
        _theme: Theme,
        slot: Rect,
        value: &str,
        role: TextRole,
        ink: Rgba,
    ) {
        self.paint(scene, slot, value, role, ink, false);
    }
    fn wrapped(
        &mut self,
        scene: &mut Scene,
        _theme: Theme,
        slot: Rect,
        value: &str,
        role: TextRole,
        ink: Rgba,
    ) {
        self.paint(scene, slot, value, role, ink, true);
    }
}
#[derive(Debug)]
struct Showcase {
    category: &'static str,
    state: catalog::ShowcaseState,
    frame: Rect,
    clip: Rect,
}
impl Showcase {
    fn choose(&mut self, component: &str, state: &str) -> ContentResponse {
        ContentResponse::consumed(self.state.select(component, state))
    }
}
impl Component for Showcase {
    fn requires_text_services(&self) -> bool {
        true
    }
    fn interactive(&self) -> bool {
        true
    }
    fn rectangular_input_geometry(&self) -> bool {
        true
    }
    fn measure(&mut self, _text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(
            available.width,
            crate::gallery::layout::PAGE_CONTENT.size.height,
        ))
    }
    fn arrange(&mut self, _text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        self.frame = frame;
        self.clip = clip;
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        scene.push_clip(clip);
        showcase::build_with_text(
            self.category,
            theme.palette.theme,
            &mut HostText(text),
            frame,
            scene,
            &self.state,
        );
        scene.pop_clip();
    }
    fn event(&mut self, event: &ContentEvent) -> ContentResponse {
        if let ContentEvent::PointerDown(point) = event
            && self.clip.contains(*point)
            && let Some((component, state)) = showcase::hit_state(self.category, self.frame, *point)
        {
            return self.choose(component, state);
        }
        ContentResponse::default()
    }
    fn semantics(&self) -> ContentSemantics {
        let mut semantics =
            ContentSemantics::new(ContentRole::Group, &catalog::group(self.category).name);
        semantics.description = Some(
            "Original fixed state illustrations; selecting a state is not live control acceptance"
                .into(),
        );
        semantics
    }
    fn semantic_parts(&self) -> Vec<SemanticPart> {
        showcase::state_parts(self.category, self.frame)
            .into_iter()
            .enumerate()
            .map(|(index, (id, state, frame))| {
                let component = catalog::component(id);
                let mut semantics = ContentSemantics::new(
                    ContentRole::Button,
                    format!("{}: {state}", component.name),
                );
                semantics.selected = self.state.selected(component) == state;
                SemanticPart {
                    index,
                    frame,
                    focused: false,
                    semantics,
                }
            })
            .collect()
    }
    fn accessibility_action(
        &mut self,
        part: Option<usize>,
        action: &AccessibilityAction,
    ) -> ContentResponse {
        if action == &AccessibilityAction::Press
            && let Some((id, state, _)) = part.and_then(|i| {
                showcase::state_parts(self.category, self.frame)
                    .get(i)
                    .copied()
            })
        {
            return self.choose(id, state);
        }
        ContentResponse::default()
    }
    fn inspection_properties(&self) -> Option<Vec<InspectionProperty>> {
        let mut properties = vec![InspectionProperty::new(
            "evidence_scope",
            "static reference illustrations",
        )];
        properties.extend(
            catalog::group(self.category)
                .component_ids
                .iter()
                .map(|id| {
                    let component = catalog::component(id);
                    InspectionProperty::new(component.id.as_str(), self.state.selected(component))
                }),
        );
        Some(properties)
    }
}
pub(crate) fn host_for(category: &'static str) -> anyhow::Result<ContentHost> {
    ContentHost::new(Content::new(
        ComponentId(100000),
        ListContent::scrollable(
            &catalog::group(category).name,
            ComponentId(100002),
            vec![
                super::story_heading(100001, &catalog::group(category).name),
                Content::new(
                    PREVIEW,
                    Showcase {
                        category,
                        state: catalog::ShowcaseState::default(),
                        frame: Rect::default(),
                        clip: Rect::default(),
                    },
                ),
            ],
        ),
    ))
    .map_err(|e| anyhow::anyhow!("quality group: {e:?}"))
}

#[derive(Debug)]
struct Geometry;
impl Component for Geometry {
    fn requires_text_services(&self) -> bool {
        true
    }
    fn measure(&mut self, _: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        MeasuredChild::new(Size::new(available.width, 448.))
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        let backing = text.backing_scale();
        let palette = theme.palette;
        scene.push_clip(clip);
        crate::gallery::pages::primitives::build_calibration_with_labels(
            palette.theme,
            backing,
            frame,
            scene,
            |scene, slot, value| {
                HostText(text).paint(
                    scene,
                    slot,
                    value,
                    TextRole::UiBody,
                    tekes_ui_tokens::ColorRole::PrimaryText.resolve(palette.theme),
                    false,
                );
            },
        );
        scene.pop_clip();
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::Group, "Raster alignment and alpha calibration")
    }
}
pub(crate) fn geometry_host(entry: &'static super::Entry) -> anyhow::Result<ContentHost> {
    ContentHost::new(Content::new(
        ComponentId(100000),
        ListContent::scrollable(
            entry.title(),
            ComponentId(100002),
            vec![
                super::story_heading(100001, entry.title()),
                Content::new(PREVIEW, Geometry),
            ],
        ),
    ))
    .map_err(|e| anyhow::anyhow!("geometry calibration: {e:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gallery::text::TextEngine;
    #[test]
    fn apphost_quality_preserves_all_original_groups_and_state_selection() {
        let mut text = TextEngine::new();
        let mut count = 0;
        for (_, category, _) in GROUPS {
            let mut host = host_for(category).unwrap();
            let frame = Rect::new(0., 0., 932., 850.);
            host.render(
                &mut Scene::new(),
                &mut text,
                frame,
                frame,
                Theme::LIGHT.into(),
            )
            .unwrap();
            let mut parts = host
                .get(PREVIEW)
                .unwrap()
                .component::<Showcase>()
                .unwrap()
                .semantic_parts();
            for id in &catalog::group(category).component_ids {
                count += 1;
                let component = catalog::component(id);
                let state = component.states.last().unwrap();
                let part = parts
                    .iter()
                    .find(|p| p.semantics.name == format!("{}: {state}", component.name))
                    .unwrap()
                    .index;
                assert!(
                    host.accessibility_action(PREVIEW, Some(part), AccessibilityAction::Press)
                        .consumed
                );
                host.render(
                    &mut Scene::new(),
                    &mut text,
                    frame,
                    frame,
                    Theme::LIGHT.into(),
                )
                .unwrap();
                assert_eq!(
                    host.get(PREVIEW)
                        .unwrap()
                        .component::<Showcase>()
                        .unwrap()
                        .state
                        .selected(component),
                    state
                );
                parts = host
                    .get(PREVIEW)
                    .unwrap()
                    .component::<Showcase>()
                    .unwrap()
                    .semantic_parts();
            }
            let off = host.event(&ContentEvent::PointerDown(Point::new(-20., -20.)));
            assert!(!off.consumed);
        }
        assert_eq!(count, 40);
    }
}
