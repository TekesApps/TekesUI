use tekes_ui_controls::{
    Insets,
    composition::*,
    extended::{Axis, Stack},
};
use tekes_ui_render::Renderer;
use tekes_ui_scene::Size;
pub const ROOT: ComponentId = ComponentId(20400);
pub const VIEW: ComponentId = ComponentId(20401);
pub const IMAGE: ComponentId = ComponentId(20402);
pub const BODY: ComponentId = ComponentId(20403);
pub const CLOSE: ComponentId = ComponentId(20416);
pub const RETRY: ComponentId = ComponentId(20417);
pub const COMMANDS: [(ComponentId, &str, MediaViewerCommand); 4] = [
    (ComponentId(20411), "Zoom out", MediaViewerCommand::ZoomOut),
    (ComponentId(20412), "Zoom in", MediaViewerCommand::ZoomIn),
    (ComponentId(20413), "Fit", MediaViewerCommand::Fit),
    (
        ComponentId(20414),
        "Actual logical size",
        MediaViewerCommand::ActualLogicalSize,
    ),
];
pub fn viewer(host: &mut ContentHost) -> &mut MediaViewerContent {
    host.get_mut(VIEW).unwrap().component_mut().unwrap()
}
pub fn fixture(gpu: &mut Renderer) -> ContentHost {
    let mut host = ContentHost::new(content(gpu)).unwrap();
    refresh_status(&mut host);
    host
}
fn content(gpu: &mut Renderer) -> Content {
    // Transparent center with unmistakable RG/BY corner quadrants. Supplied
    // already decoded: the consumer never creates another resource/load owner.
    let mut rgba = vec![0; 16 * 8 * 4];
    for y in 0..8 {
        for x in 0..16 {
            if (6..10).contains(&x) && (2..6).contains(&y) {
                continue;
            }
            let color = match (x < 8, y < 4) {
                (true, true) => [255, 0, 0, 255],
                (false, true) => [0, 255, 0, 255],
                (true, false) => [0, 0, 255, 255],
                (false, false) => [255, 255, 0, 255],
            };
            rgba[(y * 16 + x) * 4..(y * 16 + x + 1) * 4].copy_from_slice(&color);
        }
    }
    let image = ImageContent::ready(
        gpu.register_bitmap_image(&rgba, 16, 8).unwrap(),
        Size::new(200., 100.),
        ImageSemantics::Alternative("Red green blue yellow corners; transparent center".into()),
    )
    .unwrap();
    let mut children: Vec<Content> = COMMANDS
        .iter()
        .map(|(id, label, _)| Content::new(*id, Button::new(*label)))
        .collect();
    children.push(Content::new(CLOSE, Button::new("Close")));
    children.push(Content::new(RETRY, Button::new("Retry")));
    children.push(Content::new(
        ComponentId(20418),
        TextContent::new("Arrow keys pan. Fit resets. Logical size differs from bitmap pixels."),
    ));
    children.push(Content::new(
        VIEW,
        MediaViewerContent::new(IMAGE, image, "Media viewer").unwrap(),
    ));
    let stack = Content::new(
        BODY,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 4.,
            },
            children,
            flexible_child: Some(VIEW),
        },
    );
    let mut panel = PanelContent::new(stack).unwrap();
    panel.set_padding(Some(Insets::default())).unwrap();
    Content::new(ROOT, panel)
}
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
                .activations;
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

/// Disabled state is a read-only projection of the same Viewer owner.
pub fn refresh_buttons(host: &mut ContentHost) {
    for (id, _, command) in COMMANDS {
        let enabled = viewer(host).can_execute(command);
        host.get_mut(id)
            .unwrap()
            .component_mut::<Button>()
            .unwrap()
            .button
            .state
            .disabled = !enabled;
    }
}

pub const MODAL_ROOT: ComponentId = ComponentId(20460);
pub const INVOKER: ComponentId = ComponentId(20461);
pub const DIALOG: ComponentId = ComponentId(20462);
pub const STATUS: ComponentId = ComponentId(20418);
/// The original Dialog/Host owns modal scope and return focus.
pub fn modal_fixture(gpu: &mut Renderer) -> ContentHost {
    ContentHost::new(Content::new(
        MODAL_ROOT,
        StackContent {
            model: Stack::new(Axis::Vertical),
            flexible_child: None,
            children: vec![
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
        },
    ))
    .unwrap()
}
/// Status copy is projected from original load state; no second state owner.
pub fn refresh_status(host: &mut ContentHost) {
    let status = viewer(host).image_status();
    let text = match status {
        Some(ImageLoadStatus::Loading) => {
            "Loading replacement. Previous image remains visible.".into()
        }
        Some(ImageLoadStatus::Failed) => format!(
            "Replacement failed: {}. Retry or close.",
            viewer(host).failure_message().unwrap_or("Unknown error")
        ),
        Some(ImageLoadStatus::Missing) => "Image missing. Retry or close.".into(),
        _ => "Arrow keys pan. Fit resets. Logical size differs from bitmap pixels.".into(),
    };
    host.get_mut(STATUS)
        .unwrap()
        .component_mut::<TextContent>()
        .unwrap()
        .text = text;
}
