# Tabs

TabsContent 组合实际标签和面板；TabItem 保存两个 Content 的身份。标签页选择与关闭请求是不同意图。

## 短用法

```rust
use tekes_ui_controls::composition::*;
let tabs = TabsContent::new(vec![TabItem::new(
    Content::new(ComponentId(2), TextContent::new("Overview")),
    Content::new(ComponentId(3), TextFieldContent::new("Draft", "")),
)], 0);
```

## 状态和事件

`selected()` 读取当前索引，`select(index)` 程序选择；`set_tab_disabled`、`set_tab_open`、`reorder` 操作真实声明，返回值须处理。TabItem 可声明 name、disabled、closable。

启用 Host 动作后，TabSelection 捕获 index、label、panel；TabCloseRequest 只请求关闭，面板保持挂载直到消费者批准，不自动选择该 tab。不要用当前索引代替事件里的 label/panel 身份。

隐藏面板的输入、焦点和语义由原组件管理，页面不应另做一套“透明但可点击”的隐藏层。TabsContent 负责面板组合；仅页签标题条另有 TabStripContent 合同，不把它的请求类型混用。关闭前保存策略与文档历史属于消费者。

## 源码与消费者

定义：`crates/tekes-ui-controls/src/composition/navigation.rs`。现有消费路径：`examples/composition-consumer/src/navigation_content.rs`。构造片段不包含完整窗口接入；完整消费者运行及原生输入、视觉、无障碍验收仍需独立证据。

当前片段验证：Check329 已对这 45 份手册中的 46 段原文 Rust 代码完成 `--emit=metadata` 类型检查，通过但未链接、未运行；本文代码块保持不变，复用该结果。此证据不代替 GUI、物理 IME、VoiceOver 或运行时验收。收据位于协调工作区 `outputs/gallery-apphost-upgrade-20261114/manual-snippets/RESULT.json`，提取源为同目录 `manual-snippets.rs`。
