use tekes_ui_controls::{
    extended::{Progress, StatusBadge},
    feedback_view::{ProgressView, SkeletonView, StatusBadgeView},
};
use tekes_ui_scene::{Primitive, Rect, Scene};
use tekes_ui_tokens::{Palette, Theme};

fn quads(scene: &Scene) -> Vec<&tekes_ui_scene::Quad> {
    scene
        .primitives()
        .iter()
        .filter_map(|p| match p {
            Primitive::Quad(q) => Some(q),
            _ => None,
        })
        .collect()
}

#[test]
fn progress_value_is_finite_bounded_and_rejects_invalid_updates() {
    for input in [
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        -1.,
        0.,
        0.45,
        1.,
        2.,
    ] {
        let mut model = Progress::new(input);
        let expected = if input.is_finite() {
            input.clamp(0., 1.)
        } else {
            0.
        };
        assert_eq!(model.value(), expected);
        model.indeterminate = true;
        for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert!(!model.set(invalid));
            assert_eq!(model.value(), expected);
            assert!(model.indeterminate);
        }
        assert_eq!(model.set(0.5), expected != 0.5);
        assert!(!model.set(0.5));
        assert!(model.set(5.));
        assert_eq!(model.value(), 1.);
    }
}

#[test]
fn passive_views_reject_unavailable_geometry_and_clear_previous_layout() {
    let good = Rect::new(10., 20., 200., 12.);
    let mut progress = ProgressView::default();
    let mut badge = StatusBadgeView::default();
    let mut skeleton = SkeletonView::default();
    for (frame, clip) in [
        (Rect::new(10., 20., 0., 12.), None),
        (Rect::new(10., 20., 200., 0.), None),
        (Rect::new(f32::NAN, 20., 200., 12.), None),
        (Rect::new(10., 20., -1., 12.), None),
        (good, Some(Rect::new(400., 20., 20., 20.))),
        (good, Some(Rect::new(10., 20., 0., 12.))),
    ] {
        let mut scene = Scene::new();
        progress.render(
            &mut scene,
            Progress::new(0.45),
            good,
            None,
            Theme::LIGHT.into(),
        );
        badge.render(
            &mut scene,
            StatusBadge::Info,
            good,
            None,
            Theme::LIGHT.into(),
        );
        skeleton.render(&mut scene, good, None, Theme::LIGHT.into());
        let mut hidden = Scene::new();
        assert!(
            progress
                .render(
                    &mut hidden,
                    Progress::new(0.45),
                    frame,
                    clip,
                    Theme::LIGHT.into()
                )
                .visible
                .is_none()
        );
        assert!(
            badge
                .render(
                    &mut hidden,
                    StatusBadge::Info,
                    frame,
                    clip,
                    Theme::LIGHT.into()
                )
                .visible
                .is_none()
        );
        assert!(
            skeleton
                .render(&mut hidden, frame, clip, Theme::LIGHT.into())
                .visible
                .is_none()
        );
        assert!(hidden.is_empty());
        assert!(progress.layout().visible.is_none());
        assert!(badge.layout().visible.is_none());
        assert!(skeleton.layout().visible.is_none());
    }
}

#[test]
fn public_feedback_paint_preserves_progress_and_outcome_semantic_separation() {
    for theme in Theme::ALL {
        for brand in 0..3 {
            let palette = Palette::new(theme, brand).expect("registered Gallery brand");
            let frame = Rect::new(10., 20., 200., 12.);
            let clip = Rect::new(30., 20., 100., 12.);
            for value in [0., 0.45, 1.] {
                let mut view = ProgressView::default();
                let mut scene = Scene::new();
                let model = Progress::new(value);
                assert_eq!(
                    view.render(&mut scene, model, frame, Some(clip), palette)
                        .visible,
                    Some(clip)
                );
                let q = quads(&scene);
                assert_eq!(q.len(), 2);
                assert_eq!(q[0].fill, palette.color("surface-2").unwrap());
                assert_eq!(q[1].fill, palette.color("c-progress-active").unwrap());
                assert_eq!(q[1].rect.size.width, value * frame.size.width);
                assert_eq!(model.value(), value);
            }
            for (status, role) in [
                (StatusBadge::Info, "status-info"),
                (StatusBadge::Running, "status-running"),
                (StatusBadge::Succeeded, "status-succeeded"),
                (StatusBadge::Failed, "status-failed"),
            ] {
                let mut scene = Scene::new();
                StatusBadgeView::default().render(&mut scene, status, frame, Some(clip), palette);
                assert_eq!(quads(&scene)[0].fill, palette.color(role).unwrap());
                assert_eq!(
                    StatusBadgeView::label_color(palette),
                    palette.color("c-status-badge-foreground").unwrap()
                );
            }
            let mut scene = Scene::new();
            SkeletonView::default().render(&mut scene, frame, Some(clip), palette);
            assert_eq!(quads(&scene)[0].fill, palette.color("surface-2").unwrap());
        }
    }
}

