# Disclosure

稳定 ID：`tui.disclosure` · 分类：`layout` · crate：`tekes-ui-controls`

状态：**planned；没有运行时实现证据**。负责人角色：`controls-maintainer`；实际维护者须在进入 beta 前绑定。此 crate 为建议新增，需架构评审。

## API 契约

以下为设计级签名，不是已编译的 Rust API：

```text
Disclosure(expanded, on_expanded_change, summary, content, status)
```

使用：可键盘操作的可折叠区域；承载工具详情与公开的推理摘要。

禁区：不能为流式生成强制展开；不能把隐藏内容冒充可访问；不能索取或伪造模型未提供的内部思维链。

共有品牌派生契约见 `../BRANDING.md`；组件不得读取 seed 或固定色相名。共有契约见 `../../SPEC.md` 第 5、9、10、12 节及 `../ALIGNMENT.md`。该组件已登记的具体 token 和行为映射见 `../../design/component-contracts.json`。可视样式不能改变业务回调次数、payload、权限范围或撤销边界；组件只收受控值并发出语义事件。任意 style 参数只能接收类型化 token / variant，不能接收随意颜色或尺寸。

## 状态与主题

适用状态：rest, hover, pressed, focus, disabled, expanded, loading, error, empty。

必测组合：expanded+focus, loading+focus, error+focus。

四主题：light、dark、light-ic、dark-ic；分别使用 Gallery 登记的中性、蓝色、陶土色三品牌夹具。外观/品牌切换只更新同一完整 ThemeContext，不改变焦点、组字、草稿和选区。状态通过同一个 reducer 与正常输入事件建立，Gallery 不得另写造假外观。

| 不适用状态 | 理由 |
|---|---|
| checked | 该语义不适用于此组件；可操作子组件单独验收。 |
| selected | 该语义不适用于此组件；可操作子组件单独验收。 |

## Gallery 契约

拟定稳定地址：`tekes-gallery://component/disclosure`。目前不是可打开的页面。

夹具：short, long-title, cjk, empty。页内固定顺序：规范、真实交互实例、状态矩阵、四主题并排、文字/窗口缩放、token 依赖、无障碍语义、验证记录。复制/选择原文不包含装饰信息。状态矩阵与无障碍树快照都由本登记生成。

## 验收与迁移

组件 / 实际应用 / 实机三层当前均为 `unverified`。不得把该文档或图片作为 Rust 实现证据。发布后 ID 与 slug 保持不变；废弃保留说明和替代组件，至少跨两个 minor 且不少于 90 天后才可在 major 版删除。
