# Text

静态标签用 TextContent；可选择、可复制的正文用 TextFieldContent::read_only_content；TextSurface 是共享的文字编辑与绘制基础。三种用途不能仅凭视觉相似互换。

## 短用法

```rust
use tekes_ui_controls::composition::*;
use tekes_ui_tokens::proposal::TextRole;
let label = TextContent::new("Status");
let paragraph = TextFieldContent::read_only_content("Article", "Selectable text", TextRole::UiBody);
```

## 状态、输入和排版

TextContent 的公开 `text` 和 `role` 设置文字及排版角色，提供静态显示，不因此获得编辑选区。read_only_content 使用 Content presentation，不显示普通输入框轮廓，保留原生文本选择和复制服务；父级负责正文滚动。需要富块和来源映射时使用 RichDocument 与 RichContent。

TextSurface 中的 input、角色、presentation 和滚动属于实际实例。外部正文更新走版本化发布，不能直接改文字再假设选区和 IME 几何仍然有效。静态标签通常不占编辑焦点，阅读内容的焦点和选择由 Host 处理。

动画数字另有既有手册 `docs/components/ticking-number.md`；它不是 Text 的通用数字输入能力，也不是此 catalog 新增的组件 ID。

## 源码与消费者

定义：`crates/tekes-ui-controls/src/composition/adapters.rs`。现有消费路径：`examples/composition-consumer/src/plain_text.rs`。上面的构造片段不是独立窗口程序；本批只核对源码，未编译片段或重跑该消费者，不能据此推定原生输入、视觉或无障碍验收通过。
