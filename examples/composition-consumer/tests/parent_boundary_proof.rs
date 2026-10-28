#![cfg(all(target_os = "macos", feature = "bitmap-images"))]
//! Root consumer proof oracle. Each proof input is one complete child Host scene;
//! the actual parent Host contributes only its public arranged Panel geometry.
//! The first two tests preserve the R8 panel/text and coincident-refusal baseline.
use std::sync::Arc;
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
};

use tekes_ui_controls::{
    Insets,
    composition::*,
    extended::{Axis, Stack},
};
use tekes_ui_render::{BoundaryProofLimits, Renderer};
use tekes_ui_scene::{
    ClipShape, FragmentBudget, FragmentTransform, ImageQuad, Point, Primitive, Rect, Scene,
    ShapeTable, Size,
};
use tekes_ui_tokens::{Rgba, Theme};

#[path = "support/coretext.rs"]
mod coretext;
use coretext::Text;

const PARENT: ComponentId = ComponentId(12701);
const CHILD: ComponentId = ComponentId(8404);
const BODY: ComponentId = ComponentId(8403);
const BACKING: f32 = 2.0;
const TEXT_SCALE: f32 = 0.9;
const TRANSPARENT: Rgba = Rgba::new(0.0, 0.0, 0.0, 0.0);
const OUTPUT_PREFIX: &str = "/Volumes/HP P900/DEV/TekesUI/artifacts/issue-127/";

struct Evidence {
    dir: PathBuf,
    case: &'static str,
    frames: Vec<serde_json::Value>,
    details: BTreeMap<String, serde_json::Value>,
    report_written: bool,
}
impl Evidence {
    fn new(case: &'static str) -> Self {
        let root = PathBuf::from(
            std::env::var_os("TEKESUI_PANEL82_GPU_ARTIFACT_DIR")
                .expect("runner must set a fresh Root consumer SSD output directory"),
        );
        assert!(
            root.starts_with(OUTPUT_PREFIX),
            "Root consumer output path escaped its package"
        );
        fs::create_dir_all(&root).expect("create Root consumer output root");
        let dir = root.join(case);
        fs::create_dir(&dir).expect("refuse reused Root consumer evidence directory");
        Self {
            dir,
            case,
            frames: Vec::new(),
            details: BTreeMap::new(),
            report_written: false,
        }
    }

    fn detail(&mut self, name: &str, value: serde_json::Value) {
        self.details.insert(name.to_owned(), value);
    }

    fn save_rgba(&mut self, label: &str, rgba: &[u8], size: (u32, u32)) {
        let expected = (size.0 as usize)
            .checked_mul(size.1 as usize)
            .and_then(|n| n.checked_mul(4))
            .unwrap();
        assert_eq!(rgba.len(), expected);
        let raw_name = format!("{label}.rgba");
        write_new(&self.dir.join(&raw_name), rgba)
            .expect("write raw render pixels without overwrite");
        let mut pixmap = resvg::tiny_skia::Pixmap::new(size.0, size.1).expect("allocate PNG view");
        pixmap.data_mut().copy_from_slice(rgba);
        let png_bytes = pixmap
            .encode_png()
            .expect("encode PNG from actual RGBA output");
        let png_name = format!("{label}.png");
        write_new(&self.dir.join(&png_name), &png_bytes).expect("write PNG without overwrite");
        self.frames.push(serde_json::json!({
            "label":label, "width_px":size.0, "height_px":size.1,
            "raw_file":raw_name, "raw_bytes":rgba.len(), "png_file":png_name,
            "raw_truth":"exact RGBA8 bytes returned by Renderer",
            "png_note":"tiny-skia receives premultiplied RGBA8; PNG is a visualization encoding"
        }));
    }

    fn write_report(&mut self, status: &str) -> std::io::Result<()> {
        if self.report_written {
            return Ok(());
        }
        let value = serde_json::json!({
            "format":"tekesui-issue127-root-consumer-proof-test-report/1",
            "case":self.case, "status":status, "frames":self.frames,
            "details":self.details, "raw_is_pixel_truth":true
        });
        let data = serde_json::to_vec_pretty(&value).expect("serialize report");
        write_new(&self.dir.join("test-report.json"), &data)?;
        self.report_written = true;
        Ok(())
    }

    fn finish(&mut self) {
        self.write_report("passed").expect("write success evidence");
    }
}
impl Drop for Evidence {
    fn drop(&mut self) {
        if !self.report_written {
            let status = if std::thread::panicking() {
                "failed_or_panicked"
            } else {
                "incomplete"
            };
            let _ = self.write_report(status);
        }
    }
}

