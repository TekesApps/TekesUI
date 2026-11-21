use tekes_ui_controls::{ButtonTextRenderer, composition::*};
use tekes_ui_render::Renderer;
use tekes_ui_scene::{Rect, Scene};
use tekes_ui_tokens::Palette;

#[path = "viewer_core.rs"]
mod viewer_core;
pub use viewer_core::*;

/// Product adapter consumes the original Button's activation count. It owns no
/// transform, loading or focus state. Close/Retry are returned to the caller.
#[derive(Default)]
pub struct Actions {
    seen: [usize; 6],
}
impl Actions {
    pub fn dispatch(&mut self, host: &mut ContentHost) -> (usize, usize) {
        let mut exit = (0, 0);
        for (index, id) in COMMANDS
            .iter()
            .map(|(id, _, _)| *id)
            .chain([CLOSE, RETRY])
            .enumerate()
        {
            let count = host
                .get(id)
                .unwrap()
                .component::<Button>()
                .unwrap()
                .activations();
            let delta = count
                .checked_sub(self.seen[index])
                .expect("original button was not replaced");
            self.seen[index] = count;
            for _ in 0..delta {
                if index < 4 {
                    viewer(host).execute(COMMANDS[index].2).unwrap();
                } else if index == 4 {
                    exit.0 += 1;
                } else {
                    exit.1 += 1;
                }
            }
        }
        exit
    }
}

/// Publish the native consumer's original command/status projection and scene.
/// Layout is the sole publisher of the Viewer's runnable input authority.
pub fn render_frame(
    host: &mut ContentHost,
    text: &mut dyn ButtonTextRenderer,
    frame: Rect,
    active: bool,
    palette: Palette,
) -> Result<Scene, CompositionError> {
    refresh_buttons(host);
    refresh_status(host);
    host.set_window_active(active);
    let mut scene = Scene::new();
    host.render(&mut scene, text, frame, frame, palette)?;
    // Recompute availability after layout restores runnable authority. Mutate
    // only changed projections, then acknowledge them before presenting.
    if refresh_buttons(host) {
        scene = Scene::new();
        host.render(&mut scene, text, frame, frame, palette)?;
    }
    Ok(scene)
}

pub const MODAL_ROOT: ComponentId = ComponentId(20460);
pub const INVOKER: ComponentId = ComponentId(20461);
pub const DIALOG: ComponentId = ComponentId(20462);
/// The original Dialog/Host owns modal scope and return focus.
pub fn modal_fixture(gpu: &mut Renderer) -> ContentHost {
    ContentHost::new(Content::new(
        MODAL_ROOT,
        StackContent::vertical(
            4.,
            vec![
                Content::new(INVOKER, Button::new("Open viewer")),
                Content::new(
                    DIALOG,
                    DialogContent::new(
                        "Image details",
                        Content::new(ComponentId(20463), TextContent::new("Image details")),
                        content(gpu),
                        Content::new(
                            ComponentId(20464),
                            TextContent::new("Close returns to Open viewer"),
                        ),
                    ),
                ),
            ],
        ),
    ))
    .unwrap()
}
