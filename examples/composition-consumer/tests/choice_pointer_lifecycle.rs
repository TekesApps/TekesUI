use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, Select,
    extended::{ComboBox, CommandPalette},
    layout_view::{ComboBoxView, CommandPaletteView, SelectView},
};
use tekes_ui_scene::{Point, Primitive, Rect, Scene, Size};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal};

#[derive(Default)]
struct Text;

impl ButtonTextRenderer for Text {
    fn measure(&mut self, value: &str, _: proposal::TextRole, width: f32) -> Size {
        Size::new((value.len() as f32 * 7.0).min(width), 20.0)
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

fn fill_at(scene: &Scene, frame: Rect) -> Option<Rgba> {
    scene
        .primitives()
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Quad(quad) if quad.rect == frame => Some(quad.fill),
            _ => None,
        })
}

#[test]
fn command_palette_disabled_pointer_move_cannot_restore_trigger_hover() {
    let palette = Palette::from(Theme::LIGHT);
    let frame = Rect::new(10.0, 10.0, 300.0, 50.0);
    let mut view = CommandPaletteView::default();
    let mut model = CommandPalette::default();
    let mut text = Text;
    let draw = |view: &mut CommandPaletteView, model: &mut CommandPalette, text: &mut Text| {
        let mut scene = Scene::new();
        view.render_trigger(
            &mut scene,
            text,
            model,
            "Commands",
            frame,
            None,
            palette,
            proposal::TextRole::UiBody,
        );
        scene
    };

    let rest = draw(&mut view, &mut model, &mut text);
    let trigger = view.trigger_layout().unwrap().frame;
    let rest_fill = fill_at(&rest, trigger).expect("trigger background is painted");
    let hover_point = trigger.center();
    view.pointer_move(hover_point, &mut model);
    let hover = draw(&mut view, &mut model, &mut text);
    assert_ne!(
        fill_at(&hover, trigger),
        Some(rest_fill),
        "hover is observable"
    );
    assert!(!model.combo.open, "hover must not open the popup");

    model.combo.disabled = true;
    view.pointer_move(Point::new(900.0, 900.0), &mut model);
    model.combo.disabled = false;
    let after_reenable = draw(&mut view, &mut model, &mut text);
    assert_eq!(fill_at(&after_reenable, trigger), Some(rest_fill));

    view.pointer_move(hover_point, &mut model);
    let hover = draw(&mut view, &mut model, &mut text);
    assert_ne!(fill_at(&hover, trigger), Some(rest_fill));
    assert!(!model.combo.open, "hover must not open the popup");
    view.set_window_active(false, &mut model);
    view.pointer_move(hover_point, &mut model);
    let inactive = draw(&mut view, &mut model, &mut text);
    assert_eq!(fill_at(&inactive, trigger), Some(rest_fill));
    assert!(!model.combo.open);
}

#[test]
fn select_disabled_pointer_move_cannot_restore_trigger_hover() {
    let palette = Palette::from(Theme::LIGHT);
    let frame = Rect::new(10.0, 10.0, 300.0, 50.0);
    let outside = Point::new(900.0, 900.0);
    let labels = ["One", "Two"];
    let mut view = SelectView::new(2);
    let mut model = Select::new(2);
    let mut text = Text;
    let mut draw = |view: &mut SelectView, model: &mut Select| {
        let mut scene = Scene::new();
        view.render_trigger(
            &mut scene,
            &mut text,
            model,
            &labels,
            frame,
            None,
            palette,
            proposal::TextRole::UiBody,
        );
        scene
    };

    let rest = draw(&mut view, &mut model);
    let trigger = view.layout().trigger.frame;
    let rest_fill = fill_at(&rest, trigger).expect("trigger background is painted");
    let hover_point = trigger.center();
    view.pointer_move(hover_point, &mut model);
    let hover = draw(&mut view, &mut model);
    assert_ne!(
        fill_at(&hover, trigger),
        Some(rest_fill),
        "hover is observable"
    );

    view.set_disabled(true, &mut model);
    view.pointer_move(outside, &mut model);
    view.set_disabled(false, &mut model);
    let after_reenable = draw(&mut view, &mut model);
    assert_eq!(fill_at(&after_reenable, trigger), Some(rest_fill));

    view.pointer_move(hover_point, &mut model);
    let hover = draw(&mut view, &mut model);
    assert_ne!(fill_at(&hover, trigger), Some(rest_fill));
    view.set_window_active(false, &mut model);
    view.pointer_move(outside, &mut model);
    let inactive = draw(&mut view, &mut model);
    assert_eq!(fill_at(&inactive, trigger), Some(rest_fill));
}

#[test]
fn combobox_disabled_scope_still_discards_candidate_pointer() {
    let anchor = Rect::new(10.0, 10.0, 240.0, 40.0);
    let viewport = Rect::new(0.0, 0.0, 600.0, 500.0);
    let outside = Point::new(900.0, 900.0);
    let palette = Palette::from(Theme::LIGHT);
    let mut view = ComboBoxView::default();
    let mut model = ComboBox {
        items: vec!["Alpha".into(), "Beta".into()],
        query: String::new(),
        selected: None,
        open: true,
        active: Some(1),
        disabled: false,
    };
    let mut text = Text;
    let mut render = |view: &mut ComboBoxView, model: &mut ComboBox| {
        let mut scene = Scene::new();
        let layout = view
            .render(
                &mut scene,
                &mut text,
                model,
                anchor,
                viewport,
                None,
                palette,
                proposal::TextRole::UiBody,
                false,
            )
            .expect("open ComboBox options should be mounted");
        (scene, layout)
    };

    let (baseline, layout) = render(&mut view, &mut model);
    let row = layout.items[1].frame;
    let resting_fill = fill_at(&baseline, row).expect("candidate button chrome is painted");
    view.pointer_move(row.center(), &mut model);
    let (hovered, _) = render(&mut view, &mut model);
    assert_ne!(
        fill_at(&hovered, row),
        Some(resting_fill),
        "row hover is observable"
    );

    model.disabled = true;
    view.pointer_move(outside, &mut model);
    model.disabled = false;
    let (returned, _) = render(&mut view, &mut model);
    assert_eq!(fill_at(&returned, row), Some(resting_fill));
}
