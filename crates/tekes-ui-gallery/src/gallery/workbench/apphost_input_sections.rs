//! Passive input chapters borrow the live Host's text service. Each fixture
//! keeps the production TextSurface or Select/Combo view, without a native owner.
use super::{Entry, apphost_basic::FixtureText};
use tekes_ui_controls::layout::MeasuredChild;
use tekes_ui_controls::{
    ButtonTextRenderer, ControlState,
    composition::*,
    extended::{Axis, Stack},
};
use tekes_ui_scene::{Rect, Scene, Size};

fn text(id: u64, value: impl Into<String>) -> Content {
    Content::new(ComponentId(id), TextContent::new(value))
}
pub(super) fn body(entry: &'static Entry, section: usize) -> Content {
    let mut children = vec![text(
        91200,
        [
            "Docs",
            "Playground",
            "State matrix",
            "Behaviors",
            "Edge cases",
            "Variants / sizes",
        ][section],
    )];
    match section {
        0 => {
            children.extend([
                text(91201, super::story_docs(entry)),
                text(91202, format!("Component: {}\nUsage: {}", entry.component(), super::api_reference(entry))),
                text(91203, format!("Story routes: {}", ["docs", "playground", "states", "behaviors", "edge-cases", "variants"].into_iter().filter(|s| entry.supports_story(s)).map(|s| format!("{}/{s}", entry.slug())).collect::<Vec<_>>().join(" · "))),
                text(91204, "State is local to this route. Reset restores deterministic fixture data. Visual state fixtures do not replace native input, keyboard or accessibility acceptance."),
            ]);
            children.extend(super::contract_summary(entry).into_iter().enumerate().map(|(i, line)| text(91210 + i as u64, line)));
        }
        2 => {
            children.push(text(91201, "Visual fixtures only — simulated focus is not a focus test. Real interaction: Playground."));
            children.extend(matrix(entry.renderer()));
        }
        3 => children.push(text(91201, if super::extended::supports(entry.renderer()) { super::extended::behavior_copy(entry.renderer()) } else { super::behavior_copy(entry.renderer()) })),
        5 => children.push(text(91201, match entry.renderer() {
            "area" => "Multiline plain-text variant; fills the remaining preview. TextEditor remains an internal adapter.",
            "field" => "Single-line TextInput. Native IME, selection, clipboard and undo use the shared production model.",
            _ => "One supported variant. Use the responsive preview widths to inspect constrained containers.",
        })),
        _ => unreachable!("live input chapters retain the original subtree"),
    }
    Content::new(
        ComponentId(91007),
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 16.,
            },
            flexible_child: None,
            children,
        },
    )
}
fn matrix(renderer: &str) -> Vec<Content> {
    let mut children = Vec::new();
    if matches!(renderer, "field" | "area" | "search") {
        let mut states: Vec<_> = super::apphost_basic::standard_states()
            .into_iter()
            .filter(|(name, _)| *name != "loading")
            .collect();
        states.extend(["read-only", "composing", "selection"].map(|name| {
            (
                name,
                ControlState {
                    focused: true,
                    ..Default::default()
                },
            )
        }));
        for (i, (name, state)) in states.into_iter().enumerate() {
            children.push(text(91300 + i as u64 * 2, name));
            children.push(Content::new(
                ComponentId(91301 + i as u64 * 2),
                TextFixture::new(renderer == "area", name, state),
            ));
        }
    } else {
        // Production navigation views retain their original open/filtered/menu
        // models. The fixtures themselves deliberately have no event owner.
        for (i, name) in if renderer == "select" {
            SELECT_CASES.as_slice()
        } else {
            COMBO_CASES.as_slice()
        }
        .iter()
        .enumerate()
        {
            children.push(text(91300 + i as u64 * 2, *name));
            children.push(Content::new(
                ComponentId(91301 + i as u64 * 2),
                NavigationFixture {
                    select: renderer == "select",
                    name,
                    trigger: Rect::default(),
                    height: 0.,
                    field: TextFixture::new(false, name, ControlState::default()),
                },
            ));
        }
    }
    #[cfg(any(test, feature = "composition-review"))]
    if matches!(renderer, "combo-box" | "select") {
        use std::{cell::Cell, rc::Rc};
        use tekes_ui_composition_consumer::content_status::Status;
        let cases: &[(Status, bool, &str)] = if renderer == "combo-box" {
            &[
                (
                    Status::Loading,
                    false,
                    "Loading options · retained query consumer",
                ),
                (
                    Status::Error,
                    false,
                    "Refresh failed · input stays editable",
                ),
                (
                    Status::Ready,
                    true,
                    "Validation error · separate from refresh failure",
                ),
            ]
        } else {
            &[
                (
                    Status::Loading,
                    false,
                    "Loading options · independent consumer",
                ),
                (
                    Status::Error,
                    false,
                    "Refresh failed · cached options remain usable",
                ),
            ]
        };
        for (i, (status, validation, label)) in cases.iter().enumerate() {
            let base = 91400 + i as u64 * 10;
            children.push(text(base, *label));
            let ids = [
                ComponentId(base + 1),
                ComponentId(base + 2),
                ComponentId(base + 3),
            ];
            let labels = ["Automatic", "Compact", "Comfortable"]
                .into_iter()
                .map(str::to_owned)
                .collect();
            let status = Rc::new(Cell::new(*status));
            children.push(if renderer == "combo-box" {
                tekes_ui_composition_consumer::combo_status::fixture_content(
                    ids,
                    labels,
                    status,
                    validation.then(|| "Choose a permitted layout.".into()),
                    Some("Com"),
                )
            } else {
                tekes_ui_composition_consumer::choice_status::fixture_content(ids, labels, status)
            });
        }
    }
    children
}
#[derive(Debug)]
pub(super) struct TextFixture {
    field: TextFieldContent,
    name: &'static str,
    state: ControlState,
}
impl TextFixture {
    pub(super) fn new(multiline: bool, name: &'static str, state: ControlState) -> Self {
        let value = if multiline {
            "A first complete sentence for the editor.\nA second line keeps the text layout visible.\nA third line gives the multiline control room to breathe."
        } else if name == "default" {
            "Studio"
        } else {
            "Sample text"
        };
        let mut field = TextFieldContent::new("Visual input fixture", value);
        field.show_metadata = false;
        field.viewport = TextViewport::Fixed(if multiline { 140. } else { 40. });
        field.surface = tekes_ui_controls::TextSurface::new(value, multiline, name == "read-only");
        field.surface.input.disabled = state.disabled;
        field.field.error = state.error.then(|| "A name is required.".into());
        if matches!(name, "selection" | "composing") {
            field.surface.input.select_range(0..6);
        }
        if name == "composing" {
            field.surface.input.preedit("输入", 2..2, None);
            field.surface.clauses = vec![0..1, 1..2];
        }
        field.set_focus(state.focused, state.focused);
        Self { field, name, state }
    }
}
impl Component for TextFixture {
    fn inspection_properties(&self) -> Option<Vec<InspectionProperty>> {
        self.field.inspection_properties()
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        self.field.measure(text, available)
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        self.field.arrange(text, frame, clip);
        if self.name == "pressed" {
            self.field.surface.pointer_down(
                self.field.surface.inner.center(),
                false,
                1,
                tekes_ui_tokens::proposal::metric("size", "size-row-height").unwrap(),
            );
        }
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        _: Rect,
        _: Rect,
        theme: ContentTheme,
    ) {
        if let Some(host) = text.text_surface_host() {
            self.field
                .surface
                .draw(host, scene, theme.palette, self.state.focused, true);
        }
    }
}
const SELECT_CASES: [&str; 9] = [
    "Default selection",
    "Hover",
    "Pressed",
    "Keyboard focus",
    "Disabled select",
    "Selected · Compact",
    "Open options",
    "Validation error · selection remains editable",
    "Empty option set · no menu",
];
const COMBO_CASES: [&str; 9] = [
    "Default",
    "Hover",
    "Pressed",
    "Keyboard focus",
    "Open · candidates",
    "Open · filtered",
    "Open · no results",
    "Selected",
    "Disabled",
];
#[derive(Debug)]
struct NavigationFixture {
    select: bool,
    name: &'static str,
    trigger: Rect,
    height: f32,
    field: TextFixture,
}
impl NavigationFixture {
    fn labels(&self) -> Vec<&'static str> {
        if self.name == "Empty option set · no menu" {
            vec![]
        } else if self.select {
            vec!["Automatic", "Compact", "Comfortable"]
        } else {
            vec!["Automatic", "Comfortable", "Compact", "Follow system"]
        }
    }
    fn feedback(&self) -> Option<&'static str> {
        match self.name {
            "Validation error · selection remains editable" => {
                Some("This choice is unavailable. Choose another option.")
            }
            "Empty option set · no menu" => Some("No options available."),
            "Open · no results" => Some("No matching options"),
            _ => None,
        }
    }
    fn combo(&self) -> tekes_ui_controls::extended::ComboBox {
        let mut model = tekes_ui_controls::extended::ComboBox {
            items: self.labels().into_iter().map(str::to_owned).collect(),
            ..Default::default()
        };
        model.query = match self.name {
            "Open · filtered" => "omf",
            "Open · no results" => "not-found",
            "Selected" => "Comfortable",
            _ => "",
        }
        .into();
        model.open = self.name.starts_with("Open");
        model.disabled = self.name == "Disabled";
        model.active = model.filtered().first().copied();
        model.selected = (self.name == "Selected").then_some(1);
        model
    }
}
impl Component for NavigationFixture {
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        let width = available.width.min(440.);
        let mut text = FixtureText(text);
        let labels = self.labels();
        let trigger_height = if self.select {
            tekes_ui_controls::ButtonModel::measure(
                &mut text,
                labels.first().copied().unwrap_or(""),
                width.min(230.),
                tekes_ui_controls::ButtonSize::Body,
                tekes_ui_tokens::proposal::TextRole::UiBody,
            )
            .height
        } else {
            self.field
                .measure(&mut text, Size::new(width, available.height))
                .size
                .height
        };
        let menu_labels: Vec<&str> = if self.select {
            labels
        } else {
            let model = self.combo();
            model
                .filtered()
                .into_iter()
                .map(|i| self.labels()[i])
                .collect()
        };
        let extra = if self.name.starts_with("Open") && !menu_labels.is_empty() {
            tekes_ui_controls::MenuView::measure(
                &mut text,
                &menu_labels,
                width,
                tekes_ui_tokens::proposal::TextRole::UiBody,
                false,
            )
            .height
                + 8.
        } else if let Some(message) = self.feedback() {
            8. + text
                .measure(message, tekes_ui_tokens::proposal::TextRole::UiBody, width)
                .height
        } else {
            22.
        };
        self.height = trigger_height;
        MeasuredChild::new(Size::new(width, trigger_height + extra))
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, frame: Rect, clip: Rect) {
        self.trigger = Rect::new(
            frame.min_x(),
            frame.min_y(),
            frame.size.width.min(if self.select { 230. } else { 440. }),
            self.height,
        );
        if !self.select {
            let model = self.combo();
            let state = ControlState {
                focused: matches!(self.name, "Open · candidates" | "Keyboard focus"),
                disabled: model.disabled,
                ..Default::default()
            };
            self.field = TextFixture::new(
                false,
                if self.name == "Pressed" {
                    "pressed"
                } else {
                    "default"
                },
                state,
            );
            let length = self.field.field.surface.input.len();
            self.field
                .field
                .surface
                .input
                .insert(&model.query, Some(0..length));
            self.field.arrange(text, self.trigger, clip);
        }
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        let mut text = FixtureText(text);
        if self.select {
            let labels = self.labels();
            let mut model = tekes_ui_controls::Select::new(labels.len());
            if self.name == "Selected · Compact" {
                model.choose(1);
            }
            let mut view = tekes_ui_controls::layout_view::SelectView::default();
            view.set_reduced_motion(true);
            view.set_disabled(self.name == "Disabled select", &mut model);
            view.set_error(self.name.starts_with("Validation"));
            view.render(
                &mut Scene::new(),
                &mut text,
                &mut model,
                &labels,
                self.trigger,
                frame,
                None,
                theme.palette,
                tekes_ui_tokens::proposal::TextRole::UiBody,
                false,
            );
            match self.name {
                "Hover" => view.pointer_move(self.trigger.center(), &mut model),
                "Pressed" => {
                    view.pointer_down(self.trigger.center(), &mut model);
                }
                "Keyboard focus" => view.focus(true),
                "Open options" => {
                    view.focus(true);
                    view.key(tekes_ui_controls::ControlKey::Enter, true, &mut model);
                }
                _ => {}
            }
            view.render(
                scene,
                &mut text,
                &mut model,
                &labels,
                self.trigger,
                frame,
                Some(clip),
                theme.palette,
                tekes_ui_tokens::proposal::TextRole::UiBody,
                false,
            );
        } else {
            self.field
                .paint(scene, &mut text, self.trigger, clip, theme);
            let mut model = self.combo();
            tekes_ui_controls::layout_view::ComboBoxView::default().render(
                scene,
                &mut text,
                &mut model,
                self.trigger,
                frame,
                Some(clip),
                theme.palette,
                tekes_ui_tokens::proposal::TextRole::UiBody,
                false,
            );
        }
        if let Some(message) = self.feedback() {
            scene.push_clip(clip);
            text.draw(
                scene,
                message,
                tekes_ui_tokens::proposal::TextRole::UiBody,
                Rect::new(
                    self.trigger.min_x(),
                    self.trigger.max_y() + 8.,
                    frame.size.width,
                    (frame.max_y() - self.trigger.max_y() - 8.).max(0.),
                ),
                tekes_ui_controls::ButtonTextAlign::default(),
                theme
                    .palette
                    .color("primary-text")
                    .expect("registered body ink"),
            );
            scene.pop_clip();
        }
    }
}
