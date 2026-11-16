//! Original command model and finite visual cases, using the formal Content owner.
use super::{
    StoryState,
    apphost_basic::{FixtureText, SPECIMEN},
};
use tekes_ui_controls::{
    ButtonTextRenderer, ControlKey,
    composition::*,
    extended::{ComboBox, CommandPalette},
    layout::MeasuredChild,
};
use tekes_ui_scene::{Rect, Scene, Size};
pub(super) const QUERY: ComponentId = ComponentId(121001);
pub(super) const CASES: [(&str, &str, bool, bool); 9] = [
    ("Closed · default", "", false, false),
    ("Closed · hover", "", false, false),
    ("Closed · pressed", "", false, false),
    ("Closed · keyboard focus", "", false, false),
    ("Selected · last executed command", "", false, false),
    ("Open · available commands", "", true, false),
    ("Open · filtered commands", "open", true, false),
    ("Open · no results", "no-such-command", true, false),
    ("Disabled", "", false, true),
];
pub(super) fn playground(state: &StoryState) -> Vec<Content> {
    vec![
        Content::new(
            SPECIMEN,
            CommandPaletteContent::new(
                Content::new(
                    ComponentId(121002),
                    TextContent::new("Open command palette"),
                ),
                Content::new(
                    QUERY,
                    TextFieldContent::new("Command query", &state.extended.command.combo.query),
                ),
                state.extended.command.clone(),
            ),
        ),
        Content::new(
            ComponentId(92003),
            TextContent::new(state.log.iter().cloned().collect::<Vec<_>>().join("\n")),
        ),
    ]
}
#[derive(Debug)]
struct Fixture {
    control: CommandPaletteContent,
    title: &'static str,
    open: bool,
}
impl Component for Fixture {
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, a: Size) -> MeasuredChild {
        let size = self
            .control
            .measure(&mut FixtureText(text), Size::new(a.width.min(420.), 400.))
            .size;
        MeasuredChild::new(Size::new(
            size.width,
            if self.open {
                400.
            } else if self.title == "Selected · last executed command" {
                size.height + 44.
            } else {
                size.height
            },
        ))
    }
    fn arrange(&mut self, text: &mut dyn ButtonTextRenderer, f: Rect, c: Rect) {
        self.control.arrange(&mut FixtureText(text), f, c);
    }
    fn paint(
        &mut self,
        s: &mut Scene,
        t: &mut dyn ButtonTextRenderer,
        f: Rect,
        c: Rect,
        theme: ContentTheme,
    ) {
        if matches!(
            self.title,
            "Closed · hover" | "Closed · pressed" | "Disabled"
        ) {
            self.control
                .paint(&mut Scene::new(), &mut FixtureText(t), f, c, theme);
            let p = tekes_ui_scene::Point::new(f.min_x() + 20., f.min_y() + 18.);
            self.control.event(&ContentEvent::PointerMove(p));
            if self.title == "Closed · pressed" {
                self.control.event(&ContentEvent::PointerDown(p));
            }
        }
        self.control.paint(s, &mut FixtureText(t), f, c, theme);
        self.control.view.options.set_reduced_motion(true);
        self.control.paint_overlay(
            s,
            &mut FixtureText(t),
            f,
            c.intersection(&f).unwrap_or_default(),
            theme,
        );
        let message = if self.title == "Selected · last executed command" {
            Some(format!("Last command: {}", self.control.model.combo.query))
        } else if self.open && self.control.model.combo.filtered().is_empty() {
            Some("No matching commands".into())
        } else {
            None
        };
        if let Some(message) = message {
            t.draw(
                s,
                &message,
                tekes_ui_tokens::proposal::TextRole::UiBody,
                Rect::new(f.min_x(), f.max_y() - 36., f.size.width, 36.),
                tekes_ui_controls::ButtonTextAlign::Leading,
                theme.palette.color("primary-text").unwrap(),
            );
        }
    }
}
pub(super) fn matrix() -> Vec<Content> {
    let rows: Vec<_> = CASES
        .into_iter()
        .enumerate()
        .flat_map(|(i, (title, query, open, disabled))| {
            let mut model = CommandPalette {
                combo: ComboBox {
                    items: [
                        "Open recent file",
                        "Open settings",
                        "Create workspace",
                        "Close window",
                    ]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
                    disabled,
                    ..Default::default()
                },
            };
            let mut view = tekes_ui_controls::layout_view::CommandPaletteView::default();
            if open {
                view.open(&mut model);
                model.combo.set_query(query.to_owned());
            }
            if title == "Selected · last executed command" {
                view.open(&mut model);
                view.key(ControlKey::Down, true, &mut model);
                view.key(ControlKey::Enter, true, &mut model);
            }
            let mut control = CommandPaletteContent::new(
                Content::new(
                    ComponentId(121100 + i as u64 * 10),
                    TextContent::new("Open command palette"),
                ),
                Content::new(
                    ComponentId(121101 + i as u64 * 10),
                    TextFieldContent::new("Command query", &model.combo.query),
                ),
                model,
            );
            control.view = view;
            if title == "Closed · keyboard focus" {
                control.set_focus(true, true);
            }
            [
                Content::new(ComponentId(121102 + i as u64 * 10), TextContent::new(title)),
                Content::new(
                    ComponentId(121103 + i as u64 * 10),
                    Fixture {
                        control,
                        title,
                        open,
                    },
                ),
            ]
        })
        .collect();
    #[cfg(any(test, feature = "composition-review"))]
    let rows = {
        use std::{cell::Cell, rc::Rc};
        use tekes_ui_composition_consumer::{
            command_status::fixture_content, content_status::Status,
        };
        let mut rows = rows;
        for (i, (status, cached, title)) in [
            (
                Status::Loading,
                true,
                "Loading commands · retained query consumer",
            ),
            (
                Status::Error,
                true,
                "Refresh failed · query and commands retained",
            ),
            (
                Status::Loading,
                false,
                "Initial command lookup · no cached results",
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let base = 121300 + i as u64 * 10;
            rows.push(Content::new(ComponentId(base), TextContent::new(title)));
            rows.push(fixture_content(
                [
                    ComponentId(base + 1),
                    ComponentId(base + 2),
                    ComponentId(base + 3),
                ],
                if cached {
                    vec![
                        "Open file".into(),
                        "Open settings".into(),
                        "Close window".into(),
                    ]
                } else {
                    vec![]
                },
                Rc::new(Cell::new(status)),
                Some("Open"),
            ));
        }
        rows
    };
    rows
}
