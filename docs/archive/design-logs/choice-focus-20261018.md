# 选择菜单关闭后的焦点呈现

基于 `bc3de79` 加全部 WIP。GitHub 当前仍有14个开放 Issue、0个全状态 PR，更新时间与现有账本一致。继续原 #1／#3／#8／#12／#14，不关闭原 Issue。

## 根因与共享修复

SelectView 自己持有触发器 Button。鼠标打开菜单后，键盘 Enter 确认或 Escape 取消会关闭菜单、保留/更新选择值，但没有把触发器的 focus-visible 提升为键盘模式。公共 API 红例可独立复现，不依赖 Gallery。现在在已有键盘关闭结果上调用原 `focus(true)`；所有 SelectView 消费者受益。没有新 token、状态枚举、协议或抽象层。

普通 Menu 返回 caller focus target，由调用方持有触发器。Workbench 单独处理 Enter 时只恢复逻辑目标，没有同步其 Button 焦点外观；现在使用现有 Button 状态同步。CommandPalette 的既有关闭入口已经调用 `sync_owned_focus(..., true)`，没有重复实现；ComboBox 的可编辑输入仍由 TextSurface 持有焦点，不改为 Button 外观。

合理例外：鼠标打开或再次鼠标打开不应获得键盘焦点环；选项值与候选焦点仍不同。真实完整 Gallery 的通用键盘路径原本已让普通 Menu 的 Enter/Escape 恢复焦点环，因此普通 Menu 原生前后截图相同，不声称出现原生红绿改善。Workbench 的直接事件回归保留为其自身入口的契约补齐。

## 分层验证

- 公共 API：新增1项回归，四主题×三品牌×Enter/Escape共24条路径，用原 SelectView 绘制图元验证鼠标打开无键盘环、键盘关闭有环、值正确、释放不重开、blur移除焦点环。修复前已编译运行并失败，修复后 choice_contract 共11项通过。测试排版器和无字形绘制不代表像素验收。
- 独立消费者：既有 CoreText＋原 SelectView 场景增加关闭后实际焦点图元断言，不调用宿主补救。32项消费者测试通过。
- 真实接入：完整 Gallery 的36/76确认回归增加触发器焦点图元断言；Workbench Menu 的Enter/Escape回归通过，Enter修复前失败。控件/token/design-export共339项通过；根和消费者严格 Clippy通过。
- token：共享源变更后，全部7份当前绑定观测重新执行，共180/84/192/576/612/2016/408条案例。只重新绑定当前证据，不提升未验证状态，不把有界读取声称为完整依赖提取。
- GPU：四主题×三品牌×两种比例，Select/ComboBox状态整页288张重新渲染，与上批对应288张逐字节一致。这验证已有状态夹具未变化；关闭后焦点路径由单独原生截图覆盖，不能用相同夹具截图代替。
- 原生与Gold7：使用可追溯的修复前/后独立Gallery包，签名后记录二进制和输入哈希。Light真实Select鼠标打开→Down→Enter选择Compact并恢复焦点环；再次鼠标打开无键盘环→Escape保留Compact并恢复焦点环。Menu确认/取消前后均有焦点环。代表帧核对焦点可辨识、选中值、弹出层关闭、现有层级与间距，未宣称全族体验完成。最初把多个键紧连发送的对照帧尚未完成确认，保存为诊断，随后分步观察后的帧才用于结论。
- 无障碍：本批AX仍只观察到窗口和菜单，未验收控件AX、VoiceOver或物理IME。不操作系统设置，不测性能。两个验收Gallery均已退出。

能力统计保持115项精确公共状态、141项夹具缺口、269项必需状态。选择族剩余10项：Select empty/loading/error，ComboBox loading/error，CommandPalette selected/open/empty/loading/error。后续先区分输入验证错误与调用方异步选项加载失败，优先复用TextSurface和Progress/Banner/EmptyState及既有消费者场景，不能为了补齐登记而新增状态层或把反馈简单视为不适用。

完整当前输入、源差异、红绿日志、GPU、原生前后图与WIP保护见 `design/reviews/choice-focus-20261018/receipt.json`。历史证据保持不变；未提交、推送、打tag、关闭Issue或发布。
