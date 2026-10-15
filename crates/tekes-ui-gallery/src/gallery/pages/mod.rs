//! 页面。每页是一个 `build(theme, content, scene)`：给定主题和页面内容区，往 scene 里追加图元。
//!
//! 页面只依赖 token、scene 和 `containers`，不碰窗口和输入；同一页同一主题永远编出同一份图元列表
//! （确定性，快照和测试都靠它）。
//!
//! 新增一页的步骤见 docs/gallery.md。

use tekes_ui_scene::{Rect, Scene};
use tekes_ui_tokens::Theme;

pub mod primitives;
pub mod tokens;

/// Gallery 的页面。目录里的顺序就是 `ALL` 的顺序，⌘1、⌘2 … 也按这个顺序。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Page {
    Primitives,
    Tokens,
}

impl Page {
    pub const ALL: [Page; 2] = [Page::Primitives, Page::Tokens];

    /// 菜单和窗口标题里的名字。
    pub fn name(self) -> &'static str {
        match self {
            Page::Primitives => "Primitives",
            Page::Tokens => "Token Swatches",
        }
    }

    /// 快照文件名里的名字。
    pub fn slug(self) -> &'static str {
        match self {
            Page::Primitives => "primitives",
            Page::Tokens => "tokens",
        }
    }

    pub fn index(self) -> usize {
        Page::ALL.iter().position(|p| *p == self).unwrap_or(0)
    }

    /// 编译这一页。`content` 是页面内容区（见 `layout::PAGE_CONTENT`）。
    pub fn build(self, theme: Theme, content: Rect, scene: &mut Scene) {
        match self {
            Page::Primitives => primitives::build(theme, content, scene),
            Page::Tokens => tokens::build(theme, content, scene),
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
