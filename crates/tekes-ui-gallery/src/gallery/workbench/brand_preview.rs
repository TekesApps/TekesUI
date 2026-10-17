use super::super::{PageCtx, text_demo::TextDemo};
use std::collections::{HashMap, HashSet};
use tekes_ui_controls::{
    Button, ButtonSize, ButtonTextAlign, ControlKey, DialogEvent, DialogView, paint,
};
use tekes_ui_platform_macos::{Event, KeyEventKind, MouseButton, MouseEventKind};
use tekes_ui_scene::{Point, Quad, Rect, Scene};
use tekes_ui_tokens::{ColorRole, Palette, Rgba, proposal};

const FAMILY_FIRST: usize = 700;
const PREVIEW_ACTION: usize = 720;
const PREVIEW_OVERLAY: usize = 721;
const CONFIRM: usize = 722;
const CANCEL: usize = 723;
const INPUT: usize = 1000;
const ROLE_SAMPLES: [(&str, &str, &str); 3] = [
    (
        "Action",
        "action-primary-foreground",
        "action-primary-ground",
    ),
    ("Link", "c-link-foreground", "surface-0"),
    ("Subtle", "accent-subtle-foreground", "accent-subtle-ground"),
];

#[derive(Debug)]
pub(super) struct BrandPreview {
    direction: Option<String>,
    candidate: Option<usize>,
    input: TextDemo,
    buttons: HashMap<usize, Button>,
    hits: Vec<(usize, Rect)>,
    focus: Option<usize>,
    focus_visible: bool,
    dialog: DialogView,
    dialog_layout: Option<tekes_ui_controls::DialogLayout>,
    demo_activations: usize,
    window_active: bool,
}

impl Default for BrandPreview {
    fn default() -> Self {
        Self {
            direction: None,
            candidate: None,
            input: TextDemo::field("Type to preview candidate text", false, false),
            buttons: HashMap::new(),
            hits: Vec::new(),
            focus: None,
            focus_visible: false,
            dialog: DialogView::default(),
            dialog_layout: None,
            demo_activations: 0,
            window_active: true,
        }
    }
}

impl BrandPreview {
    /// Full content height for the candidate matrix body. This mirrors the same
    /// measured flow used by `build`, so the outer Gallery viewport can expose
    /// every control at narrow widths and larger text scales.
    pub(super) fn document_extent(
        &self,
        text: &mut super::super::text::TextEngine,
        width: f32,
        density: usize,
        applied_brand: usize,
    ) -> f32 {
        let width = width.max(0.);
        let title_h = measure(
            text,
            "Choose a hue direction",
            proposal::TextRole::UiHeading,
            width,
        );
        let intro = "Selecting a direction prepares a compiler-generated candidate. It does not change the app palette; only Confirm use applies it globally.";
        let intro_h = measure(text, intro, proposal::TextRole::UiCaption, width);
        let gap = 8.;
        let columns = if width < 480. { 2 } else { 4 };
        let cell_w = ((width - gap * (columns as f32 - 1.)) / columns as f32).max(0.);
        let directions = tekes_ui_tokens::palette::gallery_brand_directions();
        let mut row_heights = vec![34.0_f32; directions.len().div_ceil(columns)];
        for (index, direction) in directions.iter().enumerate() {
            let height = text
                .measure_button(
                    direction.name,
                    cell_w,
                    ButtonSize::Body,
                    proposal::TextRole::UiBody,
                )
                .height
                .max(34.);
            row_heights[index / columns] = row_heights[index / columns].max(height);
        }
        let direction_rows_h = row_heights.iter().map(|height| height + 8.).sum::<f32>();
        let panel_y = title_h + 8. + intro_h + 12. + direction_rows_h + 4.;
        let card_gap = 12.;
        let stacked = width < 720.;
        let card_w = if stacked {
            width
        } else {
            ((width - card_gap) / 2.).max(0.)
        };
        let applied_preset = tekes_ui_tokens::palette::gallery_brand_preset(applied_brand);
        let current_h = self.measure_card_height(
            text,
            card_w,
            applied_brand,
            applied_preset,
            "Currently applied",
            false,
            density,
        );
        let preview_index = self.candidate.unwrap_or(applied_brand);
        let preview_preset = tekes_ui_tokens::palette::gallery_brand_preset(preview_index);
        let preview_title = if self.candidate.is_some() {
            "Candidate preview"
        } else {
            "Current palette preview"
        };
        let preview_h = self.measure_card_height(
            text,
            card_w,
            preview_index,
            preview_preset,
            preview_title,
            true,
            density,
        );
        let cards_h = if stacked {
            current_h + card_gap + preview_h
        } else {
            current_h.max(preview_h)
        };
        panel_y + cards_h + 8.
    }

