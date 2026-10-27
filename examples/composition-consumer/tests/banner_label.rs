//! #12: actual CoreText regression for the original 320pt/type150 Banner word split.
#![cfg(target_os = "macos")]
#[path = "support/coretext.rs"]
mod coretext;
use tekes_ui_controls::{
    ButtonModel, ButtonSize, TextSurfaceHost,
    composition::*,
    extended::{Banner, NoticeTone},
    feedback_view::BannerView,
};
use tekes_ui_scene::{Point, Primitive, Rect, Scene};
use tekes_ui_tokens::{Theme, proposal};
const ROOT: ComponentId = ComponentId(12_790);
const ACTION: ComponentId = ComponentId(12_791);
#[test]
fn banner_real_coretext_narrow_large_label_stays_whole_and_dismisses_once() {
    let role = proposal::TextRole::UiBody;
    // Include the recorded 320pt/type150 defect and existing stacked/wide/default paths.
    for (width, scale) in [(320., 1.5), (320., 1.), (220., 1.5), (520., 1.5)] {
        let mut text = coretext::Text::new(scale);
        text.services = true;
        text.reduced_motion = true;
        let model = Banner {
            message: "Your workspace has unsaved changes.".into(),
            tone: NoticeTone::Info,
            dismissible: true,
            visible: true,
        };
        let standard = ButtonModel::measure(&mut text, "Dismiss", f32::MAX, ButtonSize::Body, role);
        let unwrapped = text.shape_surface("Dismiss", role, None);
        assert_eq!(unwrapped.lines.len(), 1);
        let measured = BannerView::measure(&mut text, &model.message, width, true, role);
        let frame = Rect::new(12.25, 20.5, measured.width, measured.height);
        let clip = Rect::new(0., 0., 720., 720.);
        let mut host = ContentHost::new(
            Content::new(
                ROOT,
                BannerContent::new("Banner", model.clone(), ACTION).unwrap(),
            )
            .with_surface("panel")
            .unwrap(),
        )
        .unwrap();
        host.set_action_events(true);
        text.atlas.begin_frame();
        let mut scene = Scene::new();
        host.render(
            &mut scene,
            &mut text,
            frame,
            clip,
            Theme::LIGHT_INCREASED.into(),
        )
        .unwrap();
        let banner = host
            .get(ROOT)
            .unwrap()
            .component::<BannerContent>()
            .unwrap();
        assert_eq!(
            *banner.model(),
            model,
            "layout never truncates or rewrites the original message/model"
        );
        assert_eq!(banner.layout().frame, frame);
        let action = host
            .semantics()
            .into_iter()
            .find(|n| n.id == ACTION)
            .expect("original Banner Dismiss proxy");
        assert_eq!(action.visible, Some(action.frame));
        assert!(
            action.frame.size.width >= standard.width,
            "width={width} scale={scale}: original action {} must fit standard measured width {}",
            action.frame.size.width,
            standard.width
        );
        assert!(action.frame.size.height >= standard.height);
        assert!(action.frame.min_x() >= frame.min_x() && action.frame.max_x() <= frame.max_x());
        assert!(action.frame.min_y() >= frame.min_y() && action.frame.max_y() <= frame.max_y());
        let action_index = scene
            .primitives()
            .iter()
            .position(|p| matches!(p,Primitive::Quad(q) if q.rect==action.frame))
            .expect("original Primary action background");
        let label = scene.primitives()[action_index + 1..]
            .iter()
            .find_map(|p| match p {
                Primitive::Glyphs(run) if !run.glyphs.is_empty() => Some(run),
                _ => None,
            })
            .expect("actual CoreText Dismiss glyphs");
        assert_eq!(
            label.glyphs.len(),
            unwrapped.glyph_count(),
            "all original label glyphs remain visible"
        );
        let first_y = label.glyphs[0].origin.y;
        assert!(
            label
                .glyphs
                .iter()
                .all(|g| (g.origin.y - first_y).abs() <= 0.01),
            "width={width} scale={scale}: Dismiss must occupy one actual baseline, not Dismi/ss"
        );
        assert!(label.glyphs.iter().all(|g| action.frame.contains(g.origin)));
        let message: Vec<_> = scene.primitives()[..action_index]
            .iter()
            .filter_map(|p| match p {
                Primitive::Glyphs(run) if !run.glyphs.is_empty() => Some(run),
                _ => None,
            })
            .collect();
        assert!(
            !message.is_empty(),
            "original message still paints with actual CoreText"
        );
        assert!(
            message
                .iter()
                .flat_map(|run| &run.glyphs)
                .all(|g| frame.contains(g.origin) && !action.frame.contains(g.origin))
        );
        let center = action.frame.center();
        host.event(&ContentEvent::PointerDown(center));
        host.event(&ContentEvent::PointerUp(Point::new(
            frame.max_x() + 20.,
            frame.max_y() + 20.,
        )));
        assert!(
            host.get(ROOT)
                .unwrap()
                .component::<BannerContent>()
                .unwrap()
                .model()
                .visible,
            "outside release cancels"
        );
        assert!(host.drain_actions().is_empty());
        host.event(&ContentEvent::PointerDown(center));
        host.event(&ContentEvent::PointerUp(center));
        host.render(
            &mut Scene::new(),
            &mut text,
            frame,
            clip,
            Theme::LIGHT_INCREASED.into(),
        )
        .unwrap();
        assert!(
            !host
                .get(ROOT)
                .unwrap()
                .component::<BannerContent>()
                .unwrap()
                .model()
                .visible
        );
        let actions = host.drain_actions();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].target, ACTION);
        host.event(&ContentEvent::PointerUp(center));
        assert!(host.drain_actions().is_empty());
    }
}
