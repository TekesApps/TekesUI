#![cfg(target_os = "macos")]
//! Real empty ToolbarHost state with its ordinary full-target toolbar ground.
//! This is separate from the Button/TextField combination, whose original
//! unsupported result remains in parent_boundary_proof.rs.
use std::sync::Arc;
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
};

use tekes_ui_controls::{Insets, composition::*};
use tekes_ui_render::{BoundaryProofLimits, Renderer};
use tekes_ui_scene::{
    ClipShape, FragmentBudget, FragmentTransform, ImageQuad, Point, Primitive, Rect, Scene,
    ShapeTable,
};
use tekes_ui_tokens::{Rgba, Theme};

#[path = "support/coretext.rs"]
mod coretext;
use coretext::Text;

const ROOT_ID: ComponentId = ComponentId(12740);
const REGION_ID: ComponentId = ComponentId(12741);
const PARENT_ID: ComponentId = ComponentId(12742);
const BODY_ID: ComponentId = ComponentId(12743);
const BACKING: f32 = 2.0;
const TEXT_SCALE: f32 = 0.9;
const TRANSPARENT: Rgba = Rgba::new(0.0, 0.0, 0.0, 0.0);
const OUTPUT_PREFIX: &str = "/Volumes/HP P900/DEV/TekesUI/artifacts/issue-127/";

