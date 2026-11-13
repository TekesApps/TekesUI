# 尺寸更新期间的共享滚动输入

根因在既有 `ScrollArea::layout`：任何文档或视口尺寸变化都会取消整段输入，丢失方向锁、有效原生输入状态和弹性回弹。追加内容使原底边成为内部位置时，旧回弹还可能继续屏蔽合法的系统动量。

修复沿用同一滚动器和 JSON 生成的物理参数。布局仅释放失效的滚动条拖动、不可滚动方向和非法越界位移；有效手势、回弹相位及后续系统动量保留。显式取消、窗口失活、卸载及策略变化仍结束输入。所有使用 ScrollArea 的既有路径共同受益，不新增抽象层或 token。

374 项 controls、159 项真实消费者测试和严格 Clippy 通过；4 项公共失败回归、2 项混合 Content 消费者回归覆盖尺寸变化、动量、回弹、卸载以及独立编辑器焦点／选区／组字保留。真实 GPU 48 组、144 帧验证尾追加不改变当前回弹画面；外接屏记录构建通过 12 条 AppKit／AX／滚轮／窗口断言。工具未产生原生 momentum phase，因此不能将其计为物理动量验收。

11 份 token 探针已重跑，共 5,040 个有界场景。滚动物理参数另有 8 条实际无动态查找记录，其常量来源是 `design/behavior/scroll.json` 经既有 build.rs 生成，不能把子组件的配色查找归给滚动器。

用户指出当前压力夹具与 ClientKit 差异明显。该夹具不作为视觉达标证明：ClientKit 对照和 Gold7 的视觉部分未验收。后续应以相同内容、主题、尺寸、倍率和字号对照；共性缺口在共享定义与实现修复。此项也不等于 #22 的结构化 item 锚点／版本事务已经实现。

证据位于 `design/reviews/scroll-layout-input-20261020/acceptance.json`。记录构建之后的并行 renderer 改动未由该二进制认证，最终组合候选必须复验。全部 WIP 保留；小宿主已关闭，确认无占用后清理本轮约 280 MB 运行／测试产物，保留公共缓存和证据。未测性能，未提交、推送或关闭 Issue。


## #275 原文本滚动手势归属（2026-11-13，本批验收中）

Gallery Field/Area 迁移暴露了真实消费回归：同一 precise 手势在 Area 内 Began，后续 Changed 的位置离开编辑视口时，逐包命中会丢失原 TextSurface 的滚动归属。Check243 的真实 Gallery 用例保留该失败；不以旧选错导航 owner 的 Check241 红例作证。

本地正式接线使用 `ContentWheelEvent` / `ContentWheelPhase`，完整保留普通 phase 与 momentum phase，避免三个布尔值不能区分 Changed、无 phase 独立包和 momentum Ended。runtime 的有限 native 转换由其公共 dispatch、AppHost 本地正文和 Gallery 共同消费。原 `ContentEvent::Scroll` 兼容调用仍保留，不因本地增强扩展 wire/IPC。

Host 只为实际接受 phased precise 输入的原 TextFieldContent 保存独立、mount-scoped wheel 归属；续包仍走原 owner 及其祖先冒泡路径，TextField 调用已有 `TextSurface::wheel_captured_at`。偏移、方向锁、回弹和动量由原 ScrollArea 管理。零 delta Began 也可获得归属；非 precise、禁用滚动轴及边缘冒泡保留原行为，不统一捕获所有 ScrollContent。

finger Ended 可等待同手势 momentum；Cancelled、momentum 终止、失活、blur、卸载/同ID重挂载和换父节点撤销旧归属，旧尾包不得命中新实例。普通重排或 Theme/Surface 更新不单独撤销有效手势。wheel 终止只结束原滚动手势，不清 pointer capture、焦点、selection 或 marked text。

该保证限于续包被投递给同一 ContentHost；AppHost 跨 Container/Toolbar 的窗口级目标仲裁、远端 IPC 和其他自定义滚轮适配不随本批增强。CPU阶段序列、自动原生普通滚动和物理触控板阶段分别记录，禁止由 CPU 结果代签物理输入。
