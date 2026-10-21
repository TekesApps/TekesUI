# #13/#8 账本、#2/#6门禁与Gallery生命周期

2026-10-19，活跃checkout为gallery-design-baseline，HEAD `80d6b05649234f953ea452d42e1cbed369fdddbb`加全部WIP。保护基线27773文件；AgentKit最终交付目录931文件与公共188文件独立保护。本文是当前增量入口，历史失败/旧版本现场边界保留。

## AgentKit接收及关闭边界

Rust AgentKit会话已独立核验037候选的545 payload/188公共输入、UI28/公开consumer131/headless76、三套严格Clippy、构建与Metal渲染，建议提升开发冻结，未覆盖原冻结。原生/物理IME/VO按交付证据核查，没有声称消费会话重跑现场。已复制消费结论至本批agentkit-consumer-result.{json,md}。

#15–#21均技术接收通过，但仍缺匹配候选的公开不可变pin，消费方明确全部不可关闭。用户授权满足全部原条件后关闭；本轮没有符合条件项，未关闭Issue。提交/推送/tag/发布仍未授权。后续继续沟通及按新结论核对，不重复催问同一pin阻塞。

## #13/#8 当前账本修复

上一批TextSurface几何修复后，7份当前运行时token观测与组合审计仍绑定旧源码，门禁正确拒绝。根因属于当前证据/代码身份绑定遗漏，不是组件实现缺失，不能通过仅更新摘要绕过。

实际重新运行action、input、selection、combo-async、command-async、content-status和slider-composition公开探针，共4308个有界案例，7份新观测存本批current-tokens，17组件引用新路径，历史观测未改。生成器重新生成45组件/5模式/1示例的当前capabilities。269状态要求：131公共API核验、125夹具待验、13语义例外；完整依赖组合/体验未齐，45组件全部保持experimental。

组合矩阵逐项复核TextSurface可见clip/组字reveal差异，四个TextSurface公开声明锚点仍是70行；本批Gallery私有入口变更另有source/anchor复核。来源绑定更新不提升任何native_acceptance。无新增组件、token、协议或公共抽象。

ledger已对齐最终037及消费接收，去除当前入口中旧候选/旧“物理IME、VO未测”判断；原笔记留在historical_verification_note，不能扩大为全Gallery通过。当前总览添加显著历史边界入口。

## #2/#6 门禁

初次本机Python3.14缺jsonschema，按仓库requirements在独立、可回收venv安装jsonschema4.26.0，未改全局环境。初始旧绑定失败、缺依赖失败和后续通过日志均保留。

同一完整WIP先运行全部CI对应本地检查：scripts/tests、顶层test脚本、品牌/能力freshness、reference/Gold7、纯Rust/历史baseline负例、设计合并/schema/引用/历史goldens、Rust token导出generate/check、fmt、workspace严格Clippy、workspace测试1045通过/原有1忽略、独立consumer131通过。脚本名含performance的单元测试只验证合成报告解析，没有采集物理性能。

本批Gallery修复后另有final-gates复验，最终1046通过、1项原有忽略；数量与完整结果见本批final-gates.json/final-workspace.log；新来源绑定必须通过能力/组合负例检查。没有删检查或continue-on-error。

远端最新CI不是旧bc3de79：run39172913867在80d6b05实际仍因两项历史包逐字比较失败，后续步骤被跳过。本地WIP已有按当前作者输入比较和原包完整性分离的修复；未推送，所以本地通过不能写成远端CI成功。该远端结果和日志另存ci-remote/ci-latest-job/ci-latest-failure。

## 实际发现与修复：冷启动失活AX

根因类别为Gallery公共宿主语义动作入口。真实原生冷启动先收到Focus(false)、后首次布局生成控件时，局部按钮检查未覆盖迟到AX请求；inactive窗口的Open dialog竟能打开模态。已有“先绘制再失焦”测试不能捕获这个顺序，因此首次补充测试仍通过；改为真实启动顺序后准确红灯，已保留两个过程而不伪造red。