fn write_new(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

fn counts(scene: &Scene) -> serde_json::Value {
    let mut runs = 0usize;
    let mut glyphs = 0usize;
    let mut images = 0usize;
    for primitive in scene.primitives() {
        match primitive {
            Primitive::Glyphs(run) => {
                runs += 1;
                glyphs += run.glyphs.len();
            }
            Primitive::Image(_)
            | Primitive::TransformedImage(_)
            | Primitive::ControlledImage { .. }
            | Primitive::ControlledTransformedImage { .. } => images += 1,
            _ => {}
        }
    }
    serde_json::json!({
        "primitive_count":scene.primitives().len(), "clip_shape_count":scene.clip_shape_count(),
        "glyph_run_count":runs, "glyph_instance_count":glyphs, "image_primitive_count":images,
        "coretext_scene_glyph_instance_count":glyphs,
        "glyph_count_source":"actual Primitive::Glyphs instances emitted by the existing CoreText fixture"
    })
}

fn geometry_report(
    geometry: &PanelGeometry,
    shape: ClipShape,
    safe: Option<[f32; 4]>,
    target: (u32, u32),
) -> serde_json::Value {
    geometry_report_at_scale(geometry, shape, safe, target, BACKING)
}

fn geometry_report_at_scale(
    geometry: &PanelGeometry,
    shape: ClipShape,
    safe: Option<[f32; 4]>,
    target: (u32, u32),
    backing: f32,
) -> serde_json::Value {
    let outer = geometry.outer;
    let allocation = geometry.allocation;
    let translated = shape.rect();
    let radii = shape.radii();
    serde_json::json!({
        "outer_pt":[outer.origin.x,outer.origin.y,outer.size.width,outer.size.height],
        "allocation_pt":[allocation.origin.x,allocation.origin.y,allocation.size.width,allocation.size.height],
        "translated_shape_pt":[translated.origin.x,translated.origin.y,translated.size.width,translated.size.height],
        "translated_radii_pt":[radii.top_left,radii.top_right,radii.bottom_right,radii.bottom_left],
        "safe_rect_px":safe,"scale":backing,"target_px":[target.0,target.1]
    })
}

fn alpha_covered(rgba: &[u8]) -> usize {
    rgba.chunks_exact(4).filter(|pixel| pixel[3] > 0).count()
}

fn host(text: &str, padding: Option<Insets>, root_id: ComponentId) -> ContentHost {
    let body = TextFieldContent::read_only_content(
        "正文",
        text,
        tekes_ui_tokens::proposal::TextRole::UiBody,
    );
    let mut panel = PanelContent::new(Content::new(BODY, body)).unwrap();
    if let Some(padding) = padding {
        panel.set_padding(Some(padding)).unwrap();
    }
    ContentHost::new(Content::new(root_id, panel)).unwrap()
}

fn layout(host: &mut ContentHost, text: &mut Text, width: f32, height: f32) -> Scene {
    let frame = Rect::new(0.0, 0.0, width, height);
    let mut scene = Scene::new();
    host.render(&mut scene, text, frame, frame, Theme::LIGHT.into())
        .expect("the original public Host must arrange and paint the Panel tree");
    scene
}

fn panel_geometry(host: &ContentHost, id: ComponentId) -> PanelGeometry {
    host.get(id)
        .and_then(|node| node.component::<PanelContent>())
        .and_then(PanelContent::geometry)
        .expect("the actual Host publishes arranged PanelGeometry")
        .clone()
}

fn scaled_extent(logical: f32) -> u32 {
    let pixels = logical * BACKING;
    assert!(pixels.is_finite() && pixels > 0.0 && pixels.fract() == 0.0);
    pixels as u32
}

fn parent_shape_in_child_coordinates(geometry: &PanelGeometry) -> ClipShape {
    // Reuse the actual parent Panel shape tuple and translate it by the actual
    // arranged child allocation origin. No pixel-derived or hand-sized witness.
    ClipShape::normalize(
        geometry.outer,
        geometry.shape.radii(),
        Point::new(-geometry.allocation.origin.x, -geometry.allocation.origin.y),
    )
    .expect("translation of actual arranged parent geometry is finite")
}

fn draw_image_reference(
    renderer: &mut Renderer,
    parent_shape: ClipShape,
    pixels: &[u8],
    size_px: (u32, u32),
    atlas: &mut tekes_ui_scene::GlyphAtlas,
) -> Vec<u8> {
    draw_image_reference_at_scale(renderer, parent_shape, pixels, size_px, atlas, BACKING)
}

fn draw_image_reference_at_scale(
    renderer: &mut Renderer,
    parent_shape: ClipShape,
    pixels: &[u8],
    size_px: (u32, u32),
    atlas: &mut tekes_ui_scene::GlyphAtlas,
    backing: f32,
) -> Vec<u8> {
    let mut table = ShapeTable::new(0x127_0001, 1);
    let key = table.intern(parent_shape).unwrap();
    let mut scene = Scene::with_clip_shapes(&table);
    scene.push_shape_clip(key);
    let image = renderer
        .register_image(pixels, size_px.0, size_px.1)
        .unwrap();
    scene.push_image(ImageQuad {
        rect: Rect::new(
            0.0,
            0.0,
            size_px.0 as f32 / backing,
            size_px.1 as f32 / backing,
        ),
        image,
        opacity: 1.0,
    });
    scene.pop_clip();
    let rgba = renderer
        .render_to_rgba8(&scene, atlas, size_px, backing, TRANSPARENT)
        .unwrap();
    assert!(renderer.unregister_image(image));
    rgba
}

#[test]
#[ignore = "explicit finite real GPU acceptance under the original build/GPU locks"]
fn actual_parent_panel_allocation_contains_complete_child_host_proof_and_image_reference() {
    let mut evidence = Evidence::new("r8_panel_text_baseline");
    let mut renderer = Renderer::new_headless().expect("actual Metal renderer required");

    // This parent Host uses the ordinary token-derived Panel padding. Its actual
    // layout and painted baseline remain separately captured and unchanged.
    let mut parent_text = Text::new(TEXT_SCALE);
    parent_text.services = true;
    parent_text.backing = BACKING;
    let mut parent_host = host(
        "parent Host keeps its original background, focus decoration and subtree",
        None,
        PARENT,
    );
    let parent_scene = layout(&mut parent_host, &mut parent_text, 320.0, 144.0);
    let parent_scene_counts = counts(&parent_scene);
    let parent_geometry = panel_geometry(&parent_host, PARENT);
    assert_eq!(parent_geometry.outer, Rect::new(0.0, 0.0, 320.0, 144.0));
    assert!(parent_geometry.allocation.origin.x > 0.0);
    assert!(parent_geometry.allocation.origin.y > 0.0);
    let parent_size_px = (640, 288);
    let parent_pixels = renderer
        .render_to_rgba8(
            &parent_scene,
            &mut parent_text.atlas,
            parent_size_px,
            BACKING,
            TRANSPARENT,
        )
        .unwrap();
    evidence.save_rgba("parent_before", &parent_pixels, parent_size_px);
    let parent_primitive_count = parent_scene.primitives().len();

    // The child Host uses the existing actual Panel/TextField/CoreText fixture
    // and its full scene, including panel fill, shadows, boundary, focus and glyphs.
    // Only its viewport dimensions come from the parent Host's public allocation.
    let child_size_px = (
        scaled_extent(parent_geometry.allocation.size.width),
        scaled_extent(parent_geometry.allocation.size.height),
    );
    let child_width = parent_geometry.allocation.size.width;
    let child_height = parent_geometry.allocation.size.height;
    let mut child_text = Text::new(TEXT_SCALE);
    child_text.services = true;
    child_text.backing = BACKING;
    let mut child_host = host(
        "MMMMmmWW 中文 — actual CoreText glyphs, focus and panel decoration",
        Some(Insets::default()),
        CHILD,
    );
    let mut child_scene = layout(&mut child_host, &mut child_text, child_width, child_height);
    let child_scene_counts = counts(&child_scene);
    let parent_shape = parent_shape_in_child_coordinates(&parent_geometry);
    let witness = parent_shape
        .full_coverage_inner_rect_px(BACKING, child_size_px)
        .expect("finite parent witness query")
        .expect("actual token-derived parent inset must support the child allocation");
    let [x0, y0, x1, y1] = witness.bounds_px();
    assert!(x0 <= 0.0 && y0 <= 0.0);
    assert!(x1 >= child_size_px.0 as f32 && y1 >= child_size_px.1 as f32);

    let ordinary = renderer
        .render_to_rgba8(
            &child_scene,
            &mut child_text.atlas,
            child_size_px,
            BACKING,
            TRANSPARENT,
        )
        .unwrap();
    evidence.save_rgba("child_ordinary", &ordinary, child_size_px);
    let proof = renderer
        .render_to_rgba8_with_parent_boundary_proof(
            &child_scene,
            &mut child_text.atlas,
            child_size_px,
            BACKING,
            TRANSPARENT,
            Arc::<[ClipShape]>::from(vec![parent_shape]),
            BoundaryProofLimits::default(),
        )
        .expect("the whole actual child Host scene must be inside the real parent safe area");
    assert_eq!(proof.dimensions_px(), child_size_px);
    assert_eq!(proof.rgba8(), ordinary.as_slice());
    evidence.save_rgba("child_proof", proof.rgba8(), child_size_px);

    // Build the direct same-parent-clip Scene from the same complete child Host
    // fragment; then compare it against the original proof pixels as an ImageQuad.
    let mut table = ShapeTable::new(0x127_0002, 1);
    let key = table.intern(parent_shape).unwrap();
    let mut direct = Scene::with_clip_shapes(&table);
    direct.push_shape_clip(key);
    direct
        .try_append_fragment(
            &child_scene,
            FragmentBudget::default(),
            FragmentTransform::default(),
        )
        .expect("append preserves every child primitive and its immutable clip inventory");
    direct.pop_clip();
    let direct_pixels = renderer
        .render_to_rgba8(
            &direct,
            &mut child_text.atlas,
            child_size_px,
            BACKING,
            TRANSPARENT,
        )
        .unwrap();
    assert_eq!(direct_pixels, proof.rgba8());
    evidence.save_rgba("parent_masked_direct", &direct_pixels, child_size_px);
    let image_pixels = draw_image_reference(
        &mut renderer,
        parent_shape,
        proof.rgba8(),
        child_size_px,
        &mut child_text.atlas,
    );
    assert_eq!(image_pixels, direct_pixels);
    evidence.save_rgba("image_quad_reference", &image_pixels, child_size_px);

    // Parent output remains its separately captured ordinary baseline, untouched
    // while the child proof and image oracle run.
    assert_eq!(parent_scene.primitives().len(), parent_primitive_count);
    let parent_after = renderer
        .render_to_rgba8(
            &parent_scene,
            &mut parent_text.atlas,
            parent_size_px,
            BACKING,
            TRANSPARENT,
        )
        .unwrap();
    assert_eq!(parent_after, parent_pixels);
    evidence.save_rgba("parent_after", &parent_after, parent_size_px);

    // VerifiedFrameOutput owns immutable bytes after subsequent Scene/atlas/image changes.
    let proof_copy = proof.rgba8().to_vec();
    child_scene.clear();
    child_text.atlas.clear();
    let mutation_image = renderer
        .register_image(&proof_copy, child_size_px.0, child_size_px.1)
        .unwrap();
    renderer
        .update_image(mutation_image, &vec![0; proof_copy.len()])
        .unwrap();
    assert!(renderer.unregister_image(mutation_image));
    assert_eq!(proof.rgba8(), proof_copy.as_slice());
    assert_eq!(proof.dimensions_px(), child_size_px);
    evidence.detail(
        "geometry",
        geometry_report(
            &parent_geometry,
            parent_shape,
            Some(witness.bounds_px()),
            child_size_px,
        ),
    );
    evidence.detail("parent_scene", parent_scene_counts);
    evidence.detail("child_scene_original_before_mutation", child_scene_counts);
    evidence.detail("proof_debug", serde_json::json!(format!("{proof:?}")));
    evidence.detail("coverage_boundary", serde_json::json!({"expected_path":"complete child viewport within Class A; does not establish Class B edge-band positives","material_scope":"registered bitmap alpha, not every primitive/material combination"}));
    evidence.detail("pixel_checks", serde_json::json!({
        "proof_equals_ordinary":proof.rgba8()==ordinary.as_slice(),
        "direct_equals_proof":direct_pixels==proof.rgba8(),
        "image_quad_equals_direct":image_pixels==direct_pixels,
        "parent_after_equals_before":parent_after==parent_pixels,
        "ordinary_alpha_covered_pixels":alpha_covered(&ordinary),
        "proof_immutable_after_scene_atlas_and_image_mutation":proof.rgba8()==proof_copy.as_slice()
    }));
    evidence.finish();
}

#[test]
#[ignore = "explicit finite real GPU acceptance under the original build/GPU locks"]
fn actual_child_host_scene_with_coincident_panel_parent_is_refused_without_cropping() {
    let mut evidence = Evidence::new("r8_coincident_refusal_baseline");
    let mut renderer = Renderer::new_headless().expect("actual Metal renderer required");
    let mut parent_text = Text::new(TEXT_SCALE);
    parent_text.services = true;
    parent_text.backing = BACKING;
    let mut parent_host = host(
        "coincident parent boundary keeps its original complete child",
        Some(Insets::default()),
        PARENT,
    );
    let parent_scene = layout(&mut parent_host, &mut parent_text, 320.0, 144.0);
    let parent_scene_counts = counts(&parent_scene);
    let parent_geometry = panel_geometry(&parent_host, PARENT);
    assert_eq!(parent_geometry.allocation, parent_geometry.outer);
    let size_px = (640, 288);
    let parent_before = renderer
        .render_to_rgba8(
            &parent_scene,
            &mut parent_text.atlas,
            size_px,
            BACKING,
            TRANSPARENT,
        )
        .unwrap();
    evidence.save_rgba("parent_before", &parent_before, size_px);

    let mut child_text = Text::new(TEXT_SCALE);
    child_text.services = true;
    child_text.backing = BACKING;
    let mut child_host = host(
        "same actual original child Panel and glyph subtree",
        Some(Insets::default()),
        CHILD,
    );
    let child_scene = layout(&mut child_host, &mut child_text, 320.0, 144.0);
    let child_before = child_scene.clone();
    let child_scene_counts = counts(&child_scene);
    let child_ordinary = renderer
        .render_to_rgba8(
            &child_scene,
            &mut child_text.atlas,
            size_px,
            BACKING,
            TRANSPARENT,
        )
        .unwrap();
    evidence.save_rgba("child_ordinary", &child_ordinary, size_px);
    let coincident = parent_shape_in_child_coordinates(&parent_geometry);
    let child_primitive_count = child_scene.primitives().len();
    let result = renderer.render_to_rgba8_with_parent_boundary_proof(
        &child_scene,
        &mut child_text.atlas,
        size_px,
        BACKING,
        TRANSPARENT,
        Arc::<[ClipShape]>::from(vec![coincident]),
        BoundaryProofLimits::default(),
    );
    assert!(
        result.is_err(),
        "coincident actual parent must refuse; never crop the original child scene"
    );
    assert_eq!(
        child_scene, child_before,
        "refusal must leave the complete child Host Scene intact"
    );
    assert!(!parent_scene.primitives().is_empty());
    let parent_after = renderer
        .render_to_rgba8(
            &parent_scene,
            &mut parent_text.atlas,
            size_px,
            BACKING,
            TRANSPARENT,
        )
        .unwrap();
    assert_eq!(parent_after, parent_before);
    evidence.save_rgba("parent_after", &parent_after, size_px);
    evidence.detail(
        "geometry",
        geometry_report(&parent_geometry, coincident, None, size_px),
    );
    evidence.detail("parent_scene", parent_scene_counts);
    evidence.detail("child_scene_before_proof", child_scene_counts);
    evidence.detail(
        "refusal",
        serde_json::json!({
            "debug":format!("{:?}", result.as_ref().err().unwrap()),
            "whole_scene_unchanged":child_scene==child_before,
            "primitive_count_before":child_primitive_count,
            "primitive_count_after":child_scene.primitives().len(),
            "child_ordinary_alpha_covered_pixels":alpha_covered(&child_ordinary),
            "parent_output_unchanged":parent_after==parent_before,
            "proof_output_returned":false,
            "proof_rgba_bytes_returned":0,
            "ordinary_frame_rgba_bytes":child_ordinary.len()
        }),
    );
    evidence.finish();
}

#[test]
#[ignore = "explicit finite real GPU acceptance under the original build/GPU locks"]
fn actual_host_stack_button_field_coretext_and_bitmap_image_parent_proof_evidence() {
    complete_host_material_case(MaterialCase {
        name: "root_actual_host_internal_bitmap",
        backing: BACKING,
        padding: None,
        bitmap_alpha: 255,
    });
}

struct MaterialCase {
    name: &'static str,
    backing: f32,
    padding: Option<Insets>,
    bitmap_alpha: u8,
}

fn complete_host_material_case(case: MaterialCase) {
    let mut evidence = Evidence::new(case.name);
    let backing = case.backing;
    let bitmap_alpha = case.bitmap_alpha;
    let fractional_allocation_expected = case.padding.is_some();
    let extent = |logical: f32| {
        let pixels = logical * backing;
        assert!(pixels.is_finite() && pixels > 0.0 && pixels.fract() == 0.0);
        pixels as u32
    };
    let mut renderer = Renderer::new_headless().expect("actual Metal renderer required");

    let mut parent_text = Text::new(TEXT_SCALE);
    parent_text.services = true;
    parent_text.backing = backing;
    let mut parent_host = host("actual padded parent Panel", case.padding, PARENT);
    let parent_scene = layout(&mut parent_host, &mut parent_text, 360.0, 240.0);
    let parent_geometry = panel_geometry(&parent_host, PARENT);
    let parent_size_px = (extent(360.0), extent(240.0));
    let parent_before = renderer
        .render_to_rgba8(
            &parent_scene,
            &mut parent_text.atlas,
            parent_size_px,
            backing,
            TRANSPARENT,
        )
        .unwrap();
    evidence.save_rgba("parent_before", &parent_before, parent_size_px);

    let child_size_px = (
        extent(parent_geometry.allocation.size.width),
        extent(parent_geometry.allocation.size.height),
    );
    let child_width = parent_geometry.allocation.size.width;
    let child_height = parent_geometry.allocation.size.height;
    assert_eq!(
        [child_width, child_height],
        [328.0, 208.0],
        "matrix never shrinks the original child viewport"
    );
    if fractional_allocation_expected {
        assert!(
            (parent_geometry.allocation.origin.x * backing).fract() != 0.0
                || (parent_geometry.allocation.origin.y * backing).fract() != 0.0,
            "actual public allocation must retain a physical fractional origin"
        );
    }
    let bitmap_size_px = (32, 24);
    let bitmap_pixels: Vec<u8> = (0..bitmap_size_px.1)
        .flat_map(|y| {
            (0..bitmap_size_px.0).flat_map(move |x| {
                let rgb = if (x / 4 + y / 4) % 2 == 0 {
                    [240_u8, 48, 32]
                } else {
                    [24, 140, 224]
                };
                let premul = |c: u8| ((u16::from(c) * u16::from(bitmap_alpha)) / 255) as u8;
                [premul(rgb[0]), premul(rgb[1]), premul(rgb[2]), bitmap_alpha]
            })
        })
        .collect();
    let bitmap_lease = renderer
        .register_bitmap_image(&bitmap_pixels, bitmap_size_px.0, bitmap_size_px.1)
        .unwrap();
    evidence.save_rgba("internal_bitmap_source", &bitmap_pixels, bitmap_size_px);

    let field_id = ComponentId(8408);
    let image_id = ComponentId(8407);
    let button_id = ComponentId(8406);
    let stack_id = ComponentId(8405);
    let field = TextFieldContent::read_only_content(
        "备注",
        "真实 Host 树中的 CoreText glyphs — MWM 中文",
        tekes_ui_tokens::proposal::TextRole::UiBody,
    );
    let image = ImageContent::ready(
        bitmap_lease.clone(),
        Size::new(32.0 / BACKING, 24.0 / BACKING),
        ImageSemantics::Decorative,
    )
    .expect("the existing public ready bitmap content accepts this renderer lease");
    let stack = Content::new(
        stack_id,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 8.0,
            },
            children: vec![
                Content::new(button_id, Button::new("圆角操作按钮")).preserving_focus(),
                Content::new(image_id, image),
                Content::new(field_id, field),
            ],
            flexible_child: Some(field_id),
        },
    );
    let mut child_panel = PanelContent::new(stack).unwrap();
    child_panel.set_padding(Some(Insets::default())).unwrap();
    let mut child_host = ContentHost::new(Content::new(CHILD, child_panel)).unwrap();
    let mut child_text = Text::new(TEXT_SCALE);
    child_text.services = true;
    child_text.backing = backing;
    let mut child_scene = layout(&mut child_host, &mut child_text, child_width, child_height);
    let child_before = child_scene.clone();
    let parent_shape = parent_shape_in_child_coordinates(&parent_geometry);
    let safe = parent_shape
        .full_coverage_inner_rect_px(backing, child_size_px)
        .unwrap()
        .unwrap();
    let domain = safe.bounds_px();
    assert!(
        domain[0] <= 0.0
            && domain[1] <= 0.0
            && domain[2] >= child_size_px.0 as f32
            && domain[3] >= child_size_px.1 as f32,
        "matrix uses the existing owner's admitted full-child domain; no new predicate or input clipping"
    );
    let child_counts = counts(&child_scene);
    assert!(
        child_counts["glyph_instance_count"].as_u64().unwrap() > 0,
        "real Host tree must contain CoreText glyph instances"
    );
    assert!(
        child_counts["coretext_scene_glyph_instance_count"]
            .as_u64()
            .unwrap()
            > 0,
        "actual CoreText Shaper output must contain glyphs"
    );
    assert!(
        child_counts["image_primitive_count"].as_u64().unwrap() > 0,
        "real ImageContent must paint an internal bitmap primitive"
    );
    assert!(
        child_scene
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::ControlledTransformedImage { .. })),
        "registered bitmap must retain controlled lease identity"
    );

    let ordinary = renderer
        .render_to_rgba8(
            &child_scene,
            &mut child_text.atlas,
            child_size_px,
            backing,
            TRANSPARENT,
        )
        .unwrap();
    evidence.save_rgba("child_ordinary", &ordinary, child_size_px);
    assert!(
        alpha_covered(&ordinary) > 0,
        "real child Host output must contain nontransparent ink/image"
    );
    assert!(
        ordinary.iter().any(|byte| *byte != 0),
        "real child Host output must differ from the transparent clear frame"
    );
    let proof = renderer
        .render_to_rgba8_with_parent_boundary_proof(
            &child_scene,
            &mut child_text.atlas,
            child_size_px,
            backing,
            TRANSPARENT,
            Arc::<[ClipShape]>::from(vec![parent_shape]),
            BoundaryProofLimits::default(),
        )
        .expect("real parent allocation fully contains the complete child Host output");
    assert_eq!(proof.rgba8(), ordinary.as_slice());
    evidence.save_rgba("child_proof", proof.rgba8(), child_size_px);

    let mut table = ShapeTable::new(0x127_0003, 1);
    let key = table.intern(parent_shape).unwrap();
    let mut direct = Scene::with_clip_shapes(&table);
    direct.push_shape_clip(key);
    direct
        .try_append_fragment(
            &child_scene,
            FragmentBudget::default(),
            FragmentTransform::default(),
        )
        .unwrap();
    direct.pop_clip();
    let direct_pixels = renderer
        .render_to_rgba8(
            &direct,
            &mut child_text.atlas,
            child_size_px,
            backing,
            TRANSPARENT,
        )
        .unwrap();
    assert_eq!(direct_pixels, proof.rgba8());
    evidence.save_rgba("parent_masked_direct", &direct_pixels, child_size_px);
    let image_reference = draw_image_reference_at_scale(
        &mut renderer,
        parent_shape,
        proof.rgba8(),
        child_size_px,
        &mut child_text.atlas,
        backing,
    );
    assert_eq!(image_reference, direct_pixels);
    evidence.save_rgba("image_quad_reference", &image_reference, child_size_px);
    assert_eq!(
        child_scene, child_before,
        "proof path must not mutate the original complete child Host scene"
    );
    let child_scene_unchanged_before_explicit_mutation = child_scene == child_before;

    let parent_after = renderer
        .render_to_rgba8(
            &parent_scene,
            &mut parent_text.atlas,
            parent_size_px,
            backing,
            TRANSPARENT,
        )
        .unwrap();
    assert_eq!(parent_after, parent_before);
    evidence.save_rgba("parent_after", &parent_after, parent_size_px);
    child_scene.clear();
    child_text.atlas.clear();
    assert_eq!(
        proof.rgba8(),
        ordinary.as_slice(),
        "verified output remains immutable after source scene and atlas mutation"
    );

    evidence.detail(
        "geometry",
        geometry_report_at_scale(
            &parent_geometry,
            parent_shape,
            Some(safe.bounds_px()),
            child_size_px,
            backing,
        ),
    );
    evidence.detail("material_case", serde_json::json!({"backing":backing,"bitmap_alpha":bitmap_alpha,"bitmap_rgba_premultiplied":true,"bitmap_logical_size_fixed":[16.0,12.0],"whole_host_and_all_original_assertions_retained":true}));
    evidence.detail("parent_scene", counts(&parent_scene));
    evidence.detail("child_scene_before_mutation", child_counts);
    evidence.detail("proof_debug", serde_json::json!(format!("VerifiedFrameOutput {{ dimensions_px: {:?}, rgba8_len: {}, alpha_covered_pixels: {} }}", proof.dimensions_px(), proof.rgba8().len(), alpha_covered(proof.rgba8()))));
    evidence.detail("pixel_checks", serde_json::json!({
        "proof_equals_ordinary":proof.rgba8()==ordinary.as_slice(),
        "direct_equals_proof":direct_pixels==proof.rgba8(),
        "image_quad_equals_direct":image_reference==direct_pixels,
        "parent_after_equals_before":parent_after==parent_before,
        "ordinary_alpha_covered_pixels":alpha_covered(&ordinary),
        "ordinary_differs_from_transparent_clear":ordinary.iter().any(|byte|*byte!=0),
        "child_scene_exactly_unchanged_by_proof_before_explicit_mutation":child_scene_unchanged_before_explicit_mutation,
        "bitmap_lease_retained_through_renders":true,
        "bitmap_lease_renderer_identity":renderer.resource_identity()
    }));
    evidence.detail("internal_image_contract", serde_json::json!({
        "content":"public ImageContent::ready(registered BitmapImageLease)",
        "primitive":"ControlledTransformedImage",
        "semantics":"Decorative; alpha is not used to reduce proof scissor requirements",
        "lease_lifetime":"strong lease clone retained until after proof, direct and ImageQuad reference renders"
    }));
    evidence.finish();
    drop(bitmap_lease);
}