struct Evidence {
    dir: PathBuf,
    frames: Vec<serde_json::Value>,
    details: BTreeMap<String, serde_json::Value>,
    finished: bool,
}
impl Evidence {
    fn new() -> Self {
        let root = PathBuf::from(
            std::env::var_os("TEKESUI_PANEL82_GPU_ARTIFACT_DIR")
                .expect("runner must set the fresh SSD evidence root"),
        );
        assert!(
            root.starts_with(OUTPUT_PREFIX),
            "empty ToolbarHost evidence escaped its source epoch"
        );
        fs::create_dir_all(&root).unwrap();
        let dir =
            root.join("actual_public_empty_toolbar_host_proves_its_single_full_target_background");
        fs::create_dir(&dir).expect("evidence case path must be fresh");
        Self {
            dir,
            frames: Vec::new(),
            details: BTreeMap::new(),
            finished: false,
        }
    }
    fn save_rgba(&mut self, label: &str, rgba: &[u8], size: (u32, u32)) {
        let expected = (size.0 as usize)
            .checked_mul(size.1 as usize)
            .and_then(|v| v.checked_mul(4))
            .unwrap();
        assert_eq!(rgba.len(), expected);
        let raw_name = format!("{label}.rgba");
        write_new(&self.dir.join(&raw_name), rgba).unwrap();
        let mut pixmap = resvg::tiny_skia::Pixmap::new(size.0, size.1).unwrap();
        pixmap.data_mut().copy_from_slice(rgba);
        let png = pixmap.encode_png().unwrap();
        let png_name = format!("{label}.png");
        write_new(&self.dir.join(&png_name), &png).unwrap();
        self.frames.push(serde_json::json!({"label":label,"width_px":size.0,"height_px":size.1,"raw_file":raw_name,"raw_bytes":rgba.len(),"png_file":png_name,"raw_truth":"exact RGBA8 bytes returned by Renderer","png_note":"tiny-skia encodes the returned premultiplied RGBA8 as a visualization"}));
    }
    fn detail(&mut self, key: &str, value: serde_json::Value) {
        self.details.insert(key.to_owned(), value);
    }
    fn finish(&mut self) {
        let report = serde_json::json!({"format":"tekesui-issue127-empty-toolbar-host-proof/1","case":"actual_public_empty_toolbar_host_proves_its_single_full_target_background","status":"passed","frames":self.frames,"details":self.details,"raw_is_pixel_truth":true});
        write_new(
            &self.dir.join("test-report.json"),
            &serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
        self.finished = true;
    }
}
impl Drop for Evidence {
    fn drop(&mut self) {
        if !self.finished {
            let status = if std::thread::panicking() {
                "failed_or_panicked"
            } else {
                "incomplete"
            };
            let report = serde_json::json!({"format":"tekesui-issue127-empty-toolbar-host-proof/1","case":"actual_public_empty_toolbar_host_proves_its_single_full_target_background","status":status,"frames":self.frames,"details":self.details,"raw_is_pixel_truth":true});
            let _ = write_new(
                &self.dir.join("test-report.json"),
                &serde_json::to_vec_pretty(&report).unwrap(),
            );
        }
    }
}
fn write_new(path: &std::path::Path, data: &[u8]) -> std::io::Result<()> {
    let mut f = OpenOptions::new().write(true).create_new(true).open(path)?;
    f.write_all(data)?;
    f.sync_all()
}
fn alpha_pixels(rgba: &[u8]) -> usize {
    rgba.chunks_exact(4).filter(|p| p[3] > 0).count()
}
fn parent_host(text: &str) -> ContentHost {
    let child = TextFieldContent::read_only_content(
        "Parent",
        text,
        tekes_ui_tokens::proposal::TextRole::UiBody,
    );
    let mut panel = PanelContent::new(Content::new(BODY_ID, child)).unwrap();
    panel.set_padding(Some(Insets::default())).unwrap();
    ContentHost::new(Content::new(PARENT_ID, panel)).unwrap()
}
fn parent_scene(host: &mut ContentHost, text: &mut Text, target: Rect) -> Scene {
    let mut scene = Scene::new();
    host.render(&mut scene, text, target, target, Theme::LIGHT.into())
        .unwrap();
    scene
}
fn parent_geometry(host: &ContentHost) -> PanelGeometry {
    host.get(PARENT_ID)
        .and_then(|n| n.component::<PanelContent>())
        .and_then(PanelContent::geometry)
        .unwrap()
        .clone()
}
fn shape_in_child(geometry: &PanelGeometry) -> ClipShape {
    ClipShape::normalize(
        geometry.outer,
        geometry.shape.radii(),
        Point::new(-geometry.allocation.origin.x, -geometry.allocation.origin.y),
    )
    .unwrap()
}
fn target_quad_count(scene: &Scene, target: Rect) -> usize {
    scene
        .primitives()
        .iter()
        .filter(|p| match p {
            Primitive::Quad(q) => {
                q.rect == target
                    && q.corner_radii == tekes_ui_scene::CornerRadii::ZERO
                    && q.border_width == 0.0
            }
            _ => false,
        })
        .count()
}
fn draw_image_reference(
    renderer: &mut Renderer,
    shape: ClipShape,
    rgba: &[u8],
    size: (u32, u32),
    atlas: &mut tekes_ui_scene::GlyphAtlas,
) -> Vec<u8> {
    let mut table = ShapeTable::new(0x127_0040, 1);
    let key = table.intern(shape).unwrap();
    let mut scene = Scene::with_clip_shapes(&table);
    scene.push_shape_clip(key);
    let image = renderer.register_image(rgba, size.0, size.1).unwrap();
    scene.push_image(ImageQuad {
        rect: Rect::new(0.0, 0.0, size.0 as f32 / BACKING, size.1 as f32 / BACKING),
        image,
        opacity: 1.0,
    });
    scene.pop_clip();
    let pixels = renderer
        .render_to_rgba8(&scene, atlas, size, BACKING, TRANSPARENT)
        .unwrap();
    assert!(renderer.unregister_image(image));
    pixels
}

#[test]
#[ignore = "explicit independent natural empty ToolbarHost GPU state; not the original full combination"]
fn actual_public_empty_toolbar_host_proves_its_single_full_target_background() {
    let mut evidence = Evidence::new();
    let mut renderer = Renderer::new_headless().expect("actual Metal renderer required");
    let target = Rect::new(0.0, 0.0, 320.0, 144.0);
    let size = (640, 288);
    let mut parent_text = Text::new(TEXT_SCALE);
    parent_text.services = true;
    parent_text.backing = BACKING;
    let mut parent = parent_host("real rounded Panel parent with ordinary public zero padding");
    let pscene = parent_scene(&mut parent, &mut parent_text, target);
    let geometry = parent_geometry(&parent);
    assert_eq!(geometry.outer, target);
    assert_eq!(geometry.allocation, geometry.outer);
    let before = renderer
        .render_to_rgba8(&pscene, &mut parent_text.atlas, size, BACKING, TRANSPARENT)
        .unwrap();
    evidence.save_rgba("parent_before", &before, size);
    let shape = shape_in_child(&geometry);
    let safe = shape
        .full_coverage_inner_rect_px(BACKING, size)
        .unwrap()
        .unwrap();
    let [x0, y0, x1, y1] = safe.bounds_px();
    assert!(
        x0 > 0.0 && y0 > 0.0 && x1 < size.0 as f32 && y1 < size.1 as f32,
        "actual parent safe rectangle must exclude the target background"
    );

    // Empty is the real public host state: a window region exists and paints its
    // ordinary toolbar ground, but there are no registered command groups.
    let mut text = Text::new(TEXT_SCALE);
    text.services = true;
    text.backing = BACKING;
    let mut toolbar = ToolbarHost::<()>::new(ROOT_ID, 0x1274, target).unwrap();
    toolbar
        .set_regions(
            0x1274,
            target,
            vec![ToolbarRegion::window(
                REGION_ID,
                target,
                ToolbarWindowEdge::Leading,
            )],
        )
        .unwrap();
    let mut scene = Scene::new();
    toolbar
        .render(&mut scene, &mut text, Theme::LIGHT.into())
        .unwrap();
    let scene_before = scene.clone();
    assert_eq!(
        target_quad_count(&scene, target),
        1,
        "only the real Regions::paint full-target flat Quad is the B source"
    );
    let draw_count = scene
        .primitives()
        .iter()
        .filter(|p| {
            matches!(
                p,
                Primitive::Quad(_)
                    | Primitive::KeyedQuad { .. }
                    | Primitive::Shadow(_)
                    | Primitive::Glyphs(_)
                    | Primitive::Image(_)
                    | Primitive::TransformedImage(_)
                    | Primitive::ControlledImage { .. }
                    | Primitive::ControlledTransformedImage { .. }
            )
        })
        .count();
    assert_eq!(
        draw_count, 1,
        "the actual empty toolbar host has no hidden child drawable"
    );
    let ordinary = renderer
        .render_to_rgba8(&scene, &mut text.atlas, size, BACKING, TRANSPARENT)
        .unwrap();
    evidence.save_rgba("toolbar_ordinary", &ordinary, size);
    let proof = renderer
        .render_to_rgba8_with_parent_boundary_proof(
            &scene,
            &mut text.atlas,
            size,
            BACKING,
            TRANSPARENT,
            Arc::<[ClipShape]>::from(vec![shape]),
            BoundaryProofLimits::default(),
        )
        .expect("one real flat target ToolbarHost background qualifies under Class B");
    assert_eq!(proof.rgba8(), ordinary.as_slice());
    assert_eq!(scene, scene_before);
    evidence.save_rgba("toolbar_proof", proof.rgba8(), size);
    let mut table = ShapeTable::new(0x127_0041, 1);
    let key = table.intern(shape).unwrap();
    let mut direct = Scene::with_clip_shapes(&table);
    direct.push_shape_clip(key);
    direct
        .try_append_fragment(
            &scene,
            FragmentBudget::default(),
            FragmentTransform::default(),
        )
        .unwrap();
    direct.pop_clip();
    let direct_pixels = renderer
        .render_to_rgba8(&direct, &mut text.atlas, size, BACKING, TRANSPARENT)
        .unwrap();
    evidence.save_rgba("parent_masked_direct", &direct_pixels, size);
    let image_pixels =
        draw_image_reference(&mut renderer, shape, proof.rgba8(), size, &mut text.atlas);
    evidence.save_rgba("image_quad_reference", &image_pixels, size);
    assert_eq!(
        image_pixels, direct_pixels,
        "direct parent mask equals composition of returned proof pixels through registered ImageQuad"
    );
    assert_ne!(
        direct_pixels, ordinary,
        "rounded actual parent masks the full-target background's outer corners"
    );
    let after = renderer
        .render_to_rgba8(&pscene, &mut parent_text.atlas, size, BACKING, TRANSPARENT)
        .unwrap();
    assert_eq!(after, before);
    evidence.save_rgba("parent_after", &after, size);
    assert_eq!(
        scene, scene_before,
        "proof path preserves the entire actual empty ToolbarHost scene"
    );
    evidence.detail("parent_geometry",serde_json::json!({"outer_pt":[geometry.outer.origin.x,geometry.outer.origin.y,geometry.outer.size.width,geometry.outer.size.height],"allocation_pt":[geometry.allocation.origin.x,geometry.allocation.origin.y,geometry.allocation.size.width,geometry.allocation.size.height],"safe_rect_px":[x0,y0,x1,y1],"target_px":[size.0,size.1],"backing_scale":BACKING,"shape":format!("{:?}",shape)}));
    evidence.detail("toolbar_scene",serde_json::json!({"public_host":"ToolbarHost::new + set_regions + render; no register calls","state":"valid empty Window region","primitive_count":scene.primitives().len(),"draw_count":draw_count,"target_flat_background_count":target_quad_count(&scene,target),"primitive_kinds":scene.primitives().iter().map(|p|format!("{p:?}")).collect::<Vec<_>>()}));
    evidence.detail("b_source_correspondence",serde_json::json!({"single_draw_is_public_toolbar_background":true,"full_target_is_outside_actual_parent_safe_rect":true,"target_flat_quad_count":target_quad_count(&scene,target),"successful_proof":true,"receipt_debug":format!("{proof:?}")}));
    evidence.detail("pixel_checks",serde_json::json!({"proof_equals_ordinary":proof.rgba8()==ordinary.as_slice(),"direct_parent_mask_equals_image_quad_reference":direct_pixels==image_pixels,"parent_after_equals_before":after==before,"scene_unchanged_by_proof":scene==scene_before,"parent_mask_differs_from_full_target_ordinary":direct_pixels!=ordinary,"ordinary_alpha_pixels":alpha_pixels(&ordinary),"direct_alpha_pixels":alpha_pixels(&direct_pixels)}));
    evidence.finish();
}
