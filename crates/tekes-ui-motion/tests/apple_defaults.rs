use std::time::Duration;
use tekes_ui_motion::{
    Animated, MotionSpec, TimingCurve,
    apple_defaults::{AppleDefault, Parameters},
};

#[test]
fn defaults_match_published_configuration_and_remain_distinct() {
    assert_eq!(AppleDefault::ALL.len(), 9);
    assert_eq!(
        AppleDefault::Spring.parameters(),
        Parameters::Spring {
            duration: 0.5,
            bounce: 0.0
        }
    );
    assert_eq!(
        AppleDefault::Snappy.parameters(),
        Parameters::Spring {
            duration: 0.5,
            bounce: 0.15
        }
    );
    assert_eq!(
        AppleDefault::Bouncy.parameters(),
        Parameters::Spring {
            duration: 0.5,
            bounce: 0.3
        }
    );
    assert_ne!(
        AppleDefault::Default.parameters(),
        AppleDefault::Spring.parameters()
    );
    assert_eq!(
        AppleDefault::Spring.motion().spec,
        AppleDefault::Smooth.motion().spec
    );
    for (i, p) in AppleDefault::ALL.into_iter().enumerate() {
        assert!(p.source_url().starts_with("https://developer.apple.com/"));
        for other in &AppleDefault::ALL[i + 1..] {
            assert_ne!(p.id(), other.id());
        }
        let mut channel = Animated::new(0.0).unwrap();
        channel.animate_to(1.0, p.motion(), Duration::ZERO).unwrap();
        let frame = channel.advance(Duration::from_secs(5)).unwrap();
        assert_eq!(frame.sample.value, 1.0);
        assert!(!frame.needs_frame);
    }
}

#[test]
fn trajectories_match_public_swiftui_spring_and_unitcurve_reference() {
    let csv = include_str!("reference/apple-oracle.csv");
    let mut checked = 0;
    for line in csv
        .lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("kind,") && !l.is_empty())
    {
        let row: Vec<_> = line.split(',').collect();
        let preset = AppleDefault::ALL
            .into_iter()
            .find(|p| p.id() == row[1])
            .expect("reference preset");
        let time: f64 = row[2].parse().unwrap();
        let expected: f64 = row[3].parse().unwrap();
        match row[0] {
            "spring" => {
                let sample = preset
                    .motion()
                    .spec
                    .sample(0.0, 1.0, 0.0, Duration::from_secs_f64(time))
                    .unwrap();
                let velocity: f64 = row[4].parse().unwrap();
                assert!(
                    (sample.value - expected).abs() < 1e-11,
                    "{} {time} position",
                    preset.id()
                );
                assert!(
                    (sample.velocity - velocity).abs() < 1e-9,
                    "{} {time} velocity",
                    preset.id()
                );
            }
            "curve" => {
                let Parameters::Tween {
                    control_points: [x1, y1, x2, y2],
                    ..
                } = preset.parameters()
                else {
                    panic!("curve preset")
                };
                let actual = TimingCurve::cubic_bezier(x1, y1, x2, y2)
                    .unwrap()
                    .evaluate(time)
                    .unwrap()
                    .0;
                assert!(
                    // UnitCurve uses an approximate inverse; allow its public output precision.
                    (actual - expected).abs() < 2e-6,
                    "{} {time}: {actual} vs {expected}",
                    preset.id()
                );
            }
            _ => panic!("unknown reference kind"),
        }
        checked += 1;
    }
    assert_eq!(checked, 63);
}

#[test]
fn easing_defaults_all_have_350ms_duration() {
    for preset in [
        AppleDefault::EaseIn,
        AppleDefault::EaseOut,
        AppleDefault::EaseInOut,
        AppleDefault::Linear,
    ] {
        let MotionSpec::Tween { duration, .. } = preset.motion().spec else {
            panic!("tween")
        };
        assert_eq!(duration, Duration::from_millis(350));
    }
}
