//! Independent pixel expectations from the fixture's RG/BY source and public
//! geometry. Never calls the private Viewer geometry implementation.
use super::fixture;
use tekes_ui_controls::composition::*;
use tekes_ui_scene::{Point, Primitive, Rect, Scene};

pub fn inspect(
    host: &ContentHost,
    scene: &Scene,
    pixels: &[u8],
    frame: Rect,
    backing: f32,
    state: &str,
) -> (String, Vec<String>) {
    let mut report = String::new();
    let mut failures = Vec::new();
    let mut check = |ok: bool, message: String| {
        report.push_str(&format!("{} {message}\n", if ok { "PASS" } else { "FAIL" }));
        if !ok {
            failures.push(message);
        }
    };
    let width = (frame.size.width * backing) as usize;
    let sample = |p: Point| -> [u8; 4] {
        let x = ((p.x - frame.min_x()) * backing).floor() as usize;
        let y = ((p.y - frame.min_y()) * backing).floor() as usize;
        pixels[(y * width + x) * 4..(y * width + x + 1) * 4]
            .try_into()
            .unwrap()
    };
    let image = host.get(fixture::IMAGE).unwrap().frame();
    let viewport = host.get(fixture::VIEW).unwrap().frame();
    let quads: Vec<_> = scene
        .primitives()
        .iter()
        .filter_map(|p| match p {
            Primitive::ControlledTransformedImage { quad, .. } => Some(quad),
            _ => None,
        })
        .collect();
    check(
        quads.len() == usize::from(state != "empty"),
        format!("image quads={} state={state}", quads.len()),
    );
    let panel = host
        .get(fixture::ROOT)
        .unwrap()
        .component::<PanelContent>()
        .unwrap()
        .geometry()
        .unwrap();
    let shape = panel.shape;
    check(
        scene.primitives().iter().any(|p| match p {
            Primitive::PushShapeClip(key) => scene.clip_shape(key).ok() == Some(shape),
            _ => false,
        }),
        format!("original parent shape={shape:?} is carried into Scene"),
    );
    if let Some(quad) = quads.first() {
        check(
            quad.quad.rect == image,
            format!(
                "quad rect={:?}; original image frame={image:?}",
                quad.quad.rect
            ),
        );
        check(
            quad.uv == Rect::new(0., 0., 1., 1.),
            format!("Contain retains complete UV={:?}", quad.uv),
        );
    }
    let alt_count = host
        .semantics()
        .iter()
        .filter(|s| s.semantics.role == ContentRole::Image)
        .count();
    check(alt_count == 1, format!("image semantic count={alt_count}"));
    if state == "fit" {
        let expected_scale = (viewport.size.width / 200.).min(viewport.size.height / 100.);
        let expected = Rect::new(
            viewport.min_x() + (viewport.size.width - expected_scale * 200.) / 2.,
            viewport.min_y() + (viewport.size.height - expected_scale * 100.) / 2.,
            expected_scale * 200.,
            expected_scale * 100.,
        );
        check(
            (image.min_x() - expected.min_x()).abs() < 0.01
                && (image.min_y() - expected.min_y()).abs() < 0.01
                && (image.size.width - expected.size.width).abs() < 0.01
                && (image.size.height - expected.size.height).abs() < 0.01,
            format!("independent Fit expected={expected:?}; actual={image:?}"),
        );
        for (name, u, v, expected) in [
            ("red", 0.125, 0.125, [255, 0, 0, 255]),
            ("green", 0.875, 0.125, [0, 255, 0, 255]),
            ("blue", 0.125, 0.875, [0, 0, 255, 255]),
            ("yellow", 0.875, 0.875, [255, 255, 0, 255]),
        ] {
            let point = Point::new(
                image.min_x() + image.size.width * u,
                image.min_y() + image.size.height * v,
            );
            let actual = sample(point);
            check(
                actual
                    .iter()
                    .zip(expected)
                    .all(|(a, b)| (i32::from(*a) - b).abs() <= 8),
                format!("{name} marker at {point:?}: expected={expected:?} actual={actual:?}"),
            );
        }
        let center = image.center();
        let margin = if image.min_y() - viewport.min_y() > 2. {
            Point::new(center.x, viewport.min_y() + 1.)
        } else {
            Point::new(viewport.min_x() + 1., center.y)
        };
        check(
            !image.contains(margin),
            format!("transparent comparison uses real letterbox point={margin:?}"),
        );
        let actual = sample(center);
        let background = sample(margin);
        check(
            actual
                .iter()
                .zip(background)
                .all(|(a, b)| (i32::from(*a) - i32::from(b)).abs() <= 3),
            format!(
                "transparent center={center:?}/{actual:?}; parent background={margin:?}/{background:?}"
            ),
        );
    }
    if state == "edge" {
        // Find an actual pixel center under the image's bottom corner that the
        // original rounded parent excludes. No guessed fixed coordinate.
        let mut probe = None;
        let y0 = ((panel.outer.max_y() - shape.radii().bottom_right - 2.) * backing)
            .floor()
            .max(0.) as usize;
        let x0 = ((panel.outer.max_x() - shape.radii().bottom_right - 2.) * backing)
            .floor()
            .max(0.) as usize;
        'rows: for y in y0..(frame.size.height * backing) as usize {
            for x in x0..width {
                let point = Point::new((x as f32 + 0.5) / backing, (y as f32 + 0.5) / backing);
                if viewport.contains(point)
                    && image.contains(point)
                    && panel.outer.contains(point)
                    && shape.contains([f64::from(point.x), f64::from(point.y)]) == Ok(false)
                {
                    probe = Some(point);
                    break 'rows;
                }
            }
        }
        check(
            probe.is_some(),
            format!("edge input exposes image under rounded parent; probe={probe:?}"),
        );
        if let Some(point) = probe {
            let actual = sample(point);
            // Edge pan selects bottom-right opaque yellow. Excluded corner
            // must not contain that source contribution, in every theme.
            check(
                !(actual[0] > 240 && actual[1] > 240 && actual[2] < 15),
                format!("rounded parent excludes yellow image at {point:?}; pixel={actual:?}"),
            );
        }
    }
    (report, failures)
}
