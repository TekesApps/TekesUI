use std::rc::Rc;
use std::time::Duration;
use tekes_ui_composition_consumer::scenarios::{self, ids};
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlKey, TextSurfaceHost, TextSurfaceTextRenderer,
    composition::*,
};
use tekes_ui_scene::{Point, Rect, Scene, Size};
use tekes_ui_text::{
    coretext::{CoreTextFonts, CoreTextShaper},
    model::{Paragraph, ShapeOptions},
    style::TextStyle,
    traits::Shaper,
};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal};

struct HostText {
    shaper: CoreTextShaper,
    clocked: bool,
    now: Duration,
    reduced_motion: bool,
    frames: usize,
    visible_surfaces: usize,
}

#[test]
fn public_select_validation_uses_shared_boundary_and_empty_choices_have_no_menu() {
    use tekes_ui_controls::{Select, layout_view::SelectView};
    for theme in Theme::ALL {
        for brand in 0..3 {
            let mut text = HostText::default();
            let palette = Palette::new(theme, brand).unwrap();
            let mut view = SelectView::default();
            view.set_reduced_motion(true);
            let mut model = Select::new(3);
            model.choose(1);
            let frame = Rect::new(20., 20., 240., 44.);
            let mut paint = |view: &mut SelectView, model: &mut Select, labels: &[&str]| {
                let mut scene = Scene::new();
                view.render(
                    &mut scene,
                    &mut text,
                    model,
                    labels,
                    frame,
                    Rect::new(0., 0., 400., 500.),
                    None,
                    palette,
                    proposal::TextRole::UiBody,
                    false,
                );
                scene
            };
            let labels = ["Automatic", "Compact", "Comfortable"];
            view.set_error(true);
            view.focus(true);
            let scene = paint(&mut view, &mut model, &labels);
            let role = proposal::control_boundary("error", theme).unwrap();
            assert!(scene.primitives().iter().any(|p| matches!(p,tekes_ui_scene::Primitive::Quad(q) if q.rect==frame && q.border_color==palette.color(role).unwrap())));
            assert!(scene.primitives().iter().any(|p| matches!(p,tekes_ui_scene::Primitive::Quad(q) if q.fill.a==0. && q.border_color==palette.color("border-focused").unwrap())),"validation and keyboard focus remain independent");
            view.key(ControlKey::Enter, true, &mut model);
            paint(&mut view, &mut model, &labels);
            view.key(ControlKey::Down, true, &mut model);
            view.key(ControlKey::Enter, true, &mut model);
            assert_eq!(
                model.selected, 2,
                "validation does not disable repair selection"
            );
            view.set_error(false);
            paint(&mut view, &mut model, &labels);
            assert_eq!(model.selected, 2);
            paint(&mut view, &mut model, &[]);
            view.focus(true);
            view.key(ControlKey::Enter, true, &mut model);
            assert!(!model.menu.open);
            assert!(!view.pointer_down(frame.center(), &mut model));
            assert!(!model.choose(0));
            assert!(view.layout().popup.is_none());
            assert_eq!(
                model.selected, 2,
                "empty data does not invent a selected option"
            );
            paint(&mut view, &mut model, &labels);
            view.focus(true);
            view.key(ControlKey::Enter, true, &mut model);
            assert!(model.menu.open, "fresh input works after choices return");
            assert_eq!(model.selected, 2);
        }
    }
}

#[test]
fn repeated_choice_confirmation_reaches_an_independent_consumer_and_empty_stays_open() {
    use tekes_ui_controls::{
        TextSurface,
        extended::{ComboBox, CommandPalette},
        layout_view::{ComboBoxView, CommandPaletteView},
    };
    let mut text = HostText::default();
    let palette = Palette::new(Theme::LIGHT, 0).unwrap();
    let input = Rect::new(20., 80., 280., 44.);
    let viewport = Rect::new(0., 0., 400., 500.);
    let mut combo = ComboBox {
        items: vec!["Open recent".into(), "Open settings".into()],
        ..Default::default()
    };
    let mut combo_view = ComboBoxView::default();
    let mut command = CommandPalette {
        combo: combo.clone(),
    };
    let mut command_view = CommandPaletteView::default();
    let mut callbacks = Vec::new();
    for _ in 0..2 {
        combo.set_query("settings");
        callbacks.push(("combo", combo_view.key(ControlKey::Enter, true, &mut combo)));
        command_view.open(&mut command);
        command_view.key(ControlKey::Down, true, &mut command);
        callbacks.push((
            "command",
            command_view.key(ControlKey::Enter, true, &mut command),
        ));
        assert_eq!(
            command_view.key(ControlKey::Enter, false, &mut command),
            None
        );
        assert_eq!(combo_view.key(ControlKey::Enter, false, &mut combo), None);
        assert!(!combo.open && !command.combo.open);
    }
    assert_eq!(
        callbacks,
        vec![
            ("combo", Some(1)),
            ("command", Some(1)),
            ("combo", Some(1)),
            ("command", Some(1))
        ]
    );
    command_view.open(&mut command);
    command.combo.set_query("no commands");
    let mut surface = TextSurface::new(&command.combo.query, false, false);
    surface.layout_with(input, None, 1., 1., |s, w| {
        text.shape_surface(s, proposal::TextRole::UiBody, w)
    });
    let mut scene = Scene::new();
    surface.draw(&mut text, &mut scene, palette, true, false);
    assert!(
        command_view
            .render(
                &mut scene,
                &mut text,
                &mut command,
                input,
                viewport,
                None,
                palette,
                proposal::TextRole::UiBody,
                false
            )
            .is_none()
    );
    assert_eq!(
        command_view.key(ControlKey::Enter, true, &mut command),
        None
    );
    assert!(command.combo.open);
    assert_eq!(surface.input.text, "no commands");
    assert_eq!(command.combo.query, surface.input.text);
    assert_eq!(
        command.combo.selected,
        Some(1),
        "no match never replays the previous command"
    );
}

#[test]
fn public_select_trigger_dismissal_does_not_require_a_host_blur_workaround() {
    use tekes_ui_controls::{Select, layout_view::SelectView};
    let mut text = HostText::default();
    let mut model = Select::new(3);
    let mut view = SelectView::default();
    view.set_reduced_motion(true);
    let frame = Rect::new(20., 20., 240., 44.);
    let mut render = |view: &mut SelectView, model: &mut Select| {
        let mut scene = Scene::new();
        view.render(
            &mut scene,
            &mut text,
            model,
            &["自动 Automatic", "紧凑 Compact", "舒适 Comfortable"],
            frame,
            Rect::new(0., 0., 400., 500.),
            None,
            Palette::new(Theme::LIGHT, 0).unwrap(),
            proposal::TextRole::UiBody,
            false,
        );
        scene
    };
    render(&mut view, &mut model);
    for open in [true, false, true] {
        assert!(view.pointer_down(frame.center(), &mut model));
        view.pointer_up(frame.center(), &mut model);
        assert_eq!(model.menu.open, open);
        assert_eq!(model.selected, 0);
        render(&mut view, &mut model);
    }
    view.key(ControlKey::Down, true, &mut model);
    view.key(ControlKey::Enter, true, &mut model);
    assert_eq!(model.selected, 1);
    assert!(!model.menu.open);
    let scene = render(&mut view, &mut model);
    assert!(scene.primitives().iter().any(|p| matches!(p, tekes_ui_scene::Primitive::Quad(q) if q.fill.a == 0. && q.border_color == proposal::render_color("border-focused", Theme::LIGHT).unwrap())), "public View restores keyboard focus without a host workaround");
}

