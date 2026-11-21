//! Feedback stories use the receiving ContentHost; visual fixtures share the original painters.
use super::{Entry, StoryState};
use tekes_ui_controls::{
    ButtonModel, ButtonTextAlign, ButtonTextRenderer, ControlKey,
    composition::*,
    extended::{self as controls, Axis, Banner, EmptyState, Popover, Stack, Toast, Tooltip},
    feedback_view::{
        BannerView, DialogView, EmptyStateView, MenuView, SurfaceView, ToastView, TooltipView,
    },
    layout::MeasuredChild,
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_tokens::{
    ColorRole, Palette, Rgba, Theme,
    proposal::{self, TextRole},
};

pub(super) struct FixtureRenderer<'a>(pub &'a mut dyn ButtonTextRenderer);
impl ButtonTextRenderer for FixtureRenderer<'_> {
    fn measure(&mut self, v: &str, r: TextRole, w: f32) -> Size {
        self.0.measure(v, r, w)
    }
    fn draw(&mut self, s: &mut Scene, v: &str, r: TextRole, f: Rect, a: ButtonTextAlign, c: Rgba) {
        self.0.draw(s, v, r, f, a, c)
    }
    fn backing_scale(&self) -> f32 {
        self.0.backing_scale()
    }
    fn resolved_font_size(&mut self, r: TextRole) -> f32 {
        self.0.resolved_font_size(r)
    }
    fn first_line_height(&mut self, v: &str, r: TextRole, w: f32) -> f32 {
        self.0.first_line_height(v, r, w)
    }
    fn animation_frames_allowed(&self) -> bool {
        false
    }
}
impl FixtureRenderer<'_> {
    fn proposal_wrapped_in(
        &mut self,
        s: &mut Scene,
        _: Theme,
        f: Rect,
        v: &str,
        r: TextRole,
        c: Rgba,
    ) {
        self.draw(s, v, r, f, ButtonTextAlign::Leading, c)
    }
    fn proposal_label_in(
        &mut self,
        s: &mut Scene,
        t: Theme,
        f: Rect,
        v: &str,
        r: TextRole,
        c: Rgba,
    ) {
        self.proposal_wrapped_in(s, t, f, v, r, c)
    }
}
pub(super) struct FixtureContext<'a, 'b> {
    pub text: &'a mut FixtureRenderer<'b>,
    pub theme: Theme,
}
pub(super) fn matrix_heading(
    ctx: &mut FixtureContext<'_, '_>,
    scene: &mut Scene,
    bounds: Rect,
    y: f32,
    title: &str,
) -> f32 {
    let role = proposal::TextRole::UiBody;
    let size = ctx.text.measure(title, role, bounds.size.width.max(0.0));
    ctx.text.proposal_wrapped_in(
        scene,
        ctx.theme,
        Rect::new(
            bounds.min_x(),
            y,
            bounds.size.width.max(0.0),
            size.height.max(20.0),
        ),
        title,
        role,
        ColorRole::PrimaryText.resolve(ctx.theme),
    );
    size.height.max(20.0)
}

pub(super) fn bounded_fixture_width(width: f32, maximum: f32) -> f32 {
    // Preserve the previous min/max behavior for a malformed NaN viewport:
    // f32::min(NaN, maximum) selects maximum, while clamp(NaN, ..) stays NaN.
    if width.is_nan() {
        maximum
    } else {
        width.clamp(0.0, maximum)
    }
}

pub(super) fn matrix_clip(ctx: &FixtureContext<'_, '_>, bounds: Rect) -> Rect {
    let gutter = -controls::ControlFrame::new(Rect::new(0., 0., 1., 1.), None)
        .focus_bounds(ctx.text.backing_scale())
        .min_x();
    Rect::new(
        bounds.min_x() - gutter,
        bounds.min_y() - gutter,
        bounds.size.width + gutter * 2.0,
        bounds.size.height + gutter * 2.0,
    )
}

pub(super) fn matrix_button(
    ctx: &mut FixtureContext<'_, '_>,
    scene: &mut Scene,
    frame: Rect,
    clip: Rect,
    palette: Palette,
    label: &str,
    focused: bool,
) {
    let mut button = ButtonModel::default();
    button.state.focused = focused;
    button.state.focus_visible = focused;
    button.render(
        scene,
        ctx.text,
        label,
        frame,
        Some(clip),
        palette,
        tekes_ui_controls::ButtonVariant::Quiet,
        tekes_ui_controls::ButtonSize::Body,
        proposal::TextRole::UiBody,
        tekes_ui_controls::ButtonTextAlign::Center,
    );
}