// Independent named cases let the locked runner collect all failures without
// changing the original Host, shrinking its viewport or touching Toolbar.

#[test]
#[ignore = "explicit original-domain coordinate/material GPU case under the original locks"]
fn actual_complete_host_matrix_quarter_half_alpha() {
    complete_host_material_case(MaterialCase {
        name: "main_quarter_backing_half",
        backing: 0.25,
        padding: None,
        bitmap_alpha: 128,
    });
}

#[test]
#[ignore = "explicit original-domain coordinate/material GPU case under the original locks"]
fn actual_complete_host_matrix_half_backing_opaque() {
    complete_host_material_case(MaterialCase {
        name: "main_fractional_backing_half_opaque",
        backing: 0.5,
        padding: None,
        bitmap_alpha: 255,
    });
}

#[test]
#[ignore = "explicit original-domain coordinate/material GPU case under the original locks"]
fn actual_complete_host_matrix_integer_1x_opaque() {
    complete_host_material_case(MaterialCase {
        name: "main_integer_1x_opaque",
        backing: 1.0,
        padding: None,
        bitmap_alpha: 255,
    });
}

#[test]
#[ignore = "explicit original-domain coordinate/material GPU case under the original locks"]
fn actual_complete_host_matrix_fractional_2x_half_alpha() {
    complete_host_material_case(MaterialCase {
        name: "main_fractional_2x_half",
        backing: 2.0,
        padding: Some(Insets::new(16.5, 15.75, 15.5, 16.25)),
        bitmap_alpha: 128,
    });
}