#[test]
fn public_command_trigger_hover_drag_and_focus_transfer_reuse_original_view() {
    use tekes_ui_controls::{extended::CommandPalette, layout_view::CommandPaletteView};
    let mut text = HostText::default();
    let mut view = CommandPaletteView::default();
    let mut model = CommandPalette::default();
    let frame = Rect::new(20., 20., 220., 44.);
    let mut paint = |view: &mut CommandPaletteView, model: &mut CommandPalette| {
        let mut scene = Scene::new();
        view.render_trigger(
            &mut scene,
            &mut text,
            model,
            "打开命令 Commands",
            frame,
            None,
            Palette::new(Theme::LIGHT, 0).unwrap(),
            proposal::TextRole::UiBody,
        );
        scene
    };
    let rest = paint(&mut view, &mut model);
    view.pointer_move(frame.center(), &mut model);
    let hover = paint(&mut view, &mut model);
    assert_ne!(rest, hover);
    assert!(view.pointer_down_trigger(frame.center()));
    view.pointer_move(Point::new(300., 100.), &mut model);
    assert_eq!(rest, paint(&mut view, &mut model));
    assert!(!view.pointer_up_trigger(Point::new(300., 100.), &mut model));
    model.combo.open = true;
    model.combo.query = "existing query".into();
    view.focus_trigger(true);
    let focus = paint(&mut view, &mut model);
    view.blur_trigger();
    assert_ne!(focus, paint(&mut view, &mut model));
    assert!(model.combo.open);
    assert_eq!(model.combo.query, "existing query");
}
impl Default for HostText {
    fn default() -> Self {
        Self {
            shaper: CoreTextShaper::new(CoreTextFonts::new()),
            clocked: false,
            now: Duration::ZERO,
            reduced_motion: false,
            frames: 0,
            visible_surfaces: 0,
        }
    }
}
impl HostText {
    fn enable_clock(&mut self, reduced_motion: bool) {
        self.clocked = true;
        self.reduced_motion = reduced_motion;
    }
    fn advance_to(&mut self, now: Duration) {
        self.now = now;
    }
}
impl TextSurfaceHost for HostText {
    fn shape_surface(&mut self, s: &str, r: proposal::TextRole, w: Option<f32>) -> Rc<Paragraph> {
        let style = TextStyle::system(r.style().size).with_weight(r.style().weight);
        Rc::new(self.shaper.shape(
            s,
            &style,
            &ShapeOptions {
                width: w.map(f64::from),
                line_spacing: 0.,
            },
        ))
    }
    fn type_scale(&self) -> f32 {
        1.
    }
}
impl TextSurfaceTextRenderer for HostText {
    fn push_visible(&mut self, _: &mut Scene, _: &Paragraph, _: Point, _: Rgba, _: Rect) {
        self.visible_surfaces += 1;
    }
}
impl ButtonTextRenderer for HostText {
    fn measure(&mut self, s: &str, r: proposal::TextRole, w: f32) -> Size {
        let p = self.shape_surface(s, r, Some(w));
        Size::new(p.width as f32, p.height as f32)
    }
    fn draw(
        &mut self,
        _: &mut Scene,
        _: &str,
        _: proposal::TextRole,
        _: Rect,
        _: ButtonTextAlign,
        _: Rgba,
    ) {
    }
    fn text_surface_host(&mut self) -> Option<&mut dyn TextSurfaceHost> {
        Some(self)
    }
    fn motion_context(&self) -> Option<tekes_ui_controls::ControlMotionContext> {
        self.clocked
            .then_some(tekes_ui_controls::ControlMotionContext {
                now: self.now,
                reduced_motion: self.reduced_motion,
            })
    }
    fn request_animation_frame(&mut self) {
        self.frames += 1;
    }
}
fn render(host: &mut ContentHost, text: &mut HostText) {
    host.render(
        &mut Scene::new(),
        text,
        Rect::new(0., 0., 640., 520.),
        Rect::new(0., 0., 640., 520.),
        Palette::from(Theme::LIGHT),
    )
    .unwrap();
}
fn key(host: &mut ContentHost, k: ControlKey, down: bool) {
    host.event(&ContentEvent::Key { key: k, down });
}

#[test]
fn independent_heterogeneous_list_preserves_mixed_children_and_isolates_row_action() {
    let mut fixture = scenarios::heterogeneous_list().unwrap();
    let mut text = HostText::default();
    render(&mut fixture.host, &mut text);
    let list = fixture.host.get(ids::LIST).unwrap().frame();
    let switch = fixture
        .host
        .get(ids::LIST_ROW_SWITCH)
        .unwrap()
        .component::<SwitchContent>()
        .unwrap()
        .view
        .layout()
        .frame()
        .center();
    let old_height = fixture
        .host
        .get(ids::LIST_FIELD)
        .unwrap()
        .frame()
        .size
        .height;
    fixture.host.event(&ContentEvent::PointerDown(switch));
    fixture.host.event(&ContentEvent::PointerUp(switch));
    assert!(
        fixture
            .host
            .get(ids::LIST_ROW_SWITCH)
            .unwrap()
            .component::<SwitchContent>()
            .unwrap()
            .model
            .checked
    );
    assert_eq!(
        fixture.row_activations.get(),
        0,
        "child Switch owns the event; parent row must not activate"
    );
    let row = fixture.host.get(ids::LIST_ROW).unwrap().frame();
    let row_background = Point::new(row.max_x() - 4., row.min_y() + row.size.height * 0.5);
    fixture.host.event(&ContentEvent::PointerUp(row_background));
    assert_eq!(
        fixture.row_activations.get(),
        0,
        "an unarmed release does not activate the row"
    );
    fixture
        .host
        .event(&ContentEvent::PointerDown(row_background));
    fixture.host.event(&ContentEvent::PointerUp(Point::new(
        row.max_x() + 12.,
        row_background.y,
    )));
    assert_eq!(
        fixture.row_activations.get(),
        0,
        "release outside the row cancels activation"
    );
    fixture
        .host
        .event(&ContentEvent::PointerDown(row_background));
    fixture.host.event(&ContentEvent::Cancel);
    fixture.host.event(&ContentEvent::PointerUp(row_background));
    assert_eq!(
        fixture.row_activations.get(),
        0,
        "cancellation clears the shared press state"
    );
    fixture
        .host
        .event(&ContentEvent::PointerDown(row_background));
    fixture.host.event(&ContentEvent::PointerUp(row_background));
    assert_eq!(
        fixture.row_activations.get(),
        1,
        "the row remains independently actionable outside the child hit target"
    );
    let field = fixture
        .host
        .get_mut(ids::LIST_FIELD)
        .unwrap()
        .component_mut::<TextFieldContent>()
        .unwrap();
    field.surface.input.select(1, false);
    field.surface.input.preedit("你", 1..1, None);
    assert!(field.surface.input.marked.is_some());
    fixture
        .host
        .reorder(
            ids::LIST,
            &[
                ids::LIST_ROW,
                ids::LIST_MENU,
                ids::LIST_FIELD,
                ids::LIST_SWITCH,
                ids::LIST_BADGE,
                ids::LIST_TEXT,
            ],
        )
        .unwrap();
    render(&mut fixture.host, &mut text);
    let field = fixture
        .host
        .get(ids::LIST_FIELD)
        .unwrap()
        .component::<TextFieldContent>()
        .unwrap();
    assert!(field.surface.input.marked.is_some());
    assert!(field.surface.input.text.contains('你'));
    assert!(fixture.host.get(ids::LIST_FIELD).unwrap().frame().max_y() <= list.max_y() + 0.1);
    assert_ne!(
        fixture
            .host
            .get(ids::LIST_FIELD)
            .unwrap()
            .frame()
            .size
            .height,
        0.
    );
    assert_ne!(old_height, 0.);
    assert!(
        fixture
            .host
            .semantics()
            .iter()
            .any(|s| s.id == ids::LIST && s.semantics.role == ContentRole::List)
    );
}

#[test]
fn menu_keeps_its_semantic_keyboard_model_inside_heterogeneous_list() {
    let mut fixture = scenarios::heterogeneous_list().unwrap();
    let mut text = HostText::default();
    render(&mut fixture.host, &mut text);
    assert!(fixture.host.focus(Some(ids::LIST_MENU), true));
    key(&mut fixture.host, ControlKey::Enter, true);
    key(&mut fixture.host, ControlKey::Enter, false);
    render(&mut fixture.host, &mut text);
    assert!(
        fixture
            .host
            .get(ids::LIST_MENU)
            .unwrap()
            .component::<MenuContent>()
            .unwrap()
            .menu
            .is_open()
    );
    key(&mut fixture.host, ControlKey::Down, true);
    key(&mut fixture.host, ControlKey::Enter, true);
    let menu = fixture
        .host
        .get(ids::LIST_MENU)
        .unwrap()
        .component::<MenuContent>()
        .unwrap();
    assert_eq!(menu.selected, Some(1));
    assert!(!menu.menu.is_open());
}

#[test]
fn rich_dialog_uses_modal_focus_escape_restore_and_owned_form_state() {
    let mut host = scenarios::rich_dialog().unwrap();
    let mut text = HostText::default();
    render(&mut host, &mut text);
    assert!(host.focus(Some(ids::DIALOG_INVOKER), true));
    host.show_dialog(ids::DIALOG).unwrap();
    render(&mut host, &mut text);
    assert!(host.focused().is_some_and(|id| id != ids::DIALOG_INVOKER));
    let field = host.focused_text().unwrap();
    assert!(!field.input.text.is_empty());
    let dialog = host
        .get(ids::DIALOG)
        .unwrap()
        .component::<DialogContent>()
        .unwrap()
        .modal_frame();
    assert!(dialog.size.width <= 460. && dialog.size.height <= 520.);
    key(&mut host, ControlKey::Escape, true);
    assert_eq!(host.focused(), Some(ids::DIALOG_INVOKER));
    render(&mut host, &mut text);
    assert_eq!(host.focused(), Some(ids::DIALOG_INVOKER));
    host.show_dialog(ids::DIALOG).unwrap();
    render(&mut host, &mut text);
    assert!(host.focused().is_some_and(|id| id != ids::DIALOG_INVOKER));
    let field = host
        .get_mut(ids::DIALOG_FIELD)
        .unwrap()
        .component_mut::<TextFieldContent>()
        .unwrap();
    field.surface.input.select_all();
    field.surface.input.preedit("新工作区", 4..4, None);
    let preserved = field.surface.input.text.clone();
    host.close_dialog();
    render(&mut host, &mut text);
    host.show_dialog(ids::DIALOG).unwrap();
    render(&mut host, &mut text);
    assert_eq!(
        host.get(ids::DIALOG_FIELD)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap()
            .surface
            .input
            .text,
        preserved
    );
    assert!(host.focused_text().is_some());
}

