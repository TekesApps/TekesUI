//! 页面。每页是一个 `build(ctx, content, scene)`：给定上下文（主题、文字引擎、静态文字页状态）
//! 和页面内容区，往 scene 里追加图元。
//!
//! 页面只依赖 token、scene、文字引擎和 `containers`，不碰窗口和输入；同一页同一主题、同一状态
//! 永远编出同一份图元列表（确定性，快照和测试都靠它）。
//!
//! 新增一页的步骤见 docs/gallery.md。

use tekes_ui_scene::{Rect, Scene};
use tekes_ui_tokens::Theme;

use super::static_text::StaticText;
use super::text::TextEngine;

pub mod icons;
pub mod motion;
pub mod primitives;
pub mod showcase;
pub mod static_text;
pub mod tokens;

/// 页面编图元时用到的上下文。
#[derive(Debug)]
pub struct PageCtx<'a> {
    pub theme: Theme,
    pub text: &'a mut TextEngine,
    pub static_text: &'a mut StaticText,
}

/// Gallery 的页面。目录里的顺序就是 `ALL` 的顺序，⌘1、⌘2 … 也按这个顺序。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Page {
    Primitives,
    Tokens,
    StaticText,
    Foundations,
    Actions,
    Forms,
    Navigation,
    Overlays,
    Feedback,
    Documents,
    Data,
    TextEditing,
    Icons,
    Text,
    ScrollArea,
    Motion,
    Workbench(usize),
}

impl Page {
    pub const ALL: [Page; include!(concat!(env!("OUT_DIR"), "/workbench_count.rs"))] =
        include!(concat!(env!("OUT_DIR"), "/workbench_pages.rs"));

    pub const fn category(self) -> Option<&'static str> {
        match self {
            Self::Foundations => Some("foundations"),
            Self::Actions => Some("actions"),
            Self::Forms => Some("forms"),
            Self::Navigation => Some("navigation"),
            Self::Overlays => Some("overlays"),
            Self::Feedback => Some("feedback"),
            Self::Documents => Some("documents"),
            Self::Data => Some("data"),
            _ => None,
        }
    }

    /// 菜单和窗口标题里的名字。
    pub fn name(self) -> &'static str {
        match self {
            Page::Workbench(i) => super::workbench::entry(i).title(),
            Page::Icons => "Icons",
            Page::Text => "Text",
            Page::Motion => "Motion",
            Page::ScrollArea => "ScrollArea",
            Page::Primitives => "Primitives",
            Page::Tokens => "Token Swatches",
            Page::StaticText => "静态文字",
            Page::TextEditing => "TextEditor",
            page => &super::catalog::group(page.category().unwrap()).name,
        }
    }

    /// 快照文件名里的名字。
    pub fn slug(self) -> &'static str {
        match self {
            Page::Workbench(i) => super::workbench::entry(i).slug(),
            Page::Icons => "icons",
            Page::Text => "text",
            Page::Motion => super::motion_preview::SLUG,
            Page::ScrollArea => "scroll-area",
            Page::Primitives => "primitives",
            Page::Tokens => "tokens",
            Page::StaticText => "static-text",
            Page::Foundations => "components-foundations",
            Page::Actions => "components-actions",
            Page::Forms => "components-forms",
            Page::Navigation => "components-navigation",
            Page::Overlays => "components-overlays",
            Page::Feedback => "components-feedback",
            Page::Documents => "components-documents",
            Page::Data => "components-data",
            Page::TextEditing => "text-editing",
        }
    }

    pub fn index(self) -> usize {
        Page::ALL.iter().position(|p| *p == self).unwrap_or(0)
    }

    /// 编译这一页。`content` 是页面内容区（见 `layout::PAGE_CONTENT`）。
    pub fn build(self, ctx: &mut PageCtx<'_>, content: Rect, scene: &mut Scene) {
        match self {
            Page::Workbench(i) => {
                super::workbench::Workbench::fixture().build(i, ctx, content, scene)
            }
            Page::Icons => {
                super::text_demo::TextDemo::search_fixture().build_search(
                    ctx,
                    icons::search_rect(content),
                    scene,
                );
                icons::build(ctx, content, scene, "");
            }
            Page::Motion => {
                let mut state = super::motion_preview::MotionPreview::default();
                motion::build(ctx, content, content, scene, &mut state)
            }
            Page::TextEditing => super::text_demo::build_editors_fixture(ctx, content, scene),
            Page::Text => super::text_demo::build_text_fixture(ctx, content, scene),
            Page::ScrollArea => super::text_demo::build_scroll_fixture(ctx, content, scene),
            Page::Primitives => primitives::build(ctx, content, scene),
            Page::Tokens => tokens::build(ctx, content, scene),
            Page::StaticText => static_text::build(ctx, content, scene),
            page => showcase::build(
                page.category().unwrap(),
                ctx,
                content,
                scene,
                &super::catalog::ShowcaseState::default(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_matches_all_order() {
        for (i, p) in Page::ALL.iter().enumerate() {
            assert_eq!(p.index(), i);
        }
    }

    #[test]
    fn names_and_slugs_are_unique() {
        for (i, a) in Page::ALL.iter().enumerate() {
            for b in &Page::ALL[i + 1..] {
                assert_ne!(a.slug(), b.slug());
            }
        }
    }
}
