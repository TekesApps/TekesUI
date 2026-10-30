//! Ordinary labels and the original Button own all presentation and actions.
use tekes_ui_controls::{
    composition::*,
    extended::{Axis, Stack},
};
use tekes_ui_text::{LabelOptions, LabelOverflow};
use tekes_ui_tokens::Spacing;
pub const SOURCE: &str = "工具👩🏽‍💻 · 中文 e\u{301}    +12 −3";
pub const LABEL: ComponentId = ComponentId(2501);
pub const ACTION: ComponentId = ComponentId(2502);
pub const CUSTOM: ComponentId = ComponentId(2503);
pub const DECORATION: ComponentId = ComponentId(2504);
pub const COMPOSER: ComponentId = ComponentId(2505);
pub fn model(revision: u64) -> SemanticLabel {
    SemanticLabel::new(
        LabelSourceVersion {
            source: 25,
            revision,
        },
        SOURCE,
        vec![
            LabelForegroundRun {
                utf16: 9..17,
                foreground: LabelForeground::Secondary,
            },
            LabelForegroundRun {
                utf16: 21..24,
                foreground: LabelForeground::Positive,
            },
            LabelForegroundRun {
                utf16: 25..27,
                foreground: LabelForeground::Negative,
            },
        ],
    )
    .unwrap()
}
pub fn ellipsis_model(revision: u64) -> SemanticLabel {
    let mut text = String::new();
    let mut runs = Vec::new();
    for index in 0..6 {
        if index != 0 {
            text.push_str(" | ");
        }
        let offset = text.encode_utf16().count();
        text.push_str(SOURCE);
        runs.extend(model(revision).runs().iter().map(|run| LabelForegroundRun {
            utf16: (run.utf16.start + offset)..(run.utf16.end + offset),
            foreground: run.foreground,
        }));
    }
    SemanticLabel::new(
        LabelSourceVersion {
            source: 2525,
            revision,
        },
        text,
        runs,
    )
    .unwrap()
}
/// Coverage assertion over the actual retained native label result. The omitted
/// source must cross at least two distinct semantic foreground roles; no consumer
/// estimates, hardcoded cut, independently shaped fragments or weaker fallback.
pub fn assert_cross_run_ellipsis(label: &SemanticLabel, layout: &tekes_ui_text::LabelLayout) {
    assert_eq!(layout.source(), label.text());
    assert!(layout.is_truncated());
    assert_eq!(layout.paragraph().lines.len(), 1);
    assert_eq!(layout.elided_utf16().len(), 1);
    let omitted = &layout.elided_utf16()[0];
    assert_eq!(omitted.end, label.text().encode_utf16().count());
    assert!(omitted.start > 0 && omitted.start < omitted.end);
    let mut roles = Vec::new();
    for run in label.runs() {
        if run.utf16.start < omitted.end
            && omitted.start < run.utf16.end
            && !roles.contains(&run.foreground)
        {
            roles.push(run.foreground);
        }
    }
    assert!(
        roles.len() >= 2,
        "omission must span different foreground runs"
    );
    let glyphs: Vec<_> = layout
        .paragraph()
        .lines
        .iter()
        .flat_map(|l| &l.runs)
        .flat_map(|r| &r.glyphs)
        .collect();
    assert_eq!(
        glyphs.iter().filter(|g| g.cluster_utf16.is_empty()).count(),
        1
    );
    for g in glyphs {
        if g.cluster_utf16.is_empty() {
            assert_eq!(g.utf16_index, omitted.start);
        } else {
            assert!(
                g.cluster_utf16.end <= omitted.start,
                "hidden source glyph retained"
            );
            for unit in g.cluster_utf16.clone() {
                assert_eq!(
                    label.foreground_at(unit),
                    label.foreground_at(g.utf16_index)
                );
            }
        }
    }
    // Verify the native cut did not bisect a surrogate or extended grapheme.
    SemanticLabel::new(
        label.version(),
        label.text(),
        vec![LabelForegroundRun {
            utf16: omitted.clone(),
            foreground: LabelForeground::Secondary,
        }],
    )
    .expect("native ellipsis must cut at a legal source grapheme boundary");
}
pub fn build() -> Result<ContentHost, Box<dyn std::error::Error>> {
    build_label(model(1))
}
pub fn build_ellipsis() -> Result<ContentHost, Box<dyn std::error::Error>> {
    build_label(ellipsis_model(1))
}
fn build_label(label: SemanticLabel) -> Result<ContentHost, Box<dyn std::error::Error>> {
    let options = LabelOptions::single_line(LabelOverflow::TailEllipsis);
    let children = vec![
        Content::new(LABEL, TextContent::new(label.text()))
            .with_semantic_label(label.clone(), options)?,
        Content::new(ACTION, Button::new(label.text()))
            .with_semantic_label(label.clone(), options)?,
        Content::new(
            CUSTOM,
            Button::new(
                Content::new(DECORATION, TextContent::new(label.text()))
                    .with_semantic_label(label.clone(), options)?,
            )
            .with_accessible_name(label.text()),
        ),
        Content::new(
            COMPOSER,
            TextFieldContent::new("Composer", "Composer draft 中文"),
        ),
    ];
    Ok(ContentHost::new(Content::new(
        ComponentId(2500),
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: Spacing::Regular.value(),
            },
            flexible_child: None,
            children,
        },
    ))?)
}
