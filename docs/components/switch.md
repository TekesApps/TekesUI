# Switch

SwitchContent 表示即时开关值，组合原 Switch 与 SwitchView。是否立即保存或发起请求由业务层决定。

## 短用法

```rust
use tekes_ui_controls::composition::SwitchContent;
let toggle = SwitchContent::new("Notifications", false);
```

## 状态和动作

`model.checked` 是当前布尔值，`model.disabled` 是可用性，`label` 为显示与语义名称。View 保留原焦点、按压与动画状态。普通指针和键盘路径主要返回 changed，并不统一生成 Activate；可访问 Press 路径会报告 activated。消费者应在正式 Host 输入后读取 checked，与自己的已发布值比较，不仅依赖 Activate 队列。该入口没有 CheckboxIntent 那样的捕获值 payload，不能声称延迟读取得到的必然是事件当时值。

禁用不表示关闭，绘制或 hover 不应触发保存。布局需为开关和标签提供空间，外部不能重复注册一次点击来再次翻转。

Switch 没有 Mixed 状态，也不代表异步保存已成功；请求中、失败和回滚由消费者明确组织。

## 源码与消费者

定义：`crates/tekes-ui-controls/src/composition/adapters.rs`。现有消费路径：`examples/composition-consumer/src/popover_content.rs`。上面的构造片段不是独立窗口程序；完整消费者运行及原生输入、视觉、无障碍验收仍需独立证据。

当前片段验证：Check329 已对这 45 份手册中的 46 段原文 Rust 代码完成 `--emit=metadata` 类型检查，通过但未链接、未运行；本文代码块保持不变，复用该结果。此证据不代替 GUI、物理 IME、VoiceOver 或运行时验收。收据位于协调工作区 `outputs/gallery-apphost-upgrade-20261114/manual-snippets/RESULT.json`，提取源为同目录 `manual-snippets.rs`。
