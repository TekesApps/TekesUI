//! A neutral native module consumer. Each factory creates process-local state.
mod text;
use anyhow::Result;
use tekes_ui_app_protocol::{InputKind, Request, ToolbarItem};
use tekes_ui_controls::{
    Button, ButtonSize, ButtonTextAlign,
    composition::{Component, ContentEvent, TextContent, TextFieldContent},
    paint::ButtonVariant,
};
use tekes_ui_module_runtime::{ModuleDefinition, ModuleState, PanelModule};
use tekes_ui_scene::{GlyphAtlas, Point, Quad, Rect, Scene};
use tekes_ui_tokens::{Palette, Theme, proposal};
use text::{Fonts, TextRenderer};
const CHINESE: &str = "中文长文 · 城市的一天\n\n清晨，街角的面包店打开了门。骑车的人从树荫下经过，河边有人读书，也有人停下来听风。窗户慢慢变窄时，句子自然换行；窗户变宽时，文字重新铺开。\n\n“标点会不会落在不合适的位置？”她问。我们观察逗号、句号、引号与括号（包括这一段说明），也观察连续数字：2026 年 10 月 21 日，上午 09:30。\n\n段落之间留着停顿。短句。长一点的句子则描述远处的山、近处的树，以及每一扇窗里不同的生活。\n\n";
const ENGLISH: &str = "English prose · A small observatory\n\nAt sunset, the observatory opens its wooden doors. A visitor places a notebook beside the telescope and writes down the first stars. Resizing this column should change the line breaks while preserving the words and paragraph boundaries.\n\nShort sentence. A much longer sentence follows, with punctuation, parentheses (and a nested thought), numbers such as 123,456.78, and words like extraordinary, internationalization, and counterclockwise.\n\nThe next morning, nothing about the document has changed except the space available to read it.\n\n";
const MIXED: &str = "混排 / Unicode · 开发笔记\n\nTekesUI 的 Text 组件处理中文、English words 与数字 12345。版本 v0.1.0，路径 /projects/notes/readme.md，邮件 hello@example.com。\n\nEmoji：😀 🌍 🚀 👨‍👩‍👧‍👦 👍🏽；组合字符：café / cafe\u{301}；日本語：今日は良い天気です。한국어: 반갑습니다.\n\nRTL: العربية مرحبًا بالعالم — עברית שלום עולם\n\n长连续内容用于观察边界处理：abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ。最后回到普通中文，检查字体回退和基线是否自然。\n\n";
struct Demo {
    identity: u32,
    button: Button,
    count: u64,
    fonts: Fonts,
    body: TextFieldContent,
    layout_ms: f64,
}
impl Demo {
    fn new(identity: u32) -> Self {
        let passage = [CHINESE, ENGLISH, MIXED][identity as usize].repeat(4);
        Self {
            identity,
            button: Button::default(),
            count: 0,
            fonts: Fonts::new(),
            body: TextFieldContent::read_only_content(
                "Demo document",
                &passage,
                proposal::TextRole::UiBody,
            ),
            layout_ms: 0.,
        }
    }
}
impl PanelModule for Demo {
    fn update(
        &mut self,
        r: &Request,
        scene: &mut Scene,
        atlas: &mut GlyphAtlas,
    ) -> Result<ModuleState> {
        let point = Point::new(r.x, r.y);
        match r.kind {
            InputKind::PointerDown => {
                self.button.pointer_down_at(point);
                self.body.event(&ContentEvent::PointerDown(point));
            }
            InputKind::PointerUp => {
                if self.button.pointer_up_at(point) {
                    self.count += 1;
                }
                self.body.event(&ContentEvent::PointerUp(point));
            }
            InputKind::Command if r.command == Some(1) => {
                self.count += 1;
            }
            InputKind::Command if r.command == Some(2) => {
                self.count = 0;
            }
            InputKind::Cancel => {
                self.button.cancel();
                self.body.event(&ContentEvent::Cancel);
            }
            InputKind::PointerMove => {
                self.button.pointer_move_at(point);
                self.body.event(&ContentEvent::PointerMove(point));
            }
            _ => {}
        }
        let (width, height) = (r.width as f32, r.height as f32);
        let palette = Palette::from(Theme::default());
        scene.push_quad(Quad::new(
            Rect::new(0., 0., width, height),
            palette.color("surface-panel").unwrap(),
        ));
        let mut text = TextRenderer {
            fonts: &mut self.fonts,
            atlas,
            scale: r.scale,
        };
        let body_rect = Rect::new(24., 120., (width - 48.).max(1.), (height - 144.).max(1.));
        let previous = self.body.surface.paragraph.clone();
        let started = std::time::Instant::now();
        self.body.arrange(&mut text, body_rect, body_rect);
        if previous
            .as_ref()
            .zip(self.body.surface.paragraph.as_ref())
            .is_none_or(|(a, b)| !std::rc::Rc::ptr_eq(a, b))
        {
            self.layout_ms = started.elapsed().as_secs_f64() * 1000.;
        }
        self.body
            .paint(scene, &mut text, body_rect, body_rect, palette.into());
        self.button.render(
            scene,
            &mut text,
            &format!("计数 {}", self.count),
            Rect::new(24., 16., (width - 48.).clamp(40., 240.), 36.),
            None,
            Theme::default(),
            ButtonVariant::Primary,
            ButtonSize::Body,
            proposal::TextRole::UiBody,
            ButtonTextAlign::Center,
        );
        let lines = self
            .body
            .surface
            .paragraph
            .as_ref()
            .map_or(0, |p| p.lines.len());
        let mut status = TextContent::new(format!(
            "{} · {:.0} pt · {} 行\n组件排版 {:.2} ms",
            ["中文长文", "English prose", "Unicode 混排"][self.identity as usize],
            width,
            lines,
            self.layout_ms
        ));
        let status_rect = Rect::new(24., 60., (width - 48.).max(1.), 52.);
        status.paint(scene, &mut text, status_rect, status_rect, palette.into());
        Ok(ModuleState {
            revision: self.count,
            toolbar: vec![
                ToolbarItem {
                    id: 1,
                    title: format!("Increment ({})", self.count),
                    enabled: true,
                    geometry_dependent: false,
                },
                ToolbarItem {
                    id: 2,
                    title: "Reset count".into(),
                    enabled: self.count > 0,
                    geometry_dependent: false,
                },
            ],
        })
    }
}
fn first() -> Box<dyn PanelModule> {
    Box::new(Demo::new(0))
}
fn second() -> Box<dyn PanelModule> {
    Box::new(Demo::new(1))
}
fn third() -> Box<dyn PanelModule> {
    Box::new(Demo::new(2))
}
/// IDs identify distinct build-selected modules, not physical slot positions.
pub fn definitions() -> Vec<ModuleDefinition> {
    vec![
        ModuleDefinition {
            id: "demo-first",
            create: first,
        },
        ModuleDefinition {
            id: "demo-second",
            create: second,
        },
        ModuleDefinition {
            id: "demo-third",
            create: third,
        },
    ]
}
