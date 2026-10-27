//! State fixtures for controls whose public behavior is intentionally small.
//!
//! Every sample below gets a fresh production view and model. The Gallery's
//! Playground state is therefore independent from these visual comparisons.

use super::{Entry, PageCtx, StoryState};
use tekes_ui_controls::{
    ButtonTextRenderer, NumberField,
    extended::ControlFrame,
    feedback_view::{NumberFieldView, ToolbarView},
};
use tekes_ui_scene::{Point, Rect, Scene};
use tekes_ui_tokens::{ColorRole, Palette, proposal};

#[derive(Clone, Copy)]
enum ToolbarState {
    Default,
    Hover,
    Pressed,
    FocusVisible,
    Disabled,
}

#[derive(Clone, Copy)]
enum NumberState {
    Default,
    Minimum,
    Maximum,
    Hover,
    Pressed,
    FocusVisible,
    Disabled,
}

#[derive(Clone, Copy)]
struct Fixture<'a, S> {
    title: &'a str,
    state: S,
}

const TOOLBAR_FIXTURES: [Fixture<'static, ToolbarState>; 5] = [
    Fixture {
        title: "Default",
        state: ToolbarState::Default,
    },
    Fixture {
        title: "Hover",
        state: ToolbarState::Hover,
    },
    Fixture {
        title: "Pressed",
        state: ToolbarState::Pressed,
    },
    Fixture {
        title: "Focus visible",
        state: ToolbarState::FocusVisible,
    },
    Fixture {
        title: "Disabled",
        state: ToolbarState::Disabled,
    },
];

const NUMBER_FIXTURES: [Fixture<'static, NumberState>; 7] = [
    Fixture {
        title: "Default · 8",
        state: NumberState::Default,
    },
    Fixture {
        title: "Minimum · decrement unavailable",
        state: NumberState::Minimum,
    },
    Fixture {
        title: "Maximum · increment unavailable",
        state: NumberState::Maximum,
    },
    Fixture {
        title: "Hover · increment",
        state: NumberState::Hover,
    },
    Fixture {
        title: "Pressed · increment",
        state: NumberState::Pressed,
    },
    Fixture {
        title: "Focus visible · increment",
        state: NumberState::FocusVisible,
    },
    Fixture {
        title: "Disabled · whole control",
        state: NumberState::Disabled,
    },
];

pub(super) fn supports(renderer: &str) -> bool {
    matches!(renderer, "toolbar" | "number-field")
}

pub(super) fn matrix(
    e: &Entry,
    s: &mut StoryState,
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    c: Rect,
) {
    let gutter = -ControlFrame::new(Rect::new(0., 0., 1., 1.), None)
        .focus_bounds(ctx.text.backing_scale())
        .min_x();
    let bounds = c.inset(gutter);
    let palette = Palette::new(ctx.theme, s.brand).expect("registered Gallery brand");
    let mut y = bounds.min_y();

    match e.renderer() {
        "toolbar" => {
            y = section_intro(
                ctx,
                scene,
                bounds,
                y,
                "Toolbar",
                "Grouped actions use the same production buttons. Focus and disabled state apply to the toolbar group; the toolbar has no selected, checked, or loading state.",
            );
            y = draw_toolbar_grid(ctx, scene, bounds, y, c, palette);
        }
        "number-field" => {
            y = section_intro(
                ctx,
                scene,
                bounds,
                y,
                "NumberField",
                "A bounded numeric stepper, not a text editor. Minimum and maximum values disable only the stepper that cannot change the value; the whole control can also be disabled.",
            );
            y = draw_number_grid(ctx, scene, bounds, y, c, palette);
        }
        _ => return,
    }

    s.document_bottom = (y + gutter + 8.).max(c.min_y());
}

fn section_intro(
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    bounds: Rect,
    y: f32,
    title: &str,
    description: &str,
) -> f32 {
    let title_height = measure_text(ctx, title, proposal::TextRole::UiHeading, bounds.size.width);
    draw_text(
        ctx,
        scene,
        Rect::new(bounds.min_x(), y, bounds.size.width, title_height),
        title,
        proposal::TextRole::UiHeading,
        ColorRole::PrimaryText.resolve(ctx.theme),
    );
    let copy_y = y + title_height + 8.;
    let copy_height = measure_text(
        ctx,
        description,
        proposal::TextRole::UiBody,
        bounds.size.width,
    );
    draw_text(
        ctx,
        scene,
        Rect::new(bounds.min_x(), copy_y, bounds.size.width, copy_height),
        description,
        proposal::TextRole::UiBody,
        ColorRole::SecondaryText.resolve(ctx.theme),
    );
    copy_y + copy_height + 18.
}

