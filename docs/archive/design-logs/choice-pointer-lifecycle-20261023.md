# 选择触发器的禁用期间指针位置

关联新发现独立缺陷 #60、总规划 #56；Theme/Surface消费继续由 #1/#55 追踪。#59只负责Breadcrumb路径身份，不将两个独立缺陷合并验收。

根因在原View的事件转发层：ButtonModel.pointer_move_at已能在disabled时保存真实位置并按当前状态抑制hover，调用方却在转发前return，导致最后inside位置跨禁用区间存活。修复应复用原公开pointer_move和原ButtonModel，不创建hover副本、计时器或新入口。窗口失活仍拒绝输入并清理原状态；菜单/浮层作用域不因追踪位置而打开或穿透。

已证实CommandPalette的合法公共move入口存在；早先只读QA漏读了文件后段，不能据此新增pointer_move_trigger。进一步用原背景Primitive做真实红绿测试，证实“hover→禁用并移出（不绘制）→启用（无新move）→绘制”确有旧hover残留。文字可能在hover/rest同色，不能用文字色代替状态chrome断言。

同因范围检查Select、ComboBox与CommandPalette。只修真实红证据的路径；若原Menu runtime已清理位置，则记录它是合理例外，不改为另一套状态机。所有固定DS状态和原事件/焦点/capture/取消规则保留，不选择新颜色。

公共API/独立消费者/Scene状态检查、真实GPU、原生交互、物理IME、VoiceOver和Gold7各自留证。当前用户要求后台推进，VoiceOver关闭，不启动新原生窗口。构建复用review-cache独占锁。源码与失败/成功证据保留，未验项不标完成，不改AppHost或自动Git发布/关闭Issue。

## 本地 main 接收身份（#136）

本轮基线为 `c19f312`，仅接入 Select/CommandPalette 原有 disabled pointer 修复及必要 inactive 清理；ComboBox 保留原取消机制，并补齐 MenuRuntime 失活链对 retained row Button 指针位置的清理；不改变候选映射、选择或 popup 状态机。独立消费者保留真实 Scene 背景的 rest/hover 可区分断言、禁用区间无中间绘制、重新启用后的首帧及窗口失活检查，不依赖后续 Surface/Theme API。原 Root 收据保留其身份；本地 main 的实际红绿、原 controls 回归、strict 和程序/depinfo 记录单独存放于 SSD `artifacts/issue-60/main-c19-acceptance-20261026`。Scene 检查不代表原生窗口、物理 IME、VoiceOver、GPU 或发布验收。
