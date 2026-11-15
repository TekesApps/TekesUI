# SegmentedControl

SegmentedControlContent 是紧凑互斥选择，适合少量视图或模式切换；它只负责选择，不自动装载业务页面。

## 短用法

```rust
use tekes_ui_controls::composition::SegmentedControlContent;
let segments = SegmentedControlContent::new("View", vec!["List".into(), "Grid".into()]);
```

## 状态与意图

`model()`、`view()` 返回只读原状态。`set_selected(index)` 与 `set_item_enabled` 返回 Result，需处理无效索引；`set_disabled` 管理整体可用性，`set_compact` 选择紧凑呈现。

Host 的 SegmentSelection payload 携带 SegmentIntent，`index()` 读取捕获选择，`intent_is_current` 核对声明身份。part token 是当前挂载的可访问子项标识，不可作为持久业务 key。

排列、按压和方向键由原 View 维护，业务层负责选项对应内容。需要标题与面板的实际生命周期管理时使用 TabsContent；它不自动实现多选或任意宽度标签滚动导航。

## 源码与消费者

定义：`crates/tekes-ui-controls/src/composition/segmented.rs`。现有消费路径：`examples/composition-consumer/tests/segmented_content.rs`。上面的构造片段不是独立窗口程序；完整消费者运行及原生输入、视觉、无障碍验收仍需独立证据。

当前片段验证：Check329 已对这 45 份手册中的 46 段原文 Rust 代码完成 `--emit=metadata` 类型检查，通过但未链接、未运行；本文代码块保持不变，复用该结果。此证据不代替 GUI、物理 IME、VoiceOver 或运行时验收。收据位于协调工作区 `outputs/gallery-apphost-upgrade-20261114/manual-snippets/RESULT.json`，提取源为同目录 `manual-snippets.rs`。