fn draw_toolbar_grid(
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    bounds: Rect,
    top: f32,
    clip: Rect,
    palette: Palette,
) -> f32 {
    let fixtures = TOOLBAR_FIXTURES;
    draw_grid(
        ctx,
        scene,
        bounds,
        top,
        &fixtures,
        48.,
        |ctx, scene, frame, fixture| {
            render_toolbar(ctx, scene, frame, clip, palette, fixture.state);
        },
    )
}

fn draw_number_grid(
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    bounds: Rect,
    top: f32,
    clip: Rect,
    palette: Palette,
) -> f32 {
    let fixtures = NUMBER_FIXTURES;
    draw_grid(
        ctx,
        scene,
        bounds,
        top,
        &fixtures,
        52.,
        |ctx, scene, frame, fixture| {
            render_number_field(ctx, scene, frame, clip, palette, fixture.state);
        },
    )
}

fn draw_grid<S: Copy>(
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    bounds: Rect,
    top: f32,
    fixtures: &[Fixture<'_, S>],
    control_height: f32,
    mut render: impl FnMut(&mut PageCtx<'_>, &mut Scene, Rect, Fixture<'_, S>),
) -> f32 {
    let columns = grid_columns(bounds.size.width);
    let column_gap = 20.;
    let row_gap = 20.;
    let cell_width =
        ((bounds.size.width - column_gap * (columns - 1) as f32) / columns as f32).max(0.);
    let title_heights = fixtures
        .iter()
        .map(|fixture| {
            measure_text(
                ctx,
                fixture.title,
                proposal::TextRole::UiCaption,
                cell_width,
            )
        })
        .collect::<Vec<_>>();
    let mut row_tops = Vec::with_capacity(fixtures.len().div_ceil(columns));
    let mut row_heights = Vec::with_capacity(fixtures.len().div_ceil(columns));
    let mut row_y = top;
    for row in 0..fixtures.len().div_ceil(columns) {
        let first = row * columns;
        let last = (first + columns).min(fixtures.len());
        let label_height = title_heights[first..last]
            .iter()
            .copied()
            .fold(0., f32::max);
        let height = label_height + 8. + control_height;
        row_tops.push(row_y);
        row_heights.push(height);
        row_y += height + row_gap;
    }

    for (index, fixture) in fixtures.iter().copied().enumerate() {
        let row = index / columns;
        let column = index % columns;
        let x = bounds.min_x() + column as f32 * (cell_width + column_gap);
        let y = row_tops[row];
        let title_rect = Rect::new(x, y, cell_width, title_heights[index]);
        draw_text(
            ctx,
            scene,
            title_rect,
            fixture.title,
            proposal::TextRole::UiCaption,
            ColorRole::SecondaryText.resolve(ctx.theme),
        );
        let control = Rect::new(
            x,
            y + row_heights[row] - control_height,
            cell_width,
            control_height,
        );
        render(ctx, scene, control, fixture);
    }
    row_y - row_gap
}

fn render_toolbar(
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    frame: Rect,
    clip: Rect,
    palette: Palette,
    state: ToolbarState,
) {
    let labels = ["Add", "Rename", "More"];
    let mut view = ToolbarView::new(labels.len(), 8.);
    if matches!(state, ToolbarState::Disabled) {
        view.set_disabled(true);
    }
    let layout = {
        let mut probe = Scene::new();
        view.render(
            &mut probe,
            ctx.text,
            &labels,
            frame,
            Some(clip),
            palette,
            proposal::TextRole::UiBody,
        )
    };
    let interaction_target = match state {
        ToolbarState::Hover | ToolbarState::Pressed => layout.get(1),
        _ => layout.first(),
    };
    if let Some(target) = interaction_target {
        let center = center(target.frame);
        match state {
            ToolbarState::Hover => view.pointer_move(center),
            ToolbarState::Pressed => {
                view.pointer_move(center);
                let _ = view.pointer_down(center);
            }
            ToolbarState::FocusVisible => view.focus(Some(0), true),
            ToolbarState::Default | ToolbarState::Disabled => {}
        }
    }
    view.render(
        scene,
        ctx.text,
        &labels,
        frame,
        Some(clip),
        palette,
        proposal::TextRole::UiBody,
    );
}

fn render_number_field(
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    frame: Rect,
    clip: Rect,
    palette: Palette,
    state: NumberState,
) {
    let (value, min, max) = match state {
        NumberState::Minimum => (0., 0., 10.),
        NumberState::Maximum => (10., 0., 10.),
        _ => (8., 0., 10.),
    };
    let model = NumberField::new(value, min, max, 1.).expect("bounded fixture is valid");
    let mut view = NumberFieldView::default();
    if matches!(state, NumberState::Disabled) {
        view.set_disabled(true);
    }
    let layout = {
        let mut probe = Scene::new();
        view.render(
            &mut probe,
            ctx.text,
            &model,
            frame,
            Some(clip),
            palette,
            proposal::TextRole::UiBody,
        )
    };
    let increment_center = center(layout.increment.frame);
    match state {
        NumberState::Hover => view.pointer_move(increment_center),
        NumberState::Pressed => {
            view.pointer_move(increment_center);
            let _ = view.pointer_down(increment_center);
        }
        NumberState::FocusVisible => view.focus(true, true),
        NumberState::Default
        | NumberState::Minimum
        | NumberState::Maximum
        | NumberState::Disabled => {}
    }
    view.render(
        scene,
        ctx.text,
        &model,
        frame,
        Some(clip),
        palette,
        proposal::TextRole::UiBody,
    );
}

fn center(rect: Rect) -> Point {
    Point::new(
        rect.min_x() + rect.size.width * 0.5,
        rect.min_y() + rect.size.height * 0.5,
    )
}

fn measure_text(ctx: &mut PageCtx<'_>, text: &str, role: proposal::TextRole, width: f32) -> f32 {
    ctx.text
        .proposal_label(text, role, Some(width.max(0.)))
        .height as f32
}

fn draw_text(
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    frame: Rect,
    text: &str,
    role: proposal::TextRole,
    color: tekes_ui_tokens::Rgba,
) {
    ctx.text
        .proposal_wrapped_in(scene, ctx.theme, frame, text, role, color);
}

fn grid_columns(width: f32) -> usize {
    if width >= 660. { 2 } else { 1 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn states_use_one_column_at_narrow_viewport_and_two_when_labels_fit() {
        assert_eq!(grid_columns(360.), 1);
        assert_eq!(grid_columns(659.9), 1);
        assert_eq!(grid_columns(660.), 2);
    }

    #[test]
    fn toolbar_and_number_field_states_render_production_views_without_mutating_playground() {
        use super::super::entries;
        use crate::gallery::{static_text::StaticText, text::TextEngine};

        let mut renderer = tekes_ui_render::Renderer::new_headless()
            .expect("headless GPU is required for real state-matrix rendering");
        let output = std::env::var_os("TEKESUI_GOLD7_PAGE_OUT").map(std::path::PathBuf::from);
        if let Some(output) = &output {
            std::fs::create_dir_all(output).unwrap();
        }

        for renderer_name in ["toolbar", "number-field"] {
            let entry = entries()
                .iter()
                .find(|entry| entry.renderer() == renderer_name)
                .expect("registered component");
            let mut text = TextEngine::new();
            text.set_scale(1.);
            text.set_type_scale(1.5);
            let mut static_text = StaticText::new(None);
            let mut ctx = PageCtx {
                theme: tekes_ui_tokens::Theme::LIGHT,
                text: &mut text,
                static_text: &mut static_text,
            };
            let mut state = StoryState::default();
            let original_value = state.number.value();
            let mut scene = Scene::new();
            let viewport = Rect::new(0., 0., 360., 1800.);
            matrix(entry, &mut state, &mut ctx, &mut scene, viewport);

            assert!(
                !scene.is_empty(),
                "{renderer_name} has a real rendered matrix"
            );
            assert!(state.document_bottom.is_finite());
            assert!(state.document_bottom > 0. && state.document_bottom < viewport.max_y());
            assert_eq!(state.number.value(), original_value);
            assert!(state.toolbar_view.layouts().is_empty());
            assert_eq!(state.number_view.layout().frame, Rect::default());
            let image_height = state.document_bottom.ceil().max(1.) as u32;

            let rgba = renderer
                .render_to_rgba8(
                    &scene,
                    ctx.text.atlas_mut(),
                    (360, image_height),
                    1.,
                    crate::gallery::Gallery::background(ctx.theme),
                )
                .expect("production state fixtures render on the headless GPU");
            let background = &rgba[0..4];
            assert!(
                rgba.chunks_exact(4).any(|pixel| pixel != background),
                "{renderer_name} GPU frame contains rendered control pixels"
            );
            if let Some(output) = &output {
                let path = output.join(format!(
                    "{renderer_name}-states-360x{image_height}-type-1_5.png"
                ));
                let file = std::fs::File::create(path).unwrap();
                let mut encoder = png::Encoder::new(file, 360, image_height);
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
