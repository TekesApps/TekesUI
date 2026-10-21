//! Bounded independent consumers inside the existing component States routes.
use super::{PageCtx, StoryState, label};
use crate::gallery::pages::icons::GalleryIconRenderer;
use tekes_ui_composition_consumer::content_status::{HEADER, Kind, Scenario, Status};
use tekes_ui_controls::composition::CompositionError;
use tekes_ui_scene::{Rect, Scene};
use tekes_ui_tokens::Palette;

pub(crate) const CASES: [(Status, bool, &str); 5] = [
    (
        Status::Loading,
        false,
        "Loading content · independent consumer",
    ),
    (Status::Error, false, "Content error · independent consumer"),
    (Status::Empty, false, "Empty content · independent consumer"),
    (Status::Loading, true, "Loading content · header focus"),
    (Status::Error, true, "Content error · header focus"),
];
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_case(
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    kind: Kind,
    status: Status,
    focus: bool,
    frame: Rect,
    clip: Rect,
    palette: Palette,
) -> Result<Scenario, CompositionError> {
    let mut scenario = Scenario::new(kind)?;
    if kind == Kind::Disclosure {
        scenario.set_open(true);
    }
    scenario.set_status(status);
    scenario
        .host
        .render(scene, ctx.text, frame, clip, palette)?;
    if focus {
        scenario.host.focus(Some(HEADER), true);
        // Paint just once with the final focus. Avoid two copies of the body.
        // The caller gives a dedicated scene per case; prepare its layout first.
    }
    Ok(scenario)
}
pub(crate) fn matrix(
    kind: Kind,
    s: &mut StoryState,
    ctx: &mut PageCtx<'_>,
    scene: &mut Scene,
    clip: Rect,
    mut y: f32,
) -> f32 {
    let palette = Palette {
        theme: ctx.theme,
        brand: s.brand,
    };
    scene.push_clip(clip);
    for (status, focus, title) in CASES {
        let h = ctx
            .text
            .proposal_label(
                title,
                tekes_ui_tokens::proposal::TextRole::UiBody,
                Some(clip.size.width),
            )
            .height as f32;
        label(
            ctx,
            scene,
            Rect::new(clip.min_x(), y, clip.size.width, h),
            title,
            false,
        );
        y += h + 12.;
        let frame = Rect::new(clip.min_x() + 6., y, (clip.size.width - 12.).max(0.), 320.);
        let mut probe = Scene::new();
        let mut scenario = draw_case(ctx, &mut probe, kind, status, focus, frame, clip, palette)
            .expect("public independent content consumer");
        scenario
            .host
            .render(scene, ctx.text, frame, clip, palette)
            .unwrap();
        scenario.draw_disclosure_icon(scene, &mut GalleryIconRenderer::native(ctx), clip, palette);
        y += frame.size.height + 24.;
    }
    scene.pop_clip();
    y
}
