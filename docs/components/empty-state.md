# EmptyState

EmptyStateContent 组合标题、说明和可选操作，用于空集合或暂无内容。操作是明确用户动作，不是自动创建数据。

## 短用法

```rust
use tekes_ui_controls::{composition::*, extended::EmptyState};
let empty = EmptyStateContent::new("Documents", EmptyState {
    title: "No documents".into(), detail: "Create a document to begin.".into(),
    action: Some("Create".into()),
}, ComponentId(2)).unwrap();
```

## 更新与动作

`model()` 返回只读借用，`layout()` 读取几何，`disabled()` 读取可用性，`set_metadata` 更新名称和说明。借用不要跨 Host 更新保持。

挂载后使用 `host.replace_empty_state` 更新声明、`host.set_empty_state_disabled` 控制操作可用性。构造的 action_id 是真实动作子项身份，启用 Host 动作后按该 ID 消费；无 action 的状态不应生成可点击的假按钮。

父级分配有限区域并负责列表与空态切换；组件不决定查询是否完成，也不把加载失败解释成“没有数据”。它不内建远程请求或页面导航，消费者执行操作并根据结果更新内容。

## 源码与消费者

定义：`crates/tekes-ui-controls/src/composition/empty_state.rs`。现有消费路径：`examples/composition-consumer/src/empty_state_content.rs`。构造片段不包含完整窗口接入；完整消费者运行及原生输入、视觉、无障碍验收仍需独立证据。

当前片段验证：Check329 已对这 45 份手册中的 46 段原文 Rust 代码完成 `--emit=metadata` 类型检查，通过但未链接、未运行；本文代码块保持不变，复用该结果。此证据不代替 GUI、物理 IME、VoiceOver 或运行时验收。收据位于协调工作区 `outputs/gallery-apphost-upgrade-20261114/manual-snippets/RESULT.json`，提取源为同目录 `manual-snippets.rs`。