    pub(super) fn build(
        &mut self,
        ctx: &mut PageCtx<'_>,
        scene: &mut Scene,
        frame: Rect,
        clip: Option<Rect>,
        applied_brand: usize,
        density: usize,
    ) {
        self.hits.clear();
        self.dialog_layout = None;
        let x = frame.min_x();
        let y = frame.min_y();
        let w = frame.size.width.max(0.);
        let current = Palette {
            theme: ctx.theme,
            brand: applied_brand,
        };
        let directions = tekes_ui_tokens::palette::gallery_brand_directions();
        let title_h = measure(
            ctx.text,
            "Choose a hue direction",
            proposal::TextRole::UiHeading,
            w,
        );
        ctx.text.proposal_label_in(
            scene,
            ctx.theme,
            Rect::new(x, y, w, title_h),
            "Choose a hue direction",
            proposal::TextRole::UiHeading,
            ColorRole::PrimaryText.resolve(ctx.theme),
        );
        let intro = "Selecting a direction prepares a compiler-generated candidate. It does not change the app palette; only Confirm use applies it globally.";
        let intro_h = measure(ctx.text, intro, proposal::TextRole::UiCaption, w);
        let intro_y = y + title_h + 8.;
        self.draw_wrapped(
            ctx,
            scene,
            Rect::new(x, intro_y, w, intro_h),
            intro,
            ctx.theme,
        );

        let gap = 8.;
        let columns = if w < 480. { 2 } else { 4 };
        let cell_w = ((w - gap * (columns as f32 - 1.)) / columns as f32).max(0.);
        let mut direction_row_heights = vec![34.0_f32; directions.len().div_ceil(columns)];
        for (index, direction) in directions.iter().enumerate() {
            let height = ctx
                .text
                .measure_button(
                    direction.name,
                    cell_w,
                    ButtonSize::Body,
                    proposal::TextRole::UiBody,
                )
                .height
                .max(34.);
            direction_row_heights[index / columns] =
                direction_row_heights[index / columns].max(height);
        }
        let direction_y = intro_y + intro_h + 12.;
        let mut row_tops = Vec::with_capacity(direction_row_heights.len());
        let mut row_y = direction_y;
        for height in &direction_row_heights {
            row_tops.push(row_y);
            row_y += *height + 8.;
        }
        for (index, direction) in directions.iter().enumerate() {
            let col = index % columns;
            let row = index / columns;
            let rect = Rect::new(
                x + col as f32 * (cell_w + gap),
                row_tops[row],
                cell_w,
                direction_row_heights[row],
            );
            self.button(
                ctx,
                scene,
                FAMILY_FIRST + index,
                rect,
                direction.name,
                self.direction.as_deref() == Some(direction.id),
                current,
                clip,
                paint::ButtonVariant::Quiet,
                density,
            );
        }

        let panel_y = row_y - 8. + 12.;
        let panel_h = (frame.max_y() - panel_y - 8.).max(0.);
        if panel_h <= 0. {
            self.cancel_inactive_buttons();
            return;
        }
        let applied = tekes_ui_tokens::palette::gallery_brand_preset(applied_brand);
        let preview_index = self.candidate.unwrap_or(applied_brand);
        let preview_preset = tekes_ui_tokens::palette::gallery_brand_preset(preview_index);
        let preview = Palette {
            theme: ctx.theme,
            brand: preview_index,
        };
        let card_gap = 12.;
        let stacked = w < 720.;
        let card_w = if stacked {
            w
        } else {
            ((w - card_gap) / 2.).max(0.)
        };
        let right_title = if self.candidate.is_some() {
            "Candidate preview"
        } else {
            "Current palette preview"
        };
        let left_top = panel_y;
        let right_top = if stacked {
            left_top
                + self.measure_card_height(
                    ctx.text,
                    card_w,
                    applied_brand,
                    applied,
                    "Currently applied",
                    false,
                    density,
                )
                + card_gap
        } else {
            panel_y
        };
        let left_h = self.measure_card_height(
            ctx.text,
            card_w,
            applied_brand,
            applied,
            "Currently applied",
            false,
            density,
        );
        let right_h = self.measure_card_height(
            ctx.text,
            card_w,
            preview_index,
            preview_preset,
            right_title,
            true,
            density,
        );
        let left = Rect::new(x, left_top, card_w, left_h);
        let right = Rect::new(
            if stacked { x } else { x + card_w + card_gap },
            right_top,
            card_w,
            right_h,
        );
        self.card(ctx, scene, left, current);
        self.card(ctx, scene, right, preview);

        let left_inner_x = left.min_x() + 14.;
        let left_inner_w = (left.size.width - 28.).max(0.);
        let left_title_h = measure(
            ctx.text,
            "Currently applied",
            proposal::TextRole::UiBody,
            left_inner_w,
        );
        ctx.text.proposal_label_in(
            scene,
            ctx.theme,
            Rect::new(left_inner_x, left.min_y() + 12., left_inner_w, left_title_h),
            "Currently applied",
            proposal::TextRole::UiBody,
            ColorRole::PrimaryText.resolve(ctx.theme),
        );
        let current_name = applied.map_or("Consumer reference", |p| p.name);
        let current_detail_y = left.min_y() + 12. + left_title_h + 6.;
        let current_detail_h = measure(
            ctx.text,
            current_name,
            proposal::TextRole::UiCaption,
            left_inner_w,
        );
        self.draw_wrapped(
            ctx,
            scene,
            Rect::new(
                left_inner_x,
                current_detail_y,
                left_inner_w,
                current_detail_h,
            ),
            current_name,
            ctx.theme,
        );
        let left_roles_y = current_detail_y + current_detail_h + 10.;
        self.palette_roles(ctx, scene, left, current, None, left_roles_y - left.min_y());

        let right_inner_x = right.min_x() + 14.;
        let right_inner_w = (right.size.width - 28.).max(0.);
        let right_title_h = measure(
            ctx.text,
            right_title,
            proposal::TextRole::UiBody,
            right_inner_w,
        );
        ctx.text.proposal_label_in(
            scene,
            ctx.theme,
            Rect::new(
                right_inner_x,
                right.min_y() + 12.,
                right_inner_w,
                right_title_h,
            ),
            right_title,
            proposal::TextRole::UiBody,
            ColorRole::PrimaryText.resolve(ctx.theme),
        );
        let right_detail = if self.candidate.is_some() {
            preview_preset.map_or_else(
                || "Choose a direction to preview its recommended candidate.".into(),
                |preset| format!("{} · {}", preset.name, preset.seed_srgb),
            )
        } else {
            preview_preset.map_or_else(|| "Consumer reference".into(), |preset| preset.name.into())
        };
        let right_detail_y = right.min_y() + 12. + right_title_h + 6.;
        let right_detail_h = measure(
            ctx.text,
            &right_detail,
            proposal::TextRole::UiCaption,
            right_inner_w,
        );
        self.draw_wrapped(
            ctx,
            scene,
            Rect::new(right_inner_x, right_detail_y, right_inner_w, right_detail_h),
            &right_detail,
            ctx.theme,
        );
        let right_roles_y = right_detail_y + right_detail_h + 10.;
        let roles_bottom = if self.candidate.is_some() {
            if let Some(candidate) = preview_preset {
                self.palette_roles(
                    ctx,
                    scene,
                    right,
                    preview,
                    Some(candidate.seed_srgb),
                    right_roles_y - right.min_y(),
                )
            } else {
                self.palette_roles(
                    ctx,
                    scene,
                    right,
                    preview,
                    None,
                    right_roles_y - right.min_y(),
                )
            }
        } else {
            self.palette_roles(
                ctx,
                scene,
                right,
                preview,
                None,
                right_roles_y - right.min_y(),
            )
        };
        let content_x = right_inner_x;
        let content_w = right_inner_w;
        let mut demo_y = roles_bottom + 14.;
        let action_h = measured_button_height(
            ctx.text,
            "Preview primary action",
            content_w.min(210.),
            36.,
            density,
        );
        self.button(
            ctx,
            scene,
            PREVIEW_ACTION,
            Rect::new(content_x, demo_y, content_w.min(210.), action_h),
            "Preview primary action",
            false,
            preview,
            clip,
            paint::ButtonVariant::Primary,
            density,
        );
        demo_y += action_h + 12.;
        let field_label_h = measure(
            ctx.text,
            "Real text input · click and type",
            proposal::TextRole::UiCaption,
            content_w,
        );
        line(
            ctx,
            scene,
            Rect::new(content_x, demo_y, content_w, field_label_h),
            "Real text input · click and type",
        );
        demo_y += field_label_h + 4.;
        let input_h =
            (proposal::TextRole::UiBody.style().line_height * ctx.text.type_scale() + 16.).max(38.);
        let input_rect = Rect::new(content_x, demo_y, content_w.min(300.), input_h);
        self.input
            .build_search_clipped(ctx, input_rect, clip, scene);
        if let Some(visible) = self.input.visible_frame() {
            self.hits.push((INPUT, visible));
        }
        demo_y = input_rect.max_y() + 12.;
        let overlay_h = measured_button_height(
            ctx.text,
            "Open candidate-themed dialog",
            content_w.min(210.),
            34.,
            density,
        );
        self.button(
            ctx,
            scene,
            PREVIEW_OVERLAY,
            Rect::new(content_x, demo_y, content_w.min(210.), overlay_h),
            "Open candidate-themed dialog",
            false,
            preview,
            clip,
            paint::ButtonVariant::Quiet,
            density,
        );
        demo_y += overlay_h + 8.;
        let can_apply = self.candidate.is_some();
        let candidate_action_h = if can_apply {
            measured_button_height(
                ctx.text,
                "Confirm use",
                ((content_w - 8.) / 2.).max(0.),
                36.,
                density,
            )
            .max(measured_button_height(
                ctx.text,
                "Cancel",
                ((content_w - 8.) / 2.).max(0.),
                36.,
                density,
            ))
        } else {
            measure(
                ctx.text,
                &format!("{} interactive preview activations", self.demo_activations),
                proposal::TextRole::UiCaption,
                content_w,
            )
        };
        if can_apply {
            let half = ((content_w - 8.) / 2.).max(0.);
            self.button(
                ctx,
                scene,
                CONFIRM,
                Rect::new(content_x, demo_y, half, candidate_action_h),
                "Confirm use",
                false,
                preview,
                clip,
                paint::ButtonVariant::Primary,
                density,
            );
            self.button(
                ctx,
                scene,
                CANCEL,
                Rect::new(content_x + half + 8., demo_y, half, candidate_action_h),
                "Cancel",
                false,
                current,
                clip,
                paint::ButtonVariant::Quiet,
                density,
            );
        } else {
            self.draw_wrapped(
                ctx,
                scene,
                Rect::new(content_x, demo_y, content_w, candidate_action_h),
                &format!("{} interactive preview activations", self.demo_activations),
                ctx.theme,
            );
        }
        if self.dialog.is_open() {
            self.dialog_layout = self.dialog.render(
                scene,
                ctx.text,
                "Candidate palette preview",
                frame,
                clip,
                preview,
                proposal::TextRole::UiBody,
                density == 1,
            );
        }
        self.cancel_inactive_buttons();
    }

