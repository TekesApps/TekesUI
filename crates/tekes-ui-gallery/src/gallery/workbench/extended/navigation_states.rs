use super::*;

const BODY: proposal::TextRole = proposal::TextRole::UiBody;
const HELP: &str =
    "Production view specimens. Use Playground to test real pointer, keyboard, and focus behavior.";

#[derive(Clone, Copy)]
struct BreadcrumbCase {
    title: &'static str,
    labels: &'static [&'static str],
    focused: Option<usize>,
    hovered: Option<usize>,
    focus_visible: bool,
    pressed: bool,
    disabled: bool,
}

pub(super) fn supports_states(renderer: &str) -> bool {
    matches!(
        renderer,
        "select" | "combo-box" | "breadcrumb" | "command-palette"
    )
}

pub(in crate::gallery::workbench) fn matrix(
    e: &Entry,
    s: &mut StoryState,
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    viewport: Rect,
) {
    scene.push_clip(viewport);
    let gutter = -controls::ControlFrame::new(Rect::new(0., 0., 1., 1.), None)
        .focus_bounds(ctx.text.backing_scale())
        .min_x();
    let bounds = viewport.inset(gutter);
    let x = bounds.min_x();
    let width = bounds.size.width.max(0.);
    let mut y = bounds.min_y();
    let help_height = caption_size(ctx, HELP, width).height;
    wrapped_caption(ctx, scene, x, y, width, HELP);
    y += help_height.max(18.) + 18.;
    let palette = Palette::new(ctx.theme, s.brand).expect("registered Gallery brand");

    match e.renderer() {
        "select" => select(ctx, scene, bounds, viewport, &mut y, width, palette),
        "combo-box" => combo_box(ctx, scene, bounds, viewport, &mut y, width, palette),
        "breadcrumb" => breadcrumb(ctx, scene, bounds, viewport, &mut y, width, palette),
        "command-palette" => command_palette(ctx, scene, bounds, viewport, &mut y, width, palette),
        _ => return,
    }
    s.document_bottom = y + gutter;
    scene.pop_clip();
}

fn row_heading(
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    x: f32,
    y: f32,
    w: f32,
    title: &str,
) -> f32 {
    let title_height = ctx.text.proposal_label(title, BODY, Some(w.max(0.))).height as f32;
    label(
        ctx,
        scene,
        Rect::new(x, y, w.max(0.), title_height.max(18.)),
        title,
        false,
    );
    title_height.max(18.)
}

