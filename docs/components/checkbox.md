# Checkbox

Checkbox 表示可勾选的独立值，支持 Unchecked、Checked 和 Mixed 三态。它的 checked 值与 hover、pressed、键盘焦点是独立状态。

## 最小组合

```rust
use tekes_ui_controls::{CheckState, composition::*};

let mut host = ContentHost::new(Content::new(
    ComponentId(3),
    CheckboxContent::new("Published", CheckState::Unchecked),
)).unwrap();
host.set_action_events(true);
```

对应独立消费者 `examples/composition-consumer/src/checkbox_content.rs`。

## 属性与读取

- `new(label, checked)` 使用正式 Checkbox 模型创建内容。
- `from_original(label, model)` 接收已有模型，保留其语义状态。
- `model()` 返回只读原模型；`layout()` 返回已准备的实际布局，不进行布局计算。
- `set_checked(...)` 同步消费者值；它与一次用户激活不是同一件事。
- `set_availability(disabled, loading)` 更新可用性；`set_error(...)` 更新错误状态。

## 事件

启用动作记录后，通过 `ContentHost::drain_action_records()` 取得实际激活。
`ContentActionPayload::CheckboxState` 携带 `CheckboxIntent`，用 `checked()` 取得交付时捕获的值。
不要用绘制次数、hover 或读取模型的动作触发业务保存。

## 组合与限制

使用固定稳定的 ComponentId，把标签和勾选框作为同一个语义控件。禁用控件仍可被 Gallery 的组件选择器检查，但选择 Inspector 不等于激活 Checkbox。
事件中的挂载身份与原输入授权由 Host 维护；消费者不要把旧实例的延迟动作套用到新实例。
此页不声称 VoiceOver 或完整物理输入矩阵已验收。

当前片段验证：Check329 已对这 45 份手册中的 46 段原文 Rust 代码完成 `--emit=metadata` 类型检查，通过但未链接、未运行；本文代码块保持不变，复用该结果。此证据不代替 GUI、物理 IME、VoiceOver 或运行时验收。收据位于协调工作区 `outputs/gallery-apphost-upgrade-20261114/manual-snippets/RESULT.json`，提取源为同目录 `manual-snippets.rs`。