    #[allow(clippy::too_many_arguments)]
    fn measure_card_height(
        &self,
        text: &mut super::super::text::TextEngine,
        width: f32,
        brand_index: usize,
        preset: Option<&tekes_ui_tokens::palette::GalleryBrandPreset>,
        title: &str,
        controls: bool,
        density: usize,
    ) -> f32 {
        let inner_w = (width - 28.).max(0.);
        let title_h = measure(text, title, proposal::TextRole::UiBody, inner_w);
        let detail = preset.map_or_else(
            || {
                tekes_ui_tokens::palette::gallery_brand_preset(brand_index).map_or_else(
                    || "Consumer reference".to_owned(),
                    |item| item.name.to_owned(),
                )
            },
            |item| {
                if controls && self.candidate.is_some() {
                    format!("{} · {}", item.name, item.seed_srgb)
                } else {
                    item.name.to_owned()
                }
            },
        );
        let detail_h = measure(text, &detail, proposal::TextRole::UiCaption, inner_w);
        let roles_h = role_rows_height(text, inner_w, controls && self.candidate.is_some());
        let mut height = 12. + title_h + 6. + detail_h + 10. + roles_h + 12.;
        if controls {
            let action_h = measured_button_height(
                text,
                "Preview primary action",
                inner_w.min(210.),
                36.,
                density,
            );
            let label_h = measure(
                text,
                "Real text input · click and type",
                proposal::TextRole::UiCaption,
                inner_w,
            );
            let input_h =
                (proposal::TextRole::UiBody.style().line_height * text.type_scale() + 16.).max(38.);
            let overlay_h = measured_button_height(
                text,
                "Open candidate-themed dialog",
                inner_w.min(210.),
                34.,
                density,
            );
            let final_h = if self.candidate.is_some() {
                let half = ((inner_w - 8.) / 2.).max(0.);
                measured_button_height(text, "Confirm use", half, 36., density)
                    .max(measured_button_height(text, "Cancel", half, 36., density))
            } else {
                measure(
                    text,
                    &format!("{} interactive preview activations", self.demo_activations),
                    proposal::TextRole::UiCaption,
                    inner_w,
                )
            };
            height +=
                14. + action_h + 12. + label_h + 4. + input_h + 12. + overlay_h + 8. + final_h;
        }
        height.max(150.)
    }