#[cfg(feature = "token-audit")]
#[test]
fn actual_passive_feedback_token_reads() {
    let mut cases = Vec::new();
    for theme in Theme::ALL {
        for brand in 0..3 {
            for partial in [false, true] {
                let palette = Palette::new(theme, brand).expect("registered Gallery brand");
                let frame = Rect::new(10., 20., 200., 12.);
                let clip = partial.then_some(Rect::new(30., 20., 100., 12.));
                for variant in 0..9 {
                    let mut scene = Scene::new();
                    let (component, state) = match variant {
                        0..=2 => ("tui.progress", "rest"),
                        3 => ("tui.progress", "indeterminate"),
                        4 => ("tui.status-badge", "rest"),
                        5 => ("tui.status-badge", "loading"),
                        6 => ("tui.status-badge", "success"),
                        7 => ("tui.status-badge", "error"),
                        _ => ("tui.skeleton", "rest"),
                    };
                    let (_, reads) = tekes_ui_tokens::audit::capture(|| {
                        match variant {
                            0..=3 => {
                                let mut model = Progress::new([0., 0.45, 1., 0.45][variant]);
                                model.indeterminate = variant == 3;
                                ProgressView::default()
                                    .render(&mut scene, model, frame, clip, palette);
                                let expected = if model.indeterminate {
                                    200. * 0.3
                                } else {
                                    200. * model.value()
                                };
                                assert!(
                                    (quads(&scene)[1].rect.size.width - expected).abs() < 0.0001
                                );
                            }
                            4..=7 => {
                                StatusBadgeView::default().render(
                                    &mut scene,
                                    [
                                        StatusBadge::Info,
                                        StatusBadge::Running,
                                        StatusBadge::Succeeded,
                                        StatusBadge::Failed,
                                    ][variant - 4],
                                    frame,
                                    clip,
                                    palette,
                                );
                                // Labels are caller-owned; record their existing public ink API separately within this badge scope.
                                let _ = StatusBadgeView::label_color(palette);
                            }
                            _ => {
                                SkeletonView::default().render(&mut scene, frame, clip, palette);
                            }
                        }
                    });
                    assert!(!reads.is_empty());
                    assert!(scene.primitives().iter().any(
                        |p| matches!(p, Primitive::PushClip(r) if *r == clip.unwrap_or(frame))
                    ));
                    cases.push(serde_json::json!({"component":component,"content_state":state,"state_asserted":true,"variant":variant,"theme":format!("{theme:?}"),"brand":brand,"partial":partial,"lookups":reads}));
                }
            }
        }
    }
    assert_eq!(cases.len(), 216);
    if let Some(path) = std::env::var_os("TEKESUI_PASSIVE_FEEDBACK_AUDIT_OUT") {
        std::fs::write(path,serde_json::to_vec_pretty(&serde_json::json!({"format":"tekesui-runtime-token-observation/1","complete":false,"scope":"Public passive views: Progress bounded determinate values/indeterminate geometry, four StatusBadge outcomes and caller label ink, Skeleton placeholder;4themes/3brands/ancestor clipping. No host loading/error/success composition inferred for Progress.","excluded":["animation requirements","host-owned loading/error/success composition","native and matched ClientKit/Gold7 experience","complete token dependency closure"],"cases":cases})).unwrap()).unwrap();
    }
}