#[test]
#[ignore = "explicit original-domain coordinate/material GPU case under the original locks"]
fn actual_complete_host_matrix_fractional_4x_transparent() {
    complete_host_material_case(MaterialCase {
        name: "main_fractional_4x_transparent",
        backing: 4.0,
        padding: Some(Insets::new(16.375, 15.875, 15.625, 16.125)),
        bitmap_alpha: 0,
    });
}

const TOOLBAR_ROOT: ComponentId = ComponentId(12710);
const TOOLBAR_REGION: ComponentId = ComponentId(12711);
const TOOLBAR_GROUP: ComponentId = ComponentId(12712);
const TOOLBAR_BUTTON: ComponentId = ComponentId(12713);
const TOOLBAR_FIELD: ComponentId = ComponentId(12714);

fn actual_toolbar_scene(text: &mut Text, viewport: Rect, regions: Vec<ToolbarRegion>) -> Scene {
    let mut toolbar = ToolbarHost::<()>::new(TOOLBAR_ROOT, 0x127, viewport)
        .expect("public ToolbarHost constructor");
    toolbar
        .set_regions(0x127, viewport, regions)
        .expect("actual public window regions");
    toolbar
        .register(ToolbarRegistration {
            region: TOOLBAR_REGION,
            order: 0,
            group: Content::new(
                TOOLBAR_GROUP,
                ToolbarGroup::new(
                    ToolbarPlacement::Principal,
                    vec![
                        Content::new(TOOLBAR_BUTTON, Button::new("真实圆角按钮")),
                        Content::new(
                            TOOLBAR_FIELD,
                            TextFieldContent::read_only_content(
                                "搜索",
                                "真实 ToolbarHost CoreText — MWM 中文",
                                tekes_ui_tokens::proposal::TextRole::UiBody,
                            ),
                        ),
                    ],
                ),
            ),
            bindings: Vec::new(),
        })
        .expect("mount original toolbar group in original host");
    let mut scene = Scene::new();
    toolbar
        .render(&mut scene, text, Theme::LIGHT.into())
        .expect("original public ToolbarHost renders normally");
    scene
}