pub(super) fn matrix_tooltip(
    ctx: &mut FixtureContext<'_, '_>,
    scene: &mut Scene,
    bounds: Rect,
    y: &mut f32,
    palette: Palette,
) {
    let clip = matrix_clip(ctx, bounds);
    let width = bounded_fixture_width(bounds.size.width, 420.0);
    for (title, interaction) in [
        ("Rest · hidden", 0),
        ("Hover · shown", 1),
        ("Focus-visible · shown", 2),
        ("Escape · dismissed until leave", 3),
    ] {
        *y += matrix_heading(ctx, scene, bounds, *y, title) + 6.0;
        let button_size = ButtonModel::measure(
            ctx.text,
            "Hover or focus me",
            width.min(190.0),
            tekes_ui_controls::ButtonSize::Body,
            proposal::TextRole::UiBody,
        );
        let anchor = Rect::new(
            bounds.min_x(),
            *y,
            button_size.width.max(160.0).min(width),
            button_size.height.max(36.0),
        );
        matrix_button(
            ctx,
            scene,
            anchor,
            clip,
            palette,
            "Hover or focus me",
            interaction == 2,
        );
        let tip_text = "Tooltip content wraps naturally at the current preview width.";
        let tip_width = width.min(240.0);
        let tip_text_height = ctx
            .text
            .measure(
                tip_text,
                proposal::TextRole::UiBody,
                (tip_width - 24.0).max(0.0),
            )
            .height;
        let tip_size = Size::new(tip_width, tip_text_height + 16.0);
        let model = Tooltip {
            visible: false,
            text: tip_text.into(),
        };
        let mut view = TooltipView::default();
        if interaction == 1 || interaction == 3 {
            let mut probe = Scene::new();
            view.render(
                &mut probe,
                ctx.text,
                &model,
                anchor,
                tip_size,
                clip,
                Some(clip),
                palette,
                proposal::TextRole::UiBody,
            );
            view.pointer_move(Point::new(anchor.min_x() + 8.0, anchor.min_y() + 8.0));
        }
        if interaction == 2 {
            view.set_focus(true, true);
        }
        if interaction == 3 {
            let _ = view.key(ControlKey::Escape);
        }
        view.render(
            scene,
            ctx.text,
            &model,
            anchor,
            tip_size,
            clip,
            Some(clip),
            palette,
            proposal::TextRole::UiBody,
        );
        *y = (*y + button_size.height.max(36.0) + 54.0).max(view.layout().frame.max_y()) + 15.0;
    }
}

pub(super) fn matrix_popover(
    ctx: &mut FixtureContext<'_, '_>,
    scene: &mut Scene,
    bounds: Rect,
    y: &mut f32,
    palette: Palette,
) {
    let clip = matrix_clip(ctx, bounds);
    let width = bounded_fixture_width(bounds.size.width, 420.0);
    for (title, open, focused) in [
        ("Closed", false, false),
        ("Open", true, false),
        ("Open · dismiss action focus-visible", true, true),
    ] {
        *y += matrix_heading(ctx, scene, bounds, *y, title) + 6.0;
        let button_size = ButtonModel::measure(
            ctx.text,
            "More options",
            180.0_f32.min(width),
            tekes_ui_controls::ButtonSize::Body,
            proposal::TextRole::UiBody,
        );
        let anchor = Rect::new(
            bounds.min_x(),
            *y,
            button_size.width.max(140.0).min(width),
            button_size.height.max(36.0),
        );
        matrix_button(ctx, scene, anchor, clip, palette, "More options", false);
        let mut model = Popover::default();
        if open {
            model.show(20);
        }
        let mut view = tekes_ui_controls::feedback_view::PopoverView::default();
        if open {
            let size = tekes_ui_controls::feedback_view::PopoverView::measure(
                ctx.text,
                Size::new((width.min(232.0) - 28.0).max(0.0), 24.0),
                proposal::TextRole::UiBody,
            );
            if focused {
                let mut probe = Scene::new();
                view.render_with_text_content(
                    &mut probe,
                    ctx.text,
                    &model,
                    anchor,
                    size,
                    clip,
                    Some(clip),
                    palette,
                    proposal::TextRole::UiBody,
                    "Gallery popover focus geometry",
                    |scene, text, slot| {
                        text.proposal_label_in(
                            scene,
                            ctx.theme,
                            slot,
                            "Popover content",
                            proposal::TextRole::UiBody,
                            ColorRole::PrimaryText.resolve(ctx.theme),
                        );
                    },
                );
                view.focus_dismiss(true, true);
            }
            view.render_with_text_content(
                scene,
                ctx.text,
                &model,
                anchor,
                size,
                clip,
                Some(clip),
                palette,
                proposal::TextRole::UiBody,
                "Gallery popover state",
                |scene, text, slot| {
                    text.proposal_label_in(
                        scene,
                        ctx.theme,
                        slot,
                        "Popover content",
                        proposal::TextRole::UiBody,
                        ColorRole::PrimaryText.resolve(ctx.theme),
                    );
                },
            );
            *y = view.layout().frame.max_y() + 15.0;
        } else {
            *y += anchor.size.height + 15.0;
        }
    }
}

