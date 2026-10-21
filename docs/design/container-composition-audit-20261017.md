# #14：Container Composition 当前架构审计

> 当前全45组件矩阵、源行号、兼容性和执行依赖见 [2026-10-20 当前审计入口](composition-audit-current-20261020.md)。下文仅保留旧 Container 时点，不作为当前实现缺失或阻塞结论。

> 2026-10-17 更新：本文件是旧 Container 范围的审计记录，尚不满足最新 #14 的“全公开组件、Button 复合内容及三个独立消费者最小场景”要求。最新范围与未提交验证试验见 [最小协议记录](composition-minimum-protocol-20261017.md)。不能以本矩阵声明最新 #14 审计完成。

核对对象：`bc3de79` 加活跃 Gallery 工作树全部未提交改动。源码身份与本轮验证见 `design/reviews/issues-20261017/`；不是仅审计远端 main。未引入组件、框架或平台依赖；本文件不表示组合架构已经实现。

## 当前矩阵

“宿主接线”指消费者可以手工调用布局和绘制，但组件本身并不持有或路由任意子组件树。不能把它算作通用组合接口。

| 容器/类别 | 当前公共实现 | 任意/异构内容、消费者扩展 | Slot/状态/动态尺寸 | 事件、焦点、裁剪与无障碍 | 状态 |
| --- | --- | --- | --- | --- | --- |
| Stack / Layout | `extended.rs::Stack::layout(bounds,count)`；`layout_view.rs::StackView` | 只返回矩形，宿主接线 | 无子树；按数量均分，无子组件测量 | 无递归路由；宿主自行处理 | 部分完成 |
| Grid / Layout | `extended.rs::Grid::layout`、`layout_view.rs::GridView` | 只返回矩形 | 固定行高，无子树/具名槽 | 同上 | 部分完成 |
| SplitView / Layout | `layout_view.rs::SplitView` | leading/divider/trailing 几何，宿主接线 | 拖拽/边界已有；无 child ownership | 局部拖拽/焦点已有；子树路由未实现 | 部分完成 |
| ScrollArea / Viewport | `scroll_view.rs::ScrollView`（公开别名 ScrollArea） | 滚动状态与几何，无内容协议 | viewport/offset已有；状态卸载策略缺 | 内容裁剪/事件由宿主接线 | 部分完成 |
| List / Selection | `extended.rs::List`、`collections_view.rs::ListView` | label字符串与内置 Button 行 | 无任意 row content；无稳定 child key | 行选择/键盘已有；子控件消费与状态保留缺 | 部分完成 |
| Tree / Hierarchy | `extended.rs::TreeNode`、`collections_view.rs::TreeView` | label/parent/expanded/disabled固定字段 | 无自定义内容子树 | 层级/键盘已有；子树事件与AX缺 | 部分完成 |
| DataTable / Data | `collections_view.rs::DataTableView` | headers字符串、rows字符串二维数组 | 无 cell/header slot；固定模板 | 排序/行选择已有；任意cell事件缺 | 部分完成 |
| Tabs / Navigation | `composite.rs::TabsView` | labels与Button；无panel子树 | 无自定义标签/面板槽 | roving focus已有；面板身份/AX关系缺 | 部分完成 |
| Breadcrumb / Navigation | `layout_view.rs::BreadcrumbView` | 固定层级标签，非自由内容容器 | 无自定义label slot | 导航语义须保留；仍借Button呈现 | 部分完成 |
| Menu / Semantic overlay | `composite.rs::MenuView` | 动作项字符串/内置Button | 菜单项、分组、子菜单语义应约束扩展 | 键盘/滚动/生命周期已有；不得据此声称AX已接入 | 部分完成 |
| Popover / Free overlay | `feedback_view.rs::PopoverView` | 固定body文本/模板 | 缺自由content子树 | 弹层生命周期已有；复杂内容焦点缺 | 部分完成 |
| Dialog / Modal | `composite.rs::DialogView` | 固定title和两操作Button | 无header/content/footer通用槽 | modal scope/焦点恢复已有；任意表单内容与AX缺 | 部分完成 |
| Disclosure / Disclosure | `layout_view.rs::DisclosureView` | label和展开几何 | 无通用body槽 | 展开动画/裁剪已有；子树路由缺 | 部分完成 |
| Toolbar / Content actions | `widgets.rs::Toolbar`、`feedback_view.rs::ToolbarView` | 等宽label/Button内容行 | 无stable item ID、placement/group/overflow | 不是Window拥有/注册的Toolbar | 部分完成 |
| Surface / Presentation | `paint.rs::SurfaceKind` | 装饰绘制原语，宿主可在上方画任何内容 | 不拥有children，不声称是内容树 | 不提供子树事件/AX | 已实现待验收（绘制原语） |
| Accordion / Sidebar | 当前registry无独立公开组件 | 未登记；不自动增加范围 | 不适用 | 不适用 | 未实现（审计候选，非本轮新增需求） |

