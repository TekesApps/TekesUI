# 共享文本导航与动作声明修复（2026-10-18）

根因属于基础文本交互和公共组件契约。TextFieldContent 把行首/行尾别名实现成文档首尾，没有共享的垂直导航；Gallery 已有 Paragraph 导航，但消费者未复用。受控草稿 ACK、软换行行尾和短行会暴露光标亲和、目标列及段落刷新问题。Combo 的 native selector-only 路由和文本输入 AX 角色推断又分别丢失菜单导航与动作种类。

## 共享位置与使用

原 TextSurface 新增 `command(&str) -> Option<bool>`，复用 Paragraph 的 hit-test、UTF-16 字素边界、现有编辑事务和 reveal。`Some` 表示命令被认领，bool 仅表示变化；边界 no-op 仍已处理，不能继续触发提交。未知命令、失效布局或组字隔离返回 None。多行视觉行首尾、上下移动与 Shift 扩选保留软行亲和和目标列；文档/单词/水平导航、删除、换行、撤销继续复用 TextInput。组字时只有显式 Cancel/Undo/Redo 走原有语义。Tab 和提交仍由既有 TextEditPolicy/宿主决定。

TextFieldContent、Gallery TextDemo 及 AgentText 使用同一实现。ContentHost 的 `event_with_text` 在既有焦点控件派发前后，借现有 ButtonTextRenderer/TextSurfaceHost 刷新段落，支持多个输入事件之间没有绘制的宿主。透明文本包装器转交 `prepare_text_input`；未提供文本后端返回既有 MissingTextServices。先完成初次布局；父容器几何变化后仍需 render 重新分配布局。本入口只刷新已安排文本槽位，不代替布局系统。旧 event 兼容保留，几何失效时不猜测视觉导航。

KeyFrame 复用 Content 的 mount 身份，阻止同 ID 组件卸载重挂后的旧键链触发新组件。Commit/Preedit/Unmark 交给实际聚焦文本所有者，组字期间不被其他 overlay 截获。非组字 callback-only moveUp/moveDown 由原 KeyFrame 元数据进入现有菜单原语。

原 #21 同时修正：Component 默认激活为 Activate，TextFieldContent 显式声明 Submit，透明 AgentText 包装器转交声明。AX 角色不决定动作；Combo 选项确认为 Activate。旧 #21 封存证据保持原样，本批追加编译失败/修复通过的回归。

## 适用范围与例外

TextField/TextArea/SearchField 的共享 TextSurface，以及现有 Combo/Command 查询包装器、Gallery 文本与 AgentKit Composer/只读 Transcript；未扩大迁移到新组件。单行不响应多行垂直编辑，Gallery 的 PageUp/PageDown 保持原阅读策略。逻辑水平字素行为沿用原实现；本批不声称完整双向文本/TextKit 等价。消费者业务发送、草稿归属、原 ReadingController 阅读保护、viewport、样式和 JSON token 均继续各自原职责。未新增类型、token 或抽象层。

## 证据与边界

旧 #21 全 WIP 闭包上的三个真实行为回归先编译后失败；相同公共 API 回归在当前实现通过。另有 Combo 动作分类的编译后失败/修复对照。独立消费者涵盖视觉行首尾、短行目标列、软行亲和、指针、ACK、无中间绘制、UTF-16 硬换行、元数据、IME 隔离、菜单以及同 ID 重挂。

AgentKit 实际 Composer 经原 controller/presentation 同步，测试连续导航、相同草稿 ACK 和无中间绘制；原 native host 已转交带文本后端入口。headless、UI workspace、host/第二消费者分别构建回归，未启动 AgentKit 原生窗口。

Gallery 使用真实 CoreText/Metal，窗口已确认位于外接 U2790B，scale2、报告 max_fps30；未测帧率。原生粘贴和键盘验证 5→8→14 的短行跨越、Shift 扩选、Cmd 行首尾、混合中英/emoji/组合字符的软行行尾及往返，保存 JSON/PNG。Gold7 核对焦点、输入边界、文字及反馈保持原共享规则，无局部样式覆盖。机械门禁另记，不能代表全主题体验。

8 组现有 token 探针重跑 4500 个有界案例，新增 24 个导航/只读案例（4 主题×3 品牌×2 状态）记录实际读取。`complete=false`，不是依赖全集。只读复用 surface-1，可编辑复用 transcript-input，未新增 token。

完整结果、源码/二进制摘要、WIP 保护与补丁重放见 `design/reviews/editor-navigation-20261018/`。原 #17 物理中文 IME、VoiceOver、AgentKit 原生验收和不可变公开 pin 仍未完成；pin 等待明确 Git 授权。无性能测试、提交、推送、tag、Issue 关闭或发布。
