//! Read-only native query data from the original TextSurface, never an editor model.
use crate::TextSurface;
use std::ops::Range;
use tekes_ui_scene::Rect;

#[derive(Clone, Debug, PartialEq)]
pub struct ContentTextInput {
    /// Opaque original editor/document binding. Geometry changes do not change it.
    pub binding: Vec<u8>,
    pub text: String,
    pub caret_index: usize,
    pub selection: Range<usize>,
    pub marked: Option<Range<usize>>,
    pub read_only: bool,
    /// Original shaped geometry in host logical coordinates. None means the
    /// geometry is unavailable or clipped, not that the text owner is absent.
    pub caret: Option<Rect>,
    pub clip: Option<Rect>,
    pub carets: Vec<(usize, Rect)>,
}

impl ContentTextInput {
    pub(crate) fn local_binding(surface: &TextSurface) -> Vec<u8> {
        let (editor, document) = surface.input.controlled_binding();
        [editor, document]
            .into_iter()
            .flat_map(u64::to_le_bytes)
            .collect()
    }

    pub fn from_surface(surface: &TextSurface) -> Self {
        let clip = surface
            .scroll
            .geometry(surface.inner, surface.ancestor_clip())
            .visible_viewport_rect();
        let visible = |rect: Rect| {
            clip.and_then(|c| c.intersection(&rect))
                .filter(|r| r.size.height > 0.)
        };
        let origin = surface.origin();
        let carets = surface.paragraph.as_ref().map_or_else(Vec::new, |p| {
            p.lines
                .iter()
                .flat_map(|line| {
                    line.carets.iter().filter_map(|(index, x)| {
                        let rect = Rect::new(
                            origin.x + *x as f32,
                            origin.y + line.top as f32,
                            tekes_ui_tokens::Stroke::Rule.value(),
                            line.height as f32,
                        );
                        visible(rect).map(|rect| (*index, rect))
                    })
                })
                .collect()
        });
        Self {
            binding: Self::local_binding(surface),
            text: surface.input.text.clone(),
            caret_index: surface.input.caret,
            selection: surface.input.selection(),
            marked: surface.input.marked.clone(),
            read_only: surface.input.read_only,
            caret: visible(surface.caret()),
            clip,
            carets,
        }
    }
}
