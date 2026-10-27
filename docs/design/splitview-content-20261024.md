# SplitViewContent：双 owned pane 与 checked 生命周期

本地 main 的有限 A 接收，关联 #102、#85 的必要 generic core、交付入口 #136。原 Gallery Root 的 #87 A/B 验收保持原源码身份；本文件不将其转签为本地 main 验收。原完整设计及 amendment 的逐字节副本保存在 SSD 本批 receipts/original-design-source.md，SHA256 `61ad0e12427502abb469dffa6ae44a08c3930fdc336052ba9195f15ebf22d961`。

## 当前职责和公共边界

`SplitViewContent` 固定 owned leading/trailing pane 与末尾唯一 divider。Divider 持有原 `SplitView` / `SplitViewView`，原布局、拖动、取消及比例 reducer 继续唯一负责几何和交互；两个私有 slot 仅分配实际异构 `Content`，不引入第二套 split engine。调用者显式提供节点 ID。

`SplitViewConstruction`、`SplitViewConfig`、`SplitViewExpected`、`SplitViewReplacement` 经 Host typed 入口检查。非法配置、结构/身份损坏、重复 ID、过期 expected、预算或容量不足均在相应事务的首个副作用前拒绝；`Rejected<I>` 保留完整 owned 输入。Keep 不消耗身份。批量 mount 与 reading attachment 复用原 allocator 的 checked reservation；不得引入另一身份域。

`TableTransactionCompatibility::StableHooks` 是共享的显式语义合同，默认 Unknown；不是 DataTable 已接收或任意代码被沙箱保护的声明。Prepared lifecycle/action 与只读 Host reveal 保持独立责任：先只读准备/验证，再沿实际路径访问并提交。实际参与且无合同的节点拒绝，不要求未动的前方 Unknown 兄弟声明能力。普通 `ContentHost::new` 仍不发初始 activity 通知。

公共 Host 的原普通 action reducer 与只读几何能力分别处理。普通无 reveal 需求的既有路径保持兼容；需要滚动或显式 `requires_focus_reveal` 的路径必须有实际有效几何，不能因未声明 prepared-action capability 绕过 readonly reveal。可预测的自定义 translation wrapper 通过原 arranged pure query 接入；未知 layout/reflow 仍按 #98 原 AC 拒绝。新 prepared/typed 事务、stale、IME、身份/计数容量与几何拒绝保持严格。CurveLocal owner 通过实际 focus/reveal 门禁后可继续原 reducer，不伪造 StableHooks。

## 状态和文本

Tab 按 leading 子树、trailing 子树、divider 遍历。pane 替换/重挂、布局变化、取消和窗口失活退休原 capture/action；marked 状态在破坏性替换/拖动前拒绝。零 pane 保留逻辑文本 owner、模型、selection、marked 和 binding，但不发布旧 caret/clip/carets；恢复时重新布局。macOS 无有效几何的 point 查询返回 not-found，不猜当前 caret。

## 明确保留的后续范围

#103 负责真实 Splitter 本地/适用 wire 投影、默认与外部 Theme、原 View 对照和实际 GPU；本 A 不声称完整 #87、本地 main 的 B 或全库原生验收完成。DataTable、Tree、KeyedList、Popover 等尚未进入本地 main 的具体业务接入仍归原 Issue，本批不提供 stub。真实窗口、物理 IME、VoiceOver、性能与公开交付分别记录，不从编译推导。

## 本地接收验证

本批从 `078735c40e3d859ea456a93e6d4f5dddb6376469` 接收，由 root 唯一整合并在本地 main 验证。实际通过：原 SDK atomic/MAX 12 项（round02）；原 public A 20 项、pure guards 3 项、padding 21 项、普通 native-key/text-mount/reading/Toolbar 18 项、plain-text 3 项、prepared action 5 项及 focus/input MAX 1 项（round07）；Notice/EmptyState 29 项（round04，另有原显式 opt-in GPU 项未在该 CPU 轮运行）；API binding 3 项（round05）；macOS 零几何 1 项（round06）；Controls/macOS all-target 与 public consumer 严格检查（round08）。既有 Python 47 项及修正后的单项 stale-receipt 回归通过，生成器 `--check` 通过。

各结果保留其实际轮次源码身份。round08 相对 round07 只删除无调用者的私有 `admit_focus` 转发方法并刷新生成库存，未改变任何运行分支、配置或测试；只补原未完成的 strict，不重签旧程序。最终文档不参与 Rust 构建。共享 schema/Theme/渲染输入未变，未重跑既有 GPU；B 的 GPU 仍未验。源码/程序/dep-info/失败及各轮输入身份位于 SSD `artifacts/issue-136/split87-main078735c-intake/`；原证据身份不变，不因提交自动成为新运行。

Panel 接收批次（基线 `8511dad`，#81/#98/#136）进一步区分 Slot 自身的原 pane 边界与普通 custom descendant clip，仅在库内以真实 arranged/intact PaneSlot 查询还原 owner 裁剪；原布局和消费者断言不变。成功 typed update/replace 撤销既有几何 lease，Keep/失败不撤销。原 Split20、Host15、local-clip3 与 text-mount3 在该批 round12 实际通过；round11 的其余有效结果保留原身份，详见 SSD `artifacts/issue-136/panel81-main8511dad-intake/`。这不接收 #103 B 或原生完整发布链。
