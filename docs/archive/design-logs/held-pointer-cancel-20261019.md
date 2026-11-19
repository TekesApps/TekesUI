# #19 主按钮未抬起期间失焦／取消

以全部最新WIP为准；公共候选4a02d30的188文件与当前checkout相符，AgentKit原冻结和主checkout128文件受保护。复用原Focus(false)→ContentHost Cancel→TextSurface.cancel_interaction；根因分类为基础交互原语／公共组件契约。本轮没有发现新的生产共享缺口，未增加抽象、token或迁移其他组件。

隔离消费者仅增加审计观察与有限原生窗口动作：原生Down被实际组件处理、pressed=true后，复用既有selftest::Miniaturize/Deminiaturize触发窗口失焦；记录失焦前后及Up后的选择、滚动、pressed和窗口active。只增审计日志、触发器与原生按钮只读查询，生产取消路径不改。完整审计源码、两文件patch及逐文件清单保留，不把审计入口当作新公共API。

U2790B、2×、Light，CUA定时按住2800ms。两个有效原生链：

- 可编辑Composer：Down14836ms（active=true），Focus(false)15803ms，Up17737ms。失焦发生在抬起之前；Cancel后pressed=false，后续Up仍false，UTF16选区478..478及正文长度527保持。
- 只读Transcript：Down21809ms（active=true），Focus(false)22776ms，Up24639ms。Cancel后pressed=false，后续Up仍false，选区329..329及正文长度548保持。

原生最小化／恢复均实际执行，窗口关闭后原wake与clone均Closed。第一轮还尝试过后台Transcript；没有active→inactive转换，明确排除，不计入验收。恢复后的脚本窗口未重新取得键盘归属，不用其后台输入冒称前台恢复体验；只读有效场景改为新的活动窗口轮次完成。

这里验证的是工具驱动的真实AppKit Down→窗口失焦→Up链，包含真实组件和真实窗口生命周期。NSEvent全局pressedMouseButtons始终为0，因此不把它声称为物理硬件鼠标测试。物理键盘/IME、VoiceOver和性能分别保持未验／未测；本次未开启VoiceOver。

公共pointer_contract2、独立真实字段pointer_metadata6、实际AgentKit原生字段转译1与真实CoreText/阅读/屏障3通过；审计宿主strict Clippy通过。Cancel后Drag不再改变选区的公共/消费者测试，与本轮原生失焦链分别留证。所有相关组件复用同一取消实现；Composer与只读Transcript是两个实际消费者样例。GPU再次输出与前轮相同SHA819c3c0d9415b7c17eacec9b47825dfd19c897e817c241e6b4a396d48ac05fc5。

Gold7：G5保留选择与阅读，取消不误编辑；G6捕获终止和生命周期边界有原生链；G7契约、独立消费者、真实宿主和审计可复现。仅声明上述范围，不宣布完整Gold7或硬件现场体验通过。

#19工具驱动原生剩余功能补证完成，整体仍为已实现待验收：AgentKit正式候选提升/确认、公开不可变pin继续单列；硬件鼠标没有测试。证据design/reviews/held-pointer-cancel-20261019/native-result.json，两个小宿主均已关闭并清理运行时；构建产物经fresh lsof与哈希检查清理，源码/日志/截图和公共依赖缓存保留。未提交、推送或关闭Issue。