#[test]
fn rich_dialog_validation_uses_field_error_and_preserves_marked_selection() {
    let mut host = scenarios::rich_dialog().unwrap();
    let mut text = HostText::default();
    host.show_dialog(ids::DIALOG).unwrap();
    render(&mut host, &mut text);
    let field = host
        .get_mut(ids::DIALOG_FIELD)
        .unwrap()
        .component_mut::<TextFieldContent>()
        .unwrap();
    field.surface.input.select_all();
    field.surface.input.insert("", None);
    field.surface.input.preedit("", 0..0, None);
    let marked = field.surface.input.marked.clone();
    let selection = field.surface.input.selection();
    assert!(!scenarios::validate_dialog(&mut host));
    assert!(host.semantics().iter().any(|s| s.id == ids::DIALOG));
    let field = host
        .get(ids::DIALOG_FIELD)
        .unwrap()
        .component::<TextFieldContent>()
        .unwrap();
    assert_eq!(
        field.field.error.as_deref(),
        Some("Enter a workspace name.")
    );
    assert_eq!(field.surface.input.marked, marked);
    assert_eq!(field.surface.input.selection(), selection);
    assert_eq!(host.focused(), Some(ids::DIALOG_FIELD));
    render(&mut host, &mut text);
    host.event(&ContentEvent::Commit {
        text: "新工作区".into(),
        replacement: None,
    });
    assert!(scenarios::validate_dialog(&mut host));
    assert_eq!(
        host.get(ids::DIALOG_FIELD)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap()
            .field
            .error,
        None
    );
    let value = host
        .get(ids::DIALOG_FIELD)
        .unwrap()
        .component::<TextFieldContent>()
        .unwrap()
        .surface
        .input
        .text
        .clone();
    assert!(host.close_dialog());
    host.show_dialog(ids::DIALOG).unwrap();
    render(&mut host, &mut text);
    let field = host
        .get(ids::DIALOG_FIELD)
        .unwrap()
        .component::<TextFieldContent>()
        .unwrap();
    assert_eq!(field.surface.input.text, value);
    assert_eq!(field.field.error, None);
}

#[test]
fn editable_fields_require_the_host_text_service_instead_of_a_fake_editor() {
    struct NoSurface;
    impl ButtonTextRenderer for NoSurface {
        fn measure(&mut self, _: &str, _: proposal::TextRole, _: f32) -> Size {
            Size::new(30., 20.)
        }
        fn draw(
            &mut self,
            _: &mut Scene,
            _: &str,
            _: proposal::TextRole,
            _: Rect,
            _: ButtonTextAlign,
            _: Rgba,
        ) {
        }
    }
    let mut host = scenarios::rich_dialog().unwrap();
    host.show_dialog(ids::DIALOG).unwrap();
    let result = host.render(
        &mut Scene::new(),
        &mut NoSurface,
        Rect::new(0., 0., 640., 520.),
        Rect::new(0., 0., 640., 520.),
        Palette::from(Theme::LIGHT),
    );
    assert_eq!(
        result,
        Err(CompositionError::MissingTextServices(ids::DIALOG_FIELD))
    );
}

#[test]
fn dialog_presence_exits_independently_of_semantics_and_honors_reduced_motion() {
    let mut host = scenarios::rich_dialog().unwrap();
    let mut text = HostText::default();
    text.enable_clock(false);
    render(&mut host, &mut text);
    assert!(host.focus(Some(ids::DIALOG_INVOKER), true));

    host.show_dialog(ids::DIALOG).unwrap();
    render(&mut host, &mut text);
    assert!(
        host.get(ids::DIALOG)
            .unwrap()
            .component::<DialogContent>()
            .unwrap()
            .is_open()
    );
    assert!(
        host.get(ids::DIALOG)
            .unwrap()
            .component::<DialogContent>()
            .unwrap()
            .is_presented()
    );
    assert!(text.frames > 0, "opening presence requests host frames");

    // Finish the entrance, then begin a clocked exit transition.
    text.advance_to(Duration::from_secs(1));
    render(&mut host, &mut text);
    assert!(host.close_dialog());
    assert_eq!(host.focused(), Some(ids::DIALOG_INVOKER));
    assert!(host.focused_text().is_none());
    assert!(
        !host
            .get(ids::DIALOG)
            .unwrap()
            .component::<DialogContent>()
            .unwrap()
            .is_open()
    );
    assert!(
        host.get(ids::DIALOG)
            .unwrap()
            .component::<DialogContent>()
            .unwrap()
            .is_presented()
    );
    assert!(host.semantics().iter().all(|node| node.id != ids::DIALOG));
    assert!(!host.focus(Some(ids::DIALOG_FIELD), true));
    assert_eq!(host.focused(), Some(ids::DIALOG_INVOKER));
    assert!(host.focused_text().is_none());

    let surfaces_before = text.visible_surfaces;
    text.advance_to(Duration::from_millis(1_010));
    render(&mut host, &mut text);
    assert!(
        text.visible_surfaces > surfaces_before,
        "exit frame rebuilds current TextSurface glyphs"
    );
    assert!(
        host.get(ids::DIALOG)
            .unwrap()
            .component::<DialogContent>()
            .unwrap()
            .is_presented()
    );
    assert!(host.needs_render());

    text.reduced_motion = true;
    text.advance_to(Duration::from_millis(1_020));
    render(&mut host, &mut text);
    let dialog = host
        .get(ids::DIALOG)
        .unwrap()
        .component::<DialogContent>()
        .unwrap();
    assert!(!dialog.is_open());
    assert!(
        !dialog.is_presented(),
        "reduced motion completes the exit immediately"
    );
    assert!(!dialog.needs_animation());
    assert!(!host.needs_render());
    assert_eq!(host.focused(), Some(ids::DIALOG_INVOKER));
}

#[test]
fn marked_cjk_preedit_escape_cancels_composition_before_modal_dismissal() {
    let mut host = scenarios::rich_dialog().unwrap();
    let mut text = HostText::default();
    host.focus(Some(ids::DIALOG_INVOKER), true);
    host.show_dialog(ids::DIALOG).unwrap();
    render(&mut host, &mut text);
    host.focus(Some(ids::DIALOG_FIELD), true);
    host.event(&ContentEvent::Preedit {
        text: "新しい".into(),
        selection: 3..3,
        clauses: std::iter::once(0..3).collect(),
        replacement: None,
    });
    assert!(host.focused_text().unwrap().input.marked.is_some());

    key(&mut host, ControlKey::Escape, true);
    assert!(
        host.get(ids::DIALOG)
            .unwrap()
            .component::<DialogContent>()
            .unwrap()
            .is_open()
    );
    assert_eq!(host.focused(), Some(ids::DIALOG_FIELD));
    assert!(host.focused_text().unwrap().input.marked.is_none());
    assert!(host.semantics().iter().any(|node| node.id == ids::DIALOG));

    key(&mut host, ControlKey::Escape, true);
    assert!(
        !host
            .get(ids::DIALOG)
            .unwrap()
            .component::<DialogContent>()
            .unwrap()
            .is_open()
    );
    assert_eq!(host.focused(), Some(ids::DIALOG_INVOKER));
}

