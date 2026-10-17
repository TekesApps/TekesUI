use std::time::Duration;
use tekes_ui_motion::typing::{TypingConfig, TypingEngine, TypingMode};
fn at(t: f64) -> Duration {
    Duration::from_secs_f64(t)
}
#[test]
fn elapsed_time_releases_multiple_units_without_frame_rate_dependency() {
    let config = TypingConfig {
        units_per_second: 10.0,
        ..TypingConfig::default()
    };
    let mut coarse = TypingEngine::new(config).unwrap();
    coarse.append(20, at(0.0)).unwrap();
    let mut fine = coarse.clone();
    coarse.advance(at(0.35)).unwrap();
    for i in 1..=35 {
        fine.advance(at(f64::from(i) * 0.01)).unwrap();
    }
    assert_eq!(coarse.visible(), 4);
    assert_eq!(coarse.visible(), fine.visible());
    for i in 0..4 {
        assert!((coarse.opacity(i) - fine.opacity(i)).abs() < 1e-10);
    }
}
#[test]
fn append_one_or_a_batch_in_arrival_mode_and_do_not_bank_idle_credit() {
    let mut p = TypingEngine::new(TypingConfig {
        mode: TypingMode::Arrivals,
        ..TypingConfig::default()
    })
    .unwrap();
    p.append(1, at(5.0)).unwrap();
    assert_eq!(p.visible(), 1);
    p.append(8, at(5.1)).unwrap();
    assert_eq!(p.visible(), 9);
    let mut p = TypingEngine::new(TypingConfig {
        units_per_second: 10.0,
        ..TypingConfig::default()
    })
    .unwrap();
    p.append(1, at(0.0)).unwrap();
    p.advance(at(100.0)).unwrap();
    p.append(8, at(100.0)).unwrap();
    assert_eq!(p.visible(), 2);
    p.advance(at(100.21)).unwrap();
    assert_eq!(p.visible(), 4);
}
#[test]
fn chunk_boundaries_do_not_duplicate_release_slots() {
    let config = TypingConfig {
        units_per_second: 10.0,
        ..TypingConfig::default()
    };
    let mut a = TypingEngine::new(config).unwrap();
    a.append(6, at(0.0)).unwrap();
    let mut b = TypingEngine::new(config).unwrap();
    for _ in 0..3 {
        b.append(2, at(0.0)).unwrap();
    }
    a.advance(at(0.21)).unwrap();
    b.advance(at(0.21)).unwrap();
    assert_eq!(a.visible(), b.visible());
    assert_eq!(a.visible(), 3);
}
#[test]
fn pause_freezes_reveal_and_fade_then_resumes_and_settles() {
    let mut p = TypingEngine::new(TypingConfig::default()).unwrap();
    p.append(20, at(0.0)).unwrap();
    p.pause(at(0.1)).unwrap();
    let count = p.visible();
    let opacity = p.opacity(count - 1);
    p.advance(at(20.0)).unwrap();
    assert_eq!(p.visible(), count);
    assert_eq!(p.opacity(count - 1), opacity);
    assert!(!p.needs_frame());
    p.resume(at(20.0)).unwrap();
    p.advance(at(25.0)).unwrap();
    assert_eq!(p.visible(), 20);
    assert_eq!(p.opacity(19), 1.0);
    assert!(!p.needs_frame());
}
#[test]
fn bounded_latency_catches_bursts_and_reduced_motion_stops_frames() {
    let mut p = TypingEngine::new(TypingConfig {
        max_latency: Some(at(0.25)),
        ..TypingConfig::default()
    })
    .unwrap();
    p.append(1000, at(0.0)).unwrap();
    p.advance(at(0.26)).unwrap();
    assert_eq!(p.visible(), 1000);
    p.set_reduced_motion(true).unwrap();
    assert_eq!(p.opacity(999), 1.0);
    assert!(!p.needs_frame());
    p.append(10, at(0.3)).unwrap();
    assert_eq!(p.visible(), 1010);
    assert!(!p.needs_frame());
}
#[test]
fn invalid_input_is_rejected_without_dropping_buffered_units() {
    let mut p = TypingEngine::new(TypingConfig::default()).unwrap();
    p.append(4, at(1.0)).unwrap();
    assert!(p.append(70000, at(2.0)).is_err());
    assert_eq!(p.available(), 4);
    assert!(p.advance(at(0.0)).is_err());
    assert!(
        p.set_config(TypingConfig {
            units_per_second: f64::NAN,
            ..TypingConfig::default()
        })
        .is_err()
    );
}