    fn palette_roles(
        &self,
        ctx: &mut PageCtx<'_>,
        scene: &mut Scene,
        card: Rect,
        palette: Palette,
        seed: Option<&str>,
        y_offset: f32,
    ) -> f32 {
        let roles = ROLE_SAMPLES;
        let width = (card.size.width - 28.).max(0.);
        let swatch_w = width;
        let mut y = card.min_y() + y_offset;
        for (label, foreground, background) in roles {
            let fg = palette
                .color(foreground)
                .unwrap_or(ColorRole::PrimaryText.resolve(ctx.theme));
            let bg = palette.color(background).unwrap_or(Rgba::gray(0.0));
            let sample = Rect::new(card.min_x() + 14., y, swatch_w, 30.);
            let (border_width, border_ink) = paint::decorative_border(palette, ctx.text.scale());
            scene.push_quad(
                Quad::new(sample, bg)
                    .with_corner_radius(proposal::metric("radius", "radius-control").unwrap())
                    .with_border(border_width, border_ink),
            );
            ctx.text.proposal_label_in(
                scene,
                ctx.theme,
                Rect::new(
                    sample.min_x() + 8.,
                    sample.min_y(),
                    sample.size.width - 16.,
                    sample.size.height,
                ),
                label,
                proposal::TextRole::UiCaption,
                fg,
            );
            let caption = format!("{label} · {foreground} / {background}");
            let caption_h = measure(ctx.text, &caption, proposal::TextRole::UiCaption, width);
            self.draw_wrapped(
                ctx,
                scene,
                Rect::new(sample.min_x(), sample.max_y() + 4., width, caption_h),
                &caption,
                ctx.theme,
            );
            y = sample.max_y() + 4. + caption_h + 8.;
        }
        if let Some(seed) = seed {
            let seed_color = parse_hex(seed)
                .unwrap_or_else(|| palette.color("surface-2").unwrap_or(Rgba::gray(0.0)));
            let box_rect = Rect::new(card.min_x() + 14., y, swatch_w, 30.);
            let (border_width, border_ink) = paint::decorative_border(palette, ctx.text.scale());
            scene.push_quad(
                Quad::new(box_rect, seed_color)
                    .with_corner_radius(proposal::metric("radius", "radius-control").unwrap())
                    .with_border(border_width, border_ink),
            );
            let seed_label = format!("Brand seed {seed} (reference only)");
            let seed_h = measure(ctx.text, &seed_label, proposal::TextRole::UiCaption, width);
            self.draw_wrapped(
                ctx,
                scene,
                Rect::new(box_rect.min_x(), box_rect.max_y() + 4., width, seed_h),
                &seed_label,
                ctx.theme,
            );
            y = box_rect.max_y() + 4. + seed_h + 8.;
        }
        y
    }