fn target_flat_quad_count(scene: &Scene, target: Rect) -> usize {
    scene
        .primitives()
        .iter()
        .filter(|primitive| match primitive {
            Primitive::Quad(q) => {
                q.rect == target
                    && q.corner_radii == tekes_ui_scene::CornerRadii::ZERO
                    && q.border_width == 0.0
            }
            _ => false,
        })
        .count()
}

#[test]
#[ignore = "explicit finite real GPU acceptance under the original build/GPU locks"]
fn actual_toolbar_host_full_target_background_uses_single_class_b_and_pixels_match_masked_reference()
 {
    let mut evidence = Evidence::new("actual_toolbar_host_class_b_positive");
    let mut renderer = Renderer::new_headless().expect("actual Metal renderer required");
    let target = Rect::new(0.0, 0.0, 320.0, 144.0);
    let target_px = (640, 288);

    // This is a real zero-padding Panel Host configuration. Its public geometry
    // supplies both the viewport and parent shape; the rounded safe rectangle
    // deliberately excludes the full-target ToolbarHost background.
    let mut parent_text = Text::new(TEXT_SCALE);
    parent_text.services = true;
    parent_text.backing = BACKING;
    let mut parent = host(
        "real zero-padding rounded Panel parent",
        Some(Insets::default()),
        PARENT,
    );
    let parent_scene = layout(
        &mut parent,
        &mut parent_text,
        target.size.width,
        target.size.height,
    );
    let geometry = panel_geometry(&parent, PARENT);
    assert_eq!(geometry.allocation, geometry.outer);
    let parent_pixels_before = renderer
        .render_to_rgba8(
            &parent_scene,
            &mut parent_text.atlas,
            target_px,
            BACKING,
            TRANSPARENT,
        )
        .unwrap();
    evidence.save_rgba("parent_before", &parent_pixels_before, target_px);
    let parent_shape = parent_shape_in_child_coordinates(&geometry);
    let safe = parent_shape
        .full_coverage_inner_rect_px(BACKING, target_px)
        .expect("finite actual Panel geometry")
        .expect("actual rounded Panel has a conservative safe rectangle");
    let [sx0, sy0, sx1, sy1] = safe.bounds_px();
    assert!(
        sx0 > 0.0 && sy0 > 0.0 && sx1 < target_px.0 as f32 && sy1 < target_px.1 as f32,
        "the parent safe rectangle must exclude the full-target background on every edge"
    );

    let mut text = Text::new(TEXT_SCALE);
    text.services = true;
    text.backing = BACKING;
    let scene = actual_toolbar_scene(
        &mut text,
        target,
        vec![ToolbarRegion::window(
            TOOLBAR_REGION,
            target,
            ToolbarWindowEdge::Leading,
        )],
    );
    let scene_before = scene.clone();
    let scene_counts = counts(&scene);
    assert!(
        scene_counts["glyph_instance_count"].as_u64().unwrap() > 0,
        "the actual toolbar descendants must emit CoreText glyphs"
    );
    assert_eq!(
        target_flat_quad_count(&scene, target),
        1,
        "Regions::paint must contribute exactly one target-aligned flat background Quad"
    );
    let ordinary = renderer
        .render_to_rgba8(&scene, &mut text.atlas, target_px, BACKING, TRANSPARENT)
        .expect("ordinary actual ToolbarHost render");
    evidence.save_rgba("toolbar_ordinary", &ordinary, target_px);

    let proof = renderer
        .render_to_rgba8_with_parent_boundary_proof(
            &scene,
            &mut text.atlas,
            target_px,
            BACKING,
            TRANSPARENT,
            Arc::<[ClipShape]>::from(vec![parent_shape]),
            BoundaryProofLimits::default(),
        )
        .expect("the only full-target drawable is the actual ToolbarHost flat background; children remain Class A");
    assert_eq!(proof.rgba8(), ordinary.as_slice());
    assert_eq!(
        scene, scene_before,
        "proof leaves the whole actual ToolbarHost scene unchanged"
    );
    evidence.save_rgba("toolbar_class_b_proof", proof.rgba8(), target_px);

    let mut shape_table = ShapeTable::new(0x127_0010, 1);
    let key = shape_table.intern(parent_shape).unwrap();
    let mut direct = Scene::with_clip_shapes(&shape_table);
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
        .render_to_rgba8(&direct, &mut text.atlas, target_px, BACKING, TRANSPARENT)
        .unwrap();
    evidence.save_rgba("toolbar_direct_parent_mask", &direct_pixels, target_px);
    let image_reference = draw_image_reference(
        &mut renderer,
        parent_shape,
        &ordinary,
        target_px,
        &mut text.atlas,
    );
    evidence.save_rgba(
        "toolbar_image_quad_parent_mask_reference",
        &image_reference,
        target_px,
    );
    assert_eq!(image_reference, direct_pixels);

    // Re-render the same actual parent Scene with its existing Text/atlas after
    // child proof and reference work; compare the returned RGBA buffers exactly.
    let parent_pixels_after = renderer
        .render_to_rgba8(
            &parent_scene,
            &mut parent_text.atlas,
            target_px,
            BACKING,
            TRANSPARENT,
        )
        .expect("re-render the unchanged actual parent Scene");
    assert_eq!(parent_pixels_after, parent_pixels_before);
    evidence.save_rgba("parent_after", &parent_pixels_after, target_px);

    evidence.detail(
        "actual_parent_geometry",
        geometry_report(&geometry, parent_shape, Some(safe.bounds_px()), target_px),
    );
    evidence.detail("actual_parent_scene", counts(&parent_scene));
    evidence.detail("actual_toolbar_scene", scene_counts);
    evidence.detail("b_candidate_source", serde_json::json!({
        "public_host":"ToolbarHost::new + set_regions + register + render",
        "background_source":"Regions::paint -> toolbar_background(region.available, Some(clip), ...)",
        "region_scope":"Window(Leading), actual full target allocation",
        "target_flat_quad_count":target_flat_quad_count(&scene, target),
        "full_target_is_outside_parent_safe_rect":true,
        "class_b_receipt_debug":format!("{proof:?}"),
        "other_drawables":"actual Button and TextField descendants, with actual CoreText glyphs"
    }));
    evidence.detail(
        "pixel_checks",
        serde_json::json!({
            "proof_equals_ordinary":proof.rgba8()==ordinary.as_slice(),
            "direct_parent_mask_equals_image_quad_reference":direct_pixels==image_reference,
            "parent_after_equals_before":parent_pixels_after==parent_pixels_before,
            "whole_scene_unchanged_by_proof":scene==scene_before,
            "ordinary_alpha_covered_pixels":alpha_covered(&ordinary),
            "ordinary_differs_from_clear":ordinary.iter().any(|b|*b!=0)
        }),
    );
    evidence.finish();
}

