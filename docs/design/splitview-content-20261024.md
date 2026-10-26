# SplitViewContent：双 owned pane 与 checked 生命周期

本地 main 的有限 A 接收，关联 #102、#85 的必要 generic core、交付入口 #136。原 Gallery Root 的 #87 A/B 验收保持原源码身份；本文件不将其转签为本地 main 验收。原完整设计及 amendment 的逐字节副本保存在 SSD 本批 receipts/original-design-source.md，SHA256 `61ad0e12427502abb469dffa6ae44a08c3930fdc336052ba9195f15ebf22d961`。

## 当前职责和公共边界

`SplitViewContent` 固定 owned leading/trailing pane 与末尾唯一 divider。Divider 持有原 `SplitView` / `SplitViewView`，原布局、拖动、取消及比例 reducer 继续唯一负责几何和交互；两个私有 slot 仅分配实际异构 `Content`，不引入第二套 split engine。调用者显式提供节点 ID。

`SplitViewConstruction`、`SplitViewConfig`、`SplitViewExpected`、`SplitViewReplacement` 经 Host typed 入口检查。非法配置、结构/身份损坏、重复 ID、过期 expected、预算或容量不足均在相应事务的首个副作用前拒绝；`Rejected<I>` 保留完整 owned 输入。Keep 不消耗身份。批量 mount 与 reading attachment 复用原 allocator 的 checked reservation；不得引入另一身份域。

`TableTransactionCompatibility::StableHooks` 是共享的显式语义合同，默认 Unknown；不是 DataTable 已接收或任意代码被沙箱保护的声明。Prepared lifecycle/action 与只读 Host reveal 保持独立责任：先只读准备/验证，再沿实际路径访问并提交。实际参与且无合同的节点拒绝，不要求未动的前方 Unknown 兄弟声明能力。普通 `ContentHost::new` 仍不发初始 activity 通知。

公共 Host 的普通既有路径须兼容未声明新 capability 的既有自定义 wrapper。新 prepared/typed 事务和显式 `requires_focus_reveal` 路径保持严格；普通路径不能借缺失 capability 绕过实际 stale、IME、身份/计数容量或几何拒绝。兼容修复沿原普通 reducer/reveal，不修改消费者或裁剪场景。

## 状态和文本

Tab 按 leading 子树、trailing 子树、divider 遍历。pane 替换/重挂、布局变化、取消和窗口失活退休原 capture/action；marked 状态在破坏性替换/拖动前拒绝。零 pane 保留逻辑文本 owner、模型、selection、marked 和 binding，但不发布旧 caret/clip/carets；恢复时重新布局。macOS 无有效几何的 point 查询返回 not-found，不猜当前 caret。

## 明确保留的后续范围

#103 负责真实 Splitter 本地/适用 wire 投影、默认与外部 Theme、原 View 对照和实际 GPU；本 A 不声称完整 #87、本地 main 的 B 或全库原生验收完成。DataTable、Tree、KeyedList、Popover 等尚未进入本地 main 的具体业务接入仍归原 Issue，本批不提供 stub。真实窗口、物理 IME、VoiceOver、性能与公开交付分别记录，不从编译推导。

## 本地接收验证

本批从 `078735c40e3d859ea456a93e6d4f5dddb6376469` 接收，由 root 唯一整合并在本地 main 验证。实际通过：原 SDK atomic/MAX 12 项（round02）；原 public A 20 项、pure guards 3 项、padding 21 项、普通 native-key/text-mount/reading/Toolbar 18 项、plain-text 3 项、prepared action 5 项及 focus/input MAX 1 项（round07）；Notice/EmptyState 29 项（round04，另有原显式 opt-in GPU 项未在该 CPU 轮运行）；API binding 3 项（round05）；macOS 零几何 1 项（round06）；Controls/macOS all-target 与 public consumer 严格检查（round08）。既有 Python 47 项及修正后的单项 stale-receipt 回归通过，生成器 `--check` 通过。

各结果保留其实际轮次源码身份。round08 相对 round07 只删除无调用者的私有 `admit_focus` 转发方法并刷新生成库存，未改变任何运行分支、配置或测试；只补原未完成的 strict，不重签旧程序。最终文档不参与 Rust 构建。共享 schema/Theme/渲染输入未变，未重跑既有 GPU；B 的 GPU 仍未验。源码/程序/dep-info/失败及各轮输入身份位于 SSD `artifacts/issue-136/split87-main078735c-intake/`；原证据身份不变，不因提交自动成为新运行。
