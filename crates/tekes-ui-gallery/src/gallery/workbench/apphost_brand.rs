//! The original brand candidate model and editor in the receiving AppHost.
use super::brand_preview::{BrandPreview, ROLE_SAMPLES, parse_hex};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, composition::*, layout::MeasuredChild,
};
use tekes_ui_scene::{Quad, Rect, Scene, Size};
use tekes_ui_tokens::{Palette, proposal};

const CURRENT: ComponentId = ComponentId(120010);
const CANDIDATE: ComponentId = ComponentId(120020);
const INPUT: ComponentId = ComponentId(120025);
const ACTION: ComponentId = ComponentId(120026);
const OPEN: ComponentId = ComponentId(120027);
const CONFIRM: ComponentId = ComponentId(120028);
const CANCEL: ComponentId = ComponentId(120029);
const COUNT: ComponentId = ComponentId(120030);
const DIALOG: ComponentId = ComponentId(120040);
const CLOSE: ComponentId = ComponentId(120044);
const DIRECTION: u64 = 120100;
fn text(id: u64, value: impl Into<String>) -> Content {
    Content::new(ComponentId(id), TextContent::new(value))
}
fn primary(label: &str) -> Button {
    Button::new(label).with_variant(tekes_ui_controls::ButtonVariant::Primary)
}
fn card(id: ComponentId, children: Vec<Content>) -> Content {
    Content::new(
        id,
        SurfaceContent::panel(Content::new(
            ComponentId(id.0 + 1000),
            StackContent::vertical(12., children),
        )),
    )
}

#[derive(Debug)]
struct Roles {
    model: Rc<RefCell<BrandPreview>>,
    applied: Rc<Cell<Palette>>,
    candidate: bool,
}
impl Roles {
    fn palette(&self) -> Palette {
        let applied = self.applied.get();
        if self.candidate {
            self.model
                .borrow()
                .candidate()
                .map(|i| {
                    crate::gallery::brands::brand_palette(applied.theme, i).expect("compiled brand")
                })
                .unwrap_or(applied)
        } else {
            applied
        }
    }
    fn seed(&self) -> Option<&'static str> {
        self.candidate
            .then(|| self.model.borrow().candidate())
            .flatten()
            .and_then(crate::gallery::brands::gallery_brand_preset)
            .map(|p| p.seed_srgb)
    }
    fn rows(
        &self,
        palette: Palette,
    ) -> Vec<(
        String,
        String,
        tekes_ui_tokens::Rgba,
        Option<tekes_ui_tokens::Rgba>,
    )> {
        let mut rows = ROLE_SAMPLES
            .iter()
            .map(|(label, fg, bg)| {
                (
                    label.to_string(),
                    format!("{label} · {fg} / {bg}"),
                    palette.color(bg).expect("compiled role"),
                    Some(palette.color(fg).expect("compiled role")),
                )
            })
            .collect::<Vec<_>>();
        if let Some(seed) = self.seed() {
            rows.push((
                String::new(),
                format!("Brand seed {seed} (reference only)"),
                parse_hex(seed).expect("compiled seed"),
                None,
            ));
        }
        rows
    }
}
impl Component for Roles {
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        let h = self
            .rows(self.palette())
            .iter()
            .map(|(_, caption, _, _)| {
                42. + text
                    .measure(caption, proposal::TextRole::UiCaption, available.width)
                    .height
            })
            .sum();
        MeasuredChild::new(Size::new(available.width, h))
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
        let mut y = frame.min_y();
        for (label, caption, bg, fg) in self.rows(theme.palette()) {
            let sample = Rect::new(frame.min_x(), y, frame.size.width, 30.);
            let (width, ink) = tekes_ui_controls::feedback_view::SurfaceView::decorative_border(
                theme.palette(),
                text.backing_scale(),
            );
            scene.push_quad(
                Quad::new(sample, bg)
                    .with_corner_radius(proposal::metric("radius", "radius-control").unwrap())
                    .with_border(width, ink),
            );
            if let Some(fg) = fg {
                text.draw(
                    scene,
                    &label,
                    proposal::TextRole::UiCaption,
                    sample.inset(4.),
                    ButtonTextAlign::Leading,
                    fg,
                );
            }
            let h = text
                .measure(&caption, proposal::TextRole::UiCaption, frame.size.width)
                .height;
            text.draw(
                scene,
                &caption,
                proposal::TextRole::UiCaption,
                Rect::new(frame.min_x(), y + 34., frame.size.width, h),
                ButtonTextAlign::Leading,
                theme.palette().color("secondary-text").expect("text role"),
            );
            y += h + 42.;
        }
        scene.pop_clip();
    }
}

