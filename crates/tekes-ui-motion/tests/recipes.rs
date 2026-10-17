use std::time::Duration;
use tekes_ui_motion::recipes::{Phase, RecipeKind, RecipePlayer};
fn at(seconds: f64) -> Duration {
    Duration::from_secs_f64(seconds)
}

#[test]
fn recipes_reverse_continuously_and_drop_stale_cues() {
    for kind in RecipeKind::ALL {
        let mut player = RecipePlayer::new(kind);
        player.set_target(true).unwrap();
        player.advance(at(0.07)).unwrap();
        let values = player.values();
        let speeds = player.velocities();
        player.set_target(false).unwrap();
        assert_eq!(player.values(), values, "{kind:?}");
        // Opacity and progress tweens deliberately have their own tangents.
        if !matches!(kind, RecipeKind::Success | RecipeKind::TextScan) {
            assert_eq!(player.velocities()[0], speeds[0]);
        }
        player.advance(at(8.0)).unwrap();
        assert_eq!(
            player.values(),
            [0.0; 4],
            "stale delayed track fired for {kind:?}"
        );
        assert!(!player.needs_frame());
        assert_ne!(player.phase, Phase::Success);
    }
}
#[test]
fn success_large_time_steps_process_phases_in_order_and_pause_is_durable() {
    let mut player = RecipePlayer::new(RecipeKind::Success);
    player.replay().unwrap();
    player.advance(at(0.05)).unwrap();
    assert_eq!(player.phase, Phase::Preparing);
    player.advance(at(0.2)).unwrap();
    assert_eq!(player.phase, Phase::Working);
    player.interrupt().unwrap();
    let values = player.values();
    let mut parameters = player.parameters;
    parameters.duration = 0.9;
    player.set_parameters(parameters).unwrap();
    player.advance(at(20.0)).unwrap();
    assert_eq!(player.values(), values);
    assert!(!player.needs_frame());
    assert_eq!(player.phase, Phase::Cancelled);
    player.set_target(true).unwrap();
    player.advance(at(30.0)).unwrap();
    assert_eq!(player.phase, Phase::Success);
    assert_eq!(player.values(), [1.0; 4]);
    assert!(!player.needs_frame());
}
#[test]
fn cue_sampling_does_not_depend_on_frame_partition() {
    for kind in RecipeKind::ALL {
        let mut coarse = RecipePlayer::new(kind);
        coarse.replay().unwrap();
        let mut fine = coarse.clone();
        coarse.advance(at(0.74)).unwrap();
        for step in 1..=74 {
            fine.advance(at(f64::from(step) * 0.01)).unwrap();
        }
        for (a, b) in coarse.values().into_iter().zip(fine.values()) {
            assert!((a - b).abs() < 1e-12, "{kind:?}");
        }
        assert_eq!(coarse.phase, fine.phase);
    }
}
#[test]
fn reading_anchor_is_stable_during_reversal_height_edits_and_user_scrolling() {
    let mut player = RecipePlayer::new(RecipeKind::Reading);
    player.replay().unwrap();
    for tick in 1..=200 {
        player.advance(at(f64::from(tick) / 120.0)).unwrap();
        if tick % 17 == 0 {
            player.set_target(!player.target()).unwrap();
        }
        if tick == 40 {
            let mut params = player.parameters;
            params.amount = 140.0;
            player.set_parameters(params).unwrap();
        }
        if tick == 80 {
            player.scroll_by(36.0);
        }
        assert!((player.anchor_screen_y(72.0) - (72.0 - player.user_scroll)).abs() < 1e-10);
    }
    assert_eq!(player.user_scroll, 108.0);
}
#[test]
fn parameter_edits_preserve_spring_trajectory_origin_and_validate_atomically() {
    for kind in RecipeKind::ALL {
        let mut player = RecipePlayer::new(kind);
        player.replay().unwrap();
        player.advance(at(0.2)).unwrap();
        let values = player.values();
        let speed = player.velocities()[0];
        let mut params = player.parameters;
        params.duration = 0.8;
        params.damping = 1.1;
        player.set_parameters(params).unwrap();
        assert_eq!(player.values(), values);
        if !matches!(kind, RecipeKind::Success | RecipeKind::TextScan) {
            assert_eq!(player.velocities()[0], speed);
        }
        params.amount = f64::NAN;
        assert!(player.set_parameters(params).is_err());
        assert_eq!(player.values(), values);
    }
}
#[test]
fn reduce_motion_finishes_all_tracks_and_pending_phases_without_extra_frames() {
    for kind in RecipeKind::ALL {
        let mut player = RecipePlayer::new(kind);
        player.replay().unwrap();
        player.advance(at(0.05)).unwrap();
        player.set_reduced_motion(true).unwrap();
        assert!(!player.needs_frame(), "{kind:?}");
        if kind == RecipeKind::Success {
            assert_eq!(player.phase, Phase::Success);
        }
        player.set_target(false).unwrap();
        assert_eq!(player.values(), [0.0; 4]);
        player.set_target(true).unwrap();
        assert!(!player.needs_frame());
        player.set_reduced_motion(false).unwrap();
        assert!(!player.needs_frame());
    }
}

