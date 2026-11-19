# 集合焦点：可用性与完整焦点环

基于全部WIP，两项共享缺口分别留证，未改变组件模型、公开签名或token。

第一项根因是List/Tree/DataTable的公开focus入口在整组disabled或窗口inactive时仍存入焦点，恢复后重新获得键盘所有权。Chart也会在inactive时修改受控focus。现有五个入口补用disabled/window_active检查；没有焦点队列、新状态或协议。五项旧代码真实失败的回归转绿；400controls/168consumer、原样Gallery focus_changed函数和432CoreText/GPU场景通过。此阶段的像素只证明状态恢复，检查实际图片后发现焦点环仍被截断，未以像素差异冒充完整体验通过。证据：`design/reviews/collection-focus-availability-20261020/`。

第二项根因是集合自己的内容裁剪同时裁掉完整行/表头的outside-control焦点环，后续相邻chrome也可能遮挡焦点。共享修复位于`collections_view.rs::paint_collection_focus`：复用ContainerLayout的真实ancestor/background裁剪与现有paint::focus_ring，在所有行/表头chrome之后绘制一次。ButtonSize已有radius能力仅改为crate可见，直接复用Body的同一token，不复制样式定义。原行列布局、命中区域、间距、文本对齐和选择语义不变。

适用于所有ListView/TreeView行以及DataTableView行和表头，Gallery与独立消费者统一受益。合理例外：实际ancestor仍裁剪；已被自身frame裁掉的溢出行保留视口裁剪，完全不可见的行不绘焦点；Chart已有内部余量，48组前后像素完全相同。宿主继续复用ControlFrame.focus_bounds/Viewport.reveal_focus预留与揭示完整焦点范围；绘制外扩不扩大pointer目标。focus恢复后的显式请求仍有效，异步旧意图需由调用方版本/窗口契约失效，不承诺自动辨认所有跨异步回调。

| 同主题/品牌/@2x/150%字的真实表头 | 实际GPU输出 |
|---|---|
| 修复前，顶边及两侧被裁 | ![before](../../design/reviews/collection-focus-availability-20261020/gpu/t1-b1-w300-s1.5-TableHeader-disabledtrue.png) |
| 修复后，标准环完整 | ![after](../../design/reviews/collection-focus-ink-20261021/gpu/t1-b1-w300-s1.5-TableHeader-disabledtrue.png) |

六项焦点裁剪/层次回归在旧绘制路径上失败；新路径9项通过（另涵盖实际ancestor、溢出/完全隐藏行、绘制外扩不命中、分数列宽）。当前409controls、168consumer、Gallery13项collection回归与双方严格Clippy通过。432GPU场景1680帧中，384受影响场景与“原组件画面+无遮挡标准焦点quad”的参考合成逐像素相同，48Chart场景与旧版逐字节相同；真实前后PNG和源码守卫分别保存。不是重画页面或局部padding补丁。

13份真实token观测共5688有限场景已按当前源码重新执行。观测标注同时纠正：表头排序不是行选中，header-sort作为未聚焦rest/sort路径记录，selected依据真实selected_row；旧观测保留但不再绑定当前清单。未测hover/pressed、加载/错误/空态和完整alias/branch的覆盖继续不宣称完成；complete=false。治理44、组合8及3项生成绑定检查分别留证。

Gold7分层：G3/G5的此项共享契约、真实组件转接和GPU功能通过；当前版本原生键鼠/窗口、适用辅助技术、ClientKit匹配视觉和整页体验仍待独立验收。未运行原生窗口或VoiceOver，未测性能，未提交、推送或关闭Issue。AgentKit2b16与bd3源码保持不变。详见`design/reviews/collection-focus-ink-20261021/progress.json`及cleanup.json。

## 后续原生复验（2026-10-21）

外接屏 U2790B @2 的真实小宿主已验证四类集合的禁用、拒绝迟到焦点、恢复后新焦点激活，以及 Chart 的真实最小化失活隔离；源码与二进制分别绑定。真实重新激活和原生裁剪检查尚未完成，当前宿主保留等待；不提升为整页 Gold7 或 ClientKit 视觉通过。详见 `design/reviews/collection-focus-native-20261021/native-verification.json`。

## 最终版本原生补充

见 `design/reviews/collection-final-native-20261021/native-verification.json`：当前源码与二进制在外接屏U2790B @2已完成27条有界原生断言，涵盖禁用键鼠、焦点、最新模型变化、祖先裁剪、真实窄窗与最小化失活。系统重新激活后的恢复仍待验，不等于完整可访问性、ClientKit匹配视觉或Gold7整页体验通过。旧日志保留为历史，旧runtime已清理，当前小宿主等待最后复验。