pub(crate) struct BrandStory {
    pub(crate) host: ContentHost,
    model: Rc<RefCell<BrandPreview>>,
    palette: Rc<Cell<Palette>>,
}
impl BrandStory {
    pub(crate) fn new(palette: Rc<Cell<Palette>>) -> Self {
        let mut original = BrandPreview::default();
        let mut input = original.take_apphost_input();
        input.viewport = TextViewport::Fixed(40.);
        let model = Rc::new(RefCell::new(original));
        let roles = |id, candidate| {
            Content::new(
                ComponentId(id),
                Roles {
                    model: model.clone(),
                    applied: palette.clone(),
                    candidate,
                },
            )
        };
        let current = card(
            CURRENT,
            vec![
                text(120011, "Currently applied"),
                text(120012, ""),
                roles(120013, false),
            ],
        );
        let candidate = card(
            CANDIDATE,
            vec![
                text(120021, "Current palette preview"),
                text(120022, ""),
                roles(120023, true),
                Content::new(ACTION, primary("Preview primary action")),
                text(120024, "Real text input · click and type"),
                Content::new(INPUT, input),
                Content::new(OPEN, Button::new("Open candidate-themed dialog")),
                Content::new(CONFIRM, primary("Confirm use")),
                Content::new(CANCEL, Button::new("Cancel")),
                text(COUNT.0, "0 interactive preview activations"),
            ],
        );
        let dialog = Content::new(
            DIALOG,
            DialogContent::new(
                "Candidate palette preview",
                text(120041, "Candidate palette preview"),
                text(
                    120042,
                    "Preview the compiler-generated candidate before applying it.",
                ),
                Content::new(CLOSE, Button::new("Close")),
            ),
        );
        let directions = crate::gallery::brands::gallery_brand_directions()
            .iter()
            .enumerate()
            .map(|(i, d)| Content::new(ComponentId(DIRECTION + i as u64), Button::new(d.name)))
            .collect();
        let brand_scope = |id: u64, child: Content| {
            let model = model.clone();
            Content::new(
                ComponentId(id),
                ThemeScopeContent::with_resolver(
                    move |mut palette| {
                        if let Some(index) = model.borrow().candidate() {
                            palette = crate::gallery::brands::brand_palette(palette.theme, index)
                                .expect("compiled brand");
                        }
                        palette
                    },
                    child,
                ),
            )
        };
        let candidate = brand_scope(120050, candidate);
        let dialog = brand_scope(120051, dialog);
        let intro = Content::new(
            ComponentId(120006),
            StackContent::vertical(
                StackContent::SECTION_HEADER_GAP,
                vec![
                    Content::new(
                        ComponentId(120002),
                        TextContent::heading("Choose a hue direction"),
                    ),
                    text(
                        120003,
                        "Selecting a direction prepares a compiler-generated candidate. It does not change the app palette; only Confirm use applies it globally.",
                    ),
                ],
            ),
        );
        let document = Content::new(
            ComponentId(120060),
            ScrollContent::document(StackContent::document_sections(
                ComponentId(120001),
                vec![
                    intro,
                    Content::new(
                        ComponentId(120004),
                        ResponsiveGridContent::new(2, 12., 12., directions)
                            .with_breakpoints(vec![(480., 4)]),
                    ),
                    Content::new(
                        ComponentId(120005),
                        ResponsiveGridContent::new(1, 12., 12., vec![current, candidate])
                            .with_breakpoints(vec![(720., 2)]),
                    ),
                ],
            )),
        );
        let host = ContentHost::new(super::apphost_layout::dialog_viewport(
            ComponentId(120000),
            vec![document, dialog],
        ))
        .expect("unique brand content IDs");
        let mut story = Self {
            host,
            model,
            palette,
        };
        story.host.set_action_events(true);
        story.refresh();
        story
    }
    fn refresh(&mut self) {
        let applied = self.palette.get();
        let model = self.model.borrow();
        let preview = model
            .candidate()
            .map(|i| {
                crate::gallery::brands::brand_palette(applied.theme, i).expect("compiled brand")
            })
            .unwrap_or(applied);
        let name = |p: Palette| {
            p.brand_index()
                .and_then(crate::gallery::brands::gallery_brand_preset)
                .map_or_else(|| "Consumer reference".to_string(), |p| p.name.to_string())
        };
        let detail = model
            .candidate()
            .and_then(crate::gallery::brands::gallery_brand_preset)
            .map_or_else(
                || name(preview),
                |p| format!("{} · {}", p.name, p.seed_srgb),
            );
        for (id, value) in [
            (120012, name(applied)),
            (
                120021,
                if model.candidate().is_some() {
                    "Candidate preview"
                } else {
                    "Current palette preview"
                }
                .into(),
            ),
            (120022, detail),
            (
                COUNT.0,
                format!(
                    "{} interactive preview activations",
                    model.demo_activations()
                ),
            ),
        ] {
            // Visits include paint/read-only queries. Mutate only changed values
            // so an unchanged projection does not retire freshly painted input geometry.
            if self
                .host
                .get(ComponentId(id))
                .unwrap()
                .component::<TextContent>()
                .unwrap()
                .text()
                != value
            {
                self.host
                    .get_mut(ComponentId(id))
                    .unwrap()
                    .component_mut::<TextContent>()
                    .unwrap()
                    .set_text(value);
            }
        }
        for id in [CONFIRM, CANCEL] {
            let disabled = model.candidate().is_none();
            if self
                .host
                .get(id)
                .unwrap()
                .component::<Button>()
                .unwrap()
                .is_disabled()
                != disabled
            {
                self.host
                    .get_mut(id)
                    .unwrap()
                    .component_mut::<Button>()
                    .unwrap()
                    .set_disabled(disabled);
            }
        }
        for (i, d) in crate::gallery::brands::gallery_brand_directions()
            .iter()
            .enumerate()
        {
            let id = ComponentId(DIRECTION + i as u64);
            let selected = model.direction() == Some(d.id);
            if self
                .host
                .get(id)
                .unwrap()
                .component::<Button>()
                .unwrap()
                .is_selected()
                != selected
            {
                self.host
                    .get_mut(id)
                    .unwrap()
                    .component_mut::<Button>()
                    .unwrap()
                    .set_selected(selected);
            }
        }
    }
    pub(crate) fn after_host_visit(&mut self) {
        for record in self.host.drain_action_records() {
            let id = record.action.target;
            if (DIRECTION
                ..DIRECTION + crate::gallery::brands::gallery_brand_directions().len() as u64)
                .contains(&id.0)
            {
                self.model
                    .borrow_mut()
                    .select_direction((id.0 - DIRECTION) as usize);
            } else if id == ACTION {
                self.model.borrow_mut().activate_demo();
            } else if id == OPEN {
                self.host.show_dialog(DIALOG).expect("mounted brand dialog");
            } else if id == CLOSE {
                self.host.close_dialog();
            } else if id == CANCEL {
                self.model.borrow_mut().cancel_candidate();
            } else if id == CONFIRM
                && let Some(index) = self.model.borrow_mut().confirm_candidate()
            {
                self.palette.set(
                    crate::gallery::brands::brand_palette(self.palette.get().theme, index)
                        .expect("compiled brand"),
                );
            }
        }
        self.refresh();
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::text::TextEngine;
    use super::*;
    use tekes_ui_tokens::Theme;
    fn render(story: &mut BrandStory, text: &mut TextEngine, width: f32) {
        let frame = Rect::new(0., 0., width, 5000.);
        story
            .host
            .render(&mut Scene::new(), text, frame, frame, story.palette.get())
            .unwrap();
    }
    fn press(story: &mut BrandStory, text: &mut TextEngine, id: ComponentId) {
        assert!(
            story
                .host
                .accessibility_action(id, None, AccessibilityAction::Press)
                .consumed
        );
        story.after_host_visit();
        render(story, text, 900.);
    }
    #[test]
    fn preview_cancel_and_confirm_preserve_application_palette_and_editor_owner() {
        let palette = Rc::new(Cell::new(
            crate::gallery::brands::brand_palette(Theme::LIGHT, 0).unwrap(),
        ));
        let mut story = BrandStory::new(palette.clone());
        let mut text = TextEngine::new();
        render(&mut story, &mut text, 900.);
        let initial = palette.get();
        let input_mount = story.host.get(INPUT).unwrap().mount_identity();
        press(&mut story, &mut text, ComponentId(DIRECTION));
        let candidate = story.model.borrow().candidate().unwrap();
        assert_eq!(
            palette.get(),
            initial,
            "direction only stages the compiler candidate"
        );
        render(&mut story, &mut text, 900.);
        press(&mut story, &mut text, ACTION);
        assert_eq!(story.model.borrow().demo_activations(), 1);
        press(&mut story, &mut text, OPEN);
        assert!(
            story
                .host
                .get(DIALOG)
                .unwrap()
                .component::<DialogContent>()
                .unwrap()
                .is_open()
        );
        press(&mut story, &mut text, CLOSE);
        press(&mut story, &mut text, CANCEL);
        assert_eq!(palette.get(), initial);
        press(&mut story, &mut text, ComponentId(DIRECTION));
        press(&mut story, &mut text, CONFIRM);
        assert_eq!(palette.get().brand_index(), Some(candidate));
        press(&mut story, &mut text, ComponentId(DIRECTION + 1));
        press(&mut story, &mut text, CANCEL);
        assert_eq!(
            palette.get().brand_index(),
            Some(candidate),
            "Cancel retains last confirmed palette"
        );
        assert_eq!(
            story.host.get(INPUT).unwrap().mount_identity(),
            input_mount,
            "candidate changes retain the real editor owner"
        );
    }
    #[test]
    fn narrow_scrolled_editor_focus_reveals_and_commits_through_original_host() {
        let mut story = BrandStory::new(Rc::new(Cell::new(Theme::LIGHT.into())));
        let mut text = TextEngine::new();
        text.set_type_scale(1.5);
        let frame = Rect::new(0., 0., 360., 480.);
        story
            .host
            .render(
                &mut Scene::new(),
                &mut text,
                frame,
                frame,
                Theme::LIGHT.into(),
            )
            .unwrap();
        assert!(story.host.focus(Some(INPUT), true));
        assert_eq!(story.host.focused(), Some(INPUT));
        // Focus reveal commits a new Scroll offset; present that arrangement
        // before the next native input, as the AppHost event/paint loop does.
        story
            .host
            .render(
                &mut Scene::new(),
                &mut text,
                frame,
                frame,
                Theme::LIGHT.into(),
            )
            .unwrap();
        story
            .host
            .event_with_text(
                &ContentEvent::Commit {
                    text: "edited brand".into(),
                    replacement: Some(0..1),
                },
                &mut text,
            )
            .unwrap();
        assert!(
            story
                .host
                .text_surface(INPUT)
                .unwrap()
                .text()
                .contains("edited brand")
        );
        story
            .host
            .render(
                &mut Scene::new(),
                &mut text,
                frame,
                frame,
                Theme::LIGHT.into(),
            )
            .unwrap();
        assert!(story.host.focus(Some(OPEN), true));
        assert_eq!(story.host.focused(), Some(OPEN));
    }

    #[test]
    fn brand_closed_dialog_keeps_scroll_end_real_and_open_dialog_in_viewport() {
        let mut story = BrandStory::new(Rc::new(Cell::new(Theme::LIGHT.into())));
        let mut text = TextEngine::new();
        let frame = Rect::new(0., 0., 480., 480.);
        let present = |story: &mut BrandStory, text: &mut TextEngine| {
            story
                .host
                .render(&mut Scene::new(), text, frame, frame, story.palette.get())
                .unwrap();
        };
        present(&mut story, &mut text);
        assert!(
            story
                .host
                .accessibility_action(ComponentId(DIRECTION), None, AccessibilityAction::Press)
                .consumed
        );
        story.after_host_visit();
        present(&mut story, &mut text);
        let document = story
            .host
            .get(ComponentId(120060))
            .unwrap()
            .component::<ScrollContent>()
            .unwrap()
            .scroll()
            .document_size();
        assert!(document.height > frame.size.height);
        assert!(
            document.height < 10_000.,
            "closed modal must not add the unbounded document measurement height: {document:?}"
        );
        assert!(
            story
                .host
                .event(&ContentEvent::Scroll {
                    position: frame.center(),
                    delta: tekes_ui_scene::Point::new(0., 1_000_000.),
                    precise: true,
                    begin: false,
                    end: false,
                    momentum: false,
                    time: 1.,
                })
                .consumed
        );
        present(&mut story, &mut text);
        assert!(
            story
                .host
                .get(COUNT)
                .unwrap()
                .visible()
                .is_some_and(|r| r.size.width > 0. && r.size.height > 0.),
            "scroll end must expose the original last content row, not blank modal allocation"
        );
        assert!(
            story
                .host
                .accessibility_action(OPEN, None, AccessibilityAction::Press)
                .consumed
        );
        story.after_host_visit();
        present(&mut story, &mut text);
        let dialog = story.host.get(DIALOG).unwrap();
        assert_eq!(
            dialog.frame(),
            frame,
            "modal receives the finite parent viewport"
        );
        let control = dialog.component::<DialogContent>().unwrap();
        assert!(control.is_open());
        let modal = control.semantic_frame(frame);
        assert!(modal.size.width > 0. && modal.size.height > 0.);
        assert!(modal.min_x() >= frame.min_x() && modal.max_x() <= frame.max_x());
        assert!(modal.min_y() >= frame.min_y() && modal.max_y() <= frame.max_y());
        assert!(story.host.get(CLOSE).unwrap().visible().is_some());
    }

    #[test]
    fn brand_pointer_clicks_stage_candidate_activate_preview_and_open_dialog() {
        let mut story = BrandStory::new(Rc::new(Cell::new(Theme::LIGHT.into())));
        let mut text = TextEngine::new();
        let present = |story: &mut BrandStory, text: &mut TextEngine| {
            render(story, text, 480.);
            // FocusContent::with_host invokes this after every visit, including paint.
            story.after_host_visit();
        };
        story.host.set_window_active(true);
        present(&mut story, &mut text);
        for id in [ComponentId(DIRECTION), ACTION, OPEN] {
            let point = story.host.get(id).unwrap().visible().unwrap().center();
            story.host.event(&ContentEvent::PointerMove(point));
            story.after_host_visit();
            present(&mut story, &mut text);
            assert!(
                story.host.event(&ContentEvent::PointerDown(point)).consumed,
                "{id:?} down"
            );
            assert!(story.host.pointer_capture().is_some(), "{id:?} capture");
            story.after_host_visit();
            present(&mut story, &mut text);
            assert!(
                story.host.event(&ContentEvent::PointerUp(point)).activated,
                "{id:?} up must activate"
            );
            story.after_host_visit();
            present(&mut story, &mut text);
            assert!(story.host.pointer_capture().is_none());
            if id == ComponentId(DIRECTION) {
                assert!(story.model.borrow().candidate().is_some());
            } else if id == ACTION {
                assert_eq!(story.model.borrow().demo_activations(), 1);
            } else {
                assert!(
                    story
                        .host
                        .get(DIALOG)
                        .unwrap()
                        .component::<DialogContent>()
                        .unwrap()
                        .is_open()
                );
            }
        }
    }

    #[test]
    fn staged_candidate_tracks_each_receiving_window_theme_without_changing_source() {
        let palette = Rc::new(Cell::new(
            crate::gallery::brands::brand_palette(Theme::LIGHT, 0).unwrap(),
        ));
        let story = BrandStory::new(palette.clone());
        story.model.borrow_mut().select_direction(0);
        let index = story.model.borrow().candidate().unwrap();
        for id in [120050, 120051] {
            let scope = story
                .host
                .get(ComponentId(id))
                .unwrap()
                .component::<ThemeScopeContent>()
                .unwrap();
            for theme in [Theme::LIGHT, Theme::DARK] {
                let projected = scope.child_theme(
                    crate::gallery::brands::brand_palette(theme, 0)
                        .unwrap()
                        .into(),
                );
                assert_eq!(
                    projected.palette(),
                    crate::gallery::brands::brand_palette(theme, index).unwrap()
                );
            }
        }
        assert_eq!(
            palette.get(),
            crate::gallery::brands::brand_palette(Theme::LIGHT, 0).unwrap()
        );
    }

    #[test]
    fn narrow_large_text_stacks_original_cards_and_keeps_actions_reachable() {
        let mut story = BrandStory::new(Rc::new(Cell::new(Theme::LIGHT.into())));
        let mut text = TextEngine::new();
        text.set_type_scale(1.5);
        render(&mut story, &mut text, 360.);
        press(&mut story, &mut text, ComponentId(DIRECTION));
        render(&mut story, &mut text, 360.);
        let current = story.host.get(CURRENT).unwrap().frame();
        let candidate = story.host.get(CANDIDATE).unwrap().frame();
        assert!(candidate.min_y() >= current.max_y());
        for id in [INPUT, ACTION, OPEN, CONFIRM, CANCEL] {
            let frame = story.host.get(id).unwrap().frame();
            assert!(frame.size.width > 0. && frame.size.height > 0.);
            assert!(frame.min_y() >= candidate.min_y() && frame.max_y() <= candidate.max_y());
        }
    }
}