    #[allow(clippy::too_many_arguments)]
    fn button(
        &mut self,
        ctx: &mut PageCtx<'_>,
        scene: &mut Scene,
        id: usize,
        rect: Rect,
        title: &str,
        selected: bool,
        palette: Palette,
        clip: Option<Rect>,
        variant: paint::ButtonVariant,
        density: usize,
    ) {
        let mut rect = rect;
        if density == 1 {
            rect.size.height = (rect.size.height - 4.).max(24.);
        }
        let button = self.buttons.entry(id).or_default();
        button.state.focused = self.window_active && self.focus == Some(id);
        button.state.focus_visible = button.state.focused && self.focus_visible;
        button.state.selected = selected;
        let layout = ctx.text.render_button(
            scene,
            button,
            title,
            rect,
            clip,
            palette,
            variant,
            ButtonSize::Body,
            proposal::TextRole::UiBody,
            ButtonTextAlign::default(),
        );
        if let Some(visible) = layout.visible {
            self.hits.push((id, visible));
        }
    }

    fn card(&self, ctx: &PageCtx<'_>, scene: &mut Scene, rect: Rect, palette: Palette) {
        paint::surface_kind_at_scale(
            scene,
            rect,
            palette,
            paint::SurfaceKind::Panel,
            ctx.text.scale(),
        );
    }

    fn draw_wrapped(
        &self,
        ctx: &mut PageCtx<'_>,
        scene: &mut Scene,
        rect: Rect,
        text: &str,
        theme: tekes_ui_tokens::Theme,
    ) {
        let height = ctx
            .text
            .proposal_label(text, proposal::TextRole::UiCaption, Some(rect.size.width))
            .height as f32;
        ctx.text.proposal_wrapped_in(
            scene,
            theme,
            Rect::new(
                rect.min_x(),
                rect.min_y(),
                rect.size.width,
                rect.size.height.max(height),
            ),
            text,
            proposal::TextRole::UiCaption,
            ColorRole::SecondaryText.resolve(theme),
        );
    }

