# Link 公开模型的禁用观察与旧指针取消

本轮以全部当前WIP为基线。此前可用性审计验证了 LinkView，却没有覆盖直接调用公开 extended::Link 模型的这个入口；旧结论不能推广到该路径。本轮不新建Issue，不改已关闭#21的公共组合动作队列范围；状态/能力核验接原#8及原生#12，Link语义另记原#5剩余项。

根因：Link.key 在非Enter过滤之前没有调用已有Button的激活同步。指针已按下→disabled=true→Space/Escape/Tab/导航键进入公开key→disabled=false→旧pointer_up，可能保留旧按下。本轮先记录 pressed仍为true 的真实红测，再修复。

共享修复：widgets.rs 现有 synchronize_activation 方法仅扩大为pub(crate)，extended.rs 的 Link.key 在Enter过滤之前复用它。没有新增对外API、token、状态机或抽象层。全部直接公开Link模型消费者受益；已有LinkView同步逻辑保留并与模型并列验证。

合理例外：Link只由Enter激活，Space/导航键不消费、不形成Space arm；正常可用时无关键不得取消合法指针按下。完全未被任何事件/绘制/API边界观察到的临时disabled脉冲无法被库检测。Link没有loading呈现契约，本轮不新增。窗口失活仍由宿主转发取消/失焦，不能把没有转发的宿主能力假装成库自动行为。

公共API：8个非Enter键×Down/Up共16个禁用观察案例；核对取消旧指针、旧release无动作、新点击有效及正常无关键保留按下。controls344项通过。真实消费者：四主题、三品牌、两宽度及16键阶段组合共384组，直接模型与实际LinkView并列；使用真实CoreText度量和原生产Link绘制原语，draw记录不替代字形GPU。consumer35项通过，两个包严格Clippy通过。

GPU：48个四主题/三品牌/配对280pt大字和640pt默认/@1x/@2x场景，共288帧。Escape-down为有界代表；48组观察取消与显式cancel像素相同，48组旧release保持绘制及无动作，48组新点击有效。保留held/disabled/显式cancel/恢复/旧release/新held六阶段；字色token允许同值，不把像素相同当作每个状态都有唯一颜色。抽查普通浅色held/恢复和深色IC/品牌2/窄大字恢复。没有更改Link呈现或引入按钮外观。

外接屏原生：用户物理操作并确认表现正常，事件和操作者证据分别保留。两次同一场景重复链：68按下→69原生Escape→71旧松开→73新按下→74新松开，计数0→1；75→76→78→80→81，计数1→2。每条全程窗口活动；同一Escape回调的禁用前/后/恢复记录证明pressed先清除、父键继续、没有中间绘制后才取消的伪证。更早33/36的点击在可见frame外，模型从未arm，不计通过或产品失败。

AX边界：平台 AccessibilityRole 完整枚举和公开 ContentRole 当前都没有Link角色，当前小宿主未伪装成Button。此次键鼠时序不能证明Link语义或VoiceOver；缺口继续记入#5/#12，VoiceOver按用户要求保持关闭。后续先在现有桥及实际发布器补正确Link语义，再单独验证原生角色/真实动作；不因本次状态修复晋级整个Link或动作族。

九个实际token探针重跑4416个案例，complete=false；42项治理、8项组合审计、45组件矩阵及格式通过。仅4个原有受保护源文件有意改变，其余559项保持；生产改动是既有helper可见性及一行调用。原AgentKit冻结包和公开/准备候选不改。本轮不提交、推送、打tag、测性能或写Issue。

清理：小宿主包产物在签名复制后清理；原生完成后关闭窗口、确认PID58656消失、核对签名哈希并新鲜检查无占用后清理复制App。源码、288张GPU图、日志、签名和哈希清单保留。入口：design/reviews/link-model-availability-20261020/acceptance.json，原生：design/reviews/link-native-availability-20261020/native/acceptance.json；verify-native.py只读复核事件顺序，不生成输入。
