# CodeBlock

CodeBlock 保存 language 和 source；CodeBlockView 是轻量只读绘制入口，选择和复制可复用 RichDocument::from_code + RichContent。

## 短用法

```rust
use tekes_ui_controls::{composition::{RichContent, RichDocument}, extended::CodeBlock};
let code = CodeBlock { language: "rust".into(), source: "let count = 1;".into() };
let content = RichContent::new("source-1", 1, RichDocument::from_code(&code)).unwrap();
```

## 状态和文档职责

`CodeBlock::lines()` 读取源代码行，language 是元信息，不会启动解释器或语法服务。RichContent 的 document/version 与 expected-version update 保留同一文档更新合同；更新代码后选区映射仍由原文字引擎管理。

代码正文是只读，不提供运行、编辑、断点或编译动作。复制和选区来自原 Host 文本路径，想要额外复制按钮应显式组合正式 Button 并处理业务动作，不假设 CodeBlockView 自带工具栏。

外层 ScrollContent 分配视口，组件不自动创建横向窗口或远程语法高亮服务。language 标签不代表已实现完整 syntax highlighting；复杂编辑需求应选择专门编辑器合同。

## 源码与消费者

定义：`crates/tekes-ui-controls/src/extended.rs`。现有消费路径：`examples/composition-consumer/src/rich_content.rs`。构造片段不包含完整窗口接入；本批只核对源码，未编译片段或重跑消费者，不据此声称原生输入、视觉或无障碍验收通过。