#[test]
#[ignore = "explicit finite real GPU acceptance under the original build/GPU locks"]
fn actual_toolbar_host_rejects_two_full_target_background_candidates() {
    let mut evidence = Evidence::new("actual_toolbar_host_multiple_b_refusal");
    let mut renderer = Renderer::new_headless().expect("actual Metal renderer required");
    let target = Rect::new(0.0, 0.0, 320.0, 144.0);
    let target_px = (640, 288);
    let mut parent_text = Text::new(TEXT_SCALE);
    parent_text.services = true;
    parent_text.backing = BACKING;
    let mut parent = host(
        "real zero-padding rounded Panel parent",
        Some(Insets::default()),
        PARENT,
    );
    let parent_scene = layout(
        &mut parent,
        &mut parent_text,
        target.size.width,
        target.size.height,
    );
    let geometry = panel_geometry(&parent, PARENT);
    let shape = parent_shape_in_child_coordinates(&geometry);
    let parent_safe = shape
        .full_coverage_inner_rect_px(BACKING, target_px)
        .unwrap();
    let parent_before = renderer
        .render_to_rgba8(
            &parent_scene,
            &mut parent_text.atlas,
            target_px,
            BACKING,
            TRANSPARENT,
        )
        .unwrap();
    evidence.save_rgba("parent_before", &parent_before, target_px);

    // Two ordinary public Window regions paint two actual target-sized flat
    // quads. This is a negative proof case, not a synthetic Scene primitive.
    let region_b = ComponentId(12715);
    let group_b = ComponentId(12716);
    let button_b = ComponentId(12717);
    let field_b = ComponentId(12718);
    let mut text = Text::new(TEXT_SCALE);
    text.services = true;
    text.backing = BACKING;
    let mut toolbar = ToolbarHost::<()>::new(TOOLBAR_ROOT, 0x128, target).unwrap();
    toolbar
        .set_regions(
            0x128,
            target,
            vec![
                ToolbarRegion::window(TOOLBAR_REGION, target, ToolbarWindowEdge::Leading),
                ToolbarRegion::window(region_b, target, ToolbarWindowEdge::Trailing),
            ],
        )
        .unwrap();
    for (region, group, button, field, title) in [
        (
            TOOLBAR_REGION,
            TOOLBAR_GROUP,
            TOOLBAR_BUTTON,
            TOOLBAR_FIELD,
            "第一组",
        ),
        (region_b, group_b, button_b, field_b, "第二组"),
    ] {
        toolbar
            .register(ToolbarRegistration {
                region,
                order: 0,
                group: Content::new(
                    group,
                    ToolbarGroup::new(
                        ToolbarPlacement::Principal,
                        vec![
                            Content::new(button, Button::new(title)),
                            Content::new(
                                field,
                                TextFieldContent::read_only_content(
                                    "搜索",
                                    "真实文字",
                                    tekes_ui_tokens::proposal::TextRole::UiBody,
                                ),
                            ),
                        ],
                    ),
                ),
                bindings: Vec::new(),
            })
            .unwrap();
    }
    let mut scene = Scene::new();
    toolbar
        .render(&mut scene, &mut text, Theme::LIGHT.into())
        .unwrap();
    let b_count = target_flat_quad_count(&scene, target);
    assert_eq!(
        b_count, 2,
        "both real Window regions retain their backgrounds"
    );
    let ordinary = renderer
        .render_to_rgba8(&scene, &mut text.atlas, target_px, BACKING, TRANSPARENT)
        .unwrap();
    evidence.save_rgba("two_toolbar_backgrounds_ordinary", &ordinary, target_px);
    let scene_before = scene.clone();
    let result = renderer.render_to_rgba8_with_parent_boundary_proof(
        &scene,
        &mut text.atlas,
        target_px,
        BACKING,
        TRANSPARENT,
        Arc::<[ClipShape]>::from(vec![shape]),
        BoundaryProofLimits::default(),
    );
    assert!(
        result.is_err(),
        "the API admits at most one Class B drawable"
    );
    let parent_after = renderer
        .render_to_rgba8(
            &parent_scene,
            &mut parent_text.atlas,
            target_px,
            BACKING,
            TRANSPARENT,
        )
        .unwrap();
    assert_eq!(parent_after, parent_before);
    evidence.save_rgba("parent_after", &parent_after, target_px);
    evidence.detail(
        "actual_parent_geometry",
        geometry_report(
            &geometry,
            shape,
            parent_safe.map(|r| r.bounds_px()),
            target_px,
        ),
    );
    evidence.detail("actual_parent_scene", counts(&parent_scene));
    evidence.detail("actual_scene", counts(&scene));
    evidence.detail(
        "refusal",
        serde_json::json!({
            "target_flat_quad_count":b_count,
            "error":format!("{:?}", result.err().unwrap()),
            "ordinary_alpha_covered_pixels":alpha_covered(&ordinary),
            "whole_scene_unchanged_by_refusal":scene==scene_before,
            "parent_after_equals_before":parent_after==parent_before
        }),
    );
    evidence.finish();
}

