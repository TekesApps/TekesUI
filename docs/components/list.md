# List

普通纵向内容用 ListContent；需要稳定业务选择的有限集合用 SelectableListContent::keyed，返回 KeyedSelectableListContent。不要把显示序号当业务身份。

## 短用法

普通滚动列表使用现成组合入口，默认在视口 hover 时显示溢出滚动条：

该入口默认使用共享 `Spacing::Loose` 内容留白，列表行保持 Leading 左对齐；消费者只声明分组和条目，无需逐行补 padding。显式 `content_padding(...)` 替换默认值，不叠加。内部 `ListContent::new` 用来组织条目或嵌套分组，不再次增加视口留白；列表外的 Search 等兄弟控件由父布局安排。

```rust
use tekes_ui_controls::composition::*;
let list = ListContent::scrollable(
    "Documents",
    ComponentId(2),
    vec![Content::new(ComponentId(3), TextContent::new("Document"))],
);
// 不显示滚动条时：let list = list.scrollbar(false);
```

需要稳定业务选择时：

```rust
use tekes_ui_controls::{composition::*, extended::*};
// snapshot: KeyedListSnapshot；rows: Vec<ListRowDisplay>，由消费者按稳定 key 构造。
fn mount_list(snapshot: &KeyedListSnapshot, rows: &mut Vec<ListRowDisplay>)
    -> Result<KeyedSelectableListContent, ListContentError>
{
    SelectableListContent::keyed("Documents", snapshot, rows, ListBudget {
        max_items: 100, max_display_items: 100,
        max_utf8_bytes: 1_000_000, max_staging_units: 100_000,
    })
}
```

## 身份、更新与事件

快照区分 dataset、items 和 display_items。每行的 ListRowDisplay 提供稳定 row_id，连续存在的实体不能因为排序而换挂载身份。Keep/Replace 决定保留或更换被动显示内容。

`selected_key()`、`focused_key()`、`model()` 读取真实状态。挂载集合更新通过 Host 的 `replace_keyed_list`，选择可用 `set_keyed_list_selection`；与阅读锚点关联时携带原更新合同，不绕过 Host 修改子树。ListIntent 区分 SelectionChanged 和 Activated，含 dataset/key；延迟消费使用 `intent_is_current`。

普通滚动 List 使用 ListContent::scrollable，已包含原 ScrollArea，默认在溢出且视口 hover 或拖动滚动条时显示；scrollbar(false) 只关闭显示，不关闭滚轮或 reveal。消费者无需补写滚动绘制。ListContent::new 仅组织内容。KeyedSelectableListContent 负责行导航、选择与语义，通过上述滚动入口或正式 ScrollContent 提供视口与阅读，避免重复嵌套滚动 owner。被动行装饰不是任意可交互子控件容器；自定义装饰须满足事务 hooks 合同。预算来自示例，需按实际规模选择；有限集合接口不等于远程分页或自动虚拟化。

## 源码与消费者

定义：`crates/tekes-ui-controls/src/composition/selectable_list.rs`。现有消费路径：`examples/composition-consumer/src/selectable_list_content.rs`。构造片段不包含完整窗口接入；完整消费者运行及原生输入、视觉、无障碍验收仍需独立证据。

当前片段验证：Check329 已对这 45 份手册中的 46 段原文 Rust 代码完成 `--emit=metadata` 类型检查，通过但未链接、未运行；本文代码块保持不变，复用该结果。此证据不代替 GUI、物理 IME、VoiceOver 或运行时验收。收据位于协调工作区 `outputs/gallery-apphost-upgrade-20261114/manual-snippets/RESULT.json`，提取源为同目录 `manual-snippets.rs`。