pub(super) fn matrix_toast(
    ctx: &mut FixtureContext<'_, '_>,
    scene: &mut Scene,
    bounds: Rect,
    y: &mut f32,
    palette: Palette,
) {
    let clip = matrix_clip(ctx, bounds);
    let width = bounded_fixture_width(bounds.size.width, 500.0);
    for (title, visible, focused) in [
        ("Visible · default", true, false),
        ("Visible · dismiss action focus-visible", true, true),
        ("Dismissed", false, false),
    ] {
        *y += matrix_heading(ctx, scene, bounds, *y, title) + 6.0;
        let message = if visible {
            "Changes saved"
        } else {
            "Toast dismissed."
        };
        let mut model = Toast::new(message, controls::NoticeTone::Success, 5000);
        model.paused = true;
        model.visible = visible;
        let size = ToastView::measure(ctx.text, message, width, proposal::TextRole::UiBody);
        let frame = Rect::new(bounds.min_x(), *y, width, size.height);
        let mut view = ToastView::default();
        if visible && focused {
            let mut probe = Scene::new();
            view.render(
                &mut probe,
                ctx.text,
                &model,
                frame,
                Some(clip),
                palette,
                proposal::TextRole::UiBody,
            );
            view.focus_dismiss(true, true);
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
        *y += size.height + 14.0;
    }
}

pub(super) fn matrix_banner(
    ctx: &mut FixtureContext<'_, '_>,
    scene: &mut Scene,
    bounds: Rect,
    y: &mut f32,
    palette: Palette,
) {
    let clip = matrix_clip(ctx, bounds);
    let width = bounded_fixture_width(bounds.size.width, 600.0);
    for (title, visible, dismissible, focused) in [
        ("Visible · dismissible", true, true, false),
        ("Dismiss action focus-visible", true, true, true),
        ("Visible · persistent", true, false, false),
        ("Dismissed", false, true, false),
    ] {
        *y += matrix_heading(ctx, scene, bounds, *y, title) + 6.0;
        let model = Banner {
            message: "Your changes are saved. You can continue working.".into(),
            tone: controls::NoticeTone::Info,
            dismissible,
            visible,
        };
        let size = BannerView::measure(
            ctx.text,
            &model.message,
            width,
            dismissible,
            proposal::TextRole::UiBody,
        );
        let frame = Rect::new(bounds.min_x(), *y, width, size.height);
        let mut view = BannerView::default();
        if focused {
            let mut probe = Scene::new();
            view.render(
                &mut probe,
                ctx.text,
                &model,
                frame,
                Some(clip),
                palette,
                proposal::TextRole::UiBody,
            );
            view.focus_dismiss(true, true);
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
        *y += size.height.max(30.0) + 14.0;
    }
}

pub(super) fn matrix_empty_state(
    ctx: &mut FixtureContext<'_, '_>,
    scene: &mut Scene,
    bounds: Rect,
    y: &mut f32,
    palette: Palette,
) {
    let clip = matrix_clip(ctx, bounds);
    let width = bounded_fixture_width(bounds.size.width, 520.0);
    for (title, action, disabled, focused) in [
        ("Action enabled", Some("Create document"), false, false),
        ("Action focus-visible", Some("Create document"), false, true),
        ("Action disabled", Some("Create document"), true, false),
        ("No action", None, false, false),
    ] {
        *y += matrix_heading(ctx, scene, bounds, *y, title) + 6.0;
        let model = EmptyState {
            title: "No documents yet".into(),
            detail: "Create a document to begin your workspace.".into(),
            action: action.map(str::to_owned),
        };
        let height = empty_state_matrix_height(ctx, &model, width);
        let frame = Rect::new(bounds.min_x(), *y, width, height);
        let mut surface = SurfaceView::default();
        let mut probe = Scene::new();
        let panel = surface.render_at_scale(
            &mut probe,
            frame,
            Some(clip),
            palette,
            true,
            ctx.text.backing_scale(),
        );
        let mut view = EmptyStateView::default();
        view.set_disabled(disabled);
        if focused {
            view.render(
                &mut probe,
                ctx.text,
                &model,
                panel.content,
                Some(clip),
                palette,
                proposal::TextRole::UiBody,
            );
            view.focus_action(true, true);
        }
        let panel = surface.render_at_scale(
            scene,
            frame,
            Some(clip),
            palette,
            true,
            ctx.text.backing_scale(),
        );
        view.render(
            scene,
            ctx.text,
            &model,
            panel.content,
            Some(clip),
            palette,
            proposal::TextRole::UiBody,
        );
        *y += height + 14.0;
    }
}

pub(super) fn matrix_toast_variants(
    ctx: &mut FixtureContext<'_, '_>,
    scene: &mut Scene,
    bounds: Rect,
    y: &mut f32,
    palette: Palette,
) {
    let clip = matrix_clip(ctx, bounds);
    let width = bounded_fixture_width(bounds.size.width, 520.0);
    for (label, tone) in [
        ("Info", controls::NoticeTone::Info),
        ("Success", controls::NoticeTone::Success),
        ("Warning", controls::NoticeTone::Warning),
        ("Error", controls::NoticeTone::Error),
    ] {
        *y += matrix_heading(ctx, scene, bounds, *y, label) + 6.0;
        let mut model = Toast::new("Changes saved", tone, 5000);
        model.paused = true;
        let size = ToastView::measure(ctx.text, &model.message, width, proposal::TextRole::UiBody);
        let frame = Rect::new(bounds.min_x(), *y, width, size.height);
        ToastView::default().render(
            scene,
            ctx.text,
            &model,
            frame,
            Some(clip),
            palette,
            proposal::TextRole::UiBody,
        );
        *y += size.height + 12.0;
    }
}

pub(super) fn matrix_banner_variants(
    ctx: &mut FixtureContext<'_, '_>,
    scene: &mut Scene,
    bounds: Rect,
    y: &mut f32,
    palette: Palette,
) {
    let clip = matrix_clip(ctx, bounds);
    let width = bounded_fixture_width(bounds.size.width, 600.0);
    for (label, tone) in [
        ("Info", controls::NoticeTone::Info),
        ("Success", controls::NoticeTone::Success),
        ("Warning", controls::NoticeTone::Warning),
        ("Error", controls::NoticeTone::Error),
    ] {
        *y += matrix_heading(ctx, scene, bounds, *y, label) + 6.0;
        let model = Banner {
            message: "Your changes are saved. You can continue working.".into(),
            tone,
            dismissible: true,
            visible: true,
        };
        let size = BannerView::measure(
            ctx.text,
            &model.message,
            width,
            model.dismissible,
            proposal::TextRole::UiBody,
        );
        let frame = Rect::new(bounds.min_x(), *y, width, size.height);
        BannerView::default().render(
            scene,
            ctx.text,
            &model,
            frame,
            Some(clip),
            palette,
            proposal::TextRole::UiBody,
        );
        *y += size.height + 12.0;
    }
}

pub(super) fn matrix_empty_variants(
    ctx: &mut FixtureContext<'_, '_>,
    scene: &mut Scene,
    bounds: Rect,
    y: &mut f32,
    palette: Palette,
) {
    let clip = matrix_clip(ctx, bounds);
    for (title, action) in [
        ("With primary action", Some("Create document")),
        ("Without action", None),
    ] {
        *y += matrix_heading(ctx, scene, bounds, *y, title) + 6.0;
        let model = EmptyState {
            title: "No documents yet".into(),
            detail: "Create a document to begin your workspace.".into(),
            action: action.map(str::to_owned),
        };
        let width = bounded_fixture_width(bounds.size.width, 520.0);
        let height = empty_state_matrix_height(ctx, &model, width);
        let frame = Rect::new(bounds.min_x(), *y, width, height);
        let mut surface = SurfaceView::default();
        let panel = surface.render_at_scale(
            scene,
            frame,
            Some(clip),
            palette,
            true,
            ctx.text.backing_scale(),
        );
        EmptyStateView::default().render(
            scene,
            ctx.text,
            &model,
            panel.content,
            Some(clip),
            palette,
            proposal::TextRole::UiBody,
        );
        *y += height + 14.0;
    }
}

pub(super) fn empty_state_matrix_height(
    ctx: &mut FixtureContext<'_, '_>,
    model: &EmptyState,
    width: f32,
) -> f32 {
    let inset = proposal::metric("spacing", "space-loose").unwrap_or(16.0);
    EmptyStateView::measure(
        ctx.text,
        model,
        (width - 2.0 * inset).max(0.0),
        proposal::TextRole::UiBody,
    )
    .height
        + 2.0 * inset
}

pub(crate) fn supports(renderer: &str) -> bool {
    matches!(
        renderer,
        "tooltip" | "popover" | "toast" | "banner" | "empty-state" | "menu" | "dialog"
    )
}
const ROOT: ComponentId = ComponentId(103000);
const ROWS: ComponentId = ComponentId(103001);
const BODY: ComponentId = ComponentId(103002);
const LOG: ComponentId = ComponentId(103003);
const CONTROL: ComponentId = ComponentId(103020);
const ACTION: ComponentId = ComponentId(103021);
const OVERLAY: ComponentId = ComponentId(103030);
const DIALOG_VIEWPORT: ComponentId = ComponentId(103040);
const COMMAND: u64 = 103100;
fn text(id: u64, value: impl Into<String>) -> Content {
    Content::new(ComponentId(id), TextContent::new(value))
}
fn stack(id: ComponentId, children: Vec<Content>) -> Content {
    Content::new(
        id,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 12.,
            },
            flexible_child: None,
            children,
        },
    )
}
#[derive(Debug)]
struct Matrix {
    renderer: &'static str,
    variants: bool,
}
impl Matrix {
    fn render(
        &self,
        scene: &mut Scene,
        text: &mut dyn ButtonTextRenderer,
        bounds: Rect,
        theme: ContentTheme,
    ) -> f32 {
        let mut renderer = FixtureRenderer(text);
        let mut ctx = FixtureContext {
            text: &mut renderer,
            theme: theme.palette.theme,
        };
        let mut y = bounds.min_y();
        let palette = theme.palette;
        match (self.renderer, self.variants) {
            ("tooltip", _) => matrix_tooltip(&mut ctx, scene, bounds, &mut y, palette),
            ("popover", _) => matrix_popover(&mut ctx, scene, bounds, &mut y, palette),
            ("toast", false) => matrix_toast(&mut ctx, scene, bounds, &mut y, palette),
            ("toast", true) => matrix_toast_variants(&mut ctx, scene, bounds, &mut y, palette),
            ("banner", false) => matrix_banner(&mut ctx, scene, bounds, &mut y, palette),
            ("banner", true) => matrix_banner_variants(&mut ctx, scene, bounds, &mut y, palette),
            ("empty-state", false) => matrix_empty_state(&mut ctx, scene, bounds, &mut y, palette),
            ("empty-state", true) => {
                matrix_empty_variants(&mut ctx, scene, bounds, &mut y, palette)
            }
            ("menu" | "dialog", _) => {
                let titles = if self.renderer == "menu" {
                    [
                        "Open menu",
                        "Keyboard next item",
                        "Pressed first item",
                        "Disabled menu items",
                    ]
                } else {
                    [
                        "Open dialog",
                        "Keyboard focus",
                        "Pressed confirm",
                        "Disabled dialog actions",
                    ]
                };
                for (n, title) in titles.into_iter().enumerate() {
                    y += matrix_heading(&mut ctx, scene, bounds, y, title) + 8.;
                    let body = Rect::new(bounds.min_x(), y, bounds.size.width, 230.);
                    let mut probe = Scene::new();
                    if self.renderer == "menu" {
                        let mut view = MenuView::new(3);
                        view.set_reduced_motion(true);
                        view.show(0);
                        if n == 1 {
                            view.key(ControlKey::Down, true);
                        }
                        view.set_disabled(n == 3);
                        let anchor =
                            Rect::new(body.min_x(), body.min_y(), body.size.width.min(230.), 1.);
                        if let Some(layout) = view.render(
                            &mut probe,
                            ctx.text,
                            &["Open", "Duplicate", "Archive"],
                            anchor,
                            body,
                            Some(body),
                            palette,
                            TextRole::UiBody,
                            false,
                        ) && n == 2
                        {
                            view.pointer_down_at(layout.items[0].frame.center());
                        }
                        view.render(
                            scene,
                            ctx.text,
                            &["Open", "Duplicate", "Archive"],
                            anchor,
                            body,
                            Some(body),
                            palette,
                            TextRole::UiBody,
                            false,
                        );
                    } else {
                        let mut view = DialogView::default();
                        view.show(0);
                        if n == 1 {
                            view.key(ControlKey::Tab { reverse: false }, true);
                        }
                        view.set_disabled(n == 3);
                        if let Some(layout) = view.render(
                            &mut probe,
                            ctx.text,
                            "Confirm changes?",
                            body,
                            Some(body),
                            palette,
                            TextRole::UiBody,
                            false,
                        ) && n == 2
                        {
                            view.pointer_down_at(layout.buttons[1].frame.center());
                        }
                        view.render(
                            scene,
                            ctx.text,
                            "Confirm changes?",
                            body,
                            Some(body),
                            palette,
                            TextRole::UiBody,
                            false,
                        );
                    }
                    y = body.max_y() + 24.;
                }
            }
            _ => unreachable!("registered feedback fixture"),
        }
        y - bounds.min_y()
    }
}
impl Component for Matrix {
    fn measure(&mut self, text: &mut dyn ButtonTextRenderer, available: Size) -> MeasuredChild {
        let height = self.render(
            &mut Scene::new(),
            text,
            Rect::new(0., 0., available.width, 100_000.),
            ContentTheme::from(Palette::from(Theme::LIGHT)),
        );
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
        scene.push_clip(clip);
        self.render(scene, text, frame, theme);
        scene.pop_clip();
    }
    fn semantics(&self) -> ContentSemantics {
        ContentSemantics::new(
            ContentRole::Group,
            "Visual state specimens; Playground owns interaction",
        )
    }
}

