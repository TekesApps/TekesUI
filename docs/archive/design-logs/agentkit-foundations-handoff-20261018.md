# AgentKit 公共基础与 #15–#21 开发候选交付

本交付基于 bc3de79 加全部现有 WIP，供既有 AgentKit 会话继续消费侧验收。不是 Issue 关闭、发布或正式 pin。公开 Issue 最新读取为 21 个开放、0 个开放 PR；AgentKit #6 已合并，main 的 39095040644 CI 成功。GitHub CI 只对应已提交源码；不能证明当前 WIP 候选通过远端 CI。

## 当前候选与复现

- TekesUI 实现 checkout：`/Users/frankqin/.codex/worktrees/gallery-design-baseline/TekesUI`。
- AgentKit 真实接入：`/Users/frankqin/dev/TekesAgentKit`；现有 WIP 原地保留，未提交。
- `ui/tekesui-source.json` 明确为 development-snapshot-not-release，SHA-256 `e4c6cb6f725ff9df5ec4688927dd265d64dbbdb2380572d963c5b1017dde5bbc`。不得拿 base commit 单独复现，不得静默切回兄弟仓库 HEAD。
- 本轮公共 15 路径、AgentKit 4 路径增量 patch；原先各批源码与 patch 保持封存。当前完整所需 WIP 基线、增量和 canonical runtime archive 位于 `design/reviews/editor-navigation-20261018/patchset/`。其 README 提供不操作 Git 的复现命令；302 个公共结果文件、96 个 AgentKit 构建源文件均实际 patch 重放且逐字节一致。
- 完整 CI 复查另发现 Gallery 验收宿主的 3 处既有格式差异，已做 rustfmt 排序/换行，并修复 #12 导航状态截图夹具的固定高度假设（Select实际1410pt超过旧1300pt），按真实排版测量后完整捕获，重绑 2 个元数据文件；没有改生产控件或页面样式；在上述公共增量之后追加 `design/reviews/foundation-gates-20261018/0003-gallery-ci-gates.patch`。共享 runtime 快照保持不变，原生证据保留原二进制/源身份。
- 这些是可独立构建的开发源码候选及受限增量提交材料；没有生成实际 Git commit，也没有把所有未提交修改视为本轮新增。archive 中既有 WIP 明确是所需前置基线。正式提交范围、拆分和发布仍待明确授权。

## 按依赖逐项对照

| Issue | 共享契约与实际消费 | 当前判断与剩余条件 |
| --- | --- | --- |
| #15 | 原 WindowWake 的弱目标、主线程投递、合并及关闭撤销；AgentKit watch 通知和结果队列先发布再 wake，移除业务 frame pump | 已实现待验收；Gallery 单目标原生、确定性竞态及生产目标表32断言已有证据。仍缺活多窗口现场组合、AgentKit 原生流式/取消/连接变化及公开 pin |
| #16 | 原 ReadingController 的版本化保留 UTF-16 映射；AgentKit 按稳定 TranscriptItem.id 同步 | 已实现待验收；实际 Earlier 连续两页、宽度/字号重排、A/B/选区/焦点通过。Gallery GPU/原生独立留证；AgentKit 原生分页及公开 pin 待验收 |
| #19 | 原 PointerEvent/宿主原语完整传递 button/modifiers/click_count；统一 TextSurface 词/段落/字素选择 | 已实现待验收；实际多行单击/Shift/emoji/组合字符/三击以及可编辑和只读捕获滚动通过。仍按住拖动时发生窗口失焦的现场组合、AgentKit 原生、pin 待验收 |
| #17 | 原 KeyFrame 单键链归属与 disposition、TextEditPolicy、Tab fallback、IME 隔离；原 TextSurface 统一命令及 shape-aware event_with_text | 已实现待验收；不双换行/提交、组字屏障、修饰/重复/焦点/重挂、段落导航和真实 Composer 接入通过。物理中文拼音现场、AgentKit 原生及 pin 待验收 |
| #18 | TextInput/TextSurface 外部值事务、DraftRevision/baseline、冲突及选区/undo 策略；AgentKit controller 原同步归属 | 已实现待验收；同值 ACK、过期/外来实例/组字拒绝、清空可 undo、A/B 隔离通过。物理 IME 体验、消费侧最终验收与 pin 分开保留 |
| #20 | 原 FieldPresentation/TextViewport 与 ScrollArea/ReadingController；真实 Transcript Fill、Composer Grow | 已实现待验收；纯公开配置、上限内滚动、真实重排与混排、readonly、clip/scale 自动化通过。原生候选窗/AX完整矩阵、AgentKit 原生及 pin 待验收 |
| #21 | 原 Component 明确动作 kind，ContentHost 可选收集/有序一次 drain；原 mount 身份防旧动作；AgentKit owned Intent 保留产生时目的地 | 已实现待验收；真实指针/键盘/native AXPress 单次且来源正确，Combo 为 Activate，Field 为 Submit；A/B 切换不重定向，真实接入通过。完整消费侧最终体验及 pin 待验收 |

## 分层验证

当前完整本地 CI 复查：workspace1007通过、1项既有忽略，fmt/严格Clippy、完整Python/设计门禁和导出一致性检查通过；远端旧运行仍保持其原失败状态。

当前源码共享 controls316 + platform/text79 =395；独立公共消费者114；编辑/视口/动作/导航 feature43；Gallery 文本31；AgentKit UI27/headless76。严格 Clippy、Rust 边界、38项能力门禁、6项组合门禁、45个组件的分类/源码锚点均通过。独立重建同样通过 shared395、consumer114、feature43、AgentKit UI27。完整日志和哈希位于 `design/reviews/editor-navigation-20261018/`。

8组既有真实 token 探针4500案例，加导航24案例，均 `complete=false`。没有新增 token、状态机、UI 框架或组件范围。共性修复位于原共享层；文本包装器只转交，业务目的地/发送/状态继续归 AgentKit。

Gallery 实际 CoreText/Metal/native 证据与 API 测试分开。最新窗口全部在外接 U2790B（builtin=false、scale2、报告max_fps30），未测帧率、未改系统设置。Gold7 核对输入边界、焦点、文本、选区和反馈；Light代表场景不能替代全部主题/无障碍验收。#19 多行最新补证在 `design/reviews/pointer-viewport-native-20261018/receipt.json`。此前 GPU 四主题证据保持原源码身份，不伪装成最新全矩阵重跑。

## 消费侧移交要求

先读取全部当前 WIP、现有 AGENTS 和显式 snapshot，避免重复实现。只消费 TekesUI；如公共契约仍缺失，回传具体原 Issue 和可复现证据，不 fork 通用控件/文本算法。先验证 UI workspace/demo host/第二 consumer/headless，再按已有授权边界做原生体验。保留 experimental `0.0.0/publish=false`。

当前本会话桌面授权仅 Gallery。AgentKit 原生窗口需消费侧确认其自身授权；物理中文 IME、VoiceOver 不得用合成事件代替。系统设置另行确认。除明确120Hz测试外，有外接屏就使用外接屏。性能不测。不可变公开 pin 与提交/推送/tag/关闭 Issue/发布均没有授权，不得因本次移交扩大。

原 #13 总览、#14 架构、#12 统一验收仍由 TekesUI 原账本跟踪；#15–#21 保持开放。候选移交后，本会话返回其余原 Issue 的共享层整改，并保留消费侧回报入口。
