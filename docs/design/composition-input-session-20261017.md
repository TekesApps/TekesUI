# #14 / #12 输入会话与候选坐标增量整改

2026-10-17，接续 [文本辅助契约与组字撤销批次](composition-ime-accessibility-20261017.md)。使用活跃 checkout 的 `bc3de79466834e99062a1b4ef0872323e2860001` 加全部 WIP。证据与冻结源身份见 `design/reviews/composition-input-session-20261017/receipt.json`。不扩大组件迁移、不测性能、不修改远端、不提交或关闭 Issue，继续 experimental。

## 根因、共享修复与适用范围

| 根因 | 修复位置与既有能力 | 范围与合理例外 |
| --- | --- | --- |
| 平台输入会话不同步：Rust 清除 marked range 之后，AppKit 转换会话仍可能保留旧音节 | `Window::set_text_input_state` 发布新快照后，在 marked 清除或 text owner 消失时调用原 NSTextInputContext::discardMarkedText，再更新坐标；闭窗后拒绝迟到的快照 | 所有使用现有公开平台文本快照的宿主，不仅本例。发布和原生回调之间不持有 RefCell 借用；没有新增会话管理框架或业务 ID 协议 |
| 公共编辑契约：内容节点失焦后，旧 marked state 仍能随 Dialog 重开回来 | `TextFieldContent::set_focus` 复用 TextInput::unmark 完成当前可见草稿事务，再清理 clause 与已有指针/滚动交互 | 所有 TextFieldContent。正常 blur 保留可见草稿，显式 Escape/undo 才恢复组字前状态；稳定 ID 重排仍保留正在编辑的实例和焦点，不视作 blur。没有给其他组件族新增迁移 |
| 平台候选坐标查询：非空字符范围被错误降为零长度插入点 | `TextInputState::first_rect_for_range` 复用已发布的 shaping carets，返回第一可见行片段及实际 UTF-16 范围；NSTextInputClient 将同一几何转换到屏幕坐标 | 非空范围有实际宽度；插入点按 AppKit 契约返回零宽、正高度。覆盖多行、可见片段、反向视觉 caret；无效范围、半个代理对、缺失几何不伪造其他位置。此辅助查询不是独立布局层；完整复杂双向文本及跨显示器定位仍需验收 |
| 基础编辑原语：原生 Escape 的空 marked 更新留下空 composition，继续拦截 Tab/AX 请求 | 共享 `TextInput::preedit` 在空文本且无显式 replacement 时取消原 composition；有显式 replacement 时仍按原 insert/undo 合同执行删除 | 所有 TextInput 使用者。取消会恢复原选中文字（含 emoji）及选区，不留下虚假空组字；没有已有 composition 的空更新不删除用户选择。只读/禁用合同保持 |

AppKit 来源：[discardMarkedText](https://developer.apple.com/documentation/appkit/nstextinputcontext/discardmarkedtext%28%29)、[firstRectForCharacterRange:actualRange:](https://developer.apple.com/documentation/appkit/nstextinputclient/1438240-firstrectforcharacterrange)。仅用于平台协议语义；视觉真源继续是仓库 JSON。

## 公共 API、真实接入与体验

**公共 API：** 平台单测验证 conversion 结束判断、精确 UTF-16 范围、首个可见行片段、零宽 caret、反向视觉位置与缺失几何。TextInput 单测验证空 marked 更新恢复 emoji 原选区、没有 phantom composition、显式 replacement 删除及撤销。独立消费者用真实 Rich Dialog 输入验证 blur/关闭保留可见草稿、重开无旧 marked、undo 按一次完成的事务恢复，以及原生空 preedit 后 Tab/AX 可继续。

**真实后端接入：** 独立原生消费者继续只用公开 controls/platform/text/render。AppKit 脚本直接查询非空范围和插入点的屏幕 rect/actualRange；与原有 AX 字符、选区、替换、marked 优先、取消/关闭脚本一起回归。旧 NSTextInputClient 精确范围脚本继续回归；这些合成脚本与真实系统 IME 观察分别保存。Gallery 仅测试 fixture 接入，生产页面未迁移。

首次增量脚本在窗口创建前执行了 Click，随后因没有编辑节点而失败。失败日志保留；原生 example 的脚本启动已改为 Event::Created 中 GPU/首次绘制就绪之后，重跑 9 项 AX/rect/组字检查及 8 项原有文本回调检查通过，不把首次失败或跳过步骤当通过。

**系统 IME 体验：** 修复前，在“草稿”末尾输入拼音 `n`、⌘Z、再输入 `i`，实际文本变成“草稿ni”，证明已取消的音节仍留在系统会话。修复后同路得到“草稿i”，Escape 返回“草稿”；随后 `ni` + 空格产生真实 `Commit("你")`，保留原草稿。切换至已经启用的日文输入法后，观察到假名 Preedit、转换为“日本”，再确认后产生真实 `Commit("日本")`。AX 文本、marked/selection 日志和窗口截图分别留证；不是直接粘贴中文冒充转换成功。

最终构建另验证：选择 emoji 后组字、系统 Escape 恢复 emoji 和原选区，Tab 能正常转到 Menu；组字时通过真实 Switch 操作失焦，保留可见 `n` 草稿并结束 marked，回到输入后可作为单笔事务撤销。中日文转换、取消、提交的日志均使用最终构建另行采集，早期版本记录仍保留。

系统候选窗进程的 CUA 查询返回 `noWindowsAvailable`，所以没有候选列表视觉位置/鼠标选候选证据；IMKCFRunLoopWakeUpReliable 警告仍出现，不能声称该警告已解决。已验证的会话残留与警告是否同源未确定。物理键盘、跨屏、缩放、窗口移动及所有中日文输入配置仍未完整验收。

**VoiceOver：** 本批只读检查确认字幕面板原本已启用，未改偏好；现有自动化观察仍没有可核对的朗读/导航结果，不将 AX 通过升级为 VoiceOver 通过。未启用 AppleScript 控制权限，未改变安全设置。输入源测试后恢复原拼音，VoiceOver 保持关闭。

## Gold7 与状态边界

- G1–G3：复用全部既有 JSON 表面、字形和焦点规则，没有页面局部视觉补丁；窗口样图与 GPU 回归仅用于此批接入。
- G4：既有 Dialog presence / Reduced Motion 回归；物理流畅度与性能不验收。
- G5：会话残留、空组字与 blur 三个共享缺口已修正；系统拼音和日文的上述实际路径通过。候选窗完整矩阵仍是部分完成。
- G6：文本范围几何与公共/原生查询已修正；VoiceOver 体验仍未验证，富文本辅助协议不因本批变为支持。
- G7：独立消费者、真实后端和统一回归继续成立；不晋级全部组件或全页 registry。

工作区最终 895 passed、0 failed、1 原有 ignored；独立消费者 13 passed。Clippy/fmt、DS、纯 Rust、Gold7 mechanical、reference alignment、能力清单通过，真实 GPU 回归 80 帧（4 主题 × @1x/@2x × 10 过渡状态）。完整身份与日志以收据为准。计时测试显式过滤，ignored 不算通过，机械门禁不代表最终体验。上述剩余项继续归 #14/#12/#5；不创建重复 Issue，不以局部改善关闭原任务。
