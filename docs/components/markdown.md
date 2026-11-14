# Markdown

Markdown::parse 提供有界块解析，MarkdownView 是旧直接绘制入口；需要连续选择、复制和来源映射时使用 RichDocument::from_markdown 与 RichContent。

## 短用法

```rust
use tekes_ui_controls::composition::{RichContent, RichDocument};
let document = RichDocument::from_markdown("# Title\n\nA short paragraph.");
let content = RichContent::new("manual", 1, document).unwrap();
```

## 文档状态和事件

支持标题、段落、项目符号与 fenced code 的有界子集，并保留实现支持的链接映射。`RichDocument::text/blocks/links` 读取解析结果；`display_to_source` 与 `source_to_display` 使用原 UTF-16 范围映射，不按 UTF-8 字节偏移猜选区。

RichContent 的 `document()`、`version()` 只读；`update(expected, revision, document)` 要求同一 identity 和递增版本，旧版本错误不能当作更新成功。文档切换需要新的组件。`drain_link_actions()` 交付捕获的 target/document/revision/source range，由消费者决定如何打开，解析链接不自动联网。

父 ScrollContent 负责正文滚动，原文字服务负责选区与复制。没有完整 CommonMark、HTML 浏览器、远程图片或任意脚本支持。Gallery Documents 可能使用更窄的纯文字投影；不能把底层 RichContent 能力直接写成该面板已接入的功能。

## 源码与消费者

定义：`crates/tekes-ui-controls/src/composition/rich_document.rs`。现有消费路径：`examples/composition-consumer/src/rich_content.rs`。构造片段不包含完整窗口接入；本批只核对源码，未编译片段或重跑消费者，不据此声称原生输入、视觉或无障碍验收通过。
