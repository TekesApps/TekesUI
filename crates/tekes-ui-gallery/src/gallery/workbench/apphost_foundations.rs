//! Original foundation data in the receiving AppHost tree and text service.
use super::{Entry, StoryState};
use crate::gallery::containers::ComponentState;
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, composition::*, layout::MeasuredChild,
};
use tekes_ui_scene::{Quad, Rect, Scene, Size};
use tekes_ui_tokens::{Alpha, ColorRole, CornerRadius, Rgba, Spacing, Stroke, proposal::TextRole};

pub(super) const SEMANTIC_COLORS: [&str; 20] = [
    "surface-0",
    "surface-1",
    "surface-2",
    "surface-popover",
    "primary-text",
    "secondary-text",
    "border",
    "border-focused",
    "row-selection",
    "row-selection-hover",
    "action-primary-ground",
    "action-primary-hover",
    "action-primary-pressed",
    "action-primary-foreground",
    "control-checked-ground",
    "control-checked-mark",
    "text-selection",
    "status-failed",
    "status-succeeded",
    "warning",
];
const ROOT: ComponentId = ComponentId(97800);
const ROWS: ComponentId = ComponentId(97801);
const BODY: ComponentId = ComponentId(97804);
const SPECIMEN: u64 = 98000;
const CLEAR: Rgba = Rgba::new(0., 0., 0., 0.);
#[derive(Debug, Clone, Copy)]
enum Sample {
    Semantic(&'static str),
    Color(ColorRole),
    Spacing(Spacing, f32),
    Radius(CornerRadius),
    Stroke(Stroke),
    Alpha(Alpha),
    State(ComponentState),
}
impl Sample {
    fn label(self) -> String {
        match self {
            Self::Semantic(role) => role.into(),
            Self::Color(role) => super::super::pages::tokens::role_label(role),
            Self::Spacing(value, _) => format!("{value:?}: {} pt", value.value()),
            Self::Radius(value) => format!("{value:?}: {}", value.value()),
            Self::Stroke(value) => format!("{value:?}: {} pt", value.value()),
            Self::Alpha(value) => format!("{value:?}"),
            Self::State(value) => value.name().into(),
        }
    }
}
#[derive(Debug)]
struct SampleContent(Sample, Option<tekes_ui_tokens::Palette>);
impl Component for SampleContent {
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        let label = text.measure(&self.0.label(), TextRole::UiCaption, available.width);
        MeasuredChild::new(Size::new(available.width, 64. + label.height))
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        self.1 = Some(theme.palette);
        scene.push_clip(clip);
        let sample = Rect::new(frame.min_x(), frame.min_y(), frame.size.width, 56.);
        let palette = theme.palette;
        let color = |role| palette.color(role).expect("registered foundation token");
        match self.0 {
            Sample::Semantic(role) => scene.push_quad(
                Quad::new(sample, color(role))
                    .with_corner_radius(6.)
                    .with_border(1., color("border")),
            ),
            Sample::Color(role) => {
                let radius = CornerRadius::Row.value();
                let border = ColorRole::FloatingPanelBorder.resolve(palette.theme);
                scene.push_quad(
                    Quad::new(sample, ColorRole::DocumentCanvas.resolve(palette.theme))
                        .with_corner_radius(radius)
                        .with_border(Stroke::Hairline.value(), border),
                );
                scene.push_quad(
                    Quad::new(
                        sample.inset(Spacing::Tight.value()),
                        role.resolve(palette.theme),
                    )
                    .with_corner_radius(CornerRadius::concentric(radius, Spacing::Tight.value()))
                    .with_border(Stroke::Hairline.value(), border),
                );
            }
            Sample::Spacing(value, multiplier) => scene.push_quad(Quad::new(
                Rect::new(
                    sample.min_x(),
                    sample.min_y(),
                    (value.value() * multiplier).max(Stroke::Hairline.value()),
                    24.,
                ),
                color("row-selection"),
            )),
            Sample::Radius(value) => scene.push_quad(
                Quad::new(sample, color("surface-2"))
                    .with_corner_radius(value.value())
                    .with_border(Stroke::Rule.value(), color("border")),
            ),
            Sample::Stroke(value) => {
                let box_width = sample.size.width * 0.5;
                scene.push_quad(
                    Quad::new(
                        Rect::new(
                            sample.min_x(),
                            sample.min_y(),
                            box_width,
                            sample.size.height,
                        ),
                        CLEAR,
                    )
                    .with_corner_radius(CornerRadius::Row.value())
                    .with_border(value.value(), color("border-focused")),
                );
                scene.push_quad(Quad::new(
                    Rect::new(
                        sample.min_x() + box_width + 12.,
                        sample.center().y,
                        (box_width - 12.).max(0.),
                        value.value(),
                    ),
                    color("primary-text"),
                ));
            }
            Sample::Alpha(alpha) => {
                let a = alpha.resolve(palette.theme.contrast);
                let width = sample.size.width.min(108.);
                scene.push_quad(
                    Quad::new(
                        Rect::new(sample.min_x(), sample.min_y(), width * 2. / 3., 56.),
                        ColorRole::Accent.resolve(palette.theme).with_alpha(a),
                    )
                    .with_corner_radius(CornerRadius::Row.value()),
                );
                scene.push_quad(
                    Quad::new(
                        Rect::new(
                            sample.min_x() + width / 3.,
                            sample.min_y() + 8.,
                            width * 2. / 3.,
                            40.,
                        ),
                        ColorRole::StatusFailed.resolve(palette.theme).with_alpha(a),
                    )
                    .with_corner_radius(CornerRadius::Row.value()),
                );
            }
            Sample::State(state) => {
                super::super::pages::primitives::button_sample(scene, sample, state, palette.theme)
            }
        }
        let label = match self.0 {
            Sample::Alpha(a) => format!("{a:?}: {}", a.resolve(palette.theme.contrast)),
            _ => self.0.label(),
        };
        text.draw(
            scene,
            &label,
            TextRole::UiCaption,
            Rect::new(
                frame.min_x(),
                frame.min_y() + 64.,
                frame.size.width,
                (frame.size.height - 64.).max(0.),
            ),
            ButtonTextAlign::default(),
            color("secondary-text"),
        );
        scene.pop_clip();
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(ContentRole::Group, self.0.label())
    }
    fn inspection_properties(&self) -> Option<Vec<InspectionProperty>> {
        let mut values = vec![InspectionProperty::new("sample", self.0.label())];
        if let Some(palette) = self.1 {
            let value = match self.0 {
                Sample::Semantic(role) => {
                    format!("{:?}", palette.color(role).expect("registered token"))
                }
                Sample::Color(role) => format!("{:?}", role.resolve(palette.theme)),
                Sample::Spacing(value, multiplier) => {
                    format!("{} pt; specimen multiplier {multiplier}", value.value())
                }
                Sample::Radius(value) => format!("{} pt", value.value()),
                Sample::Stroke(value) => format!("{} pt", value.value()),
                Sample::Alpha(value) => value.resolve(palette.theme.contrast).to_string(),
                Sample::State(value) => format!(
                    "{}; passive visual specimen, not an interactive control",
                    value.name()
                ),
            };
            values.push(InspectionProperty::new("painted_value", value));
            values.push(InspectionProperty::new(
                "painted_theme",
                format!("{:?}", palette.theme),
            ));
        }
        Some(values)
    }
}
fn text(id: u64, copy: impl Into<String>) -> Content {
    Content::new(ComponentId(id), TextContent::new(copy))
}
fn grid(id: u64, samples: impl IntoIterator<Item = Sample>) -> Content {
    Content::new(
        ComponentId(id),
        ResponsiveGridContent::new(
            1,
            Spacing::Relaxed.value(),
            Spacing::Loose.value(),
            samples
                .into_iter()
                .enumerate()
                .map(|(n, s)| Content::new(ComponentId(id + 1 + n as u64), SampleContent(s, None)))
                .collect(),
        )
        .with_breakpoints(vec![(600., 4), (260., 2)]),
    )
}
fn section(group_id: u64, heading_id: u64, heading: &str, content: Content) -> Content {
    StackContent::titled_section(
        ComponentId(group_id),
        ComponentId(heading_id),
        heading,
        content,
    )
}
fn spacing(atlas: bool) -> Content {
    grid(
        SPECIMEN + 100,
        Spacing::ALL
            .into_iter()
            .map(|s| Sample::Spacing(s, if atlas { 8. } else { 1. })),
    )
}
fn radii() -> Content {
    grid(
        SPECIMEN + 200,
        CornerRadius::ALL.into_iter().map(Sample::Radius),
    )
}
fn strokes() -> Content {
    grid(
        SPECIMEN + 300,
        [Stroke::Hairline, Stroke::Rule, Stroke::Emphasis]
            .into_iter()
            .map(Sample::Stroke),
    )
}