    pub(super) fn handle(
        &mut self,
        event: &Event,
        _text: &mut super::super::text::TextEngine,
    ) -> Option<Option<usize>> {
        if self.dialog.is_open() {
            match event {
                Event::Mouse(mouse) => {
                    let point = Point::new(mouse.position.x as f32, mouse.position.y as f32);
                    let event = match mouse.kind {
                        MouseEventKind::Moved | MouseEventKind::Entered => {
                            self.dialog.pointer_move_at(point);
                            DialogEvent::None
                        }
                        MouseEventKind::Down {
                            button: MouseButton::Left,
                            ..
                        } => self.dialog.pointer_down_at(point),
                        MouseEventKind::Up {
                            button: MouseButton::Left,
                            ..
                        } => self.dialog.pointer_up_at(point),
                        _ => DialogEvent::None,
                    };
                    if let DialogEvent::Confirmed { restore_focus }
                    | DialogEvent::Cancelled { restore_focus } = event
                    {
                        self.focus = restore_focus;
                        self.focus_visible = true;
                        return Some(None);
                    }
                    return Some(None);
                }
                Event::Key(key) => {
                    let control = map_key(key.key_code, key.modifiers.shift);
                    if let Some(control) = control {
                        let result = self.dialog.key(control, key.kind == KeyEventKind::Down);
                        if let DialogEvent::Confirmed { restore_focus }
                        | DialogEvent::Cancelled { restore_focus } = result
                        {
                            self.focus = restore_focus;
                            self.focus_visible = true;
                            return Some(None);
                        }
                        return Some(None);
                    }
                    return Some(None);
                }
                Event::Scroll(_) => {
                    if self.dialog.blocks_background() {
                        return Some(None);
                    }
                    return Some(None);
                }
                _ => return Some(None),
            }
        }
        match event {
            Event::Mouse(mouse) => {
                let point = Point::new(mouse.position.x as f32, mouse.position.y as f32);
                match mouse.kind {
                    MouseEventKind::Moved | MouseEventKind::Entered => {
                        let mut changed = false;
                        for button in self.buttons.values_mut() {
                            let was_hovered = button.state.hovered;
                            button.pointer_move_at(point);
                            changed |= was_hovered != button.state.hovered;
                        }
                        if self.input.handle(event, Some(point)).unwrap_or(false) || changed {
                            return Some(None);
                        }
                    }
                    MouseEventKind::Down {
                        button: MouseButton::Left,
                        ..
                    } => {
                        if self.id_at(point) == Some(INPUT) {
                            self.input.handle(event, Some(point));
                            self.focus = Some(INPUT);
                            return Some(None);
                        }
                        if let Some(id) = self.id_at(point) {
                            self.focus = Some(id);
                            self.focus_visible = false;
                            self.input.blur();
                            if let Some(button) = self.buttons.get_mut(&id) {
                                button.pointer_down_at(point);
                            }
                            return Some(None);
                        }
                    }
                    MouseEventKind::Up {
                        button: MouseButton::Left,
                        ..
                    } => {
                        let activated = self
                            .buttons
                            .iter_mut()
                            .find_map(|(id, button)| button.pointer_up_at(point).then_some(*id));
                        if let Some(id) = activated {
                            return self.activate(id);
                        }
                    }
                    _ => {}
                }
            }
            Event::Key(key) => {
                if let Some(ControlKey::Tab { reverse }) =
                    map_key(key.key_code, key.modifiers.shift)
                {
                    if key.kind == KeyEventKind::Down {
                        let mut seen = HashSet::new();
                        let mut ids: Vec<_> = self
                            .hits
                            .iter()
                            .map(|(id, _)| *id)
                            .filter(|id| seen.insert(*id))
                            .collect();
                        if self.candidate.is_none() {
                            ids.retain(|id| ![CONFIRM, CANCEL].contains(id));
                        }
                        let current = self.focus.and_then(|id| ids.iter().position(|n| *n == id));
                        let next = match (current, reverse) {
                            (None, false) => 0,
                            (None, true) => ids.len().saturating_sub(1),
                            (Some(i), false) => (i + 1) % ids.len(),
                            (Some(i), true) => (i + ids.len() - 1) % ids.len(),
                        };
                        self.focus = ids.get(next).copied();
                        self.focus_visible = true;
                        if self.focus == Some(INPUT) {
                            self.input.focus_field();
                        } else {
                            self.input.blur();
                        }
                    }
                    return Some(None);
                }
                let keycode = key.key_code;
                if self.focus == Some(INPUT) {
                    if keycode == 53 && key.kind == KeyEventKind::Down {
                        self.input.blur();
                        self.focus = None;
                        return Some(None);
                    }
                    return self.input.handle(event, None).map(|_| None);
                }
                if let Some(id) = self.focus
                    && let Some(button) = self.buttons.get_mut(&id)
                    && let Some(control) = map_key(keycode, key.modifiers.shift)
                    && button.key(control, key.kind == KeyEventKind::Down)
                {
                    return self.activate(id);
                }
            }
            _ => {}
        }
        let point = match event {
            Event::Scroll(scroll) => Some(Point::new(
                scroll.position.x as f32,
                scroll.position.y as f32,
            )),
            _ => None,
        };
        self.input.handle(event, point).map(|_| None)
    }

