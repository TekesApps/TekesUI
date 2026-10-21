//! Demo-specific assertions. Kept out of the shared Host/runtime.
use anyhow::{Result, ensure};
use tekes_ui_app_host::module_process::{ModuleProcess, import_frame};
use tekes_ui_app_protocol::{InputKind, Request, VERSION};
use tekes_ui_render::Renderer;
use tekes_ui_scene::{GlyphAtlas, ImageQuad, Rect, Scene};
use tekes_ui_tokens::Rgba;
pub fn run(module_ids: &[String]) -> Result<()> {
    let mut renderer = Renderer::new_headless()?;
    let context = renderer.gpu_context();
    let mut atlas = GlyphAtlas::default();
    let mut connections: Vec<_> = module_ids
        .iter()
        .map(|p| ModuleProcess::spawn(p))
        .collect::<Result<_>>()?;
    let pids = connections
        .iter()
        .map(|c| c.pid())
        .collect::<std::collections::HashSet<_>>();
    ensure!(
        pids.len() == module_ids.len(),
        "module processes must be distinct"
    );
    let mut proof = Vec::new();
    for (i, c) in connections.iter_mut().enumerate() {
        let mut samples = Vec::new();
        let mut label_digests = Vec::new();
        let mut count = 0;
        let mut text_pixels = 0;
        let steps = [
            (InputKind::Layout, 0),
            (InputKind::PointerDown, 0),
            (InputKind::PointerUp, 1),
            (InputKind::Command, 2),
            (InputKind::Command, 3),
            (InputKind::PointerDown, 3),
            (InputKind::Cancel, 3),
            (InputKind::PointerUp, 3),
            (InputKind::PointerMove, 3),
            (InputKind::Layout, 3),
        ];
        for (seq, (kind, expected)) in steps.into_iter().enumerate() {
            let resized = seq == steps.len() - 1;
            let width = if resized { 360 } else { 320 };
            let r = Request {
                version: VERSION,
                command: (kind == InputKind::Command).then_some(1),
                seq: seq as u64,
                viewport: if resized { 2 } else { 1 },
                kind,
                x: 40.,
                y: 40.,
                width,
                height: 240,
                identity: i as u32,
                scale: if resized { 2. } else { 1. },
            };
            let mut frame = c.request(&r)?;
            count = expected;
            ensure!(
                frame.revision == count,
                "native button/action state mismatch"
            );
            ensure!(
                frame.toolbar.first().is_some_and(|item| item.id == 1),
                "no toolbar registered by native module"
            );
            let texture = import_frame(&context, &mut frame)?;
            let image = context.import_image(texture)?;
            let id = renderer.register_gpu_image(&image)?;
            let mut s = Scene::new();
            s.push_image(ImageQuad {
                rect: Rect::new(0., 0., width as f32, 240.),
                image: id,
                opacity: 1.,
            });
            // Readback exists only in this verifier, after cross-process GPU
            // import/composition. It is never used as IPC or production transport.
            let pixels =
                renderer.render_to_rgba8(&s, &mut atlas, (width, 240), 1., Rgba::gray(0.))?;
            if seq == 0 {
                for y in 120..200usize {
                    for x in 40..(width as usize - 40) {
                        let k = (y * width as usize + x) * 4;
                        if pixels[k..k + 3].iter().all(|c| *c < 120) {
                            text_pixels += 1;
                        }
                    }
                }
                ensure!(
                    text_pixels > 100,
                    "native text glyphs missing from imported GPU frame"
                );
            }
            let label_hash = pixels[..(width as usize * 90 * 4)]
                .iter()
                .fold(0xcbf29ce484222325u64, |hash, byte| {
                    (hash ^ *byte as u64).wrapping_mul(0x100000001b3)
                });
            label_digests.push(label_hash);
            renderer.unregister_image(id);
            // Sample the padded background, outside the reflowing glyphs.
            let offset = (110 * width as usize + 28) * 4;
            let sample = pixels[offset..offset + 4].to_vec();
            ensure!(
                sample[3] == 255 && sample[2] > 150,
                "shared texture sample invalid: {sample:?}"
            );
            samples.push(sample);
        }
        ensure!(
            label_digests[0] != label_digests[2],
            "native module change did not reach shared GPU image"
        );
        proof.push(serde_json::json!({"pid":c.pid(),"module":module_ids[i],"counter":count,"text_pixels":text_pixels,"samples":samples,"label_digests":label_digests}));
    }
    println!(
        "{}",
        serde_json::json!({"result":"NATIVE_GPU_IPC_VERIFIED","host_pid":std::process::id(),"panels":module_ids.len(),"workers":proof,"pixel_readback":"verification only; never IPC","producer_sync":"worker GPU completion before frame-ready","consumer_sync":"verifier serial; interactive Host bounds two immutable frames per panel with GPU completion acknowledgements"})
    );
    Ok(())
}
