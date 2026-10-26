//! Consumer-owned app. Public crates only; no Gallery imports or runtime JSON input.
use std::{fs, path::PathBuf};
use tekes_ui_controls::{
    Button, ButtonSize, ButtonTextAlign, ButtonTextRenderer, ControlState, paint,
};
use tekes_ui_render::Renderer;
use tekes_ui_scene::{GlyphAtlas, Point, Primitive, Rect, Scene, Size};
use tekes_ui_text::{
    coretext::{CoreTextFonts, CoreTextRasterizer, CoreTextShaper},
    model::{Paragraph, ShapeOptions},
    push::push_paragraph_visible,
    style::TextStyle,
    traits::Shaper,
};
use tekes_ui_tokens::{Palette, Rgba, Theme, proposal};
struct Text {
    shaper: CoreTextShaper,
    rasterizer: CoreTextRasterizer,
    atlas: GlyphAtlas,
    scale: f32,
}
impl Text {
    fn shape(&mut self, text: &str, role: proposal::TextRole, width: f32) -> Paragraph {
        let style = role.style();
        self.shaper.shape(
            text,
            &TextStyle::system(style.size).with_weight(style.weight),
            &ShapeOptions {
                width: Some(f64::from(width)),
                line_spacing: 0.,
            },
        )
    }
}
impl ButtonTextRenderer for Text {
    fn measure(&mut self, text: &str, role: proposal::TextRole, width: f32) -> Size {
        let p = self.shape(text, role, width);
        Size::new(p.width as f32, p.height as f32)
    }
    fn backing_scale(&self) -> f32 {
        self.scale
    }
    fn draw(
        &mut self,
        scene: &mut Scene,
        text: &str,
        role: proposal::TextRole,
        rect: Rect,
        align: ButtonTextAlign,
        ink: Rgba,
    ) {
        let p = self.shape(text, role, rect.size.width);
        let x = rect.min_x()
            + if align == ButtonTextAlign::Center {
                (rect.size.width - p.width as f32) * 0.5
            } else {
                0.
            };
        let y = rect.min_y() + (rect.size.height - p.height as f32) * 0.5;
        push_paragraph_visible(
            scene,
            &mut self.atlas,
            &mut self.rasterizer,
            &p,
            Point::new(x.round(), y.round()),
            self.scale,
            ink,
            rect.min_y()..rect.max_y(),
        );
    }
}
fn rgba(c: Rgba) -> [f32; 4] {
    [c.r, c.g, c.b, c.a]
}
fn main() {
    let mut args = std::env::args_os().skip(1);
    let first = args.next().expect("[--check] output directory required");
    let check_only = first == "--check";
    let output = PathBuf::from(if check_only {
        args.next().expect("output directory required")
    } else {
        first
    });
    fs::create_dir_all(&output).unwrap();
    let mut renderer = (!check_only).then(|| Renderer::new_headless().expect("real GPU required"));
    let fonts = CoreTextFonts::new();
    let mut text = Text {
        shaper: CoreTextShaper::new(fonts.clone()),
        rasterizer: CoreTextRasterizer::new(fonts),
        atlas: GlyphAtlas::default(),
        scale: 1.,
    };
    let mut themes = Vec::new();
    for (theme, name) in Theme::ALL
        .into_iter()
        .zip(["light", "dark", "light-ic", "dark-ic"])
    {
        let palette = Palette::from(theme);
        assert!(
            Palette::new(theme, usize::MAX)
                .and_then(|palette| palette.color("primary-text"))
                .is_none()
        );
        assert!(palette.color("unknown-consumer-role").is_none());
        let roles = [
            "action-primary-ground",
            "action-primary-hover",
            "action-primary-pressed",
            "action-primary-foreground",
            "primary-text",
            "secondary-text",
            "border-focused",
            "text-selection",
            "row-selection",
            "status-failed",
            "diff-added-surface",
            "separator",
        ];
        let values: serde_json::Map<String, serde_json::Value> = roles
            .into_iter()
            .map(|role| {
                (
                    role.into(),
                    serde_json::json!(rgba(
                        palette
                            .color(role)
                            .unwrap_or_else(|| panic!("missing role {role}"))
                    )),
                )
            })
            .collect();
        themes.push(serde_json::json!({"theme":name,"roles":values}));
        for scale in [1_u32, 2] {
            text.scale = scale as f32;
            let mut scene = Scene::new();
            let bounds = Rect::new(0., 0., 800., 320.);
            paint::surface(&mut scene, bounds, palette, false);
            text.draw(
                &mut scene,
                &format!("{} / {name}", proposal::BRAND_ID),
                proposal::TextRole::UiBody,
                Rect::new(24., 16., 750., 36.),
                ButtonTextAlign::Leading,
                paint::color(palette, "primary-text"),
            );
            for (row, variant) in [
                paint::ButtonVariant::Primary,
                paint::ButtonVariant::Quiet,
                paint::ButtonVariant::Panel,
            ]
            .into_iter()
            .enumerate()
            {
                for (column, label) in ["Rest", "Hover", "Pressed", "Disabled"]
                    .into_iter()
                    .enumerate()
                {
                    let mut button = Button::default();
                    button.state = ControlState {
                        hovered: column == 1,
                        pressed: column == 2,
                        disabled: column == 3,
                        ..ControlState::default()
                    };
                    let start = scene.len();
                    button.render(
                        &mut scene,
                        &mut text,
                        label,
                        Rect::new(
                            24. + column as f32 * 194.,
                            72. + row as f32 * 70.,
                            164.,
                            40.,
                        ),
                        Some(bounds),
                        palette,
                        variant,
                        ButtonSize::Body,
                        proposal::TextRole::UiBody,
                        ButtonTextAlign::Center,
                    );
                    if row == 0 && column < 3 {
                        let role = [
                            "action-primary-ground",
                            "action-primary-hover",
                            "action-primary-pressed",
                        ][column];
                        let fill = palette.color(role).unwrap();
                        assert!(
                            scene.primitives()[start..]
                                .iter()
                                .any(|p| matches!(p, Primitive::Quad(q) if q.fill == fill)),
                            "real Button must paint the compiled consumer state {role}"
                        );
                    }
                }
            }
            if let Some(renderer) = &mut renderer {
                let bytes = renderer
                    .render_to_rgba8(
                        &scene,
                        &mut text.atlas,
                        (800 * scale, 320 * scale),
                        scale as f32,
                        paint::color(palette, "surface-0"),
                    )
                    .unwrap();
                let mut png = png::Encoder::new(
                    fs::File::create(output.join(format!("{name}@{scale}x.png"))).unwrap(),
                    800 * scale,
                    320 * scale,
                );
                png.set_color(png::ColorType::Rgba);
                png.set_depth(png::BitDepth::Eight);
                png.write_header()
                    .unwrap()
                    .write_image_data(&bytes)
                    .unwrap();
            }
        }
    }
    fs::write(output.join("result.json"),serde_json::to_vec_pretty(&serde_json::json!({"brand":proposal::BRAND_ID,"seed":proposal::BRAND_SEED_SRGB,"version":proposal::BRAND_VERSION,"themes":themes,"actual_gpu":!check_only,"native":false})).unwrap()).unwrap();
}