#[test]
fn public_accessibility_actions_use_live_component_and_menu_part_state() {
    let mut fixture = scenarios::heterogeneous_list().unwrap();
    let host = &mut fixture.host;
    let mut text = HostText::default();
    render(host, &mut text);

    let set_value = host.accessibility_action(
        ids::LIST_FIELD,
        None,
        AccessibilityAction::SetValue("AX workspace".into()),
    );
    assert!(set_value.consumed && set_value.changed);
    assert!(
        host.focused_text().is_none(),
        "SetValue does not fabricate focus"
    );
    assert_eq!(
        host.get(ids::LIST_FIELD)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap()
            .surface
            .input
            .text,
        "AX workspace"
    );

    let toggle = host.accessibility_action(ids::LIST_SWITCH, None, AccessibilityAction::Press);
    assert!(toggle.consumed && toggle.activated);
    assert!(
        !host
            .get(ids::LIST_SWITCH)
            .unwrap()
            .component::<SwitchContent>()
            .unwrap()
            .model
            .checked
    );

    let opened = host.accessibility_action(ids::LIST_MENU, None, AccessibilityAction::Press);
    assert!(opened.consumed && opened.changed);
    render(host, &mut text);
    let parts = host
        .semantics()
        .into_iter()
        .find(|node| node.id == ids::LIST_MENU)
        .unwrap()
        .parts;
    assert_eq!(parts.len(), 3);
    assert_eq!(
        parts
            .iter()
            .map(|part| part.semantics.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Personal", "Team", "Custom"]
    );
    assert!(
        parts
            .iter()
            .all(|part| part.frame.size.width > 0. && part.frame.size.height > 0.)
    );

    let selected = host.accessibility_action(ids::LIST_MENU, Some(1), AccessibilityAction::Press);
    assert!(selected.consumed && selected.activated);
    let menu = host
        .get(ids::LIST_MENU)
        .unwrap()
        .component::<MenuContent>()
        .unwrap();
    assert_eq!(menu.selected, Some(1));
    assert_eq!(menu.semantics().value.as_deref(), Some("Team"));
    assert!(!menu.menu.is_open());
    assert!(
        !host
            .accessibility_action(ids::LIST_MENU, Some(1), AccessibilityAction::Press)
            .consumed,
        "a stale menu part is rejected after close"
    );

    host.get_mut(ids::LIST_SWITCH)
        .unwrap()
        .component_mut::<SwitchContent>()
        .unwrap()
        .model
        .disabled = true;
    let disabled = host.accessibility_action(ids::LIST_SWITCH, None, AccessibilityAction::Press);
    assert!(!disabled.consumed);
    assert!(
        !host
            .get(ids::LIST_SWITCH)
            .unwrap()
            .component::<SwitchContent>()
            .unwrap()
            .model
            .checked
    );
}

#[test]
fn accessibility_actions_respect_modal_scope_and_reject_closed_dialog_targets() {
    let mut host = scenarios::rich_dialog().unwrap();
    let mut text = HostText::default();
    render(&mut host, &mut text);
    host.focus(Some(ids::DIALOG_INVOKER), true);
    host.show_dialog(ids::DIALOG).unwrap();
    render(&mut host, &mut text);

    let background =
        host.accessibility_action(ids::DIALOG_INVOKER, None, AccessibilityAction::Press);
    assert!(
        !background.consumed,
        "modal scope rejects background invoker actions"
    );
    let set_value = host.accessibility_action(
        ids::DIALOG_FIELD,
        None,
        AccessibilityAction::SetValue("Saved through AX".into()),
    );
    assert!(set_value.consumed && set_value.changed);
    let value = host
        .get(ids::DIALOG_FIELD)
        .unwrap()
        .component::<TextFieldContent>()
        .unwrap()
        .surface
        .input
        .text
        .clone();

    assert!(host.close_dialog());
    assert_eq!(host.focused(), Some(ids::DIALOG_INVOKER));
    assert!(
        !host
            .accessibility_action(
                ids::DIALOG_FIELD,
                None,
                AccessibilityAction::SetValue("stale".into()),
            )
            .consumed
    );
    assert_eq!(
        host.get(ids::DIALOG_FIELD)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap()
            .surface
            .input
            .text,
        value,
        "a closed dialog target cannot receive stale actions"
    );
}

#[test]
fn accessibility_text_selection_reuses_editor_graphemes_undo_and_composition_priority() {
    let mut host = scenarios::rich_dialog().unwrap();
    let mut text = HostText::default();
    host.show_dialog(ids::DIALOG).unwrap();
    render(&mut host, &mut text);
    let act =
        |host: &mut ContentHost, action| host.accessibility_action(ids::DIALOG_FIELD, None, action);
    assert!(
        act(
            &mut host,
            AccessibilityAction::SetValue("A😀e\u{301}中".into())
        )
        .consumed
    );
    assert!(act(&mut host, AccessibilityAction::SetSelection(2..4)).consumed);
    let input = &host.text_surface(ids::DIALOG_FIELD).unwrap().input;
    assert_eq!(input.selection(), 1..5);
    assert_eq!(input.selected_text(), "😀e\u{301}");
    assert_eq!(host.focused(), Some(ids::DIALOG_FIELD));
    assert!(
        act(
            &mut host,
            AccessibilityAction::ReplaceSelectedText("你".into())
        )
        .consumed
    );
    assert_eq!(host.focused_text().unwrap().input.text, "A你中");
    host.event(&ContentEvent::TextCommand("undo:".into()));
    assert_eq!(host.focused_text().unwrap().input.text, "A😀e\u{301}中");
    assert_eq!(host.focused_text().unwrap().input.selection(), 1..5);
    assert!(!act(&mut host, AccessibilityAction::SetSelection(0..999)).consumed);
    host.get_mut(ids::DIALOG_FIELD)
        .unwrap()
        .component_mut::<TextFieldContent>()
        .unwrap()
        .surface
        .input
        .read_only = true;
    assert!(act(&mut host, AccessibilityAction::SetSelection(5..6)).consumed);
    assert!(
        !act(
            &mut host,
            AccessibilityAction::ReplaceSelectedText("x".into())
        )
        .consumed
    );
    host.get_mut(ids::DIALOG_FIELD)
        .unwrap()
        .component_mut::<TextFieldContent>()
        .unwrap()
        .surface
        .input
        .read_only = false;
    host.event(&ContentEvent::Preedit {
        text: "pin".into(),
        selection: 3..3,
        clauses: Vec::new(),
        replacement: None,
    });
    let before = host.focused_text().unwrap().input.text.clone();
    for action in [
        AccessibilityAction::SetSelection(0..0),
        AccessibilityAction::SetValue("x".into()),
        AccessibilityAction::ReplaceSelectedText("x".into()),
    ] {
        assert!(!act(&mut host, action).consumed);
    }
    assert_eq!(host.focused_text().unwrap().input.text, before);
    assert!(host.focused_text().unwrap().input.marked.is_some());
    host.event(&ContentEvent::TextCommand("undo:".into()));
    assert_eq!(host.focused_text().unwrap().input.text, "A😀e\u{301}中");
    assert_eq!(host.focused_text().unwrap().input.selection(), 5..6);
    assert!(host.focused_text().unwrap().input.marked.is_none());
    host.close_dialog();
    assert!(host.text_surface(ids::DIALOG_FIELD).is_none());
    assert!(!act(&mut host, AccessibilityAction::SetSelection(0..0)).consumed);
}

#[test]
fn input_blur_finalizes_the_draft_and_dialog_reopen_has_no_stale_composition() {
    let mut host = scenarios::rich_dialog().unwrap();
    let mut text = HostText::default();
    host.show_dialog(ids::DIALOG).unwrap();
    render(&mut host, &mut text);
    host.accessibility_action(
        ids::DIALOG_FIELD,
        None,
        AccessibilityAction::SetValue("draft".into()),
    );
    host.focus(Some(ids::DIALOG_FIELD), true);
    host.event(&ContentEvent::Preedit {
        text: "pin".into(),
        selection: 3..3,
        clauses: std::iter::once(0..3).collect(),
        replacement: None,
    });
    let visible = host.focused_text().unwrap().input.text.clone();
    host.close_dialog();
    assert!(host.focused_text().is_none());
    let input = &host
        .get(ids::DIALOG_FIELD)
        .unwrap()
        .component::<TextFieldContent>()
        .unwrap()
        .surface
        .input;
    assert_eq!(input.text, visible);
    assert!(input.marked.is_none());
    host.show_dialog(ids::DIALOG).unwrap();
    render(&mut host, &mut text);
    assert!(host.focused_text().unwrap().input.marked.is_none());
    host.event(&ContentEvent::TextCommand("undo:".into()));
    assert_eq!(host.focused_text().unwrap().input.text, "draft");
}

#[test]
fn native_empty_preedit_restores_draft_and_releases_modal_tab_scope() {
    let mut host = scenarios::rich_dialog().unwrap();
    let mut text = HostText::default();
    host.show_dialog(ids::DIALOG).unwrap();
    render(&mut host, &mut text);
    host.accessibility_action(
        ids::DIALOG_FIELD,
        None,
        AccessibilityAction::SetValue("A😀B".into()),
    );
    host.accessibility_action(
        ids::DIALOG_FIELD,
        None,
        AccessibilityAction::SetSelection(1..3),
    );
    host.event(&ContentEvent::Preedit {
        text: "ni".into(),
        selection: 2..2,
        clauses: Vec::new(),
        replacement: None,
    });
    host.event(&ContentEvent::Preedit {
        text: "".into(),
        selection: 0..0,
        clauses: Vec::new(),
        replacement: None,
    });
    assert_eq!(host.focused_text().unwrap().input.text, "A😀B");
    assert_eq!(host.focused_text().unwrap().input.selection(), 1..3);
    assert!(host.focused_text().unwrap().input.marked.is_none());
    key(&mut host, ControlKey::Tab { reverse: false }, true);
    assert_ne!(host.focused(), Some(ids::DIALOG_FIELD));
    assert!(
        host.accessibility_action(
            ids::DIALOG_FIELD,
            None,
            AccessibilityAction::SetValue("recovered".into())
        )
        .consumed
    );
}

#[test]
fn selection_validation_reaches_live_composition_semantics_without_disabling_the_value() {
    let (mut host, id) = scenarios::build_list();
    let mut text = HostText::default();
    host.get_mut(id)
        .unwrap()
        .component_mut::<SwitchContent>()
        .unwrap()
        .view
        .set_error(true);
    render(&mut host, &mut text);
    let checked = host
        .get(id)
        .unwrap()
        .component::<SwitchContent>()
        .unwrap()
        .model
        .checked;
    assert!(
        host.semantics()
            .iter()
            .find(|node| node.id == id)
            .unwrap()
            .semantics
            .invalid
    );
    assert!(host.focus(Some(id), true));
    host.event(&ContentEvent::Key {
        key: ControlKey::Space,
        down: true,
    });
    host.event(&ContentEvent::Key {
        key: ControlKey::Space,
        down: false,
    });
    render(&mut host, &mut text);
    assert_ne!(
        host.get(id)
            .unwrap()
            .component::<SwitchContent>()
            .unwrap()
            .model
            .checked,
        checked
    );
    let control = host
        .get_mut(id)
        .unwrap()
        .component_mut::<SwitchContent>()
        .unwrap();
    control.model.disabled = true;
    let value = control.model.checked;
    render(&mut host, &mut text);
    let semantic = host
        .semantics()
        .into_iter()
        .find(|node| node.id == id)
        .unwrap();
    assert!(semantic.semantics.invalid && semantic.semantics.disabled);
    host.event(&ContentEvent::Key {
        key: ControlKey::Space,
        down: true,
    });
    host.event(&ContentEvent::Key {
        key: ControlKey::Space,
        down: false,
    });
    let control = host
        .get_mut(id)
        .unwrap()
        .component_mut::<SwitchContent>()
        .unwrap();
    assert_eq!(control.model.checked, value);
    control.view.set_error(false);
    render(&mut host, &mut text);
    assert!(
        !host
            .semantics()
            .iter()
            .find(|node| node.id == id)
            .unwrap()
            .semantics
            .invalid
    );
}

#[test]
fn input_family_reuses_composed_text_services_for_multiline_read_only_and_ime() {
    for multiline in [false, true] {
        let mut host = scenarios::rich_dialog().unwrap();
        let mut text = HostText::default();
        host.get_mut(ids::DIALOG_FIELD)
            .unwrap()
            .component_mut::<TextFieldContent>()
            .unwrap()
            .surface = tekes_ui_controls::TextSurface::new("A😀e\u{301}中", multiline, false);
        host.show_dialog(ids::DIALOG).unwrap();
        render(&mut host, &mut text);
        assert!(text.visible_surfaces > 0);
        assert_eq!(host.focused(), Some(ids::DIALOG_FIELD));
        assert!(
            host.accessibility_action(
                ids::DIALOG_FIELD,
                None,
                AccessibilityAction::SetSelection(2..4)
            )
            .consumed
        );
        assert_eq!(host.focused_text().unwrap().input.selection(), 1..5);
        assert!(
            host.event(&ContentEvent::Preedit {
                text: "输入".into(),
                selection: 2..2,
                clauses: vec![0..1, 1..2],
                replacement: None
            })
            .consumed
        );
        render(&mut host, &mut text);
        assert_eq!(host.focused_text().unwrap().input.marked, Some(1..3));
        assert_eq!(host.focused_text().unwrap().clauses, [0..1, 1..2]);
        assert!(
            host.event(&ContentEvent::Preedit {
                text: String::new(),
                selection: 0..0,
                clauses: vec![],
                replacement: None
            })
            .consumed
        );
        assert_eq!(host.focused_text().unwrap().input.text, "A😀e\u{301}中");
        host.get_mut(ids::DIALOG_FIELD)
            .unwrap()
            .component_mut::<TextFieldContent>()
            .unwrap()
            .surface
            .input
            .read_only = true;
        assert!(
            host.accessibility_action(
                ids::DIALOG_FIELD,
                None,
                AccessibilityAction::SetSelection(5..6)
            )
            .consumed
        );
        host.event(&ContentEvent::Commit {
            text: "x".into(),
            replacement: None,
        });
        host.event(&ContentEvent::Preedit {
            text: "pin".into(),
            selection: 3..3,
            clauses: vec![],
            replacement: None,
        });
        render(&mut host, &mut text);
        let input = &host.focused_text().unwrap().input;
        assert_eq!(input.text, "A😀e\u{301}中");
        assert_eq!(input.selection(), 5..6);
        assert!(input.marked.is_none());
        host.get_mut(ids::DIALOG_FIELD)
            .unwrap()
            .component_mut::<TextFieldContent>()
            .unwrap()
            .surface
            .input
            .disabled = true;
        host.event(&ContentEvent::Commit {
            text: "y".into(),
            replacement: None,
        });
        render(&mut host, &mut text);
        assert_eq!(
            host.text_surface(ids::DIALOG_FIELD).unwrap().input.text,
            "A😀e\u{301}中"
        );
    }
}

#[test]
fn input_pointer_contract_reuses_live_field_capture_in_a_modal_consumer() {
    for multiline in [false, true] {
        for read_only in [false, true] {
            let mut host = scenarios::rich_dialog().unwrap();
            let mut text = HostText::default();
            host.get_mut(ids::DIALOG_FIELD)
                .unwrap()
                .component_mut::<TextFieldContent>()
                .unwrap()
                .surface = tekes_ui_controls::TextSurface::new(
                "A😀 e\u{301} 中文 sample",
                multiline,
                read_only,
            );
            host.show_dialog(ids::DIALOG).unwrap();
            render(&mut host, &mut text);
            let point = host.text_surface(ids::DIALOG_FIELD).unwrap().inner.center();
            let before = host
                .text_surface(ids::DIALOG_FIELD)
                .unwrap()
                .input
                .text
                .clone();
            host.event(&ContentEvent::PointerMove(point));
            let state = host
                .text_surface(ids::DIALOG_FIELD)
                .unwrap()
                .pointer_state(Some(point));
            assert!(state.hovered && !state.pressed);
            assert!(host.event(&ContentEvent::PointerDown(point)).consumed);
            assert!(
                host.text_surface(ids::DIALOG_FIELD)
                    .unwrap()
                    .pointer_state(None)
                    .pressed
            );
            let outside = Point::new(1500., 1500.);
            host.event(&ContentEvent::PointerMove(outside));
            let surface = host.text_surface(ids::DIALOG_FIELD).unwrap();
            let state = surface.pointer_state(Some(outside));
            assert!(!state.hovered && state.pressed);
            let selection = surface.input.selection();
            host.event(&ContentEvent::PointerUp(outside));
            let surface = host.text_surface(ids::DIALOG_FIELD).unwrap();
            assert!(!surface.pointer_state(Some(point)).pressed);
            assert_eq!(surface.input.selection(), selection);
            assert_eq!(surface.input.text, before);
            assert_eq!(host.focused(), Some(ids::DIALOG_FIELD));
            host.event(&ContentEvent::PointerDown(point));
            host.event(&ContentEvent::Cancel);
            assert!(
                !host
                    .text_surface(ids::DIALOG_FIELD)
                    .unwrap()
                    .pointer_state(None)
                    .pressed
            );
            render(&mut host, &mut text);
        }
    }
}

#[test]
fn composed_reading_policy_keeps_history_and_keyboard_latest_does_not_steal_focus() {
    let mut host = scenarios::rich_dialog().unwrap();
    let mut text = HostText::default();
    let document: String = (0..100)
        .map(|i| format!("Paragraph {i}. 中文 reading remains anchored across streamed updates.\n"))
        .collect();
    let field = host
        .get_mut(ids::DIALOG_FIELD)
        .unwrap()
        .component_mut::<TextFieldContent>()
        .unwrap();
    assert!(field.reading.is_none());
    field.surface = tekes_ui_controls::TextSurface::new(&document, true, true);
    field.reading = Some(tekes_ui_controls::ReadingController::default());
    host.get_mut(ids::DIALOG_SAVE)
        .unwrap()
        .component_mut::<ButtonContent>()
        .unwrap()
        .label = "Return to latest".into();
    host.show_dialog(ids::DIALOG).unwrap();
    render(&mut host, &mut text);
    let field = host
        .get_mut(ids::DIALOG_FIELD)
        .unwrap()
        .component_mut::<TextFieldContent>()
        .unwrap();
    field
        .reading
        .as_mut()
        .unwrap()
        .follow_latest(&mut field.surface);
    field
        .surface
        .input
        .text
        .push_str("\nFirst streamed update.");
    render(&mut host, &mut text);
    let surface = host.text_surface(ids::DIALOG_FIELD).unwrap();
    assert_eq!(surface.scroll.offset.y, surface.scroll.limit().y);
    let point = surface.inner.center();
    let scroll = host.event(&ContentEvent::Scroll {
        position: point,
        delta: Point::new(0., -80.),
        precise: true,
        begin: true,
        end: false,
        momentum: false,
        time: 1.,
    });
    assert!(scroll.consumed);
    let field = host
        .get_mut(ids::DIALOG_FIELD)
        .unwrap()
        .component_mut::<TextFieldContent>()
        .unwrap();
    assert!(!field.reading.as_ref().unwrap().is_following_latest());
    let offset = field.surface.scroll.offset.y;
    assert!(offset > 0. && offset < field.surface.scroll.limit().y);
    field
        .surface
        .input
        .text
        .push_str("\nSecond streamed update while reading.");
    render(&mut host, &mut text);
    assert!(
        (host
            .text_surface(ids::DIALOG_FIELD)
            .unwrap()
            .scroll
            .offset
            .y
            - offset)
            .abs()
            < 0.01
    );
    assert!(host.focus(Some(ids::DIALOG_SAVE), true));
    let latest = host.event(&ContentEvent::TextCommand("insertNewline:".into()));
    assert!(latest.activated);
    let field = host
        .get_mut(ids::DIALOG_FIELD)
        .unwrap()
        .component_mut::<TextFieldContent>()
        .unwrap();
    field
        .reading
        .as_mut()
        .unwrap()
        .follow_latest(&mut field.surface);
    render(&mut host, &mut text);
    assert_eq!(host.focused(), Some(ids::DIALOG_SAVE));
    let surface = host.text_surface(ids::DIALOG_FIELD).unwrap();
    assert_eq!(surface.scroll.offset.y, surface.scroll.limit().y);
    assert!(host.focus(Some(ids::DIALOG_FIELD), true));
    let field = host
        .get_mut(ids::DIALOG_FIELD)
        .unwrap()
        .component_mut::<TextFieldContent>()
        .unwrap();
    field
        .reading
        .as_mut()
        .unwrap()
        .follow_latest(&mut field.surface);
    assert_eq!(host.focused(), Some(ids::DIALOG_FIELD));
    assert!(
        host.semantics()
            .iter()
            .any(|node| node.id == ids::DIALOG_SAVE
                && node.semantics.name == "Return to latest"
                && node.semantics.role == ContentRole::Button)
    );
    assert!(
        host.accessibility_action(ids::DIALOG_SAVE, None, AccessibilityAction::Press)
            .activated
    );
    let field = host
        .get_mut(ids::DIALOG_FIELD)
        .unwrap()
        .component_mut::<TextFieldContent>()
        .unwrap();
    field
        .reading
        .as_mut()
        .unwrap()
        .follow_latest(&mut field.surface);
    assert_eq!(host.focused(), Some(ids::DIALOG_SAVE));
}

#[test]
fn ordinary_text_field_wheel_does_not_reveal_old_caret_or_enable_following() {
    let mut host = scenarios::rich_dialog().unwrap();
    let mut text = HostText::default();
    let field = host
        .get_mut(ids::DIALOG_FIELD)
        .unwrap()
        .component_mut::<TextFieldContent>()
        .unwrap();
    field.surface =
        tekes_ui_controls::TextSurface::new(&"Long document line.\n".repeat(100), true, true);
    host.show_dialog(ids::DIALOG).unwrap();
    render(&mut host, &mut text);
    let point = host.text_surface(ids::DIALOG_FIELD).unwrap().inner.center();
    assert!(
        host.event(&ContentEvent::Scroll {
            position: point,
            delta: Point::new(0., 90.),
            precise: true,
            begin: true,
            end: true,
            momentum: false,
            time: 1.
        })
        .consumed
    );
    let field = host
        .get(ids::DIALOG_FIELD)
        .unwrap()
        .component::<TextFieldContent>()
        .unwrap();
    assert!(field.surface.scroll.offset.y > 0.);
    assert!(field.reading.is_none());
    assert_eq!(field.surface.input.caret, 0);
}

#[test]
fn real_editor_keeps_unsaved_input_and_retries_the_frozen_public_draft() {
    use tekes_ui_controls::DraftJournal;
    let mut host = scenarios::rich_dialog().unwrap();
    let mut text = HostText::default();
    host.show_dialog(ids::DIALOG).unwrap();
    render(&mut host, &mut text);
    let mut journal = DraftJournal::new(
        "consumer/account-A/settings",
        host.focused_text().unwrap().input.text.clone(),
    )
    .unwrap();
    let mut saved = None;
    let first = journal
        .begin_submission("request-1", |next| {
            saved = Some(next.clone());
            Ok(())
        })
        .unwrap();
    host.event(&ContentEvent::TextCommand("selectAll:".into()));
    host.event(&ContentEvent::Commit {
        text: "new typing".into(),
        replacement: None,
    });
    let edit = host.focused_text().unwrap().input.text.clone();
    assert!(
        journal
            .edit(&edit, |_| Err(std::io::Error::other("storage unavailable")))
            .is_err()
    );
    assert_eq!(host.focused_text().unwrap().input.text, "new typing");
    assert_eq!(journal.current().payload, first.draft.payload);
    journal
        .edit(&edit, |next| {
            saved = Some(next.clone());
            Ok(())
        })
        .unwrap();
    let disk = saved.unwrap();
    drop(journal);
    let mut journal = DraftJournal::restore(
        "consumer/account-A/settings",
        disk.current().clone(),
        disk.pending().to_vec(),
        disk.acknowledged(),
    )
    .unwrap();
    let retry = journal.retry_pending(|_| Ok(())).unwrap();
    assert_eq!(retry.draft, first.draft);
    assert_eq!(retry.idempotency_key, first.idempotency_key);
    assert!(
        journal
            .acknowledge(
                &first.draft.identity,
                first.draft.version,
                &first.request_id,
                |_| Ok(())
            )
            .unwrap()
    );
    assert!(!journal.is_acknowledged());
    assert_eq!(journal.current().payload, "new typing");
    assert!(
        !journal
            .acknowledge(
                &first.draft.identity,
                first.draft.version,
                &first.request_id,
                |_| Ok(())
            )
            .unwrap()
    );
    assert_eq!(host.focused_text().unwrap().input.text, "new typing");
    render(&mut host, &mut text);
}

#[test]
fn rejected_root_removal_keeps_rich_dialog_real_input_preedit_and_focus() {
    let mut host = scenarios::rich_dialog().unwrap();
    let mut text = HostText::default();
    render(&mut host, &mut text);
    host.focus(Some(ids::DIALOG_INVOKER), true);
    host.show_dialog(ids::DIALOG).unwrap();
    render(&mut host, &mut text);
    host.focus(Some(ids::DIALOG_FIELD), true);
    host.event(&ContentEvent::Preedit {
        text: "候補".into(),
        selection: 1..1,
        replacement: Some(0..0),
        clauses: std::iter::once(0..2).collect(),
    });
    let input = &host.focused_text().unwrap().input;
    let before = (input.text.clone(), input.selection(), input.marked.clone());
    let semantics = host.semantics();
    let legacy_draw: fn(
        &tekes_ui_controls::TextSurface,
        &mut HostText,
        &mut Scene,
        Theme,
        bool,
        bool,
    ) = tekes_ui_controls::TextSurface::draw::<HostText>;
    legacy_draw(
        host.focused_text().unwrap(),
        &mut text,
        &mut Scene::new(),
        Theme::LIGHT,
        true,
        true,
    );
    assert!(matches!(
        host.remove(ids::DIALOG_SCREEN),
        Err(CompositionError::MissingId(_))
    ));
    assert!(
        host.get(ids::DIALOG)
            .unwrap()
            .component::<DialogContent>()
            .unwrap()
            .is_open()
    );
    assert_eq!(host.focused(), Some(ids::DIALOG_FIELD));
    let after = &host.focused_text().unwrap().input;
    assert_eq!(after.text, before.0);
    assert_eq!(after.selection(), before.1);
    assert_eq!(after.marked, before.2);
    assert_eq!(host.semantics(), semantics);
    // A valid removal still dismisses and restores the stable invoker once.
    assert!(host.remove(ids::DIALOG).is_ok());
    assert_eq!(host.focused(), Some(ids::DIALOG_INVOKER));
    assert!(host.semantics().iter().all(|n| n.id != ids::DIALOG));
}

#[test]
fn native_text_backend_action_views_honor_model_changes_between_host_events() {
    use tekes_ui_controls::{
        extended::{IconButton, Link, SegmentedControl},
        layout_view::{IconButtonView, LinkView, SegmentedControlView},
    };
    let mut text = HostText::default();
    let palette = Palette::from(Theme::LIGHT);
    let mut icon = IconButton::new("Create item").unwrap();
    let mut icon_view = IconButtonView::new();
    let icon_frame = Rect::new(0., 0., 40., 40.);
    icon_view.render(
        &mut Scene::new(),
        &mut text,
        &mut icon,
        icon_frame,
        None,
        palette,
    );
    assert!(icon_view.pointer_down(icon_frame.center()));
    // Controlled application state arrives between pointer down and release.
    icon.button.state.loading = true;
    assert!(!icon_view.pointer_up(icon_frame.center(), &mut icon));
    assert!(icon.button.state.loading);
    icon.button.state.loading = false;
    icon_view.render(
        &mut Scene::new(),
        &mut text,
        &mut icon,
        icon_frame,
        None,
        palette,
    );
    assert!(!icon_view.pointer_up(icon_frame.center(), &mut icon));
    assert!(icon_view.pointer_down(icon_frame.center()));
    assert!(icon_view.pointer_up(icon_frame.center(), &mut icon));
    let mut link = Link::default();
    let mut link_view = LinkView::default();
    let link_frame = Rect::new(60., 0., 240., 40.);
    link_view.render(
        &mut Scene::new(),
        &mut text,
        &mut link,
        "Read documentation",
        link_frame,
        None,
        palette,
        proposal::TextRole::UiBody,
    );
    assert!(link_view.pointer_down(link_frame.center()));
    link.button.state.disabled = true;
    assert!(!link_view.pointer_up(link_frame.center(), &mut link));
    assert!(link.button.state.disabled);
    assert!(
        !link_view
            .key(tekes_ui_controls::ControlKey::Enter, true, &mut link)
            .changed
    );
    link.button.state.disabled = false;
    link_view.render(
        &mut Scene::new(),
        &mut text,
        &mut link,
        "Read documentation",
        link_frame,
        None,
        palette,
        proposal::TextRole::UiBody,
    );
    assert!(!link_view.pointer_up(link_frame.center(), &mut link));
    let mut model = SegmentedControl::new(3);
    let mut view = SegmentedControlView::new(3);
    let frame = Rect::new(0., 60., 360., 40.);
    view.render(
        &mut Scene::new(),
        &mut text,
        &mut model,
        &["General", "Advanced", "Security"],
        frame,
        None,
        palette,
        proposal::TextRole::UiBody,
        false,
    );
    view.focus(1);
    assert!(
        view.key_event(tekes_ui_controls::ControlKey::Space, true, &mut model)
            .consumed
    );
    model.set_enabled(1, false);
    assert!(
        !view
            .key_event(tekes_ui_controls::ControlKey::Space, false, &mut model)
            .changed
    );
    model.set_enabled(1, true);
    view.focus(1);
    assert!(
        !view
            .key_event(tekes_ui_controls::ControlKey::Space, false, &mut model)
            .changed
    );
    assert_eq!(model.selected, 0);
}

#[test]
fn native_text_segment_group_availability_preserves_caller_choices() {
    use tekes_ui_controls::{extended::SegmentedControl, layout_view::SegmentedControlView};
    let mut text = HostText::default();
    let mut model = SegmentedControl::new(3);
    model.set_enabled(2, false);
    let mut view = SegmentedControlView::new(3);
    let frame = Rect::new(0., 0., 360., 40.);
    view.render(
        &mut Scene::new(),
        &mut text,
        &mut model,
        &["General", "Advanced", "Security"],
        frame,
        None,
        Theme::LIGHT.into(),
        proposal::TextRole::UiBody,
        false,
    );
    let point = view.layout().items[1].frame.center();
    assert_eq!(view.pointer_down(point), Some(1));
    view.set_disabled(true);
    view.render(
        &mut Scene::new(),
        &mut text,
        &mut model,
        &["General", "Advanced", "Security"],
        frame,
        None,
        Theme::LIGHT.into(),
        proposal::TextRole::UiBody,
        false,
    );
    assert_eq!(model.selected, 0);
    assert!(!model.is_enabled(2));
    assert!(view.pointer_up(point, &mut model).is_none());
    view.set_disabled(false);
    view.render(
        &mut Scene::new(),
        &mut text,
        &mut model,
        &["General", "Advanced", "Security"],
        frame,
        None,
        Theme::LIGHT.into(),
        proposal::TextRole::UiBody,
        false,
    );
    assert!(view.pointer_up(point, &mut model).is_none());
    assert_eq!(model.selected, 0);
    assert!(!model.is_enabled(2));
    assert_eq!(view.pointer_down(point), Some(1));
    assert_eq!(view.pointer_up(point, &mut model), Some(1));
}

#[test]
fn native_text_icon_defaults_resolve_shared_tokens_and_preserve_explicit_layout() {
    use tekes_ui_controls::{
        extended::IconButton,
        layout_view::{IconButtonView, IconRenderer},
    };
    #[derive(Default)]
    struct Assets(Vec<Rect>);
    impl IconRenderer for Assets {
        fn draw_icon(&mut self, _: &mut Scene, _: &str, frame: Rect, _: Rgba) {
            self.0.push(frame);
        }
    }
    let mut text = HostText::default();
    let mut assets = Assets::default();
    let mut model = IconButton::new("Create item").unwrap();
    let mut view = IconButtonView::new();
    for compact in [false, true] {
        let natural = IconButtonView::measure(compact);
        let frame = Rect::new(4., 6., natural.width, natural.height);
        let layout = view.render_with_icon(
            &mut Scene::new(),
            &mut text,
            &mut assets,
            "plus",
            &mut model,
            frame,
            None,
            Theme::LIGHT.into(),
            0.,
        );
        assert_eq!(layout.frame, frame);
        let icon = *assets.0.last().unwrap();
        let side = proposal::metric("size", "size-icon-standard").unwrap();
        assert_eq!(icon.size, Size::new(side, side));
        assert_eq!(icon.center(), frame.center());
        assert!(view.pointer_down(frame.center()));
        assert!(view.pointer_up(frame.center(), &mut model));
    }
    let frame = Rect::new(0., 0., 56., 48.);
    view.render_with_icon(
        &mut Scene::new(),
        &mut text,
        &mut assets,
        "plus",
        &mut model,
        frame,
        None,
        Theme::LIGHT.into(),
        20.,
    );
    assert_eq!(view.layout().frame, frame);
    assert_eq!(assets.0.last().unwrap().size, Size::new(20., 20.));
}

#[test]
fn native_text_navigation_views_observe_availability_without_a_repaint() {
    use tekes_ui_controls::{
        extended::{Breadcrumb, Disclosure},
        layout_view::{BreadcrumbView, DisclosureView},
    };
    let mut text = HostText::default();
    let mut model = Disclosure::default();
    let mut view = DisclosureView::default();
    view.render(
        &mut Scene::new(),
        &mut text,
        &model,
        "Details",
        Rect::new(0., 0., 240., 40.),
        None,
        Theme::LIGHT.into(),
        proposal::TextRole::UiBody,
    );
    view.focus(true);
    model.disabled = true;
    assert!(!view.key_event(ControlKey::Space, true, &mut model).consumed);
    model.disabled = false;
    assert!(!view.key_event(ControlKey::Space, false, &mut model).changed);
    assert!(!model.open);
    assert!(view.key_event(ControlKey::Space, true, &mut model).consumed);
    assert!(view.key_event(ControlKey::Space, false, &mut model).changed);
    assert!(model.open);
    let model = Breadcrumb {
        items: vec!["Home".into(), "Docs".into(), "Current".into()],
    };
    let mut view = BreadcrumbView::default();
    view.render(
        &mut Scene::new(),
        &mut text,
        &model,
        &["Home", "Docs", "Current"],
        Rect::new(0., 50., 480., 40.),
        None,
        Theme::LIGHT.into(),
        proposal::TextRole::UiBody,
    );
    let point = view.layout().items[0].frame().center();
    assert_eq!(view.pointer_down(point), Some(0));
    view.set_disabled(true);
    assert!(view.pointer_up(point, &model).is_none());
    assert!(view.activate(0, &model).is_none());
    view.set_disabled(false);
    view.focus(0);
    assert_eq!(
        view.key_event(ControlKey::Space, true, &model),
        (false, None)
    );
    assert_eq!(view.activate(usize::MAX, &model), None);
    assert_eq!(model.items.len(), 3);
}

#[test]
fn retained_content_status_changes_cancel_hidden_ime_input_without_losing_the_draft() {
    use tekes_ui_composition_consumer::content_status::{self as c, Kind, Scenario, Status};
    for kind in [Kind::Tabs, Kind::Disclosure] {
        let mut scenario = Scenario::new(kind).unwrap();
        let mut text = HostText::default();
        if kind == Kind::Disclosure {
            scenario.set_open(true);
        }
        // Public Component callbacks can pass a borrowed dyn renderer into the
        // same existing View APIs; native text and motion capabilities survive.
        {
            let mut borrowed: &mut dyn ButtonTextRenderer = &mut text;
            scenario
                .host
                .render(
                    &mut Scene::new(),
                    &mut borrowed,
                    Rect::new(0., 0., 640., 520.),
                    Rect::new(0., 0., 640., 520.),
                    Theme::LIGHT.into(),
                )
                .unwrap();
        }
        assert!(scenario.host.focus(Some(c::EDITOR), true));
        assert!(
            scenario
                .host
                .accessibility_action(c::EDITOR, None, AccessibilityAction::SetSelection(2..5))
                .changed
        );
        let before = scenario
            .host
            .get(c::EDITOR)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap()
            .surface
            .input
            .text
            .clone();
        assert!(
            scenario
                .host
                .event(&ContentEvent::Preedit {
                    text: "未提交".into(),
                    selection: 0..3,
                    clauses: std::iter::once(0..3).collect(),
                    replacement: None
                })
                .consumed
        );
        let visible_draft = scenario
            .host
            .get(c::EDITOR)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap()
            .surface
            .input
            .text
            .clone();
        scenario.set_status(Status::Loading);
        assert!(
            !scenario
                .host
                .event(&ContentEvent::Commit {
                    text: "must not leak".into(),
                    replacement: None
                })
                .consumed
        );
        assert_eq!(scenario.host.focused(), None);
        assert!(scenario.host.focused_text().is_none());
        assert!(!scenario.host.semantics().iter().any(|n| n.id == c::EDITOR));
        scenario.set_status(Status::Ready);
        render(&mut scenario.host, &mut text);
        let field = scenario
            .host
            .get(c::EDITOR)
            .unwrap()
            .component::<TextFieldContent>()
            .unwrap();
        assert_eq!(field.surface.input.text, visible_draft);
        assert_eq!(field.surface.input.selection(), 2..5);
        assert!(field.surface.input.marked.is_none());
        assert!(scenario.host.focus(Some(c::EDITOR), true));
        if kind == Kind::Tabs {
            scenario.select(1);
        } else {
            scenario.set_open(false);
        }
        assert!(
            !scenario
                .host
                .event(&ContentEvent::Commit {
                    text: "hidden panel".into(),
                    replacement: None
                })
                .consumed
        );
        assert_eq!(scenario.host.focused(), None);
        assert!(!scenario.visible());
        scenario.set_status(Status::Error);
        scenario.set_status(Status::Empty);
        assert!(
            !scenario.visible(),
            "content status never forces a panel open or changes tab selection"
        );
        if kind == Kind::Tabs {
            assert_eq!(scenario.selected(), 1);
            scenario.select(0);
        } else {
            scenario.set_open(true);
        }
        scenario.set_status(Status::Ready);
        render(&mut scenario.host, &mut text);
        assert_eq!(
            scenario
                .host
                .get(c::EDITOR)
                .unwrap()
                .component::<TextFieldContent>()
                .unwrap()
                .surface
                .input
                .text,
            visible_draft
        );
        assert!(scenario.host.focus(Some(c::EDITOR), true));
        assert!(
            scenario
                .host
                .event(&ContentEvent::TextCommand("undo:".into()))
                .consumed
        );
        assert_eq!(
            scenario
                .host
                .get(c::EDITOR)
                .unwrap()
                .component::<TextFieldContent>()
                .unwrap()
                .surface
                .input
                .text,
            before
        );
    }
}

#[test]
fn content_statuses_and_header_focus_are_independent_public_consumer_cases() {
    use tekes_ui_composition_consumer::content_status::{self as c, Kind, Scenario, Status};
    let mut cases = Vec::new();
    for kind in [Kind::Tabs, Kind::Disclosure] {
        for state in ["loading", "error", "empty", "loading+focus", "error+focus"] {
            for theme in Theme::ALL {
                for brand in 0..3 {
                    let palette = Palette::new(theme, brand).unwrap();
                    let mut scenario = Scenario::new(kind).unwrap();
                    let mut text = HostText::default();
                    text.enable_clock(true);
                    if kind == Kind::Disclosure {
                        scenario.set_open(true);
                    }
                    let status = match state.split('+').next().unwrap() {
                        "loading" => Status::Loading,
                        "error" => Status::Error,
                        _ => Status::Empty,
                    };
                    scenario.set_status(status);
                    let mut scene = Scene::new();
                    let run = || {
                        scenario
                            .host
                            .render(
                                &mut scene,
                                &mut text,
                                Rect::new(0., 0., 640., 520.),
                                Rect::new(0., 0., 640., 520.),
                                palette,
                            )
                            .unwrap();
                        if state.contains("focus") {
                            assert!(scenario.host.focus(Some(c::HEADER), true));
                        }
                        scenario
                            .host
                            .render(
                                &mut scene,
                                &mut text,
                                Rect::new(0., 0., 640., 520.),
                                Rect::new(0., 0., 640., 520.),
                                palette,
                            )
                            .unwrap();
                        let nodes = scenario.host.semantics();
                        let feedback = nodes.iter().find(|n| n.id == c::FEEDBACK).unwrap();
                        assert_eq!(feedback.semantics.name, status.label());
                        assert_eq!(feedback.semantics.invalid, status == Status::Error);
                        assert!(!nodes.iter().any(|n| n.id == c::EDITOR));
                        assert!(scenario.visible());
                        assert_eq!(scenario.selected(), 0);
                        assert_eq!(
                            scenario.host.focused(),
                            state.contains("focus").then_some(c::HEADER)
                        );
                        assert!(!scene.is_empty());
                    };
                    #[cfg(feature = "token-audit")]
                    let (_, reads) = tekes_ui_tokens::audit::capture(run);
                    #[cfg(not(feature = "token-audit"))]
                    let reads: Vec<(String, String)> = {
                        let mut run = run;
                        run();
                        Vec::new()
                    };
                    cases.push(serde_json::json!({"component":if kind==Kind::Tabs{"tui.tabs"}else{"tui.disclosure"},"content_state":state,"state_asserted":true,"theme":format!("{theme:?}"),"brand":brand,"lookups":reads}));
                }
            }
        }
    }
    assert_eq!(cases.len(), 120);
    if let Ok(path) = std::env::var("TEKESUI_CONTENT_STATUS_AUDIT_OUT") {
        std::fs::write(path,serde_json::to_vec_pretty(&serde_json::json!({"format":"tekesui-runtime-token-observation/1","complete":false,"scope":"Independent consumer using existing public TabsView/DisclosureView, Component::active, ContentHost and real Progress/Banner/EmptyState.6 required content states+4 combinations, four themes/three brands; no new builtin content adapter or status protocol.","excluded":["native input/AX","builtin Tabs/Disclosure Component adapter","compiled aliases","unexecuted paths"],"cases":cases})).unwrap()).unwrap();
    }
}

#[test]
fn public_choice_views_reject_old_pointer_mapping_with_real_text_services() {
    use tekes_ui_controls::{
        extended::{ComboBox, CommandPalette},
        layout_view::{ComboBoxView, CommandPaletteView},
    };
    let mut text = HostText::default();
    for palette in [false, true] {
        let mut combo = ComboBoxView::default();
        let mut commands = CommandPaletteView::default();
        let mut model = CommandPalette {
            combo: ComboBox {
                items: vec!["First choice".into(), "Second choice".into()],
                open: true,
                active: Some(0),
                ..Default::default()
            },
        };
        let mut scene = Scene::new();
        let anchor = Rect::new(16., 16., 240., 40.);
        let viewport = Rect::new(0., 0., 360., 320.);
        let layout = if palette {
            commands.render(
                &mut scene,
                &mut text,
                &mut model,
                anchor,
                viewport,
                None,
                Theme::LIGHT.into(),
                proposal::TextRole::UiBody,
                false,
            )
        } else {
            combo.render(
                &mut scene,
                &mut text,
                &mut model.combo,
                anchor,
                viewport,
                None,
                Theme::LIGHT.into(),
                proposal::TextRole::UiBody,
                false,
            )
        }
        .unwrap();
        // HostText supplies CoreText measurement but deliberately omits glyph
        // rasterization. Actual pixels are checked by the Gallery GPU receipt.
        assert!(layout.items[0].label_size.width > 0.);
        assert!(!scene.primitives().is_empty());
        let point = layout.items[0].frame.center();
        if palette {
            commands.pointer_down(point);
        } else {
            combo.pointer_down(point);
        }
        model.combo.items.clear();
        let selected = if palette {
            commands.pointer_up(point, &mut model)
        } else {
            combo.pointer_up(point, &mut model.combo)
        };
        assert_eq!(selected, None);
        assert_eq!(model.combo.selected, None);
        assert!(model.combo.query.is_empty());
    }
}

#[test]
fn public_menu_measurement_reserves_real_text_rows_without_a_paint_probe() {
    use tekes_ui_controls::{Menu, MenuView};
    let mut text = HostText::default();
    let labels = [
        "Open a recent project 项目計画",
        "Review changes in the workspace",
        "A final choice",
    ];
    for width in [180., 320.] {
        let size = MenuView::measure(&mut text, &labels, width, proposal::TextRole::UiBody, false);
        let mut menu = Menu::new(labels.len());
        menu.open = true;
        let mut view = MenuView::new_external();
        let layout = view
            .render_with_model(
                &mut menu,
                &mut Scene::new(),
                &mut text,
                &labels,
                Rect::new(16., 16., width - 32., 72.),
                Rect::new(0., 0., width, 1000.),
                None,
                Theme::LIGHT.into(),
                proposal::TextRole::UiBody,
                false,
            )
            .unwrap();
        assert_eq!(layout.popup.size, size);
        assert!(size.height > 0.);
        assert!(layout.popup.min_y() >= 88.);
        assert!(layout.items.iter().all(|item| item.visible.is_some()));
    }
}
