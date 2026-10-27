#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{
    ButtonTextAlign, ButtonTextRenderer, ControlKey, extended::Disclosure,
    layout_view::DisclosureView, paint,
};
use tekes_ui_scene::{Point, Primitive, Rect, Scene};
use tekes_ui_tokens::{Palette, Theme, proposal};

#[test]
fn retained_disclosure_themes_keep_header_states_open_body_and_real_input() {
    let mut model = Disclosure::default();
    let mut view = DisclosureView::default();
    let mut text = coretext::Text::new(1.);
    text.services = true;
    let frame = Rect::new(24., 24., 400., 48.);
    let body_frame = Rect::new(24., 90., 400., 110.);
    let clip = Rect::new(0., 0., 480., 300.);
    let mut geometry = None;
    for theme in Theme::ALL {
        let palette = Palette::from(theme);
        for open in [false, true] {
            if model.open != open {
                view.focus(true);
                let down = view.key_event(ControlKey::Enter, true, &mut model);
                let up = view.key_event(ControlKey::Enter, false, &mut model);
                assert!(down.consumed && up.consumed);
                assert!(down.changed || up.changed);
            }
            assert_eq!(model.open, open);
            for state in ["rest", "hover", "pressed", "disabled", "rest"] {
                model.disabled = state == "disabled";
                view.render(
                    &mut Scene::new(),
                    &mut text,
                    &model,
                    "Details 详情",
                    frame,
                    Some(clip),
                    palette,
                    proposal::TextRole::UiBody,
                );
                view.pointer_move(Point::new(470., 280.));
                if !model.disabled {
                    view.focus(true);
                }
                if matches!(state, "hover" | "pressed") {
                    view.pointer_move(frame.center());
                }
                if state == "pressed" {
                    assert!(view.pointer_down(frame.center()));
                }
                let mut scene = Scene::new();
                let layout = view.render(
                    &mut scene,
                    &mut text,
                    &model,
                    "Details 详情",
                    frame,
                    Some(clip),
                    palette,
                    proposal::TextRole::UiBody,
                );
                assert_eq!(model.open, open, "paint/press does not toggle");
                if let Some(original) = geometry {
                    assert_eq!(layout, original);
                } else {
                    geometry = Some(layout);
                }
                let ground = scene
                    .primitives()
                    .iter()
                    .find_map(|p| match p {
                        Primitive::Quad(q) if q.rect == layout.frame => Some(*q),
                        _ => None,
                    })
                    .unwrap();
                assert_eq!(
                    ground.fill,
                    palette
                        .color(match state {
                            "hover" => "button-hover",
                            "pressed" => "button-pressed",
                            _ => "surface-2",
                        })
                        .unwrap()
                );
                assert_eq!(
                    ground.corner_radii,
                    tekes_ui_scene::CornerRadii::uniform(
                        proposal::metric("radius", "radius-row").unwrap()
                    )
                );
                let ink = palette
                    .color(if model.disabled {
                        "tertiary-text"
                    } else {
                        "primary-text"
                    })
                    .unwrap();
                let labels: Vec<_> = scene
                    .primitives()
                    .iter()
                    .filter_map(|p| match p {
                        Primitive::Glyphs(run)
                            if !run.glyphs.is_empty()
                                && run.glyphs.iter().all(|g| layout.content.contains(g.origin)) =>
                        {
                            Some(run)
                        }
                        _ => None,
                    })
                    .collect();
                assert!(!labels.is_empty());
                assert!(labels.iter().all(|r| r.color == ink));
                if !model.disabled {
                    assert_eq!((labels[0].color.a, ground.fill.a), (1., 1.));
                    let luminance = |c: tekes_ui_tokens::Rgba| {
                        let c = c.to_linear();
                        0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
                    };
                    let (a, b) = (luminance(labels[0].color), luminance(ground.fill));
                    assert!(
                        (a.max(b) + 0.05) / (a.min(b) + 0.05)
                            >= if theme.is_increased() { 7. } else { 4.5 }
                    );
                }
                if state == "rest" {
                    assert!(scene.primitives().iter().any(|p|matches!(p,Primitive::Quad(q) if q.fill.a==0.&&q.border_color==palette.color("border-focused").unwrap())));
                }
                let body = view.layout_body(&mut text, &model, body_frame, Some(clip));
                assert_eq!(body.body_rect, body_frame);
                assert_eq!(body.progress, if open { 1. } else { 0. });
                assert_eq!(body.visible_clip, open.then_some(body_frame));
                assert_eq!(body.interactive, open);
                if let Some(body_clip) = body.visible_clip {
                    scene.push_clip(body_clip);
                    paint::surface_at_scale(
                        &mut scene,
                        body.body_rect,
                        palette,
                        true,
                        text.backing,
                    );
                    text.draw(
                        &mut scene,
                        "Retained body 正文",
                        proposal::TextRole::UiBody,
                        body_frame.inset(14.),
                        ButtonTextAlign::Leading,
                        palette.color("primary-text").unwrap(),
                    );
                    scene.pop_clip();
                    let panel = scene
                        .primitives()
                        .iter()
                        .find_map(|p| match p {
                            Primitive::Quad(q) if q.rect == body_frame => Some(q),
                            _ => None,
                        })
                        .unwrap();
                    assert_eq!(panel.fill, palette.color("surface-panel").unwrap());
                    let labels: Vec<_> = scene
                        .primitives()
                        .iter()
                        .filter_map(|p| match p {
                            Primitive::Glyphs(r)
                                if !r.glyphs.is_empty()
                                    && r.glyphs.iter().all(|g| body_frame.contains(g.origin)) =>
                            {
                                Some(r)
                            }
                            _ => None,
                        })
                        .collect();
                    assert!(!labels.is_empty());
                    assert!(
                        labels
                            .iter()
                            .all(|r| r.color == palette.color("primary-text").unwrap())
                    );
                    let luminance = |c: tekes_ui_tokens::Rgba| {
                        let c = c.to_linear();
                        0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
                    };
                    assert_eq!((labels[0].color.a, panel.fill.a), (1., 1.));
                    let (a, b) = (luminance(labels[0].color), luminance(panel.fill));
                    assert!(
                        (a.max(b) + 0.05) / (a.min(b) + 0.05)
                            >= if theme.is_increased() { 7. } else { 4.5 }
                    );
                }
                assert!(scene.composition_error().is_none());
                if model.disabled {
                    assert!(!view.pointer_down(frame.center()));
                    assert!(!view.pointer_up(frame.center(), &mut model));
                    assert!(!view.key_event(ControlKey::Space, true, &mut model).consumed);
                    assert_eq!(model.open, open);
                }
                view.cancel();
            }
        }
    }
    let open = model.open;
    let point = view.layout().frame.center();
    assert!(!view.pointer_up(point, &mut model), "release without press");
    assert!(view.pointer_down(point));
    view.pointer_move(Point::new(470., 280.));
    assert!(!view.pointer_up(Point::new(470., 280.), &mut model));
    assert_eq!(model.open, open, "drag-out release cancels");
    assert!(view.pointer_down(point));
    assert!(view.pointer_up(point, &mut model));
    assert_ne!(model.open, open);
    view.focus(true);
    assert!(!view.key_event(ControlKey::Space, true, &mut model).changed);
    assert!(view.key_event(ControlKey::Space, false, &mut model).changed);
    assert_eq!(model.open, open);
}