#[test]
fn list_identity_positions_are_continuous_and_finish_in_both_orders() {
    let mut p = RecipePlayer::new(RecipeKind::ListReorder);
    assert_eq!(p.item_values(), [0.0, 1.0, 2.0, 3.0, 4.0, 5.0]);
    p.set_target(true).unwrap();
    p.advance(at(0.15)).unwrap();
    let presented = p.item_values();
    p.set_target(false).unwrap();
    assert_eq!(p.item_values(), presented);
    p.advance(at(10.0)).unwrap();
    assert_eq!(p.item_values(), [0.0, 1.0, 2.0, 3.0, 4.0, 5.0]);
    p.set_target(true).unwrap();
    p.advance(at(20.0)).unwrap();
    assert_eq!(p.item_values(), [3.0, 2.0, 0.0, 1.0, 4.0, 0.0]);
    assert!(!p.needs_frame());
}
#[test]
fn stagger_pause_cancels_future_items_and_resume_finishes_latest_direction() {
    let mut p = RecipePlayer::new(RecipeKind::Stagger);
    p.set_target(true).unwrap();
    p.advance(at(0.15)).unwrap();
    assert_eq!(p.item_values()[5], 0.0);
    p.interrupt().unwrap();
    let values = p.item_values();
    p.advance(at(3.0)).unwrap();
    assert_eq!(p.item_values(), values);
    p.resume().unwrap();
    p.advance(at(8.0)).unwrap();
    assert_eq!(p.item_values(), [1.0; 6]);
    p.set_target(false).unwrap();
    p.advance(at(15.0)).unwrap();
    assert_eq!(p.item_values(), [0.0; 6]);
    assert!(!p.needs_frame());
}
#[test]
fn drag_release_uses_velocity_and_can_be_grabbed_during_inertia() {
    let mut p = RecipePlayer::new(RecipeKind::DragSnap);
    p.drag_to(0.3).unwrap();
    p.release_drag(2.0).unwrap();
    assert_eq!(p.snap_target(), 0.75);
    p.advance(at(0.08)).unwrap();
    assert!(p.values()[0] > 0.3);
    p.drag_to(0.4).unwrap();
    assert_eq!(p.values()[0], 0.4);
    assert!(!p.needs_frame());
    p.release_drag(-2.0).unwrap();
    assert_eq!(p.snap_target(), 0.0);
    p.advance(at(5.0)).unwrap();
    assert_eq!(p.values()[0], 0.0);
    assert!(!p.needs_frame());
    assert!(p.release_drag(f64::NAN).is_err());
}
#[test]
fn numeric_new_value_supersedes_previous_target_without_position_jump() {
    let mut p = RecipePlayer::new(RecipeKind::Numeric);
    p.replay().unwrap();
    p.advance(at(0.2)).unwrap();
    let shown = p.item_values()[0];
    let mut params = p.parameters;
    params.amount = 140.0;
    p.set_parameters(params).unwrap();
    assert_eq!(p.item_values()[0], shown);
    p.interrupt().unwrap();
    p.advance(at(3.0)).unwrap();
    assert_eq!(p.item_values()[0], shown);
    p.resume().unwrap();
    p.advance(at(6.0)).unwrap();
    assert_eq!(p.item_values()[0], 1528.0);
    p.set_target(false).unwrap();
    p.advance(at(8.0)).unwrap();
    assert_eq!(p.item_values()[0], 128.0);
}

#[test]
fn text_scan_band_projects_diagonally_and_starts_and_ends_outside_text() {
    use tekes_ui_motion::recipes::TextScanBand;
    let band = TextScanBand {
        angle_degrees: 45.0,
        width: 40.0,
    };
    for y in [0.0, 25.0, 75.0, 100.0] {
        assert!(band.span([300.0, 100.0], y, 0.0).unwrap().1 <= 1e-10);
        assert!(band.span([300.0, 100.0], y, 1.0).unwrap().0 >= 300.0 - 1e-10);
    }
    let top = band.span([300.0, 100.0], 0.0, 0.5).unwrap();
    let bottom = band.span([300.0, 100.0], 100.0, 0.5).unwrap();
    assert!((top.0 - bottom.0 - 100.0).abs() < 1e-9);
    assert!(band.span([300.0, 100.0], 10.0, f64::NAN).is_none());
}
#[test]
fn text_scan_pause_resume_and_reverse_keep_presented_position() {
    let mut p = RecipePlayer::new(RecipeKind::TextScan);
    p.replay().unwrap();
    p.advance(at(0.5)).unwrap();
    let value = p.values()[0];
    assert!(value > 0.0 && value < 1.0);
    p.interrupt().unwrap();
    p.advance(at(2.0)).unwrap();
    assert_eq!(p.values()[0], value);
    p.resume().unwrap();
    assert_eq!(p.values()[0], value);
    p.advance(at(2.4)).unwrap();
    let value = p.values()[0];
    p.set_target(false).unwrap();
    assert_eq!(p.values()[0], value);
    p.advance(at(8.0)).unwrap();
    assert_eq!(p.values()[0], 0.0);
    assert!(!p.needs_frame());
}

