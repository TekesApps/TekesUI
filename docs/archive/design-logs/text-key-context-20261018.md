# #17 文本按键来源：第一批公共基础（2026-10-18）

状态：部分完成。根因属于基础交互原语和平台路由，不是单个 TextField 外观。已有 NSTextInputClient 回调、KeyEvent 和 TextInput/TextSurface 继续复用。本批没有增加编辑器、提交状态机或 UI 抽象层。

## 公共契约与边界

`Window::set_text_key_events(true)` 显式启用 `Event::TextKey(Started/Finished)`；默认关闭。带进程唯一 ID、原始完整 KeyEvent（修饰键、repeat、Down）和按键开始时的组字状态。KeyUp 仍通过原有 Key 事件保留。模式不能在进行中的按键链内改变。

Started/Finished 包围原有 FIFO 中的 Commit/Preedit/Unmark/TextCommand 或普通 Key，不重复包装或回放编辑。同步回调即时分发，保留 NSTextInputClient 查询语义。嵌套链由 ID 区分；没有活动链的异步回调不得猜成提交键。

Apple 的 [handleEvent 契约](https://developer.apple.com/documentation/appkit/nstextinputcontext/handleevent(_:)) 区分输入系统是否消费事件。本实现只调用一次：返回 true 不再发普通 Key；返回 false/没有交给输入系统时，仅在没有任何文本或命令回调的情况下回退。发现并修复了“已有回调但返回 false”可能重复处理同一物理键的问题。

Finished 明确区分 TextDispatched、CommandDispatched、InputContextConsumed、Unhandled、NotOffered。前两者只证明回调已发送，不能当作控件确认已编辑。Command 快捷键沿用既有普通键路径，标为 NotOffered。未配置任何业务发送行为。

适用范围是所有显式启用的原生文本宿主。合理例外：非文本焦点不启用链，旧宿主保持原有 interpretKeyEvents 路径；本批不扩大迁移。

## 分层验证

- 公共平台 API：36 项，包括完整元数据、嵌套/repeat 唯一身份、回调之外的来源未知、切换模式和 false-return 回调不回退。
- 共享控件/文本：359 项；独立公共消费者：57 项。独立原生示例启用并观察公共事件，仍复用原有控件处理回调。
- AgentKit：显式开发快照 `89b93cc78b49213498fa4977e1270fa6c2129cdc847e1d808ad27d0a8dc994b0`，UI 13 项、headless 76 项、严格 Clippy、Rust 所有权边界通过。没有修改业务状态或启动 AgentKit 窗口。
- Gallery 原生：同一真实 TextFieldContent 编辑器中，a 只增加一个字符；Enter 发出一条命令；Command-a 保留 Command 并记录 NotOffered；Shift-Tab 通过已有导航由 8604 到 8603。四份步骤 JSON、最终真实 GPU 窗口 PNG 和签名后二进制/源码指纹分开保存。
- Gold7：本批没有视觉/token 改动；原生截图核对字形、边界、裁剪和焦点。机械 Gold7 检查通过，不代表整库体验验收。8 份既有 token 探针全部输入哈希仍匹配，沿用原证据而不伪造新采样。
- 严格 Clippy、默认 Gallery 构建、组件能力和组合门禁通过。纯 Rust、参考对齐检查单独留证。

## 可复现补丁与未完成工作

`design/reviews/text-key-context-20261018/patchset/` 只准备第一批基础补丁：6 个公共平台/独立示例路径，附明确的既有 WIP 依赖归档与逐文件指纹。脱离 Git checkout 的独立目录 395 项回归通过，独立 consumer 的 all-targets 编译通过；子工作区独立 lock 和必要测试 fixture 明示列入。补丁应用后必须与被测快照逐字节一致。未执行 Git commit、push、tag、Issue 修改或发布。

这不是 #15–#21 完整提交集。下一步依赖：本批来源基础 → ContentHost 编辑命令确认/组字屏障/单多行与 Tab/提交意图（#17）→ 受控编辑事务（#18）与视口（#20）；#19 保留已有指针接入；#21 接公共动作目标/来源。#15/#16 的已有实现继续复用，消费侧原生和不可变公开 pin 条件仍需完成。

真实中文拼音、VoiceOver、AgentKit 原生和本批以外的 Gold7 矩阵未验收；性能按要求不测。完整 #15–#21 交付可构建可复现后，才移交 AgentKit 会话验收。