    fn activate(&mut self, id: usize) -> Option<Option<usize>> {
        self.input.blur();
        if (FAMILY_FIRST..FAMILY_FIRST + tekes_ui_tokens::palette::gallery_brand_directions().len())
            .contains(&id)
        {
            let direction =
                &tekes_ui_tokens::palette::gallery_brand_directions()[id - FAMILY_FIRST];
            self.direction = Some(direction.id.to_string());
            self.candidate = tekes_ui_tokens::palette::gallery_brand_presets()
                .iter()
                .find(|preset| preset.recommended_for_direction.contains(&direction.id))
                .map(|preset| preset.palette_index);
            return Some(None);
        }
        match id {
            CONFIRM => self.candidate.take().map(Some),
            CANCEL => {
                self.candidate = None;
                Some(None)
            }
            PREVIEW_OVERLAY => {
                self.dialog.show(PREVIEW_OVERLAY);
                self.focus = Some(PREVIEW_OVERLAY);
                self.focus_visible = false;
                Some(None)
            }
            PREVIEW_ACTION => {
                self.demo_activations += 1;
                Some(None)
            }
            _ => None,
        }
    }

    fn id_at(&self, point: Point) -> Option<usize> {
        self.hits
            .iter()
            .rev()
            .find_map(|(id, rect)| rect.contains(point).then_some(*id))
    }

    #[cfg(test)]
    pub(super) fn visible_for_id(&self, id: usize) -> Option<Rect> {
        self.hits
            .iter()
            .find(|(hit_id, _)| *hit_id == id)
            .map(|(_, rect)| *rect)
    }

    #[cfg(test)]
    pub(super) fn dialog_layout(&self) -> Option<&tekes_ui_controls::DialogLayout> {
        self.dialog_layout.as_ref()
    }

    #[cfg(test)]
    pub(super) fn input_value(&self) -> &str {
        self.input.query()
    }

    pub(super) fn text_state(&self) -> Option<tekes_ui_platform_macos::TextInputState> {
        if self.dialog.is_open() {
            None
        } else {
            self.input.state()
        }
    }

    pub(super) fn text_command(
        &mut self,
        command: &str,
        clipboard: Option<&str>,
        text: &mut super::super::text::TextEngine,
    ) -> (bool, Option<String>) {
        if self.dialog.is_open() {
            return (false, None);
        }
        let mut copied = None;
        let changed = match command {
            "copy:" => {
                copied = self.input.clipboard_selection();
                false
            }
            "cut:" => {
                copied = self.input.clipboard_selection();
                self.input.cut()
            }
            "paste:" => self.input.paste(clipboard.unwrap_or("")),
            _ => self.input.command(command).unwrap_or(false),
        };
        self.input.refresh(text);
        (changed, copied)
    }

    pub(super) fn set_window_active(&mut self, active: bool) {
        self.window_active = active;
        self.input.set_window_active(active);
        self.dialog.set_window_active(active);
        if !active {
            self.dialog.deactivate();
            self.input.blur();
            self.focus = None;
            self.focus_visible = false;
            for button in self.buttons.values_mut() {
                button.blur();
            }
            self.hits.clear();
            self.dialog_layout = None;
        }
    }

    pub(super) fn set_reduced_motion(&mut self, reduced: bool) {
        self.input.set_reduced_motion(reduced);
    }

    pub(super) fn needs_animation(&self) -> bool {
        !self.dialog.is_open() && self.input.needs_animation()
    }

    pub(super) fn animate(&mut self, dt: f32) -> bool {
        !self.dialog.is_open() && self.input.animate(dt)
    }

    pub(super) fn blur(&mut self) {
        self.input.blur();
        self.focus = None;
        self.focus_visible = false;
        if self.dialog.is_open() {
            let _ = self.dialog.close();
        }
        self.dialog_layout = None;
        self.hits.clear();
        for button in self.buttons.values_mut() {
            button.blur();
        }
    }

    fn cancel_inactive_buttons(&mut self) {
        let active: HashSet<_> = self.hits.iter().map(|(id, _)| *id).collect();
        for (id, button) in &mut self.buttons {
            if !active.contains(id) {
                button.cancel();
            }
        }
    }

