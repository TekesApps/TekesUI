# #19/#21 最终候选指针与动作补验

基于全部最新WIP和P0收口之后的同一公共候选4a02d30、真实AgentKit二进制59dd663。未改生产代码、token、AgentKit主checkout或原冻结快照。仅复用TextSurface已有选区/scroll.reveal/capture/cancel、ContentHost原始指针元数据和公共动作队列。根因分类分别为基础交互原语/公共组件契约；本轮没有发现需要另加抽象或重复实现的共享缺口。

## 自动与真实接入

公共action_contract7、pointer_contract2通过；实际AgentKit content_actions3、native_pointer1通过。前一精确候选完整独立消费者130及UI27/headless76结果保持源码绑定；本轮仅重跑上述相关测试。实际原生宿主运行在U2790B、2×、Light，未开启VoiceOver，没有性能测试。

Composer输入24行中文、emoji与组合字符e+重音。真实指针从第23行附近向上拖到Composer可见区域之外，仍由Composer捕获；视口从第13行向上滚到第10行，选区延伸至视口外的文档位置。原生AX返回选中文本。以该文本计算期望替换，输入X后的全文精确相符，undo恢复原始全文；选区内中英文、emoji、组合字符未损坏。

次按钮分别点击编辑区及发送按钮，全文和所选文本保持逐字相同，未新增消息。原Shift-click、双击、三击与字素边界的公共/原生证据继续有效。适用于通过现有TextFieldContent/TextSurface的Composer及只读Transcript；没有扩大迁移到其他组件族。

#19仍为已实现待验收：本次真实出界拖选/自动滚动/次按钮补证完成，但“保持主按钮按下期间失焦或取消，恢复后无残留拖选”的现场组合尚未执行。公共协议及真实字段消费者的Cancel测试不替代这一原生组合。

## 动作契约

确认按钮焦点后，Space与Enter分别激活会话B。原始指针点击发送P21_POINTER_ONCE、聚焦发送按钮后Enter发送P21_KEYBOARD_ONCE，实际Transcript各出现一次；公共测试另验证正确ID/source、顺序、drain、重建不重放和回调重入边界。

A发送P21_SCOPE_A后立即选B：B中没有该消息，返回A恰有一次对应消息及完整响应。B的停止按钮不可用，点击无动作；宿主正常窗口CloseRequested退出，原wake及clone均Closed。这些是真实消费者行为证据；不把CUA操作冒称为VoiceOver朗读验收。

探索过程中会话变化会清空旧组件焦点，Shift-Tab从空焦点环绕到发送，而不是先前假定的会话A；因此一次空提交被AgentKit既有业务规则拒绝。原始记录保留，不计作Enter切换通过。正式通过记录是keyboard-enter-confirmed.ax.txt，操作前已确认焦点为会话B。空草稿发送可用性属于消费侧业务策略，本轮未改其规则或冻结快照。

#21功能验收证据已补齐，整体仍待AgentKit正式提升候选/确认和公开不可变pin。旧VoiceOver会话B激活/停止不可用的人声反馈有版本边界，#20文本独立进入/逐行仍未通过，不随本轮提升。

## Gold7及清理

G5验证选区、自动滚动、草稿/目标隔离；G6验证真实拖选、次按钮及键盘入口；G7公共契约、真实消费者、回归与原生记录分别可追溯。只覆盖这些场景，不宣布完整Gold7通过。无视觉源变更，沿用本轮P0精确版本相同的GPU图，不伪称另做全主题视觉矩阵。

证据位于design/reviews/pointer-action-native-20261019和design/reviews/component-runs/agentkit-pointer-action-20261019-125444。小宿主已关闭，fresh lsof及哈希检查后清理12个确切构建产物；源码、截图、AX、事件日志及共享依赖缓存保留。未提交、推送、打tag或关闭Issue。
