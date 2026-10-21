# Tooltip 原生生命周期补验（2026-10-21）

基于全部当前 WIP，复用现有 TooltipView、ButtonContent／ContentHost、独立消费者的原生指针转换器及 CoreText／Metal。没有改生产共享源码、状态协议、token、Gallery 页面或 AgentKit 冻结输入。

## 实际完成

`design/reviews/tooltip-native-lifecycle-20261021/acceptance.json` 和 `source-manifest.json` 固定本轮宿主、共享源码、二进制身份和来源。32 个原生检查点保留实际窗口截图、AX 文本、运行时状态及原始事件。

- U2790B@2 的实际活动窗口中，Tab 显示提示，Escape 关闭，焦点离开并重新进入可恢复。
- 四主题下，调用者 controlled-only 显示、Escape 关闭、仍为 true 时保持抑制、false 后 true 重新显示均通过。没有为了示例另做控制状态机。
- 真实坐标鼠标 Down/Up 事件经过已有无损适配器到 ButtonContent；单独的 AX Press 记录明确不算鼠标操作。
- 提示显示才发布 StaticText 子节点；关闭／失活后节点消失，锚点帮助描述保留。平台 AX 记录不等于 VoiceOver 听读。
- 真实最小化触发 Focus(false)，立即清除提示可用几何和焦点。随后宿主显式注入的迟到 pointer/focus/Escape 不重开提示。这是已标记的迟到调用注入，不冒充物理键盘输入。

## 未收口行为

混合 hover/focus 时，Escape 后当前实现要求旧来源全部释放，才解除共同抑制。原生记录 `pointer-left-owner.json` 显示指针已离开而键盘焦点保留，`pointer-fresh-return.json` 显示重新进入后仍不显示。接着显式释放两个来源，再进入能够恢复。

这些记录证明当前行为，但规范没有明确认可“必须全部释放”，不能把实现本身当作正确性的依据，也不能把这几个观察点当作体验通过。下一步依据新的用户输入意图，评估是否应由新的独立 hover／键盘焦点进入解除抑制；优先复用现有状态，不因评估就增加一层抽象。

真实最小化后的菜单恢复只能恢复可见性，运行时仍为 active=false。没有向失活窗口发送按键后声称恢复通过；真正系统重新激活、无旧来源自动复活、随后新来源进入的原生链继续未验。集合宿主的同类待验也独立保留。

## 范围与清理

本轮没有新共享修复，不重跑无变化的全部控件／GPU 测试来制造“当前全库通过”的结论。小宿主构建与严格 Clippy 通过，实际 native 输入和 AX 范围如上；旧 public API／GPU 收据保持原来范围。Gold7 整体、ClientKit 匹配视觉、听读和最终组合版本未通过。没有测性能。

窗口已通过本机 UI 关闭，完整打开文件清单确认不再使用后清理自己的应用副本和构建包。源码、锁文件、原始记录、截图、源码快照及公共缓存保留。未提交、推送、打 tag、合入或关闭 Issue。