#[test]
#[ignore = "explicit finite real GPU acceptance under the original build/GPU locks"]
fn actual_toolbar_host_offset_and_narrow_regions_cannot_claim_full_target_background() {
    let mut evidence = Evidence::new("actual_toolbar_host_offset_narrow_refusal");
    let mut renderer = Renderer::new_headless().expect("actual Metal renderer required");
    let target = Rect::new(0.0, 0.0, 320.0, 144.0);
    let target_px = (640, 288);
    let mut parent_text = Text::new(TEXT_SCALE);
    parent_text.services = true;
    parent_text.backing = BACKING;
    let mut parent = host(
        "real zero-padding rounded Panel parent",
        Some(Insets::default()),
        PARENT,
    );
    let parent_scene = layout(
        &mut parent,
        &mut parent_text,
        target.size.width,
        target.size.height,
    );
    let geometry = panel_geometry(&parent, PARENT);
    let shape = parent_shape_in_child_coordinates(&geometry);
    let parent_safe = shape
        .full_coverage_inner_rect_px(BACKING, target_px)
        .unwrap();
    let parent_before = renderer
        .render_to_rgba8(
            &parent_scene,
            &mut parent_text.atlas,
            target_px,
            BACKING,
            TRANSPARENT,
        )
        .unwrap();
    evidence.save_rgba("parent_before", &parent_before, target_px);

    for (label, region_rect) in [
        ("offset", Rect::new(-8.0, 0.0, 240.0, 144.0)),
        ("narrow", Rect::new(0.0, 0.0, 240.0, 144.0)),
    ] {
        let mut text = Text::new(TEXT_SCALE);
        text.services = true;
        text.backing = BACKING;
        let scene = actual_toolbar_scene(
            &mut text,
            target,
            vec![ToolbarRegion::window(
                TOOLBAR_REGION,
                region_rect,
                ToolbarWindowEdge::Leading,
            )],
        );
        let scene_before = scene.clone();
        assert_eq!(
            target_flat_quad_count(&scene, target),
            0,
            "offset or narrow public region is not an exact full-target Class B quad"
        );
        let ordinary = renderer
            .render_to_rgba8(&scene, &mut text.atlas, target_px, BACKING, TRANSPARENT)
            .unwrap();
        evidence.save_rgba(&format!("{label}_ordinary"), &ordinary, target_px);
        let result = renderer.render_to_rgba8_with_parent_boundary_proof(
            &scene,
            &mut text.atlas,
            target_px,
            BACKING,
            TRANSPARENT,
            Arc::<[ClipShape]>::from(vec![shape]),
            BoundaryProofLimits::default(),
        );
        assert!(
            result.is_err(),
            "a public offset/narrow ToolbarRegion cannot be certified as the unique full-target background"
        );
        assert_eq!(
            scene, scene_before,
            "refusal preserves whole ToolbarHost scene"
        );
        evidence.detail(&format!("{label}_case"), serde_json::json!({
            "region_available_pt":[region_rect.origin.x, region_rect.origin.y, region_rect.size.width, region_rect.size.height],
            "actual_scene":counts(&scene),
            "full_target_flat_quad_count":target_flat_quad_count(&scene,target),
            "proof_refusal":format!("{:?}",result.err().unwrap()),
            "ordinary_alpha_covered_pixels":alpha_covered(&ordinary),
            "whole_scene_unchanged_by_refusal":scene==scene_before
        }));
    }
    let parent_after = renderer
        .render_to_rgba8(
            &parent_scene,
            &mut parent_text.atlas,
            target_px,
            BACKING,
            TRANSPARENT,
        )
        .unwrap();
    assert_eq!(parent_after, parent_before);
    evidence.save_rgba("parent_after", &parent_after, target_px);
    evidence.detail(
        "actual_parent_geometry",
        geometry_report(
            &geometry,
            shape,
            parent_safe.map(|r| r.bounds_px()),
            target_px,
        ),
    );
    evidence.detail("actual_parent_scene", counts(&parent_scene));
    evidence.detail(
        "parent_output_checks",
        serde_json::json!({"parent_after_equals_before":parent_after==parent_before}),
    );
    evidence.finish();
}

