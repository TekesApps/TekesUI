# Skeleton

SkeletonContent 是静态占位，复用 SkeletonView。默认装饰语义，不创建输入、计时器或业务加载状态。

## 短用法

```rust
use tekes_ui_controls::composition::SkeletonContent;
let mut placeholder = SkeletonContent::default();
placeholder.set_accessibility(Some("Loading preview".into()), None).unwrap();
```

## 分配和限制

父级提供有限宽高，Skeleton 没有固有高度。`layout()` 读取已有 ViewLayout；`set_accessibility(name, description)` 在装饰和有名 Group 之间切换，空名称错误不会覆盖原声明。

需要用户理解加载状态时使用普通兄弟文字和真实结果反馈，不把每个骨架块都变成焦点项。占位组件本身没有激活事件、shimmer 动画或自动请求能力。

不要用 Skeleton 掩盖错误或无限保留加载状态；何时替换成内容由消费者负责。它不会根据未来子控件自动推导占位尺寸。

## 源码与消费者

定义：`crates/tekes-ui-controls/src/composition/skeleton.rs`。现有消费路径：`examples/composition-consumer/src/skeleton_content.rs`。构造片段不包含完整窗口接入；完整消费者运行及原生输入、视觉、无障碍验收仍需独立证据。

当前片段验证：Check329 已对这 45 份手册中的 46 段原文 Rust 代码完成 `--emit=metadata` 类型检查，通过但未链接、未运行；本文代码块保持不变，复用该结果。此证据不代替 GUI、物理 IME、VoiceOver 或运行时验收。收据位于协调工作区 `outputs/gallery-apphost-upgrade-20261114/manual-snippets/RESULT.json`，提取源为同目录 `manual-snippets.rs`。
