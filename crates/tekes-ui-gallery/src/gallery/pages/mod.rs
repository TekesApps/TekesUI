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

pub mod primitives;
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
}

impl Page {
    pub const ALL: [Page; 3] = [Page::Primitives, Page::Tokens, Page::StaticText];

    /// 菜单和窗口标题里的名字。
    pub fn name(self) -> &'static str {
        match self {
            Page::Primitives => "Primitives",
            Page::Tokens => "Token Swatches",
            Page::StaticText => "静态文字",
        }
    }

    /// 快照文件名里的名字。
    pub fn slug(self) -> &'static str {
        match self {
            Page::Primitives => "primitives",
            Page::Tokens => "tokens",
            Page::StaticText => "static-text",
        }
    }

    pub fn index(self) -> usize {
        Page::ALL.iter().position(|p| *p == self).unwrap_or(0)
    }

    /// 编译这一页。`content` 是页面内容区（见 `layout::PAGE_CONTENT`）。
    pub fn build(self, ctx: &mut PageCtx<'_>, content: Rect, scene: &mut Scene) {
        match self {
            Page::Primitives => primitives::build(ctx, content, scene),
            Page::Tokens => tokens::build(ctx, content, scene),
            Page::StaticText => static_text::build(ctx, content, scene),
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
                assert_ne!(a.name(), b.name());
                assert_ne!(a.slug(), b.slug());
            }
        }
    }
}