fn combo_box(
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    bounds: Rect,
    clip: Rect,
    y: &mut f32,
    width: f32,
    palette: Palette,
) {
    let states = [
        ("Default", "", false, false),
        ("Hover", "", false, false),
        ("Pressed", "", false, false),
        ("Keyboard focus", "", false, false),
        ("Open · candidates", "", true, false),
        ("Open · filtered", "omf", true, false),
        ("Open · no results", "not-found", true, false),
        ("Selected", "Comfortable", false, false),
        ("Disabled", "", false, true),
    ];
    for (title, query, open, disabled) in states {
        let heading = row_heading(ctx, scene, bounds.min_x(), *y, width, title);
        let row_y = *y + heading + 7.;
        let mut input_view = crate::gallery::text_demo::TextDemo::search();
        input_view.set_value(query);
        input_view.set_disabled(disabled);
        if matches!(title, "Open · candidates" | "Keyboard focus") {
            input_view.focus_field();
        }
        let input_h = 38.0_f32.max(input_view.measure_search_height(ctx.text));
        let input = Rect::new(bounds.min_x(), row_y, width.min(440.), input_h);
        input_view.prepare_input_pointer_fixture(
            ctx.text,
            input,
            Some(clip),
            match title {
                "Hover" => "hover",
                "Pressed" => "pressed",
                _ => "rest",
            },
        );

        let mut model = controls::ComboBox {
            items: ["Automatic", "Comfortable", "Compact", "Follow system"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            ..Default::default()
        };
        model.disabled = disabled;
        model.open = open;
        model.query = query.to_owned();
        model.active = model.filtered().first().copied();
        if title == "Selected" {
            model.selected = Some(1);
        }
        let choices = model.filtered();
        let labels: Vec<&str> = choices.iter().map(|i| model.items[*i].as_str()).collect();
        let extra = if open && !labels.is_empty() {
            tekes_ui_controls::MenuView::measure(ctx.text, &labels, width, BODY, false).height
                + proposal::metric("spacing", "space-micro").unwrap()
        } else if open {
            8. + caption_size(ctx, "No matching options", input.size.width).height
        } else {
            22.
        };
        let row = Rect::new(bounds.min_x(), row_y, width, input_h + extra);
        input_view.build_search_clipped(ctx, input, Some(clip), scene);
        let mut view = ComboBoxView::default();
        view.render(
            scene,
            ctx.text,
            &mut model,
            input,
            row,
            Some(clip),
            palette,
            BODY,
            false,
        );
        if open && model.filtered().is_empty() {
            wrapped_caption(
                ctx,
                scene,
                input.min_x(),
                input.max_y() + 8.,
                input.size.width,
                "No matching options",
            );
        }
        *y = row.max_y() + 19.;
    }
    #[cfg(any(test, feature = "composition-review"))]
    for (status, validation, title) in [
        (
            tekes_ui_composition_consumer::content_status::Status::Loading,
            false,
            "Loading options · retained query consumer",
        ),
        (
            tekes_ui_composition_consumer::content_status::Status::Error,
            false,
            "Refresh failed · input stays editable",
        ),
        (
            tekes_ui_composition_consumer::content_status::Status::Ready,
            true,
            "Validation error · separate from refresh failure",
        ),
    ] {
        let heading = row_heading(ctx, scene, bounds.min_x(), *y, width, title);
        let top = *y + heading + 7.;
        let mut scenario = tekes_ui_composition_consumer::combo_status::Scenario::new(
            ["Automatic", "Compact", "Comfortable"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
        )
        .unwrap();
        scenario.set_status(status);
        if validation {
            scenario.set_validation_error(Some("Choose a permitted layout.".into()));
        }
        let size = scenario.measure(ctx.text, width.min(440.));
        let frame = Rect::new(bounds.min_x(), top, size.width, size.height);
        scenario
            .render(&mut Scene::new(), ctx.text, frame, clip, palette)
            .unwrap();
        scenario.host.focus(
            Some(tekes_ui_composition_consumer::combo_status::QUERY),
            true,
        );
        scenario
            .host
            .event(&tekes_ui_controls::composition::ContentEvent::Commit {
                text: "Com".into(),
                replacement: None,
            });
        scenario
            .render(&mut Scene::new(), ctx.text, frame, clip, palette)
            .unwrap();
        scenario
            .host
            .event(&tekes_ui_controls::composition::ContentEvent::Key {
                key: ControlKey::Enter,
                down: true,
            });
        scenario
            .render(scene, ctx.text, frame, clip, palette)
            .unwrap();
        *y = frame.max_y() + 19.;
    }
}

fn select(
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    bounds: Rect,
    clip: Rect,
    y: &mut f32,
    width: f32,
    palette: Palette,
) {
    use tekes_ui_controls::{Select, layout_view::SelectView};
    let all_labels = ["Automatic", "Compact", "Comfortable"];
    for (state, title) in [
        ("rest", "Default selection"),
        ("hover", "Hover"),
        ("pressed", "Pressed"),
        ("focus", "Keyboard focus"),
        ("disabled", "Disabled select"),
        ("selected", "Selected · Compact"),
        ("open", "Open options"),
        ("error", "Validation error · selection remains editable"),
        ("empty", "Empty option set · no menu"),
    ] {
        let labels: &[&str] = if state == "empty" { &[] } else { &all_labels };
        let feedback = match state {
            "error" => Some("This choice is unavailable. Choose another option."),
            "empty" => Some("No options available."),
            _ => None,
        };
        let heading = row_heading(ctx, scene, bounds.min_x(), *y, width, title);
        let top = *y + heading + 7.;
        let mut model = Select::new(labels.len());
        if state == "selected" {
            model.choose(1);
        }
        let trigger_width = width.min(230.);
        let height = ButtonModel::measure(
            ctx.text,
            labels.get(model.selected).copied().unwrap_or(""),
            trigger_width,
            ButtonSize::Body,
            BODY,
        )
        .height;
        let trigger = Rect::new(bounds.min_x(), top, trigger_width, height);
        let extra = if state == "open" {
            tekes_ui_controls::MenuView::measure(ctx.text, labels, width, BODY, false).height
                + proposal::metric("spacing", "space-micro").unwrap()
        } else if let Some(message) = feedback {
            8. + caption_size(ctx, message, width).height
        } else {
            0.
        };
        let row = Rect::new(bounds.min_x(), top, width, height + extra);
        let mut view = SelectView::default();
        view.set_reduced_motion(true);
        view.set_disabled(state == "disabled", &mut model);
        view.set_error(state == "error");
        view.render(
            &mut Scene::new(),
            ctx.text,
            &mut model,
            labels,
            trigger,
            row,
            None,
            palette,
            BODY,
            false,
        );
        match state {
            "hover" => view.pointer_move(trigger.center(), &mut model),
            "pressed" => {
                view.pointer_down(trigger.center(), &mut model);
            }
            "focus" => view.focus(true),
            "open" => {
                view.focus(true);
                view.key(ControlKey::Enter, true, &mut model);
            }
            _ => {}
        }
        view.render(
            scene,
            ctx.text,
            &mut model,
            labels,
            trigger,
            row,
            Some(clip),
            palette,
            BODY,
            false,
        );
        if let Some(message) = feedback {
            wrapped_caption(
                ctx,
                scene,
                bounds.min_x(),
                trigger.max_y() + 8.,
                width,
                message,
            );
        }
        *y = row.max_y() + 19.;
    }
    // The caller owns refresh status; the original Select stays mounted in a
    // public ContentHost and its menu uses the existing overlay pass.
    #[cfg(any(test, feature = "composition-review"))]
    for (status, title) in [
        (
            tekes_ui_composition_consumer::content_status::Status::Loading,
            "Loading options · independent consumer",
        ),
        (
            tekes_ui_composition_consumer::content_status::Status::Error,
            "Refresh failed · cached options remain usable",
        ),
    ] {
        let heading = row_heading(ctx, scene, bounds.min_x(), *y, width, title);
        let top = *y + heading + 7.;
        let mut scenario = tekes_ui_composition_consumer::choice_status::Scenario::new(
            all_labels.iter().map(|s| (*s).to_owned()).collect(),
        )
        .expect("independent public Select consumer");
        scenario.set_status(status);
        let size = scenario.measure(ctx.text, width.min(440.));
        let frame = Rect::new(bounds.min_x(), top, size.width, size.height);
        scenario
            .render(&mut Scene::new(), ctx.text, frame, clip, palette)
            .unwrap();
        scenario.host.focus(
            Some(tekes_ui_composition_consumer::choice_status::CHOICE),
            true,
        );
        scenario
            .render(scene, ctx.text, frame, clip, palette)
            .unwrap();
        *y = frame.max_y() + 19.;
    }
}

fn breadcrumb(
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    bounds: Rect,
    clip: Rect,
    y: &mut f32,
    width: f32,
    palette: Palette,
) {
    let cases = [
        BreadcrumbCase {
            title: "Default · current location is non-interactive",
            labels: &["Home", "Docs", "Drafts"],
            focused: None,
            hovered: None,
            focus_visible: false,
            pressed: false,
            disabled: false,
        },
        BreadcrumbCase {
            title: "Ancestor · hover",
            labels: &["Home", "Docs", "Drafts"],
            focused: None,
            hovered: Some(1),
            focus_visible: false,
            pressed: false,
            disabled: false,
        },
        BreadcrumbCase {
            title: "Ancestor · pressed",
            labels: &["Home", "Docs", "Drafts"],
            focused: None,
            hovered: Some(1),
            focus_visible: false,
            pressed: true,
            disabled: false,
        },
        BreadcrumbCase {
            title: "Ancestor · focus-visible",
            labels: &["Home", "Docs", "Drafts"],
            focused: Some(0),
            hovered: None,
            focus_visible: true,
            pressed: false,
            disabled: false,
        },
        BreadcrumbCase {
            title: "Long mixed-language path",
            labels: &["工作区 Workspace", "客户资料 Client files"],
            focused: None,
            hovered: None,
            focus_visible: false,
            pressed: false,
            disabled: false,
        },
        BreadcrumbCase {
            title: "Disabled · all ancestors",
            labels: &["Home", "Docs", "Drafts"],
            focused: None,
            hovered: None,
            focus_visible: false,
            pressed: false,
            disabled: true,
        },
    ];
    for case in cases {
        let BreadcrumbCase {
            title,
            labels,
            focused,
            hovered,
            focus_visible,
            pressed,
            disabled,
        } = case;
        let heading = row_heading(ctx, scene, bounds.min_x(), *y, width, title);
        let path_y = *y + heading + 7.;
        let mut remaining = width;
        let mut text_height = 0.0_f32;
        for (index, text) in labels.iter().enumerate() {
            let measured = tekes_ui_controls::ButtonModel::measure(
                ctx.text,
                text,
                remaining.max(0.),
                ButtonSize::Body,
                BODY,
            );
            text_height = text_height.max(measured.height);
            let cell_width = if index + 1 == labels.len() {
                remaining
            } else {
                (measured.width + 22.).min(remaining.max(0.))
            };
            remaining -= cell_width + if index + 1 < labels.len() { 8. } else { 0. };
        }
        let height = text_height.max(34.);
        let frame = Rect::new(bounds.min_x(), path_y, width, height);
        let model = controls::Breadcrumb {
            items: labels.iter().map(|s| (*s).to_owned()).collect(),
        };
        let mut view = BreadcrumbView::default();
        view.set_window_active(true);
        view.set_disabled(disabled);
        let mut probe = Scene::new();
        let layout = view.render(
            &mut probe,
            ctx.text,
            &model,
            labels,
            frame,
            Some(clip),
            palette,
            BODY,
        );
        if let Some(index) = focused {
            view.focus_with_visibility(index, focus_visible);
        }
        if let Some(index) = hovered
            && let Some(target) = layout
                .items
                .get(index)
                .and_then(|item| item.visible_frame())
        {
            view.pointer_move(target.center());
            if pressed {
                view.pointer_down(target.center());
            }
        }
        view.render(
            scene,
            ctx.text,
            &model,
            labels,
            frame,
            Some(clip),
            palette,
            BODY,
        );
        *y = frame.max_y() + 19.;
    }
}

fn command_palette(
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    bounds: Rect,
    clip: Rect,
    y: &mut f32,
    width: f32,
    palette: Palette,
) {
    let cases = [
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
    for (title, query, open, disabled) in cases {
        let heading = row_heading(ctx, scene, bounds.min_x(), *y, width, title);
        let top = *y + heading + 7.;
        let mut model = controls::CommandPalette {
            combo: controls::ComboBox {
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
        let mut view = CommandPaletteView::default();
        if open {
            view.open(&mut model);
            model.combo.query = query.to_owned();
            model.combo.active = model.combo.filtered().first().copied();
        }
        let executed = if title == "Selected · last executed command" {
            view.open(&mut model);
            view.key(ControlKey::Down, true, &mut model);
            assert_eq!(view.key(ControlKey::Enter, true, &mut model), Some(1));
            Some(format!("Last command: {}", model.combo.query))
        } else {
            None
        };
        let mut input_view = crate::gallery::text_demo::TextDemo::search();
        input_view.set_value(query);
        input_view.set_disabled(disabled);
        let trigger_width = width.min(240.);
        let trigger_height = ButtonModel::measure(
            ctx.text,
            "Open command palette",
            trigger_width,
            ButtonSize::Body,
            BODY,
        )
        .height;
        let trigger = Rect::new(bounds.min_x(), top, trigger_width, trigger_height);
        let input_height = 38.0_f32.max(input_view.measure_search_height(ctx.text));
        let input = Rect::new(
            bounds.min_x(),
            trigger.max_y() + 8.,
            width.min(420.),
            input_height,
        );
        let choices = model.combo.filtered();
        let labels: Vec<&str> = choices
            .iter()
            .map(|i| model.combo.items[*i].as_str())
            .collect();
        let bottom = if open {
            let extra = if labels.is_empty() {
                8. + caption_size(ctx, "No matching commands", input.size.width).height
            } else {
                tekes_ui_controls::MenuView::measure(ctx.text, &labels, width, BODY, false).height
                    + proposal::metric("spacing", "space-micro").unwrap()
            };
            input.max_y() + extra
        } else if let Some(message) = &executed {
            trigger.max_y() + 8. + caption_size(ctx, message, width).height
        } else {
            trigger.max_y() + 22.
        };
        let row = Rect::new(bounds.min_x(), top, width, bottom - top);
        if title == "Closed · keyboard focus" {
            view.focus_trigger(true);
        }
        if matches!(title, "Closed · hover" | "Closed · pressed" | "Disabled") {
            view.render_trigger(
                &mut Scene::new(),
                ctx.text,
                &mut model,
                "Open command palette",
                trigger,
                None,
                palette,
                BODY,
            );
            view.pointer_move(trigger.center(), &mut model);
            if title == "Closed · pressed" {
                assert!(view.pointer_down_trigger(trigger.center()));
            }
        }
        view.render_trigger(
            scene,
            ctx.text,
            &mut model,
            "Open command palette",
            trigger,
            Some(clip),
            palette,
            BODY,
        );
        if open {
            input_view.focus_field();
            input_view.build_search_clipped(ctx, input, Some(clip), scene);
        }
        view.render(
            scene,
            ctx.text,
            &mut model,
            if open { input } else { trigger },
            row,
            Some(clip),
            palette,
            BODY,
            false,
        );
        if open && model.combo.filtered().is_empty() {
            wrapped_caption(
                ctx,
                scene,
                input.min_x(),
                input.max_y() + 8.,
                input.size.width,
                "No matching commands",
            );
        }
        if let Some(message) = &executed {
            wrapped_caption(
                ctx,
                scene,
                bounds.min_x(),
                trigger.max_y() + 8.,
                width,
                message,
            );
        }
        *y = row.max_y() + 19.;
    }
    #[cfg(any(test, feature = "composition-review"))]
    for (status, cached, title) in [
        (
            tekes_ui_composition_consumer::content_status::Status::Loading,
            true,
            "Loading commands · retained query consumer",
        ),
        (
            tekes_ui_composition_consumer::content_status::Status::Error,
            true,
            "Refresh failed · query and commands retained",
        ),
        (
            tekes_ui_composition_consumer::content_status::Status::Loading,
            false,
            "Initial command lookup · no cached results",
        ),
    ] {
        use tekes_ui_composition_consumer::command_status::{COMMAND, Scenario};
        use tekes_ui_controls::composition::ContentEvent;
        let heading = row_heading(ctx, scene, bounds.min_x(), *y, width, title);
        let mut sample = Scenario::new(if cached {
            vec![
                "Open file".into(),
                "Open settings".into(),
                "Close window".into(),
            ]
        } else {
            Vec::new()
        })
        .expect("public command caller");
        let probe = Rect::new(bounds.min_x(), *y + heading + 7., width.min(420.), 400.);
        sample
            .render(&mut Scene::new(), ctx.text, probe, clip, palette)
            .unwrap();
        sample.host.focus(Some(COMMAND), true);
        sample.host.event(&ContentEvent::Key {
            key: ControlKey::Enter,
            down: true,
        });
        sample.host.event(&ContentEvent::Commit {
            text: "Open".into(),
            replacement: None,
        });
        sample.set_status(status);
        let measured = sample.measure(ctx.text, probe.size.width);
        let frame = Rect::new(
            probe.min_x(),
            probe.min_y(),
            probe.size.width,
            measured.height,
        );
        sample
            .render(scene, ctx.text, frame, clip, palette)
            .unwrap();
        let menu_height = if cached {
            tekes_ui_controls::MenuView::measure(
                ctx.text,
                &["Open file", "Open settings"],
                frame.size.width,
                BODY,
                false,
            )
            .height
                + tekes_ui_tokens::Spacing::Tight.value()
        } else {
            0.
        };
        *y = frame.max_y() + menu_height + 19.;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gallery::{static_text::StaticText, text::TextEngine};

    #[test]
    fn navigation_state_stories_render_production_views_at_narrow_large_type() {
        for renderer in ["select", "combo-box", "breadcrumb", "command-palette"] {
            let entry = super::super::super::entries()
                .iter()
                .find(|entry| entry.renderer() == renderer)
                .expect("registered navigation component");
            assert!(entry.supports_story("states"));
            for scale in [1.0, 1.5] {
                let mut text = TextEngine::new();
                text.set_scale(1.);
                text.set_type_scale(scale);
                text.begin_frame();
                let mut static_text = StaticText::new(None);
                let mut ctx = PageCtx {
                    theme: tekes_ui_tokens::Theme::LIGHT,
                    text: &mut text,
                    static_text: &mut static_text,
                };
                let viewport = Rect::new(0., 0., 360., 4000.);
                let mut state = StoryState::default();
                state.input.set_value("preserve playground input");
                let input_before = state.input.query().to_owned();
                let combo_before = state.extended.combo.query.clone();
                let mut scene = Scene::new();
                matrix(entry, &mut state, &mut ctx, &mut scene, viewport);
                assert!(
                    !scene.is_empty(),
                    "{renderer} at {scale}x should render real views"
                );
                assert!(state.document_bottom > 0. && state.document_bottom < 4000.);
                assert!(state.document_bottom.is_finite());
                assert_eq!(state.input.query(), input_before);
                assert_eq!(state.extended.combo.query, combo_before);
            }
        }
    }

    #[test]
    fn navigation_state_stories_render_headless_gpu_and_export_when_requested() {
        let mut renderer = tekes_ui_render::Renderer::new_headless()
            .expect("headless GPU is required for navigation state review");
        let output = std::env::var_os("TEKESUI_GOLD7_PAGE_OUT").map(std::path::PathBuf::from);
        if let Some(output) = &output {
            std::fs::create_dir_all(output).unwrap();
        }
        for renderer_name in ["select", "combo-box", "breadcrumb", "command-palette"] {
            let entry = super::super::super::entries()
                .iter()
                .find(|entry| entry.renderer() == renderer_name)
                .expect("registered navigation component");
            let mut text = TextEngine::new();
            text.set_scale(1.);
            text.set_type_scale(1.5);
            text.begin_frame();
            let mut static_text = StaticText::new(None);
            let mut ctx = PageCtx {
                theme: tekes_ui_tokens::Theme::LIGHT,
                text: &mut text,
                static_text: &mut static_text,
            };
            // State matrices are scrollable documents. Measure the current real
            // specimens before choosing the full-document capture height, so new
            // existing-consumer states cannot be silently cropped by an old size.
            let measure_viewport = Rect::new(0., 0., 360., 4000.);
            let mut state = StoryState::default();
            let mut scene = Scene::new();
            matrix(entry, &mut state, &mut ctx, &mut scene, measure_viewport);
            assert!(state.document_bottom.is_finite());
            assert!(state.document_bottom > 0. && state.document_bottom < 4000.);
            let height = state.document_bottom.ceil() as u32;
            let viewport = Rect::new(0., 0., 360., height as f32);
            scene.clear();
            matrix(entry, &mut state, &mut ctx, &mut scene, viewport);
            assert!(
                state.document_bottom <= viewport.max_y(),
                "{renderer_name}: document_bottom={} viewport_bottom={}",
                state.document_bottom,
                viewport.max_y()
            );
            let rgba = renderer
                .render_to_rgba8(
                    &scene,
                    ctx.text.atlas_mut(),
                    (360, height),
                    1.,
                    crate::gallery::Gallery::background(ctx.theme),
                )
                .expect("GPU should render the production state scene");
            assert!(rgba.chunks_exact(4).any(|pixel| pixel != &rgba[0..4]));
            if let Some(output) = &output {
                let path = output.join(format!(
                    "navigation-{renderer_name}-states-360x{height}.png"
                ));
                let file = std::fs::File::create(path).unwrap();
                let mut encoder = png::Encoder::new(file, 360, height);
                encoder.set_color(png::ColorType::Rgba);
                encoder.set_depth(png::BitDepth::Eight);
                encoder
                    .write_header()
                    .unwrap()
                    .write_image_data(&rgba)
                    .unwrap();
            }
        }
    }
}
