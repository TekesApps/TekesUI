# #15–#21：当前 WIP 对照与执行顺序

2026-10-18 从 GitHub 重新读取全部 21 个开放 Issue，开放 PR 为 0。不是按 base bc3de79 判断缺口：当前权威为 gallery-design-baseline/TekesUI 的 HEAD + 全部未提交/未跟踪 WIP。原 #1–#14 的验收条件保留；#13 总览、#14 公共组合审计、#12 统一验收入口不变。逐条验收项进入 design/remediation/ledger.json；原文快照进入 wake-contract-20261018 证据包。另核实关联 [AgentKit PR #6](https://github.com/TekesApps/TekesAgentKit/pull/6) 已 MERGED，head 为 e06670e5b546644bc0e4eff92d77a36dc1e9b6b9；Issue 原文仍称“草稿 PR”，不沿用该旧状态。PR 明确保留持续 --frame-pump 与前插锚点阻塞，其已有 headless/UI 测试是消费侧历史证据，不算本轮联调通过。

## 复用判断

| Issue | 根因层 | 已有能力及实际缺口 | 当前安排 |
| --- | --- | --- | --- |
| #15 P0 | 平台事件与窗口生命周期 | Window 有 UI 线程 request_redraw/set_animating，缺后台通知代理；复用 Shared 非重入事件队列和主 dispatch queue | 首先补 WindowWake / Event::Wake；本轮已实现并进行独立消费/原生验证，消费方正式迁移仍待配合 |
| #16 P0 | 共享文本阅读契约 | ReadingController 有尾追加、宽度/字号重排与 follow-latest；layout_with 对非尾追加视为新文档，不能表达前插映射/版本 | 接着在现有控制器补文档身份、版本、UTF-16 映射与 capture/restore；真实布局验证，不做像素补偿新滚动器 |
| #17 P1 | 共享文本事件/编辑模式 | ContentHost 已有 Tab/Backtab fallback、composing 优先权；缺修饰键/repeat/text-services disposition 与多行/提交策略 | 与 #18–#20 一份编辑契约，先统一事件路由再接入，保留单行默认行为 |
| #18 P1 | 编辑器受控同步事务 | TextInput 已有选区、undo、preedit；DraftDelivery 处理调用者 scope/revision/投递，不能代替编辑器受控值事务 | 在原 TextInput 上表达版本/选区/undo/组字冲突策略，复用原编辑历史，不新建持久化系统 |
| #19 P1 | 基础指针事件透传 | 平台已有 button/modifiers/click_count，TextSurface 已有选择算法；ContentEvent 只传位置，TextFieldContent 固定 false/1 | 补完整事件路径和迁移规则，再复用原 shift/词/段落选择；次按钮不能走普通激活 |
| #20 P1 | 公共组件测量/视口契约 | TextSurface 有布局/滚动/候选矩形，Stack 有 flexible_child；TextFieldContent 固定表单 slot | 复用既有 viewport/FieldView/Stack 表达 fill/grow/min/max；与 #16/#17 联合验收，不重写 arrange/滚动 |
| #21 P2 | 公共动作契约 | ContentResponse 有 consumed/changed/activated，ButtonContent 有真实 activations；缺带 ID/kind/source 的有序一次交付出口 | 随 #17 提交意图/#19 指针来源统一动作出口；UI 仅传通用 ID，业务 scope 由消费方捕获 |

状态不是 Issue 关闭结论。#15–#21 的“正式不可变 pin”受用户禁止提交/推送/tag约束；跨仓 demo 联调也不能用本地 TekesUI 测试替代。既有等价部分只补证明和指南，不重做。

## 实施顺序与负责人

先封存 Slider 公共组合批次（270 个证据文件；数值 AX 实际操作仍列 #5/#12/#14 未验收），再做 #15，之后 #16。#17–#20 共享设计与审查统一，实施顺序为事件/指针元数据 → 编辑命令与组字消费 → 受控同步事务 → 视口尺寸及组合场景；#21 随动作契约落地。原生 IME/AX 接口的交叉影响必须同步回归，不能凭单个 TextField 改善认定整个族完成。

当前所有共享模块由主 Agent 单一负责；本轮没有派发并行写入。独立验证可并发执行，但不并发修改共享文本、平台或组合实现。每批先完成本批公共 API、真实接入、适用 Gold7 的验证/证据，再切下一实现；未测体验明确保留。

性能暂不测；不提交、推送、打 tag、关闭 Issue或发布，不新增组件范围、不引入框架/业务 Agent 状态机。真实输入操作限 Gallery，系统设置仍须另行确认。