    pub(super) fn use_fixture_cache(&mut self) {
        self.input.use_fixture_cache();
    }

    #[cfg(test)]
    pub(super) fn focus_for_test(&self) -> Option<usize> {
        self.focus
    }

    #[cfg(test)]
    pub(super) fn candidate_for_test(&self) -> Option<usize> {
        self.candidate
    }

    #[cfg(test)]
    pub(super) fn target_center_for_test(&self, id: usize) -> Option<Point> {
        self.visible_for_id(id).map(|rect| rect.center())
    }
}

fn map_key(key: u16, shift: bool) -> Option<ControlKey> {
    match key {
        36 => Some(ControlKey::Enter),
        49 => Some(ControlKey::Space),
        123 => Some(ControlKey::Left),
        124 => Some(ControlKey::Right),
        125 => Some(ControlKey::Down),
        126 => Some(ControlKey::Up),
        53 => Some(ControlKey::Escape),
        48 => Some(ControlKey::Tab { reverse: shift }),
        _ => None,
    }
}

fn parse_hex(hex: &str) -> Option<Rgba> {
    if hex.len() != 7 || !hex.starts_with('#') {
        return None;
    }
    let values = [1, 3, 5].map(|index| u8::from_str_radix(&hex[index..index + 2], 16).ok());
    Some(Rgba::rgb8(values[0]?, values[1]?, values[2]?))
}

fn line(ctx: &mut PageCtx<'_>, scene: &mut Scene, rect: Rect, text: &str) {
    ctx.text.proposal_label_in(
        scene,
        ctx.theme,
        rect,
        text,
        proposal::TextRole::UiCaption,
        ColorRole::SecondaryText.resolve(ctx.theme),
    );
}

fn measure(
    text: &mut super::super::text::TextEngine,
    value: &str,
    role: proposal::TextRole,
    width: f32,
) -> f32 {
    text.proposal_label(value, role, Some(width.max(0.))).height as f32
}

fn measured_button_height(
    text: &mut super::super::text::TextEngine,
    title: &str,
    width: f32,
    minimum: f32,
    density: usize,
) -> f32 {
    let measured = text
        .measure_button(
            title,
            width.max(0.),
            ButtonSize::Body,
            proposal::TextRole::UiBody,
        )
        .height;
    measured
        .max(minimum - if density == 1 { 4. } else { 0. })
        .max(24.)
}

fn role_rows_height(
    text: &mut super::super::text::TextEngine,
    width: f32,
    include_seed: bool,
) -> f32 {
    let roles = ROLE_SAMPLES;
    let mut height = 0.;
    for (label, foreground, background) in roles {
        let caption = format!("{label} · {foreground} / {background}");
        height += 30. + 4. + measure(text, &caption, proposal::TextRole::UiCaption, width) + 8.;
    }
    if include_seed {
        let caption = "Brand seed #FFFFFF (reference only)";
        height += 30. + 4. + measure(text, caption, proposal::TextRole::UiCaption, width) + 8.;
    }
    height
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn narrow_large_text_layout_measures_to_all_interactive_controls() {
        let mut preview = BrandPreview {
            direction: Some("red".into()),
            candidate: tekes_ui_tokens::palette::gallery_brand_presets()
                .iter()
                .find(|preset| preset.id == "rose")
                .map(|preset| preset.palette_index),
            ..Default::default()
        };

        let mut text = super::super::super::text::TextEngine::new();
        text.set_type_scale(1.5);
        let width = 320.0;
        let extent = preview.document_extent(&mut text, width, 0, 0);
        let mut static_text = super::super::super::static_text::StaticText::new(None);
        let mut ctx = super::super::super::pages::PageCtx {
            theme: tekes_ui_tokens::Theme::LIGHT,
            text: &mut text,
            static_text: &mut static_text,
        };
        let frame = Rect::new(24.0, 40.0, width, extent);
        let mut scene = Scene::new();
        preview.build(&mut ctx, &mut scene, frame, Some(frame), 0, 0);

        let controls = [PREVIEW_ACTION, INPUT, PREVIEW_OVERLAY, CONFIRM, CANCEL]
            .map(|id| preview.visible_for_id(id).expect("control stays reachable"));
        for (index, rect) in controls.iter().enumerate() {
            assert!(rect.min_y() >= frame.min_y());
            assert!(rect.max_y() <= frame.max_y() + 0.1);
            for other in controls.iter().skip(index + 1) {
                assert!(
                    rect.intersection(other)
                        .is_none_or(|intersection| intersection.is_empty())
                );
            }
        }
        assert!(
            extent > 600.0,
            "wrapped captions and controls contribute to scroll extent"
        );
    }
}