修复位置为Gallery::accessibility_action，在统一入口读取既有Workbench.window_active拒绝所有失活动作；只加内部只读getter，未复制窗口状态，未新增公共层。适用全部Gallery语义生产者（导航、搜索、表单、集合、浮层等）；只读后台语义查询仍允许，是合理例外。独立composition宿主和AgentKit已有对应Window::is_active保护，不重复实现；037公共188文件及封存源码/证据包不变。

新增冷启动回归验证不打开Dialog、不导航、不发布活动焦点，恢复后正常打开；先红后绿。实际Gallery语义测试29项与严格Clippy通过；首次Clippy布尔比较报错已修正，日志保留。

## 原生与Gold7的当前范围

外接U2790B、2×、Light。用现有Dialog小宿主，共享review-cache；只额外构建一次必要的真实Gallery接入（二进制在修复后重建，未复制target）。修复前轮次标discarded，修复后使用新轮次和新哈希。

- 修复后冷启动AX Press被拒绝，active=true后同操作可打开Dialog。
- Gallery Dialog实际Tab到Confirm、Shift+Tab回Cancel；最小化时捕获window_active=false/window_key=false且零活动焦点；窗口菜单恢复并激活后两者true、Dialog保留。
- 独立Rich Dialog真实多行/选择/文本命令接入：Tab到Switch、Shift+Tab回编辑器，Escape经过cancelOperation/TextKeyEnd仅关闭一次，焦点回invoker；原生日志保留open=false但presented=true的退出绘制，结束后presented/animating/frame_requested均false。
- 应用Hide后Gallery无活动焦点；另一小宿主活动可操作，同时采样Gallery false/peer true；迟到Cancel AX Press被拒绝且Dialog仍在。shell wrapper包的独立系统切回记录不足，没有判通过；改为同源直接Mach-O打包后，由用户物理Command+Tab切出/切回并按Tab，连续文件记录为active/key真→假→真，焦点随后到Docs，返回Codex后再次失活并清焦点。direct-focus-history.jsonl保留完整变化，不以逐应用工具按键代替系统操作。

公共API、实际Gallery接入、GPU及原生/Gold7分开记录。只覆盖当前代表场景，不宣称#3/#4/#5/#9全组件全主题通过，不等同完整#12；本轮不开VoiceOver、不改系统设置、不测性能。最终收尾和保护检查见completion-check.json；本轮代表场景的六层receipt已accepted，未扩大到全族矩阵。

## 收尾与下一步

补测Gallery的应用内Reduced Motion开关后关闭即回invoker；CommandPalette真实打开、Escape关闭、Tab回搜索且不重开，均有原生记录。真实Metal离屏保存80张图：四主题、1×/2×、进入/退出/反向/Reduced Motion十阶段。Gold7仅记录本轮G5/G6/G7有界场景，未声称整套认证。

小宿主和必要的Gallery集成包均已退出；标准流程accepted后删除本轮runtime，另按编译日志归属和活跃句柄检查清理229个精确构建文件，逻辑大小561395138字节（约535 MiB；硬链接和文件系统压缩可能影响实际释放量）。本轮Python venv已删除，共享依赖缓存、全部源码和验收证据保留。VoiceOver本轮未开启。

下一步按#3/#4/#5/#9剩余焦点、浮层与窗口适用矩阵推进；#2/#6远端同一提交CI、#8完整状态体验和#12逐组件逐页面统一验收仍待完成。消费方再次确认037未受影响、无需重跑；#15–#21仍没有可关闭项。没有提交、推送或关闭Issue。

保护复核：27773个原有文件中只有本轮七个预期文件修改；931封存文件和188公共候选输入均未变。AgentKit主工作区相对旧移交基线有六个外部文件变化，消费会话已确认全部属于其本轮CI/测试工具和文档改动，原样保留，无需还原或重跑037；不能写成其全部128输入未变。
