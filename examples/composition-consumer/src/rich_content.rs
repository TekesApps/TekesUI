//! A fixed public consumer: one continuous rich message, code and diff, beside
//! an independent original composer. All shaping/mapping/selection stays in SDK.
use tekes_ui_controls::{
    ReadingDocumentVersion, ScrollArea,
    composition::*,
    extended::{Axis, CodeBlock, DiffView, Stack},
};
pub const ROOT: ComponentId = ComponentId(25000);
pub const READER: ComponentId = ComponentId(25001);
pub const LIST: ComponentId = ComponentId(25002);
pub const MARKDOWN: ComponentId = ComponentId(25003);
pub const CODE: ComponentId = ComponentId(25004);
pub const DIFF: ComponentId = ComponentId(25005);
pub const COMPOSER: ComponentId = ComponentId(25006);
pub const EARLIER: ComponentId = ComponentId(25007);
pub const SOURCE: &str = "# Release 中文😀\n\n正文 e\u{301} 与 👩🏽‍💻，连续消息，不复制文本布局。 Visit [documentation](https://example.com/docs).\n\n- 第一项 😀\n- Second item\n\n```rust\nlet greeting = \"中文😀\";\nprintln!(\"{greeting}\");\n```";
pub fn build() -> Result<ContentHost, CompositionError> {
    let docs = [
        (MARKDOWN, "message-1", RichDocument::from_markdown(SOURCE)),
        (
            CODE,
            "source-1",
            RichDocument::from_code(&CodeBlock {
                language: "rust".into(),
                source: "fn main() {\n    println!(\"中文😀 e\u{301}\");\n}".into(),
            }),
        ),
        (
            DIFF,
            "diff-1",
            RichDocument::from_diff(&DiffView::between(
                "旧行\nretain 😀\nlast e\u{301}",
                "新行\nretain 😀\nlast e\u{301}\nadded",
            )),
        ),
    ];
    let children = docs
        .into_iter()
        .map(|(id, key, document)| Content::new(id, RichContent::new(key, 1, document).unwrap()))
        .collect();
    let mut host = ContentHost::new(Content::new(
        ROOT,
        StackContent {
            model: Stack {
                axis: Axis::Vertical,
                gap: 12.,
            },
            flexible_child: Some(READER),
            children: vec![
                Content::new(
                    READER,
                    ScrollContent::new(
                        ScrollArea::default(),
                        Content::new(LIST, ListContent::new("Rich history", children)),
                    ),
                ),
                Content::new(
                    COMPOSER,
                    TextFieldContent::new("Independent draft", "Untouched 中文😀"),
                ),
            ],
        },
    ))?;
    host.set_scroll_reading_document(
        READER,
        "mixed-rich-history",
        1,
        vec![MARKDOWN, CODE, DIFF],
        |_| (),
    )
    .unwrap();
    // This fixed reader opens at its heading; live tail following is an
    // explicit existing ScrollContent intent, independent of document updates.
    host.get_mut(READER)
        .unwrap()
        .component_mut::<ScrollContent>()
        .unwrap()
        .stop_following();
    host.set_action_events(true);
    Ok(host)
}
pub fn rich(host: &ContentHost, id: ComponentId) -> &RichContent {
    host.get(id).unwrap().component::<RichContent>().unwrap()
}
pub fn rich_mut(host: &mut ContentHost, id: ComponentId) -> &mut RichContent {
    host.get_mut(id)
        .unwrap()
        .component_mut::<RichContent>()
        .unwrap()
}
pub fn reading_version(host: &ContentHost) -> ReadingDocumentVersion {
    host.get(READER)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .reading_document_version()
        .unwrap()
        .clone()
}
/// The consumer supplies semantic facts; the existing reading transaction owns
/// viewport retention, not a guessed pixel delta for the replaced message.
pub fn append(host: &mut ContentHost) -> RichContentVersion {
    let expected = rich(host, MARKDOWN).version();
    let source = format!(
        "{}\n\nStreaming tail 中文😀",
        rich(host, MARKDOWN).document().source()
    );
    let document = RichDocument::from_markdown(&source);
    let version = reading_version(host);
    let items = host
        .get(READER)
        .unwrap()
        .component::<ScrollContent>()
        .unwrap()
        .reading_items()
        .to_vec();
    let (_, updated) = host
        .update_scroll_reading_document(READER, &version, version.revision() + 1, items, |host| {
            rich_mut(host, MARKDOWN)
                .update(&expected, expected.revision() + 1, document)
                .unwrap()
        })
        .unwrap();
    updated
}
pub fn prepend(host: &mut ContentHost) {
    let version = reading_version(host);
    host.update_scroll_reading_document(
        READER,
        &version,
        version.revision() + 1,
        vec![EARLIER, MARKDOWN, CODE, DIFF],
        |host| {
            host.insert(
                LIST,
                Content::new(EARLIER, TextContent::new("Earlier mixed item 中文😀")),
            )
            .unwrap();
            host.reorder(LIST, &[EARLIER, MARKDOWN, CODE, DIFF])
                .unwrap();
        },
    )
    .unwrap();
}