#[test]
fn rotation_brake_accelerates_then_stops_and_preserves_velocity_on_retarget() {
    let mut p = RecipePlayer::new(RecipeKind::SpinBrake);
    let mut params = p.parameters;
    params.duration = 2.0;
    params.delay = 0.0;
    p.set_parameters(params).unwrap();
    p.replay().unwrap();
    p.advance(at(0.2)).unwrap();
    let slow = p.velocities()[0];
    p.advance(at(1.0)).unwrap();
    assert!(p.velocities()[0] > slow * 4.0);
    let position = p.values()[0];
    let velocity = p.velocities()[0];
    p.set_target(false).unwrap();
    assert_eq!(p.values()[0], position);
    assert_eq!(p.velocities()[0], velocity);
    p.advance(at(1.01)).unwrap();
    assert!(
        p.values()[0] > position,
        "braking preserves forward momentum before reversing"
    );
    p.advance(at(3.0)).unwrap();
    assert_eq!(p.values()[0], 0.0);
    assert_eq!(p.velocities()[0], 0.0);
    assert!(!p.needs_frame());
    p.set_target(true).unwrap();
    p.advance(at(5.0)).unwrap();
    assert_eq!(p.values()[0], 1.0);
    assert_eq!(p.velocities()[0], 0.0);
}
#[test]
fn changing_rotation_target_angle_preserves_presented_angle_and_angular_velocity() {
    for kind in RecipeKind::ALL.into_iter().filter(|k| k.is_rotation()) {
        let mut p = RecipePlayer::new(kind);
        p.replay().unwrap();
        p.advance(at(0.35)).unwrap();
        let angle = p.values()[0] * p.parameters.rotation_degrees;
        let speed = p.velocities()[0] * p.parameters.rotation_degrees;
        let mut params = p.parameters;
        params.rotation_degrees = 270.0;
        p.set_parameters(params).unwrap();
        assert!((p.values()[0] * 270.0 - angle).abs() < 1e-10, "{kind:?}");
        assert!((p.velocities()[0] * 270.0 - speed).abs() < 1e-9, "{kind:?}");
        p.interrupt().unwrap();
        let value = p.values()[0];
        p.advance(at(10.0)).unwrap();
        assert_eq!(p.values()[0], value);
        p.resume().unwrap();
        p.advance(at(20.0)).unwrap();
        assert_eq!(p.values()[0], 1.0);
        assert!(!p.needs_frame());
    }
}
#[test]
fn rotation_projection_distinguishes_axes_and_remains_finite_through_full_turns() {
    use tekes_ui_motion::recipes::{RotationAxis, project_rotation};
    let z = project_rotation([20.0, 10.0], 90.0, RotationAxis::Z, 420.0).unwrap();
    assert!((z[0] + 10.0).abs() < 1e-10);
    assert!((z[1] - 20.0).abs() < 1e-10);
    let x = project_rotation([20.0, 10.0], 90.0, RotationAxis::X, 420.0).unwrap();
    assert!(x[1].abs() < 1e-10);
    assert!(x[0] > 20.0);
    let y = project_rotation([20.0, 10.0], 90.0, RotationAxis::Y, 420.0).unwrap();
    assert!(y[0].abs() < 1e-10);
    assert!(y[1] < 10.0);
    for axis in [RotationAxis::X, RotationAxis::Y, RotationAxis::Z] {
        for degree in 0..2160 {
            assert!(
                project_rotation([52.0, 36.0], f64::from(degree), axis, 420.0)
                    .unwrap()
                    .iter()
                    .all(|v| v.is_finite())
            );
        }
    }
    assert!(project_rotation([20.0, 10.0], f64::NAN, RotationAxis::X, 420.0).is_none());
}

#[test]
fn streaming_typing_batches_are_frame_independent_and_target_switch_cancels_old_queue() {
    for kind in [RecipeKind::Typing, RecipeKind::StreamTyping] {
        let mut a = RecipePlayer::new(kind);
        a.typing_unit_count = 40;
        a.replay().unwrap();
        let mut b = a.clone();
        a.advance(at(0.75)).unwrap();
        for i in 1..=75 {
            b.advance(at(f64::from(i) * 0.01)).unwrap();
        }
        assert_eq!(a.typing.available(), b.typing.available());
        assert_eq!(a.typing.visible(), b.typing.visible());
        for i in 0..a.typing.visible() {
            assert!((a.typing.opacity(i) - b.typing.opacity(i)).abs() < 1e-9);
        }
        a.interrupt().unwrap();
        let visible = a.typing.visible();
        a.advance(at(4.0)).unwrap();
        assert_eq!(a.typing.visible(), visible);
        assert!(!a.needs_frame());
        a.resume().unwrap();
        a.advance(at(8.0)).unwrap();
        assert_eq!(a.typing.visible(), 40);
        assert!(!a.needs_frame());
        a.typing_unit_count = 3;
        a.set_target(false).unwrap();
        a.advance(at(12.0)).unwrap();
        assert_eq!(a.typing.available(), 3);
        assert_eq!(a.typing.visible(), 3);
        assert!(!a.needs_frame());
    }
}