pub(crate) struct FeedbackStory {
    entry: &'static Entry,
    state: StoryState,
    dialog_open: bool,
    popover_session: Option<PopoverSession>,
    pub(crate) host: ContentHost,
}
impl FeedbackStory {
    pub(crate) fn new(entry: &'static Entry) -> Self {
        let state = StoryState::for_entry(entry);
        let intro = Content::new(
            ComponentId(103010),
            StackContent {
                model: Stack {
                    axis: Axis::Vertical,
                    gap: StackContent::SECTION_HEADER_GAP,
                },
                children: vec![
                    super::story_heading(103004, entry.title()),
                    text(103005, super::story_docs(entry)),
                ],
                flexible_child: None,
            },
        );
        let children = vec![
            intro,
            super::apphost_basic::section_bar(entry, COMMAND),
            Self::body(entry, &state),
        ];
        let document = Content::new(
            ROOT,
            ScrollContent::document(StackContent::document_sections(ROWS, children)),
        );
        let root = if entry.renderer() == "dialog" {
            super::apphost_layout::dialog_viewport(DIALOG_VIEWPORT, vec![document, Self::dialog()])
        } else {
            document
        };
        let mut host = ContentHost::new(root).expect("unique feedback IDs");
        host.set_action_events(true);
        let mut story = Self {
            entry,
            state,
            dialog_open: false,
            popover_session: None,
            host,
        };
        story.apply_disabled();
        story
    }
    fn dialog() -> Content {
        Content::new(
            OVERLAY,
            DialogContent::new(
                "Confirm changes?",
                text(103031, "Confirm changes?"),
                text(103032, "Review your changes before continuing."),
                DialogContent::actions_row(
                    ComponentId(103033),
                    vec![
                        Content::new(ACTION, Button::new("Cancel")),
                        Content::new(ComponentId(103022), Button::new("Confirm")),
                    ],
                ),
            ),
        )
    }
    fn body(entry: &'static Entry, state: &StoryState) -> Content {
        let mut children = vec![Content::new(
            ComponentId(103006),
            TextContent::heading(
                [
                    "Docs",
                    "Playground",
                    "State matrix",
                    "Behaviors",
                    "Edge cases",
                    "Variants / sizes",
                ][state.section],
            ),
        )];
        if state.section == 0 {
            children.push(text(
                103007,
                format!(
                    "Component: {}\nUsage: {}",
                    entry.component(),
                    super::api_reference(entry)
                ),
            ));
            for (i, line) in super::contract_summary(entry).into_iter().enumerate() {
                children.push(text(103500 + i as u64, line));
            }
        } else if state.section == 2 || state.section == 5 {
            children.push(text(103007,"Production-view specimens. Use Playground for real pointer, keyboard and focus behavior."));
            children.push(Content::new(
                ComponentId(103008),
                Matrix {
                    renderer: entry.renderer(),
                    variants: state.section == 5,
                },
            ));
        } else {
            if state.section == 3 {
                for (i, line) in super::contract_summary(entry).into_iter().enumerate() {
                    children.push(text(103500 + i as u64, line));
                }
            }
            if matches!(entry.renderer(), "menu" | "dialog") {
                children.push(Content::new(
                    ComponentId(COMMAND + 10),
                    Button::new(if state.disabled {
                        "Disabled: on"
                    } else {
                        "Disabled: off"
                    }),
                ));
            }
            match entry.renderer() {
                "tooltip" => {
                    children.push(
                        Content::new(CONTROL, Button::new("Hover or focus me"))
                            .with_help(state.extended.tooltip.text.clone()),
                    );
                    children.push(text(
                        103009,
                        "Tooltip appears on hover or focus-visible. Escape dismisses it.",
                    ));
                }
                "popover" => {
                    children.push(Content::new(CONTROL, Button::new("More options")));
                    children.push(Content::new(
                        OVERLAY,
                        PopoverContent::new("Popover", text(103031, "Popover content")),
                    ));
                    children.push(text(
                        103009,
                        "Escape or outside click dismisses and restores the invoker focus.",
                    ));
                }
                "toast" => {
                    children.push(Content::new(
                        CONTROL,
                        ToastContent::new("Toast", state.extended.toast.clone(), ACTION).unwrap(),
                    ));
                    children.push(Content::new(
                        ComponentId(103022),
                        Button::new(if state.extended.toast.visible {
                            "Advance fixture timer"
                        } else {
                            "Show again"
                        }),
                    ));
                    children.push(text(
                        103023,
                        format!("{} ms remaining", state.extended.toast.remaining_ms()),
                    ));
                    children.push(text(
                        103009,
                        "Time follows the Host frame clock; hover pauses the original Toast.",
                    ));
                }
                "banner" => {
                    children.push(Content::new(
                        CONTROL,
                        BannerContent::new("Banner", state.extended.banner.clone(), ACTION)
                            .unwrap(),
                    ));
                }
                "empty-state" => {
                    children.push(Content::new(
                        ComponentId(103024),
                        SurfaceContent::canvas(Content::new(
                            CONTROL,
                            EmptyStateContent::new(
                                "Documents",
                                empty_model(state.extended.empty_state_created),
                                ACTION,
                            )
                            .unwrap(),
                        ))
                        .with_border(true)
                        .with_max_width(520.)
                        .with_min_height(164.),
                    ));
                }
                "menu" => {
                    children.push(Content::new(
                        CONTROL,
                        MenuContent::new(
                            "Open action menu",
                            ["Open", "Duplicate", "Archive"].map(str::to_owned).to_vec(),
                        ),
                    ));
                }
                "dialog" => {
                    children.push(Content::new(CONTROL, Button::new("Open dialog")));
                }
                _ => unreachable!(),
            }
            children.push(text(
                LOG.0,
                state.log.iter().cloned().collect::<Vec<_>>().join("\n"),
            ));
        }
        stack(BODY, children)
    }
    fn apply_disabled(&mut self) {
        if !matches!(self.entry.renderer(), "menu" | "dialog") {
            return;
        }
        if let Some(node) = self.host.get_mut(CONTROL) {
            if let Some(button) = node.component_mut::<Button>() {
                button.button.state.disabled = self.state.disabled;
            }
            if let Some(menu) = node.component_mut::<MenuContent>() {
                menu.trigger.button.state.disabled = self.state.disabled;
                menu.menu.set_disabled(self.state.disabled);
            }
        }
        if let Some(button) = self
            .host
            .get_mut(ComponentId(COMMAND + 10))
            .and_then(|n| n.component_mut::<Button>())
        {
            button.label = if self.state.disabled {
                "Disabled: on"
            } else {
                "Disabled: off"
            }
            .into();
        }
    }
    pub(crate) fn select_section(&mut self, section: usize) {
        if self.state.section == section {
            return;
        }
        self.replace_section(section + 1);
    }
    fn replace_section(&mut self, command: usize) {
        if self.entry.renderer() == "dialog" && self.host.get(OVERLAY).is_some() {
            self.host.close_dialog();
            self.host.remove(OVERLAY).expect("mounted Dialog overlay");
            self.dialog_open = false;
        }

        self.state.activate(command, self.entry.renderer());
        self.host.remove(BODY).expect("feedback body mounted");
        self.host
            .insert(ROWS, Self::body(self.entry, &self.state))
            .expect("feedback body IDs");
        if self.entry.renderer() == "dialog" && matches!(self.state.section, 1 | 3 | 4) {
            self.host
                .insert(DIALOG_VIEWPORT, Self::dialog())
                .expect("Dialog viewport slot");
        }

        self.apply_disabled();
    }
    pub(crate) fn after_host_visit(&mut self) {
        let dialog_open = self
            .host
            .get(OVERLAY)
            .and_then(|node| node.component::<DialogContent>())
            .is_some_and(DialogContent::is_open);
        if self.dialog_open && !dialog_open {
            self.state.record("dialog: cancel; focus restored");
        }
        if let Some(c) = self
            .host
            .get(CONTROL)
            .and_then(|n| n.component::<ToastContent>())
        {
            self.state.extended.toast = c.model().clone();
        }
        if let Some(c) = self
            .host
            .get(CONTROL)
            .and_then(|n| n.component::<BannerContent>())
        {
            self.state.extended.banner = c.model().clone();
        }
        for record in self.host.drain_action_records() {
            let id = record.action.target;
            if id == ComponentId(COMMAND + 10) {
                self.state.activate(10, self.entry.renderer());
                self.apply_disabled();
                continue;
            }
            if (COMMAND..=COMMAND + 6).contains(&id.0) {
                self.replace_section((id.0 - COMMAND) as usize);
                continue;
            }
            match (self.entry.renderer(), id) {
                ("popover", CONTROL) => {
                    let session = self
                        .host
                        .get(OVERLAY)
                        .unwrap()
                        .component::<PopoverContent>()
                        .unwrap()
                        .current_session();
                    if let Some(session) = session {
                        self.host.close_popover(session);
                    } else {
                        self.host.show_popover(OVERLAY, CONTROL).unwrap();
                        self.state.record("popover: open");
                    }
                }
                ("dialog", CONTROL) => {
                    self.host.show_dialog(OVERLAY).unwrap();
                    self.state.record("dialog: open");
                }
                ("dialog", ACTION) => {
                    self.host.close_dialog();
                    self.state.record("dialog: cancel; focus restored");
                }
                ("dialog", ComponentId(103022)) => {
                    self.host.close_dialog();
                    self.state.record("dialog: confirm; focus restored");
                }
                ("toast", ComponentId(103022)) => {
                    let model = &mut self.state.extended.toast;
                    let feedback = if model.visible {
                        if model.advance(1000) {
                            "toast: fixture timer elapsed"
                        } else {
                            "toast timer: advanced 1 s"
                        }
                    } else {
                        model.show("Changes saved", controls::NoticeTone::Success, 5000);
                        "toast: shown (5 s fixture)"
                    };
                    self.host
                        .show_toast(
                            CONTROL,
                            model.message.clone(),
                            model.tone,
                            model.remaining_ms(),
                        )
                        .unwrap();
                    self.host.set_toast_visible(CONTROL, model.visible).unwrap();
                    self.host.set_toast_paused(CONTROL, model.paused).unwrap();
                    self.state.record(feedback);
                }
                ("toast", ACTION) => self.state.record("toast: dismissed"),
                ("banner", ACTION) => self.state.record("banner: dismissed"),
                ("empty-state", ACTION) => {
                    self.state.extended.empty_state_created = true;
                    self.host
                        .replace_empty_state(CONTROL, empty_model(true))
                        .unwrap();
                    self.state.record("empty-state: document created");
                }
                ("menu", CONTROL) => {
                    if let Some(ContentActionPayload::MenuSelection(selection)) = record.payload {
                        self.state
                            .record(format!("menu action: {}", selection.index()));
                    }
                }
                _ => {}
            }
        }
        let popover_session = self
            .host
            .get(OVERLAY)
            .and_then(|node| node.component::<PopoverContent>())
            .and_then(PopoverContent::current_session);
        if self.popover_session.is_some()
            && self.popover_session != popover_session
            && self
                .host
                .get(OVERLAY)
                .and_then(|node| node.component::<PopoverContent>())
                .is_some()
        {
            let closed = self
                .host
                .get_mut(OVERLAY)
                .unwrap()
                .component_mut::<PopoverContent>()
                .unwrap()
                .take_closed();
            for closed in closed {
                self.state
                    .record(format!("popover: closed ({:?})", closed.reason));
            }
        }
        self.popover_session = popover_session;
        self.dialog_open = self
            .host
            .get(OVERLAY)
            .and_then(|node| node.component::<DialogContent>())
            .is_some_and(DialogContent::is_open);
        // A mutable Host lookup invalidates its layout/reveal lease even when
        // the assigned value is unchanged. Idle synchronization must stay read-only.
        for (id, value) in [
            (
                LOG,
                self.state
                    .log
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
            (
                ComponentId(103023),
                format!("{} ms remaining", self.state.extended.toast.remaining_ms()),
            ),
        ] {
            if self
                .host
                .get(id)
                .and_then(|node| node.component::<TextContent>())
                .is_some_and(|text| text.text != value)
            {
                self.host
                    .get_mut(id)
                    .unwrap()
                    .component_mut::<TextContent>()
                    .unwrap()
                    .text = value;
            }
        }
        if self.entry.renderer() == "toast" {
            let label = if self.state.extended.toast.visible {
                "Advance fixture timer"
            } else {
                "Show again"
            };
            let id = ComponentId(103022);
            if self
                .host
                .get(id)
                .and_then(|node| node.component::<Button>())
                .is_some_and(|button| button.label != label)
            {
                self.host
                    .get_mut(id)
                    .unwrap()
                    .component_mut::<Button>()
                    .unwrap()
                    .label = label.into();
            }
        }
    }
}
fn empty_model(created: bool) -> EmptyState {
    if created {
        EmptyState {
            title: "Documents".into(),
            detail: "One fixture document was created.".into(),
            action: None,
        }
    } else {
        EmptyState {
            title: "No documents yet".into(),
            detail: "Create a document to begin your workspace.".into(),
            action: Some("Create document".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gallery::text::TextEngine;
    use std::time::Duration;
    fn story(renderer: &str) -> FeedbackStory {
        let entry = super::super::entries()
            .iter()
            .find(|entry| entry.renderer() == renderer)
            .unwrap();
        FeedbackStory::new(entry)
    }
    fn render(story: &mut FeedbackStory, text: &mut TextEngine) {
        let frame = Rect::new(0., 0., 740., 5000.);
        story
            .host
            .render(&mut Scene::new(), text, frame, frame, Theme::LIGHT.into())
            .unwrap();
        story.after_host_visit();
        // Clock-driven labels and action feedback can request one new layout.
        // At the same clock sample, geometry must be stable. An overlay may
        // still request animation frames, which does not invalidate its layout.
        if story.host.needs_render() {
            story
                .host
                .render(&mut Scene::new(), text, frame, frame, Theme::LIGHT.into())
                .unwrap();
            story.after_host_visit();
        }
        assert!(
            story
                .host
                .inspection_snapshot(BODY)
                .is_some_and(|snapshot| snapshot.geometry.is_some()),
            "idle feedback visit invalidates geometry: {}",
            story.entry.renderer()
        );
    }
    fn press(story: &mut FeedbackStory, text: &mut TextEngine, id: ComponentId) {
        render(story, text);
        let geometry = story.host.inspection_snapshot(id).and_then(|s| s.geometry);
        let witness = story.host.required_focus_result(id);
        let reveal = story
            .host
            .plan_host_reveal(
                id,
                tekes_ui_controls::ScrollRevealOptions::default(),
                &[],
                HostRevealBudget::default(),
            )
            .map(|_| ());
        assert!(
            story
                .host
                .accessibility_action(id, None, AccessibilityAction::Press)
                .consumed,
            "{} {id:?}: geometry={geometry:?}, required_focus={witness:?}, reveal={reveal:?}",
            story.entry.renderer()
        );
        story.after_host_visit();
        render(story, text);
    }
    #[test]
    fn feedback_notice_and_empty_actions_keep_original_models() {
        let mut text = TextEngine::new();
        text.set_motion_enabled(true);
        let mut toast = story("toast");
        render(&mut toast, &mut text);
        text.set_motion_context(Duration::from_millis(1000), true);
        render(&mut toast, &mut text);
        assert_eq!(toast.state.extended.toast.remaining_ms(), 4000);
        press(&mut toast, &mut text, ComponentId(103022));
        assert_eq!(toast.state.extended.toast.remaining_ms(), 3000);
        press(&mut toast, &mut text, ACTION);
        assert!(!toast.state.extended.toast.visible);
        press(&mut toast, &mut text, ComponentId(103022));
        assert_eq!(toast.state.extended.toast.remaining_ms(), 5000);
        assert!(toast.state.extended.toast.visible);
        let mut empty = story("empty-state");
        render(&mut empty, &mut text);
        press(&mut empty, &mut text, ACTION);
        assert!(empty.state.extended.empty_state_created);
        assert!(
            empty
                .host
                .get(CONTROL)
                .unwrap()
                .component::<EmptyStateContent>()
                .unwrap()
                .model()
                .action
                .is_none()
        );
        assert!(
            !empty
                .host
                .focus_targets()
                .iter()
                .any(|(id, _)| *id == ACTION)
        );
        press(&mut empty, &mut text, ComponentId(COMMAND));
        assert!(!empty.state.extended.empty_state_created);
    }
    #[test]
    fn feedback_popover_and_dialog_restore_original_invoker() {
        let mut text = TextEngine::new();
        text.set_motion_enabled(true);
        for renderer in ["popover", "dialog"] {
            let mut story = story(renderer);
            render(&mut story, &mut text);
            story.host.focus(Some(CONTROL), true);
            press(&mut story, &mut text, CONTROL);
            assert!(
                story
                    .host
                    .get(OVERLAY)
                    .unwrap()
                    .component::<PopoverContent>()
                    .map_or_else(
                        || story
                            .host
                            .get(OVERLAY)
                            .unwrap()
                            .component::<DialogContent>()
                            .unwrap()
                            .is_open(),
                        |p| p.is_open()
                    )
            );
            if renderer == "dialog" {
                let viewport = story.host.get(DIALOG_VIEWPORT).unwrap().frame();
                let modal = story
                    .host
                    .get(OVERLAY)
                    .unwrap()
                    .component::<DialogContent>()
                    .unwrap()
                    .modal_frame();
                assert!(!modal.is_empty());
                assert_eq!(modal.intersection(&viewport), Some(modal));
                let log = story.host.get(LOG).unwrap();
                assert!(log.frame().max_y() < viewport.max_y());
                assert_eq!(log.visible(), Some(log.frame()));
            }
            assert!(
                story
                    .host
                    .event(&ContentEvent::Key {
                        key: ControlKey::Escape,
                        down: true
                    })
                    .consumed
            );
            story.after_host_visit();
            render(&mut story, &mut text);
            assert_eq!(story.host.focused(), Some(CONTROL), "{renderer}");
            if renderer == "dialog" {
                let log = story.host.get(LOG).unwrap();
                assert_eq!(log.visible(), Some(log.frame()));
                assert!(
                    log.frame().max_y() < story.host.get(DIALOG_VIEWPORT).unwrap().frame().max_y()
                );
                press(&mut story, &mut text, ComponentId(COMMAND + 1));
                assert!(story.host.get(OVERLAY).is_none());
                press(&mut story, &mut text, ComponentId(COMMAND + 2));
                assert!(
                    !story
                        .host
                        .get(OVERLAY)
                        .unwrap()
                        .component::<DialogContent>()
                        .unwrap()
                        .is_open()
                );
                press(&mut story, &mut text, ComponentId(COMMAND));
                assert!(
                    !story
                        .host
                        .get(OVERLAY)
                        .unwrap()
                        .component::<DialogContent>()
                        .unwrap()
                        .is_open()
                );
            }
        }
    }
    #[test]
    fn feedback_all_declared_sections_mount_at_narrow_large_text() {
        let mut text = TextEngine::new();
        text.set_motion_enabled(true);
        text.set_type_scale(1.5);
        for renderer in [
            "tooltip",
            "popover",
            "toast",
            "banner",
            "empty-state",
            "menu",
            "dialog",
        ] {
            let mut story = story(renderer);
            for (section, name) in [
                "docs",
                "playground",
                "states",
                "behaviors",
                "edge-cases",
                "variants",
            ]
            .into_iter()
            .enumerate()
            {
                if !story.entry.supports_story(name) {
                    continue;
                }
                story.state.section = section;
                story.host.remove(BODY).unwrap();
                story
                    .host
                    .insert(ROWS, FeedbackStory::body(story.entry, &story.state))
                    .unwrap();
                let frame = Rect::new(0., 0., 320., 1200.);
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
                if section == 2 || section == 5 {
                    assert!(
                        story.host.get(CONTROL).is_none(),
                        "fixture exposes live control: {renderer}/{name}"
                    );
                    assert!(
                        story
                            .host
                            .get(ComponentId(103008))
                            .unwrap()
                            .component::<Matrix>()
                            .is_some()
                    );
                }
            }
        }
    }
}