pub(crate) struct AppHostFoundations {
    pub(crate) host: ContentHost,
    state: StoryState,
    entry: &'static Entry,
}
impl AppHostFoundations {
    pub(crate) fn new(entry: &'static Entry) -> anyhow::Result<Self> {
        let state = StoryState::for_entry(entry);
        let intro = Content::new(
            ComponentId(97806),
            StackContent {
                model: tekes_ui_controls::extended::Stack {
                    axis: tekes_ui_controls::extended::Axis::Vertical,
                    gap: StackContent::SECTION_HEADER_GAP,
                },
                flexible_child: None,
                children: vec![
                    super::story_heading(97802, entry.title()),
                    text(97803, super::story_docs(entry)),
                ],
            },
        );
        let mut children = vec![intro];
        if entry.renderer() == "tokens" {
            children.push(Content::new(
                ComponentId(97805),
                SegmentedControlContent::from_model(
                    "Token view",
                    {
                        let mut m = tekes_ui_controls::extended::SegmentedControl::new(2);
                        m.selected = state.selected;
                        m
                    },
                    vec!["Semantic colors".into(), "Complete token atlas".into()],
                )
                .unwrap(),
            ));
        }
        children.push(Self::body(entry, state.selected));
        let mut host = ContentHost::new(Content::new(
            ROOT,
            ScrollContent::document(StackContent::document_sections(ROWS, children)),
        ))
        .map_err(|e| anyhow::anyhow!("foundation host: {e:?}"))?;
        host.set_action_events(true);
        Ok(Self { host, state, entry })
    }
    fn body(entry: &Entry, selected: usize) -> Content {
        let mut children = vec![];
        match entry.renderer() {
            "tokens" if selected == 1 => {
                children.push(section(
                    97910,
                    97900,
                    "颜色角色（ColorRole::ALL 顺序）",
                    grid(SPECIMEN, ColorRole::ALL.into_iter().map(Sample::Color)),
                ));
                children.push(section(
                    97911,
                    97901,
                    "间距 Spacing（条长 = 值 x 8）",
                    spacing(true),
                ));
                children.push(section(97912, 97902, "圆角 CornerRadius", radii()));
                children.push(section(97913, 97903, "描边 Stroke", strokes()));
            }
            "tokens" => {
                children.push(grid(
                    SPECIMEN,
                    SEMANTIC_COLORS.into_iter().map(Sample::Semantic),
                ));
                children.push(section(
                    97910,
                    97900,
                    "Opacity & compositing",
                    grid(
                        SPECIMEN + 400,
                        [Alpha::Faint, Alpha::Soft, Alpha::Wash]
                            .into_iter()
                            .map(Sample::Alpha),
                    ),
                ));
                children.push(text(97901, "Alpha is applied once during compositing. Overlap shows the resulting blend. These washes are decorative; preserve semantic text, control boundaries and focus contrast."));
            }
            "typography" => {
                for (i, (label, role)) in [
                    ("Heading — Aa", TextRole::UiHeading),
                    ("Body — The quick brown fox", TextRole::UiBody),
                    ("Caption — 0123456789", TextRole::UiCaption),
                    ("Composer — Natural paragraphs", TextRole::ComposerBody),
                ].into_iter().enumerate() {
                    let mut content = TextContent::new(label);
                    content.role = role;
                    children.push(Content::new(ComponentId(SPECIMEN + i as u64), content));
                }
            }
            "iconography" => children.push(text(97900, "Icon rules: use the documented 12 / 16 / 24 pt sizes, align optical centers, and pair semantic icons with readable labels. Open Components / Content / Icon for actual vector specimens.")),
            "spacing" => children.push(spacing(false)),
            "shape" => {
                children.push(radii());
                children.push(section(97910, 97900, "Stroke widths & boundary roles", strokes()));
                children.push(text(97901, "Required control boundary: border / stroke-rule (at least 3:1). Decorative frame: separator / 1 physical pixel. Focus: border-focused. Hairline 0.5 pt is the 2x specimen; Increase Contrast uses stroke-rule for decorative frames."));
                children.push(text(97902, "Elevation uses semantic surfaces and layer order. Shadows are not implemented in the current scene primitive; no shadow acceptance is claimed."));
            }
            "accessibility" => {
                children.push(text(97900, "Interaction states: rest, hover, pressed, disabled and focus-visible. Selection and checked states belong to each component contract. Disabled controls do not activate; show focus-visible for keyboard navigation. A visual matrix does not prove real keyboard focus.\n\nKeyboard rules are component-specific. Dialogs contain focus and restore the invoker. Manual checks remain: VoiceOver role/name/value, physical IME, system contrast and assistive input."));
                children.push(grid(
                    SPECIMEN,
                    ComponentState::ALL.into_iter().map(Sample::State),
                ));
            }
            "a11y" => children.push(text(97900, "Keyboard contracts are component-specific. Focus-visible follows keyboard input. Disabled controls do not activate. Dialogs contain focus and restore the invoker.\n\nManual checks still required: VoiceOver role/name/value, physical IME, system contrast, physical keyboard traversal and assistive input.")),
            "stress" => children.push(text(97900, "Use TEKES_PERF=scroll:60 and the existing measurement scripts. Actual presentation receipts determine 120 Hz acceptance. Build/test success does not prove 8.33 ms. Long-run isolated drops remain unresolved.\n\nText and ScrollArea pages provide real large-document scrolling and resize fixtures.")),
            _ => unreachable!("foundation reference route"),
        }
        StackContent::document_sections(BODY, children)
    }
    pub(crate) fn after_host_visit(&mut self) {
        for record in self.host.drain_action_records() {
            if self.entry.renderer() == "tokens"
                && record.action.kind == ContentActionKind::Activate
                && record.action.target == ComponentId(97805)
                && let Some(ContentActionPayload::SegmentSelection(intent)) = &record.payload
                && self.state.selected != intent.index()
            {
                self.state.activate(400 + intent.index(), "tokens");
                self.host.remove(BODY).expect("foundation body");
                self.host
                    .insert(ROWS, Self::body(self.entry, self.state.selected))
                    .expect("unique foundation body");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gallery::text::TextEngine;
    use tekes_ui_tokens::Theme;
    #[test]
    fn apphost_foundation_atlas_switch_preserves_original_choice_and_all_samples() {
        let entry = super::super::entries()
            .iter()
            .find(|e| e.renderer() == "tokens")
            .unwrap();
        let mut page = AppHostFoundations::new(entry).unwrap();
        let mut text = TextEngine::new();
        let frame = Rect::new(0., 0., 280., 700.);
        let render = |page: &mut AppHostFoundations, text: &mut TextEngine| {
            let mut scene = Scene::new();
            page.host
                .render(&mut scene, text, frame, frame, Theme::LIGHT.into())
                .unwrap();
            scene
        };
        render(&mut page, &mut text);
        assert_eq!(
            page.host
                .get(ComponentId(SPECIMEN + 20))
                .unwrap()
                .component::<SampleContent>()
                .unwrap()
                .0
                .label(),
            "warning"
        );
        let atlas_part = page
            .host
            .get(ComponentId(97805))
            .unwrap()
            .component::<SegmentedControlContent>()
            .unwrap()
            .part_token(1);
        assert!(
            page.host
                .accessibility_action(ComponentId(97805), atlas_part, AccessibilityAction::Press)
                .consumed
        );
        page.after_host_visit();
        render(&mut page, &mut text);
        assert_eq!(page.state.selected, 1);
        for i in 0..ColorRole::ALL.len() {
            let node = page.host.get(ComponentId(SPECIMEN + 1 + i as u64)).unwrap();
            assert_eq!(
                node.component::<SampleContent>().unwrap().0.label(),
                super::super::super::pages::tokens::role_label(ColorRole::ALL[i])
            );
            assert!(node.frame().max_x() <= frame.max_x());
        }
        assert!(
            page.host
                .get(ComponentId(SPECIMEN + 100 + Spacing::ALL.len() as u64))
                .is_some()
        );
        assert!(
            page.host
                .get(ComponentId(SPECIMEN + 200 + CornerRadius::ALL.len() as u64))
                .is_some()
        );
        assert!(page.host.get(ComponentId(SPECIMEN + 303)).is_some());
        // Switching back is the same original model action, not a new route or
        // replacement native host. The first semantic specimen is restored.
        let semantic_part = page
            .host
            .get(ComponentId(97805))
            .unwrap()
            .component::<SegmentedControlContent>()
            .unwrap()
            .part_token(0);
        assert!(
            page.host
                .accessibility_action(
                    ComponentId(97805),
                    semantic_part,
                    AccessibilityAction::Press
                )
                .consumed
        );
        page.after_host_visit();
        render(&mut page, &mut text);
        assert_eq!(page.state.selected, 0);
        assert_eq!(
            page.host
                .get(ComponentId(SPECIMEN + 1))
                .unwrap()
                .component::<SampleContent>()
                .unwrap()
                .0
                .label(),
            "surface-0"
        );
    }
    #[test]
    fn apphost_foundation_references_wrap_at_large_type_without_losing_specimens() {
        let mut text = TextEngine::new();
        text.set_type_scale(1.5);
        for renderer in ["spacing", "shape", "accessibility"] {
            let entry = super::super::entries()
                .iter()
                .find(|e| e.renderer() == renderer)
                .unwrap();
            let mut page = AppHostFoundations::new(entry).unwrap();
            for theme in [Theme::LIGHT, Theme::DARK] {
                let frame = Rect::new(0., 0., 280., 480.);
                page.host
                    .render(&mut Scene::new(), &mut text, frame, frame, theme.into())
                    .unwrap();
                let grid_id = match renderer {
                    "spacing" => SPECIMEN + 100,
                    "shape" => SPECIMEN + 200,
                    _ => SPECIMEN,
                };
                let grid = page
                    .host
                    .get(ComponentId(grid_id))
                    .unwrap()
                    .component::<ResponsiveGridContent>()
                    .unwrap();
                assert_eq!(grid.columns(), 1);
                for pair in grid.children().windows(2) {
                    assert!(pair[1].frame().min_y() >= pair[0].frame().max_y());
                    assert!(pair[0].frame().max_x() <= frame.max_x());
                }
            }
        }
    }
}
