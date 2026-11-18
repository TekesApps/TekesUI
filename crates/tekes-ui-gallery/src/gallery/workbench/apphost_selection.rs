//! Selection stories mount the original models in their production Content.
use super::{StoryState, apphost_basic::SPECIMEN, extended};
use tekes_ui_controls::{
    ButtonTextRenderer, ControlState, composition::*, layout::MeasuredChild,
    layout_view::RadioGroupView,
};
use tekes_ui_scene::{Rect, Scene, Size};

pub(super) fn supports(renderer: &str) -> bool {
    matches!(
        renderer,
        "switch"
            | "slider"
            | "radio-group"
            | "link"
            | "segmented-control"
            | "breadcrumb"
            | "toolbar"
            | "number-field"
            | "tabs"
            | "disclosure"
            | "command-palette"
            | "icon-button"
    )
}
#[derive(Debug)]
struct SelectionFixture {
    renderer: &'static str,
    name: &'static str,
    state: ControlState,
}
impl Component for SelectionFixture {
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        if matches!(self.renderer, "link" | "segmented-control") {
            let width = available
                .width
                .min(if self.renderer == "link" { 220. } else { 480. });
            let height = extended::matrix_control_height(
                self.renderer,
                &mut super::apphost_basic::FixtureText(text),
                width,
            );
            return MeasuredChild::new(Size::new(width, height));
        }
        let width = available.width.min(220.);
        let label = if self.renderer == "radio-group" {
            "Option"
        } else {
            "Enabled"
        };
        let measured = text.measure(
            label,
            tekes_ui_tokens::proposal::TextRole::UiBody,
            (width - 62.).max(0.),
        );
        MeasuredChild::new(Size::new(width, measured.height.max(40.) + 8.))
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        extended::paint_selection_fixture(
            self.renderer,
            (self.name, self.state),
            scene,
            &mut super::apphost_basic::FixtureText(text),
            frame,
            clip,
            theme.palette,
        );
    }
    fn inspection_properties(&self) -> Option<Vec<InspectionProperty>> {
        Some(vec![
            InspectionProperty::new("component", self.renderer),
            InspectionProperty::new("fixture", self.name),
            InspectionProperty::new("state", format!("{:?}", self.state)),
        ])
    }
}
pub(super) fn matrix(renderer: &'static str) -> Vec<Content> {
    if renderer == "icon-button" {
        return extended::matrix_states(renderer)
            .into_iter()
            .enumerate()
            .flat_map(|(i, (name, state))| {
                [
                    Content::new(ComponentId(122100 + i as u64 * 2), TextContent::new(name)),
                    Content::new(
                        ComponentId(122101 + i as u64 * 2),
                        super::apphost_actions::IconFixture::new(state),
                    ),
                ]
            })
            .collect();
    }

    if renderer == "command-palette" {
        return super::apphost_command::matrix();
    }
    if matches!(renderer, "tabs" | "disclosure") {
        return super::apphost_navigation::matrix(renderer);
    }
    if renderer == "breadcrumb" {
        return extended::navigation_states::apphost_breadcrumb_matrix();
    }
    if matches!(renderer, "toolbar" | "number-field" | "tabs") {
        return super::basic_states::apphost_matrix(renderer);
    }
    let content: Vec<Content> = extended::matrix_states(renderer)
        .into_iter()
        .enumerate()
        .flat_map(|(i, (name, state))| {
            [
                Content::new(ComponentId(92800 + i as u64 * 2), TextContent::new(name)),
                Content::new(
                    ComponentId(92801 + i as u64 * 2),
                    SelectionFixture {
                        renderer,
                        name,
                        state,
                    },
                ),
            ]
        })
        .collect();
    #[cfg(any(test, feature = "composition-review"))]
    let content = {
        let mut content = content;
        if renderer == "slider" {
            // Original independent fixture applies Increment then Right: 50 -> 60.
            content.push(tekes_ui_composition_consumer::slider_content::fixture_content(60.));
        }
        content
    };
    content
}
pub(super) fn playground(renderer: &str, state: &StoryState) -> Vec<Content> {
    if renderer == "command-palette" {
        return super::apphost_command::playground(state);
    }
    let disabled = disabled(renderer, state);
    let control = match renderer {
        "icon-button" => {
            let mut model = state.extended.icon_button_view.button().button.state;
            model.disabled = disabled;
            Content::new(SPECIMEN, super::apphost_actions::icon_button(model))
        }
        "tabs" | "disclosure" => super::apphost_navigation::playground(renderer, state),
        "number-field" => Content::new(
            SPECIMEN,
            super::apphost_actions::Stepper::new(state.number, disabled),
        ),
        "toolbar" => {
            let mut control = tekes_ui_controls::ToolbarControl::default();
            control.set_actions(
                ["Create", "Rename", "Remove"]
                    .into_iter()
                    .enumerate()
                    .map(|(i, title)| tekes_ui_controls::ToolbarAction {
                        id: 20 + i as u64,
                        title: title.into(),
                        enabled: !disabled,
                    })
                    .collect(),
            );
            Content::new(SPECIMEN, control)
        }
        "breadcrumb" => {
            let mut control = BreadcrumbContent::new(state.extended.breadcrumb.clone());
            control.set_disabled(disabled);
            Content::new(SPECIMEN, control)
        }
        "link" => {
            let mut control = LinkContent::new("Open documentation");
            control.model = state.extended.link.clone();
            control.model.button.state.disabled = disabled;
            Content::new(SPECIMEN, control)
        }
        "segmented-control" => {
            let mut control = SegmentedControlContent::from_model(
                "Calendar interval",
                state.extended.segmented.clone(),
                ["Day", "Week", "Month"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
            )
            .expect("original segment labels");
            control.set_disabled(disabled);
            Content::new(SPECIMEN, control)
        }
        "switch" => {
            let mut control = SwitchContent::new(
                if state.extended.switch.checked {
                    "Notifications on"
                } else {
                    "Notifications off"
                },
                state.extended.switch.checked,
            );
            control.model.disabled = disabled;
            Content::new(SPECIMEN, control)
        }
        "slider" => {
            let mut model = state.extended.slider;
            model.disabled = disabled;
            Content::new(SPECIMEN, SliderContent::new("Value", model))
        }
        "radio-group" => {
            let mut control = RadioGroupContent::from_original(
                "Options",
                state.extended.radio.clone(),
                RadioGroupView::new(),
                (1..=3).map(|i| format!("Option {i}")).collect(),
            )
            .expect("original three-option fixture");
            control.set_disabled(disabled);
            Content::new(SPECIMEN, control)
        }
        _ => unreachable!("selection story"),
    };
    let mut children = vec![
        Content::new(
            ComponentId(92110),
            Button::new(if disabled {
                "Disabled: on"
            } else {
                "Disabled: off"
            }),
        ),
        control,
    ];
    if renderer == "slider" {
        children.push(Content::new(
            ComponentId(92900),
            TextContent::new(format!("Value: {:.0}", state.extended.slider.value())),
        ));
        children.push(Content::new(
            ComponentId(92901),
            TextContent::new(
                "Arrow keys change by 5; Home / End move to bounds. Drag the track to set a value.",
            ),
        ));
    }
    children.push(Content::new(
        ComponentId(92003),
        TextContent::new(state.log.iter().cloned().collect::<Vec<_>>().join("\n")),
    ));
    children
}
/// Read changed values once after the formal Host visit; never replay a toggle
/// through the old Gallery reducer. Pointer drags need not emit Activate.
pub(super) fn sync_model(state: &mut StoryState, host: &mut ContentHost) {
    let Some(content) = host.get(SPECIMEN) else {
        return;
    };
    let mut message = None;
    if let Some(control) = content.component::<CommandPaletteContent>() {
        state.extended.command = control.model.clone();
    } else if let Some(control) = content.component::<TabsContent>() {
        if state.tabs_view.selected() != control.selected() {
            message = Some(format!("tab: {}", control.selected()));
        }
        state.tabs_view.select(control.selected());
    } else if let Some(control) = content.component::<super::apphost_actions::Stepper>() {
        if state.number.value() != control.model.value() {
            message = Some(format!("value: {}", control.model.value()));
        }
        state.number = control.model;
    } else if let Some(control) = content.component::<SegmentedControlContent>() {
        if state.extended.segmented.selected != control.model().selected {
            message = Some(format!("segment: {}", control.model().selected));
        }
        state.extended.segmented = control.model().clone();
    } else if let Some(control) = content.component::<LinkContent>() {
        state.extended.link = control.model.clone();
    } else if let Some(control) = content.component::<SwitchContent>() {
        if state.extended.switch.checked != control.model.checked {
            message = Some(format!("switch: {}", control.model.checked));
        }
        state.extended.switch = control.model;
    } else if let Some(control) = content.component::<SliderContent>() {
        if state.extended.slider.value() != control.model.value() {
            message = Some(format!("value: {}", control.model.value()));
        }
        state.extended.slider = control.model;
    } else if let Some(control) = content.component::<RadioGroupContent>() {
        if state.extended.radio.selected != control.model().selected {
            message = Some(format!("radio: {:?}", control.model().selected));
        }
        state.extended.radio = control.model().clone();
    }
    if let Some(message) = message {
        state.record(message);
        host.get_mut(ComponentId(92003))
            .unwrap()
            .component_mut::<TextContent>()
            .unwrap()
            .text = state.log.iter().cloned().collect::<Vec<_>>().join("\n");
        if let Some(value) = host.get_mut(ComponentId(92900)) {
            value.component_mut::<TextContent>().unwrap().text =
                format!("Value: {:.0}", state.extended.slider.value());
        }
        if let Some(control) = host
            .get_mut(SPECIMEN)
            .unwrap()
            .component_mut::<SwitchContent>()
        {
            control.label = if control.model.checked {
                "Notifications on"
            } else {
                "Notifications off"
            }
            .into();
        }
    }
}
pub(super) fn apply_disabled(host: &mut ContentHost, disabled: bool) {
    let content = host.get_mut(SPECIMEN).expect("mounted selection control");
    if let Some(control) = content.component_mut::<TabsContent>() {
        control.set_disabled(disabled);
    } else if let Some(control) = content.component_mut::<DisclosureContent>() {
        control.set_disabled(disabled);
    } else if let Some(control) = content.component_mut::<super::apphost_actions::Stepper>() {
        control.set_disabled(disabled);
    } else if let Some(control) = content.component_mut::<BreadcrumbContent>() {
        control.set_disabled(disabled);
    } else if let Some(control) = content.component_mut::<tekes_ui_controls::ToolbarControl>() {
        control.set_actions(
            ["Create", "Rename", "Remove"]
                .into_iter()
                .enumerate()
                .map(|(i, title)| tekes_ui_controls::ToolbarAction {
                    id: 20 + i as u64,
                    title: title.into(),
                    enabled: !disabled,
                })
                .collect(),
        );
    } else if let Some(control) = content.component_mut::<SegmentedControlContent>() {
        control.set_disabled(disabled);
    } else if let Some(control) = content.component_mut::<Button>() {
        control.button.cancel();
        control.button.state.disabled = disabled;
    } else if let Some(control) = content.component_mut::<LinkContent>() {
        control.event(&ContentEvent::Cancel);
        control.model.button.state.disabled = disabled;
    } else if let Some(control) = content.component_mut::<SwitchContent>() {
        control.event(&ContentEvent::Cancel);
        control.model.disabled = disabled;
    } else if let Some(control) = content.component_mut::<SliderContent>() {
        control.event(&ContentEvent::Cancel);
        control.model.disabled = disabled;
    } else if let Some(control) = content.component_mut::<RadioGroupContent>() {
        control.set_disabled(disabled);
    }
}

pub(super) fn disabled(renderer: &str, state: &StoryState) -> bool {
    if matches!(renderer, "toolbar" | "number-field" | "tabs") {
        state.disabled
    } else {
        state.extended.disabled
    }
}
