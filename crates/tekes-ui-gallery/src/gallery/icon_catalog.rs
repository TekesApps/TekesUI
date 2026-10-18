#[path = "proposal_assets.rs"]
mod proposal_assets;
// Read-only legacy catalogue plus embedded design proposals.
use resvg::{tiny_skia, usvg};
use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use tekes_ui_render::Renderer;
use tekes_ui_scene::ImageId;
use tekes_ui_tokens::{ColorRole, Theme};

pub const MAX_SVG_BYTES: usize = 256 * 1024;
const MAX_SVG_TAGS: usize = 8192;
/// Shared by rasterization and layout so token changes cannot change only one side.
pub fn preview_sizes() -> [f32; 3] {
    ["size-icon-small", "size-icon-standard", "size-icon-preview"]
        .map(|name| tekes_ui_tokens::proposal::metric("size", name).expect("icon size token"))
}

pub const RETIRED: [&str; 5] = [
    "tekes-annotation",
    "tekes-computer-use",
    "tekes-perm-full-access",
    "tekes-perm-read-only",
    "tekes-perm-workspace-write",
];

#[derive(Debug)]
pub struct Entry {
    pub label: String,
    pub kind: String,
    pub before: Option<Box<Entry>>,
    pub name: String,
    tree: usvg::Tree,
    pub images: [Option<ImageId>; 3],
}
#[derive(Debug, Default)]
pub struct IconCatalog {
    pub entries: Vec<Entry>,
    pub notices: Vec<String>,
    pub source: Option<PathBuf>,
    prepared: Option<(Theme, u32)>,
}
impl IconCatalog {
    pub fn from_env() -> Self {
        let mut catalog = Self::load_local();
        catalog.add_proposals();
        catalog
    }
    fn load_local() -> Self {
        if let Some(path) = std::env::var_os("TEKESUI_ICON_DIR") {
            return Self::load(Path::new(&path));
        }
        Self::load(&Self::shared_assets_dir())
    }
    fn shared_assets_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../tekes-ui-icons/assets")
    }

    pub fn load(dir: &Path) -> Self {
        let mut catalog = Self {
            source: Some(dir.to_owned()),
            ..Self::default()
        };
        let files = match std::fs::read_dir(dir) {
            Ok(files) => files,
            Err(error) => {
                catalog.notices.push(format!("图标目录无法读取：{error}"));
                return catalog;
            }
        };
        let mut paths: Vec<_> = files.filter_map(Result::ok).map(|e| e.path()).collect();
        paths.sort();
        let mut seen = HashSet::new();
        for path in paths {
            let Some(name) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            if RETIRED.contains(&name) {
                continue;
            }
            // A flat export may also contain scaled copies; the canonical SVG wins.
            if let Some(base) = name
                .strip_suffix("-2x")
                .or_else(|| name.strip_suffix("-3x"))
                && dir.join(format!("{base}.svg")).is_file()
            {
                continue;
            }
            if seen.contains(name) {
                continue;
            }
            let file = if path.extension().is_some_and(|s| s == "imageset") {
                path.join(format!("{name}.svg"))
            } else if path.extension().is_some_and(|s| s == "svg") {
                path.clone()
            } else {
                continue;
            };
            // Cap the actual read too, rather than trusting a potentially stale metadata size.
            let result = std::fs::File::open(&file)
                .map_err(|e| e.to_string())
                .and_then(|file| {
                    let mut bytes = Vec::new();
                    file.take((MAX_SVG_BYTES + 1) as u64)
                        .read_to_end(&mut bytes)
                        .map_err(|e| e.to_string())?;
                    if bytes.len() > MAX_SVG_BYTES {
                        Err("SVG 超过 256 KiB 本地预览预算".into())
                    } else {
                        String::from_utf8(bytes).map_err(|e| e.to_string())
                    }
                })
                .and_then(|svg| {
                    if svg.bytes().filter(|b| *b == b'<').count() > MAX_SVG_TAGS {
                        return Err("SVG 元素超过本地预览复杂度预算".into());
                    }
                    if svg.to_ascii_lowercase().contains("c2pa") {
                        return Err("已作废的出处元数据".into());
                    }
                    // No filesystem/network image resolver or font loader is enabled.
                    let options = usvg::Options {
                        image_href_resolver: usvg::ImageHrefResolver {
                            resolve_data: Box::new(|_, _, _| None),
                            resolve_string: Box::new(|_, _| None),
                        },
                        ..usvg::Options::default()
                    };
                    usvg::Tree::from_str(&svg, &options).map_err(|e| e.to_string())
                });
            match result {
                Ok(tree) => {
                    seen.insert(name.to_owned());
                    catalog.entries.push(Entry {
                        label: String::new(),
                        kind: String::new(),
                        before: None,
                        name: name.into(),
                        tree,
                        images: [None; 3],
                    });
                }
                Err(error) => catalog.notices.push(format!("{name}: {error}")),
            }
        }
        catalog
    }
    /// Published artwork is self-contained; an explicit external directory is only for comparisons.
    pub fn add_proposals(&mut self) {
        if self.entries.iter().any(|e| !e.kind.is_empty()) {
            return;
        }
        let shared_source = self
            .source
            .as_ref()
            .is_some_and(|p| *p == Self::shared_assets_dir());
        for &(name, label, kind, svg) in proposal_assets::ASSETS {
            let tree = usvg::Tree::from_str(svg, &usvg::Options::default())
                .expect("validated embedded proposal SVG");
            let before = self
                .entries
                .iter()
                .position(|e| e.name == name)
                .map(|i| Box::new(self.entries.remove(i)));
            // Shared assets already contain the current artwork, not a historical "before".
            let before = if shared_source { None } else { before };
            self.entries.push(Entry {
                name: name.into(),
                label: label.into(),
                kind: kind.into(),
                before,
                tree,
                images: [None; 3],
            });
        }
        self.entries.sort_by(|a, b| {
            let rank = |e: &Entry| {
                if e.kind == "refine" {
                    0
                } else if !e.kind.is_empty() {
                    1
                } else {
                    2
                }
            };
            rank(a).cmp(&rank(b)).then(a.name.cmp(&b.name))
        });
    }
    #[cfg(test)]
    pub(crate) fn fixture(names: &[&str]) -> Self {
        let mut catalog = Self::default();
        for name in names {
            catalog.entries.push(Entry {
                label: String::new(), kind: String::new(), before: None,
                name: (*name).into(),
                tree: usvg::Tree::from_str(r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24"><rect x="4" y="4" width="16" height="16"/></svg>"#, &usvg::Options::default()).unwrap(),
                images: [None; 3],
            });
        }
        catalog
    }
    /// Render just the visible legacy tiles; keep bounded GPU images until theme/scale changes.
    pub fn prepare(
        &mut self,
        renderer: &mut Renderer,
        theme: Theme,
        scale: f32,
        visible: &[usize],
    ) {
        // Native backing scales are small and positive; reject invalid callers before allocation.
        if !(scale.is_finite() && 0.0 < scale && scale <= 8.0) {
            return;
        }
        let key = (theme, (scale * 1000.0).round() as u32);
        if self.prepared != Some(key) {
            for entry in &mut self.entries {
                for image in entry
                    .images
                    .iter_mut()
                    .chain(entry.before.iter_mut().flat_map(|b| b.images.iter_mut()))
                {
                    if let Some(id) = image.take() {
                        renderer.unregister_image(id);
                    }
                }
            }
            self.prepared = Some(key);
        }
        let color = ColorRole::PrimaryText.resolve(theme);
        let ink = [color.r, color.g, color.b, color.a];
        for &index in visible {
            let Some(entry) = self.entries.get_mut(index) else {
                continue;
            };
            let Entry {
                tree,
                images,
                before,
                ..
            } = entry;
            for (tree, images) in std::iter::once((&*tree, images))
                .chain(before.iter_mut().map(|b| (&b.tree, &mut b.images)))
            {
                for (slot, size) in preview_sizes().into_iter().enumerate() {
                    if images[slot].is_some() {
                        continue;
                    }
                    let pixels = (size * scale).round().max(1.0) as u32;
                    let Some(mut pixmap) = tiny_skia::Pixmap::new(pixels, pixels) else {
                        continue;
                    };
                    resvg::render(
                        tree,
                        tiny_skia::Transform::from_scale(
                            pixels as f32 / tree.size().width(),
                            pixels as f32 / tree.size().height(),
                        ),
                        &mut pixmap.as_mut(),
                    );
                    for rgba in pixmap.data_mut().chunks_exact_mut(4) {
                        let alpha = f32::from(rgba[3]) / 255.0 * ink[3];
                        for i in 0..3 {
                            rgba[i] = (ink[i] * alpha * 255.0).round() as u8;
                        }
                        rgba[3] = (alpha * 255.0).round() as u8;
                    }
                    match renderer.register_image(pixmap.data(), pixels, pixels) {
                        Ok(id) => images[slot] = Some(id),
                        Err(error) => eprintln!("icon {}: {error}", entry.name),
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_catalog_is_complete_without_an_application_directory() {
        let mut catalog = IconCatalog::load(&IconCatalog::shared_assets_dir());
        assert_eq!(catalog.entries.len(), tekes_ui_icons::Glyph::ALL.len());
        assert!(catalog.notices.is_empty());
        catalog.add_proposals();
        assert_eq!(catalog.entries.len(), 176);
        assert!(catalog.entries.iter().all(|entry| entry.before.is_none()));
    }
    #[test]
    fn approved_defaults_preserve_the_original_comparison_after_merging() {
        let names: Vec<_> = tekes_ui_icons::Icon::ALL
            .iter()
            .map(|icon| icon.name())
            .collect();
        let mut catalog = IconCatalog::fixture(&names);
        catalog.add_proposals();
        for icon in tekes_ui_icons::Icon::ALL {
            let entry = catalog
                .entries
                .iter()
                .find(|e| e.name == icon.name())
                .unwrap();
            let embedded = proposal_assets::ASSETS
                .iter()
                .find(|e| e.0 == icon.name())
                .unwrap();
            assert_eq!(
                embedded.3,
                icon.svg(),
                "Gallery must use the shared default"
            );
            assert_eq!(entry.before.as_ref().unwrap().name, icon.name());
            assert_eq!(entry.kind, "refine");
        }
        assert_eq!(catalog.entries.len(), 51);
        assert_eq!(
            catalog
                .entries
                .iter()
                .filter(|e| e.before.is_some())
                .count(),
            8
        );
    }

    #[test]
    fn proposals_merge_once_preserve_old_variants_and_work_without_local_assets() {
        let mut catalog = IconCatalog::fixture(&["tekes-settings", "tekes-memory", "tekes-search"]);
        catalog.add_proposals();
        assert_eq!(catalog.entries.len(), 52);
        assert_eq!(
            catalog
                .entries
                .iter()
                .filter(|e| e.before.is_some())
                .count(),
            2
        );
        let names: HashSet<_> = catalog.entries.iter().map(|e| &e.name).collect();
        assert_eq!(names.len(), catalog.entries.len());
        catalog.add_proposals();
        assert_eq!(catalog.entries.len(), 52);
        let mut embedded = IconCatalog::default();
        embedded.add_proposals();
        assert_eq!(embedded.entries.len(), 51);
        let mut renderer = Renderer::new_headless().unwrap();
        let visible: Vec<_> = (0..catalog.entries.len()).collect();
        catalog.prepare(&mut renderer, Theme::ALL[0], 1.0, &visible);
        assert_eq!(renderer.image_count(), 54 * 3);
        catalog.prepare(&mut renderer, Theme::ALL[1], 2.0, &visible);
        assert_eq!(renderer.image_count(), 54 * 3);
        assert!(
            catalog
                .entries
                .iter()
                .filter_map(|e| e.before.as_ref())
                .all(|e| e.images.iter().all(Option::is_some))
        );
    }
    const SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path d="M2 2L22 22" stroke="#000" stroke-width="1.5"/></svg>"##;
    #[test]
    fn loading_deduplicates_scales_excludes_retired_and_reports_errors() {
        let dir = std::env::temp_dir().join(format!("tekesui-icons-test-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("tekes-search.imageset")).unwrap();
        for name in [
            "tekes-search.svg",
            "tekes-search-2x.svg",
            "tekes-search-3x.svg",
        ] {
            std::fs::write(dir.join("tekes-search.imageset").join(name), SVG).unwrap();
        }
        std::fs::write(dir.join("tekes-annotation.svg"), SVG).unwrap();
        std::fs::write(dir.join("bad.svg"), "invalid").unwrap();
        let catalog = IconCatalog::load(&dir);
        assert_eq!(catalog.entries.len(), 1);
        assert_eq!(catalog.entries[0].name, "tekes-search");
        assert_eq!(catalog.notices.len(), 1);
        assert!(catalog.notices[0].starts_with("bad:"));
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn gpu_images_are_reused_and_released_across_themes_and_scales() {
        let Some(mut renderer) = Renderer::new_headless().ok() else {
            return;
        };
        let mut catalog = IconCatalog::default();
        catalog.entries.push(Entry {
            label: String::new(),
            kind: String::new(),
            before: None,
            name: "fixture".into(),
            tree: usvg::Tree::from_str(SVG, &usvg::Options::default()).unwrap(),
            images: [None; 3],
        });
        for scale in [1.0, 2.0] {
            for theme in Theme::ALL {
                catalog.prepare(&mut renderer, theme, scale, &[0]);
                assert_eq!(renderer.image_count(), 3);
                let images = catalog.entries[0].images;
                catalog.prepare(&mut renderer, theme, scale, &[0]);
                assert_eq!(images, catalog.entries[0].images);
                assert_eq!(renderer.image_count(), 3);
            }
        }
    }
}

#[cfg(test)]
mod color_review {
    use super::*;
    use tekes_ui_scene::{GlyphAtlas, ImageQuad, Rect, Scene};
    #[test]
    fn svg_tint_matches_the_json_srgb_token_without_a_second_transfer_function() {
        let mut renderer = Renderer::new_headless().expect("Metal adapter for icon pixel review");
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24"><rect width="24" height="24" fill="#000"/></svg>"##;
        let mut icons = IconCatalog::default();
        icons.entries.push(Entry {
            label: String::new(),
            kind: String::new(),
            before: None,
            name: "opaque-fixture".into(),
            tree: usvg::Tree::from_str(svg, &usvg::Options::default()).unwrap(),
            images: [None; 3],
        });
        for theme in Theme::ALL {
            for scale in [1.0, 2.0] {
                icons.prepare(&mut renderer, theme, scale, &[0]);
                let mut scene = Scene::new();
                scene.push_image(ImageQuad {
                    rect: Rect::new(0.0, 0.0, 24.0, 24.0),
                    image: icons.entries[0].images[2].unwrap(),
                    opacity: 1.0,
                });
                let width = (24.0 * scale) as u32;
                let rgba = renderer
                    .render_to_rgba8(
                        &scene,
                        &mut GlyphAtlas::default(),
                        (width, width),
                        scale,
                        ColorRole::DocumentCanvas.resolve(theme),
                    )
                    .unwrap();
                let token = ColorRole::PrimaryText.resolve(theme);
                let expected = [token.r, token.g, token.b].map(|c| (c * 255.0).round() as u8);
                let offset = ((width / 2 * width + width / 2) * 4) as usize;
                for (got, want) in rgba[offset..offset + 3].iter().zip(expected) {
                    assert!(
                        got.abs_diff(want) <= 1,
                        "{theme:?} {scale}x token color {expected:?}, got {:?}",
                        &rgba[offset..offset + 4]
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod source_boundaries {
    use super::*;
    struct FixtureDir(PathBuf);
    impl FixtureDir {
        fn new() -> Self {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "tekesui-icons-boundary-{}-{nonce}",
                std::process::id()
            ));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
        fn put(&self, name: &str, bytes: impl AsRef<[u8]>) {
            std::fs::write(self.0.join(name), bytes).unwrap();
        }
    }
    impl Drop for FixtureDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    const SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24"><rect width="24" height="24"/></svg>"#;
    #[test]
    fn missing_source_has_a_notice_and_flat_scale_copies_do_not_become_items() {
        let dir = FixtureDir::new();
        assert_eq!(IconCatalog::load(&dir.0.join("missing")).notices.len(), 1);
        for name in [
            "tekes-search.svg",
            "tekes-search-2x.svg",
            "tekes-search-3x.svg",
        ] {
            dir.put(name, SVG);
        }
        std::fs::create_dir(dir.0.join("tekes-search.imageset")).unwrap();
        std::fs::write(dir.0.join("tekes-search.imageset/tekes-search.svg"), SVG).unwrap();
        let catalog = IconCatalog::load(&dir.0);
        assert_eq!(catalog.entries.len(), 1);
        assert!(catalog.notices.is_empty());
        assert_eq!(catalog.entries[0].name, "tekes-search");
    }
    #[test]
    fn byte_tag_encoding_and_c2pa_budgets_reject_instead_of_showing_blank_icons() {
        let dir = FixtureDir::new();
        dir.put("oversize.svg", vec![b' '; MAX_SVG_BYTES + 1]);
        dir.put(
            "too-many-tags.svg",
            format!(
                "<svg xmlns=\"http://www.w3.org/2000/svg\">{}</svg>",
                "<path/>".repeat(MAX_SVG_TAGS)
            ),
        );
        dir.put("encoding.svg", [0xff, 0xfe]);
        dir.put("c2pa.svg", "<svg><metadata>C2PA</metadata></svg>");
        let catalog = IconCatalog::load(&dir.0);
        assert!(catalog.entries.is_empty());
        assert_eq!(catalog.notices.len(), 4);
        assert!(catalog.notices.iter().any(|n| n.contains("256 KiB")));
        assert!(catalog.notices.iter().any(|n| n.contains("复杂度")));
        assert!(catalog.notices.iter().any(|n| n.contains("出处")));
    }
    #[test]
    fn invalid_scales_do_not_allocate_images_or_replace_a_valid_cache() {
        let mut renderer = Renderer::new_headless().unwrap();
        let mut catalog = IconCatalog::fixture(&["fixture"]);
        for scale in [f32::NAN, f32::INFINITY, -1.0, 0.0, 9.0] {
            catalog.prepare(&mut renderer, Theme::LIGHT, scale, &[0]);
            assert_eq!(renderer.image_count(), 0);
        }
        catalog.prepare(&mut renderer, Theme::LIGHT, 2.0, &[0, usize::MAX]);
        assert_eq!(renderer.image_count(), 3);
        let ids = catalog.entries[0].images;
        catalog.prepare(&mut renderer, Theme::LIGHT, f32::NAN, &[0]);
        assert_eq!(catalog.entries[0].images, ids);
        assert_eq!(renderer.image_count(), 3);
        assert_eq!(preview_sizes(), [12.0, 16.0, 24.0]);
    }
}