#[test]
#[ignore = "explicit finite real GPU acceptance under the original build/GPU locks"]
fn actual_toolbar_host_opaque_bitmap_image_does_not_inherit_class_b_exemption() {
    let mut evidence = Evidence::new("actual_toolbar_host_opaque_image_refusal");
    let mut renderer = Renderer::new_headless().expect("actual Metal renderer required");
    let target = Rect::new(0.0, 0.0, 320.0, 144.0);
    let target_px = (640, 288);
    let mut parent_text = Text::new(TEXT_SCALE);
    parent_text.services = true;
    parent_text.backing = BACKING;
    let mut parent = host(
        "real zero-padding rounded Panel parent",
        Some(Insets::default()),
        PARENT,
    );
    let parent_scene = layout(
        &mut parent,
        &mut parent_text,
        target.size.width,
        target.size.height,
    );
    let geometry = panel_geometry(&parent, PARENT);
    let shape = parent_shape_in_child_coordinates(&geometry);
    let parent_before = renderer
        .render_to_rgba8(
            &parent_scene,
            &mut parent_text.atlas,
            target_px,
            BACKING,
            TRANSPARENT,
        )
        .unwrap();
    evidence.save_rgba("parent_before", &parent_before, target_px);
    let safe = shape
        .full_coverage_inner_rect_px(BACKING, target_px)
        .unwrap()
        .unwrap();

    let bitmap_size = (48, 32);
    let bitmap: Vec<u8> = (0..bitmap_size.1)
        .flat_map(|y| {
            (0..bitmap_size.0).flat_map(move |x| {
                if (x / 4 + y / 4) % 2 == 0 {
                    [255, 255, 255, 255]
                } else {
                    [224, 248, 255, 255]
                }
            })
        })
        .collect();
    let lease = renderer
        .register_bitmap_image(&bitmap, bitmap_size.0, bitmap_size.1)
        .unwrap();
    evidence.save_rgba("opaque_image_source", &bitmap, bitmap_size);
    let image_id = ComponentId(12719);
    let group_id = ComponentId(12720);
    let field_id = ComponentId(12721);
    let button_id = ComponentId(12722);
    let mut text = Text::new(TEXT_SCALE);
    text.services = true;
    text.backing = BACKING;
    let mut toolbar = ToolbarHost::<()>::new(TOOLBAR_ROOT, 0x129, target).unwrap();
    toolbar
        .set_regions(
            0x129,
            target,
            vec![ToolbarRegion::window(
                TOOLBAR_REGION,
                target,
                ToolbarWindowEdge::Leading,
            )],
        )
        .unwrap();
    let image = ImageContent::ready(
        lease.clone(),
        Size::new(
            bitmap_size.0 as f32 / BACKING,
            bitmap_size.1 as f32 / BACKING,
        ),
        ImageSemantics::Decorative,
    )
    .unwrap();
    toolbar
        .register(ToolbarRegistration {
            region: TOOLBAR_REGION,
            order: 0,
            group: Content::new(
                group_id,
                ToolbarGroup::new(
                    ToolbarPlacement::Navigation,
                    vec![
                        Content::new(image_id, image),
                        Content::new(button_id, Button::new("边界按钮")),
                        Content::new(
                            field_id,
                            TextFieldContent::read_only_content(
                                "搜索",
                                "真实文字",
                                tekes_ui_tokens::proposal::TextRole::UiBody,
                            ),
                        ),
                    ],
                ),
            ),
            bindings: Vec::new(),
        })
        .unwrap();
    let mut scene = Scene::new();
    toolbar
        .render(&mut scene, &mut text, Theme::LIGHT.into())
        .unwrap();
    assert!(
        scene
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::ControlledTransformedImage { .. })),
        "actual registered opaque bitmap must remain an internal controlled image primitive"
    );
    assert_eq!(target_flat_quad_count(&scene, target), 1);
    let scene_before = scene.clone();
    let ordinary = renderer
        .render_to_rgba8(&scene, &mut text.atlas, target_px, BACKING, TRANSPARENT)
        .unwrap();
    evidence.save_rgba(
        "toolbar_with_opaque_internal_image_ordinary",
        &ordinary,
        target_px,
    );
    let image_quad = scene
        .primitives()
        .iter()
        .find_map(|p| match p {
            Primitive::ControlledTransformedImage { quad, .. } => Some(*quad),
            _ => None,
        })
        .unwrap();
    let image_rect = image_quad.bounds();
    let [safe_x0, safe_y0, safe_x1, safe_y1] = safe.bounds_px();
    let image_px = [
        image_rect.min_x() * BACKING,
        image_rect.min_y() * BACKING,
        image_rect.max_x() * BACKING,
        image_rect.max_y() * BACKING,
    ];
    let touches_boundary_band = image_px[0] < safe_x0
        || image_px[1] < safe_y0
        || image_px[2] > safe_x1
        || image_px[3] > safe_y1;
    let result = renderer.render_to_rgba8_with_parent_boundary_proof(
        &scene,
        &mut text.atlas,
        target_px,
        BACKING,
        TRANSPARENT,
        Arc::<[ClipShape]>::from(vec![shape]),
        BoundaryProofLimits::default(),
    );
    assert!(
        result.is_err(),
        "opaque internal image is not granted the unique background's Class B exception"
    );
    assert_eq!(
        scene, scene_before,
        "refusal preserves whole ToolbarHost scene"
    );
    let parent_after = renderer
        .render_to_rgba8(
            &parent_scene,
            &mut parent_text.atlas,
            target_px,
            BACKING,
            TRANSPARENT,
        )
        .unwrap();
    assert_eq!(parent_after, parent_before);
    evidence.save_rgba("parent_after", &parent_after, target_px);
    evidence.detail(
        "actual_parent_geometry",
        geometry_report(&geometry, shape, Some(safe.bounds_px()), target_px),
    );
    evidence.detail("actual_parent_scene", counts(&parent_scene));
    evidence.detail("actual_toolbar_scene", counts(&scene));
    evidence.detail(
        "parent_output_checks",
        serde_json::json!({"parent_after_equals_before":parent_after==parent_before}),
    );
    evidence.detail("opaque_image_case", serde_json::json!({
        "actual_transformed_image_rect_pt":[image_rect.origin.x,image_rect.origin.y,image_rect.size.width,image_rect.size.height],
        "actual_transformed_image_bounds_px":image_px,
        "parent_safe_rect_px":[safe_x0,safe_y0,safe_x1,safe_y1],
        "touches_parent_boundary_band":touches_boundary_band,
        "strong_bitmap_lease_retained_through_proof":true,
        "lease_renderer_identity":renderer.resource_identity(),
        "proof_refusal":format!("{:?}",result.err().unwrap()),
        "ordinary_alpha_covered_pixels":alpha_covered(&ordinary),
        "ordinary_differs_from_clear":ordinary.iter().any(|b|*b!=0),
        "whole_scene_unchanged_by_refusal":scene==scene_before,
        "parent_after_equals_before":parent_after==parent_before
    }));
    evidence.finish();
    drop(lease);
}
