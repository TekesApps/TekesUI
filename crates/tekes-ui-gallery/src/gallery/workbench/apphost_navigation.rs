//! Original navigation stories and their independent content-status consumers.
use super::{
    StoryState,
    apphost_basic::{FixtureText, SPECIMEN},
    extended,
};
use tekes_ui_controls::{
    ButtonTextRenderer, ControlState, TabsView, composition::*, extended::Disclosure,
    layout::MeasuredChild, layout_view::DisclosureView,
};
use tekes_ui_scene::{Rect, Scene, Size};
use tekes_ui_tokens::{Palette, proposal::TextRole};
pub(super) const TRIGGER: ComponentId = ComponentId(101001);
pub(super) const TAB_TITLES: [&str; 5] = [
    "Selected first",
    "Selected second · focus-visible",
    "Pressed selected tab",
    "Disabled tabs",
    "Selected tab · hover",
];

pub(super) fn playground(renderer: &str, state: &StoryState) -> Content {
    if renderer == "tabs" {
        let items = ["General", "Security", "Advanced"]
            .into_iter()
            .enumerate()
            .map(|(i, name)| {
                TabItem::new(
                    Content::new(ComponentId(101010 + i as u64 * 2), TextContent::new(name)),
                    Content::new(
                        ComponentId(101011 + i as u64 * 2),
                        TextContent::new(format!("{name} settings panel")),
                    ),
                )
            })
            .collect();
        let mut control = TabsContent::new(items, state.tabs_view.selected());
        control.set_disabled(state.disabled);
        Content::new(SPECIMEN, control)
    } else {
        let mut control = DisclosureContent::new(
            TRIGGER,
            Content::new(ComponentId(101002), TextContent::new("Details")),
            Content::new(
                ComponentId(101003),
                TextContent::new(
                    "Disclosure content is mounted only while open. State remains local to this story.",
                ),
            ),
            state.extended.disclosure.open,
        );
        control.set_disabled(state.extended.disabled);
        Content::new(SPECIMEN, control)
    }
}

// Shared by the legacy matrix and the mounted passive specimens.
pub(super) fn paint_tabs_case(
    n: usize,
    scene: &mut Scene,
    text: &mut impl ButtonTextRenderer,
    body: Rect,
    clip: Rect,
    palette: Palette,
) {
    let mut view = TabsView::new(3);
    if n == 1 {
        view.focus(1, true);
    }
    view.set_disabled(n == 3);
    let frame = Rect::new(body.min_x(), body.min_y(), body.size.width, 36.);
    let layout = view.render(
        &mut Scene::new(),
        text,
        &["General", "Security", "Advanced"],
        frame,
        Some(clip),
        palette,
        TextRole::UiBody,
        false,
    );
    if n == 4 {
        view.pointer_move_at(layout.items[0].frame.center());
    }
    if n == 2 {
        view.pointer_down_at(layout.items[0].frame.center());
    }
    let layout = view.render(
        scene,
        text,
        &["General", "Security", "Advanced"],
        frame,
        Some(clip),
        palette,
        TextRole::UiBody,
        false,
    );
    text.draw(
        scene,
        ["General panel", "Security panel", "Advanced panel"][view.selected()],
        TextRole::UiBody,
        Rect::new(
            body.min_x(),
            layout.frame.max_y() + 8.,
            body.size.width,
            48.,
        ),
        tekes_ui_controls::ButtonTextAlign::Leading,
        palette.color("primary-text").unwrap(),
    );
}
pub(super) fn paint_disclosure_case(
    state: ControlState,
    scene: &mut Scene,
    text: &mut impl ButtonTextRenderer,
    frame: Rect,
    clip: Rect,
    palette: Palette,
) -> (DisclosureView, Disclosure) {
    let model = Disclosure {
        open: state.selected,
        disabled: state.disabled,
    };
    let mut view = DisclosureView::default();
    if state.focused {
        view.focus(state.focus_visible);
    }
    let layout = view.render(
        &mut Scene::new(),
        text,
        &model,
        "Details",
        frame,
        Some(clip),
        palette,
        TextRole::UiBody,
    );
    if state.hovered {
        view.pointer_move(layout.frame.center());
    }
    if state.pressed {
        view.pointer_down(layout.frame.center());
    }
    view.render(
        scene,
        text,
        &model,
        "Details",
        frame,
        Some(clip),
        palette,
        TextRole::UiBody,
    );
    (view, model)
}
#[derive(Debug)]
struct Fixture {
    tab: Option<usize>,
    state: ControlState,
    icon: IconContent,
}
impl Component for Fixture {
    fn requires_icon_services(&self) -> bool {
        self.tab.is_none()
    }
    fn icon_error(&self) -> Option<IconRenderError> {
        self.icon.icon_error()
    }
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        let height = if self.tab.is_some() {
            100.
        } else {
            extended::matrix_control_height("disclosure", &mut FixtureText(text), available.width)
        };
        MeasuredChild::new(Size::new(available.width, height))
    }
    fn paint(
        &mut self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        frame: Rect,
        clip: Rect,
        theme: ContentTheme,
    ) {
        if let Some(n) = self.tab {
            paint_tabs_case(n, scene, &mut FixtureText(text), frame, clip, theme.palette);
        } else {
            let (view, model) = paint_disclosure_case(
                self.state,
                scene,
                &mut FixtureText(text),
                frame,
                clip,
                theme.palette,
            );
            self.icon
                .set_glyph(if model.open {
                    "tekes-expand"
                } else {
                    "tekes-forward"
                })
                .expect("canonical disclosure glyph");
            let mut theme = theme;
            theme.icon_foreground = Some(
                theme
                    .palette
                    .color(if self.state.disabled {
                        "tertiary-text"
                    } else {
                        "primary-text"
                    })
                    .unwrap(),
            );
            self.icon.paint(scene, text, view.icon_frame(), clip, theme);
        }
    }
}
pub(super) fn matrix(renderer: &str) -> Vec<Content> {
    let cases = if renderer == "tabs" {
        TAB_TITLES
            .into_iter()
            .map(|s| (s, ControlState::default()))
            .collect()
    } else {
        extended::matrix_states("disclosure")
    };
    let rows: Vec<Content> = cases
        .into_iter()
        .enumerate()
        .flat_map(|(i, (title, state))| {
            [
                Content::new(ComponentId(101100 + i as u64 * 2), TextContent::new(title)),
                Content::new(
                    ComponentId(101101 + i as u64 * 2),
                    Fixture {
                        tab: (renderer == "tabs").then_some(i),
                        state,
                        icon: IconContent::new("tekes-forward")
                            .expect("canonical disclosure glyph"),
                    },
                ),
            ]
        })
        .collect();
    #[cfg(any(test, feature = "composition-review"))]
    let rows = {
        let mut rows = rows;
        use std::{cell::Cell, rc::Rc};
        use tekes_ui_composition_consumer::content_status::{Kind, fixture_content};
        for (i, (status, focus, title)) in super::content_status::CASES.into_iter().enumerate() {
            let base = 102000 + i as u64 * 10;
            rows.push(Content::new(ComponentId(base), TextContent::new(title)));
            rows.push(fixture_content(
                if renderer == "tabs" {
                    Kind::Tabs
                } else {
                    Kind::Disclosure
                },
                std::array::from_fn(|n| ComponentId(base + 1 + n as u64)),
                Rc::new(Cell::new(status)),
                Rc::new(Cell::new(true)),
                Rc::new(Cell::new(0)),
                renderer == "disclosure",
                focus,
            ));
        }
        rows
    };
    rows
}
