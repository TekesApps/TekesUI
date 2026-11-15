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

父 ScrollContent 负责正文滚动，原文字服务负责选区与复制。没有完整 CommonMark、HTML 浏览器、远程图片或任意脚本支持。Gallery Documents 候选已接入 RichContent，并在输入完成后消费链接队列；当前仅打开离线打包的手册链接，外部链接显示未支持提示。原生 Cmd-Enter 路由仍需单列验收，不能由组件自身支持该按键推定 AppHost 已接通。

## 源码与消费者

定义：`crates/tekes-ui-controls/src/composition/rich_document.rs`。现有消费路径：`examples/composition-consumer/src/rich_content.rs`。构造片段不包含完整窗口接入；完整消费者运行及原生输入、视觉、无障碍验收仍需独立证据。

当前片段验证：Check329 已对这 45 份手册中的 46 段原文 Rust 代码完成 `--emit=metadata` 类型检查，通过但未链接、未运行；本文代码块保持不变，复用该结果。此证据不代替 GUI、物理 IME、VoiceOver 或运行时验收。收据位于协调工作区 `outputs/gallery-apphost-upgrade-20261114/manual-snippets/RESULT.json`，提取源为同目录 `manual-snippets.rs`。
