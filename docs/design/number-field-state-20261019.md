# NumberField 既有步进器状态整改（本轮有界验收完成）

本轮只处理已登记的 NumberField，不把它扩成可编辑数字输入框。根因属于公共组件事件契约：
两个内部 Button 缓存上次绘制的边界可用性，模型在事件间变化时键盘仍读取旧禁用值；
在数值边界上观察到不可用状态时又没有取消旧 Space arm，回到可用范围后可能迟到激活。
整体 disabled 后仍可重新记录焦点是同一可用性入口遗漏。

公开 API 三项红例均实际失败；修复使用 NumberFieldView 私有 sync_model 与既有 Button.cancel，
在绘制、带模型的键盘/释放入口及成功改变数值后统一同步。没有新增公开 API、状态存储、token 或抽象层。
适用于所有使用该 View 的调用者，含增减方向和 Enter/Space/方向键。旧 pointer_down 不接受模型参数，
仍以最后绘制的命中几何/可用性为前提；没有悄悄改变它的签名或自动重画。

独立消费者将两个普通 Content 焦点/语义节点委托给同一个原 NumberField 模型和 View。
节点不绘制另一套控件，也不复制激活逻辑；Tab/Shift+Tab、capture 与挂载继续由既有 ContentHost 负责。
边界上的不可用按钮和整体 disabled 两按钮均从 focus targets 排除；AX Press 走原 View.key_event。
这是调用者 newtype 的真实接入，不能据此声称库已新增内置 NumberFieldContent 协议。

五个公开测试（含四主题/三品牌的60个真实 View 状态案例）、消费者全测及严格 Clippy 通过；
七组既有 token 探针因共享源码变化全部实际重跑。NumberField 的12个查询ID仅为有界 accessor 观测，
complete=false，不包括编译常量、字体后端和未执行路径。原有48/12/80/10/48的显式步进器尺寸仍是旧布局约定，
本次没有凭查询表宣称它们全部已成为真实 token 依赖。没有改普通输入字段的 hover/pressed 外观。

160个实际GPU帧覆盖四主题、@1x/@2x、140/198pt、100%/150%和五状态；字形、稳定槽位和取消后不改值均检查。
源码快照见 design/reviews/number-field-candidate-20261019/manifest.json。
新快照保留全部本轮 NumberField 增量，以此前已验 #23 源码为底；并行Scene3D WIP留在活跃工作树中，未纳入此组件验收。
#23的旧冻结包和AgentKit复审输入不变。number-field-native-20261019 已在外接屏 U2790B 完成16个活动原生检查点：
Tab/Shift+Tab正确跳过不可用按钮；真实按钮、Space、Enter和方向键改变原模型；达到上限后不能继续增加，
减小后可再次增加；整体禁用两按钮不改值、不取焦点；360pt窄窗内布局可读。
一次边界快照短暂失活，保留但不计活动证据，后续活动指针检查重新确认上限状态。
原生、GPU与有界Gold7证据分别登记，receipt 为 accepted/cleaned。窗口已关闭，专用构建/测试产物清理，
保留公共缓存、源码与证据。VoiceOver未开启，性能不测；未提交、推送或关闭Issue。

这只接受本轮 NumberField 公共状态修复及调用者接入，不代表 #1/#3/#5/#8/#12 或整个组件族关闭。
既有固定布局尺寸、内置 Content 适配缺口及统一视觉验收继续保留在原账本。