这些路径均相对 `crates/tekes-ui-controls/src/`。完整文件字节由源清单冻结；行号会随本轮未提交修改变化，符号是定位锚点。

## 公共契约与职责边界

当前没有可供消费者直接使用的通用 Component/子节点协议。先在现有 Rust 控件之上增加组合层，不重写渲染器或替换UI栈。契约应一次覆盖以下边界，并由单一负责人维护公共核心：

1. 稳定身份与状态：节点key独立于数组位置，保留编辑、选区、IME、焦点和局部状态。重排不重建；显式替换/卸载才触发生命周期。虚拟化须明确存储与取消策略。
2. Measure/layout：容器测量/排列child，child报告自身约束；支持异构、递归、可变尺寸。保留已有简单字符串/布局便利接口。
3. Paint/theme/clip：复用Scene、公开文本适配器与JSON生成Palette；祖先viewport裁剪由公共遍历传播，焦点paint outset不扩大hit target。
4. Event/focus：命中child优先，消费后阻止行激活；区分键盘/指针capture与取消。容器持有必要导航策略，child持有局部行为。模态scope、隐藏/disabled过滤与有效调用者恢复共同定义。
5. Lifecycle/invalidation：挂载、更新、重排、卸载与失活的取消边界明确；host拥有时钟/调度，组件只声明失效。
6. Accessibility：稳定节点、父子关系、role/name/value/state/bounds/action与同一棵实际控件树绑定。公共语义协议之后接macOS桥，不用另一个演示状态机替代。
7. Slot：header/content/footer、leading/center/trailing、label/accessory等为便利层，不能用封闭ItemVariant枚举限制内容类型。

通用容器允许任意内容；ListBox、Tree、Tabs、Breadcrumb、Menu保留各自选择、层级、标签/面板关系或菜单语义；Button不默认嵌套独立交互控件。

## 顺序与A–F验收

依赖顺序：公共身份/事件/测量契约 → Stack/Grid/ScrollArea递归组合 → 自由内容Dialog/Popover与Tabs/Disclosure槽 → List/Tree/Table内容槽并保留集合语义 → Window级Toolbar注册/生命周期/placement/overflow。交互原语、语义与呈现解耦可先设计，落地共享模块由一个负责人实施。

| #14场景 | 当前状态 | 依赖/所缺证据 |
| --- | --- | --- |
| A 异构List（含消费者自定义组件） | 被依赖阻塞 | 公共child协议与List content slot；当前labels不能替代 |
| B Stack→ScrollArea→List→复合Item | 被依赖阻塞 | 递归measure/layout/paint/event/clip traversal |
| C Rich Dialog | 被依赖阻塞 | 通用内容/具名槽及模态子树焦点 |
| D 自定义Tabs label/panel | 被依赖阻塞 | Tabs槽、稳定label/panel关系 |
| E 独立消费者Window Toolbar | 被依赖阻塞 | 组合/交互基础与window registry；内容顶端Button行不合格 |
| F 动态生命周期 | 被依赖阻塞 | 稳定key、状态保留/卸载协议及真实IME/AX验证 |

当前不能宣称breaking change已发生：本轮未替换公共组合API。后续应采用添加型API与现有字符串便利接口迁移，分别验证旧消费者及新增独立消费者。#14保留架构入口，不为每个容器创建重复Issue；#5跟踪真实系统辅助技术桥；#12统一验收。
