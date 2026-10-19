# #10：公开阅读控制器与共享接入

2026-10-18，`bc3de79466834e99062a1b4ef0872323e2860001` 加活跃 Gallery checkout 的全部 WIP。开始时刷新全部14个开放Issue及全部状态PR（0个），原74条条件和后续任务保存在 `design/remediation/ledger.json`；#13总览、#14依赖、#12验收继续保留原职责。本批收据：`design/reviews/reading-20261018/receipt.json`。

## 根因与复用

#10缺口属于公共能力边界：Gallery已有阅读历史/跟随与UTF-16锚点保护，但策略写在私有Surface包装器里。现将原算法提取为可选 `ReadingController`，直接围绕既有TextSurface.layout_with运行，没有新增UI组件、token、计时器或另一套滚动实现。Gallery删除重复逻辑，原形状缓存与字体后端留在宿主。

默认history意图与原Gallery保持一致；显式follow_latest后，追加及重排跟随最新。observe_scroll在已路由的向上原生delta到来时、应用滚动之前停止跟随；手动到达底部、计时器和内容到达不会重新启用。history模式在append+宽度/字号变化同时发生时保留UTF-16行锚点及屏幕相对偏移，反复变宽/变窄不累积漂移。显式返回取消原捕获/滚动手势，不改文本、IME、选区或宿主焦点。

合理边界：这是尾部追加文本的阅读保护，不声称稳定标识支持任意中间编辑/段落重排；非尾部替换由TextSurface原钳制规则处理，history意图保持。原生滚动delta与组合ContentEvent文档delta符号相反，组合适配器沿原路径转换。普通输入框不自动启用，TextFieldContent.reading默认None；消费者可显式设Some，并沿原组件测量/绘制/事件路由使用同一控制器。宿主仍拥有内容源、回到最新的Button及网络/计时调度。

真实接入发现并修复两项共享缺口：

- TextFieldContent原本在wheel变更后无条件reveal旧caret，会撤销用户滚动。现所有实例的Scroll事件都保留滚动意图，其他编辑/选择仍按原路径显露caret；不是仅给阅读夹具打补丁。
- 共享Text/ScrollArea阅读动作行把Button焦点paint clip限定在内容边界。现使用真实祖先视口，命中仍为原按钮框；沿用既有中性focus ring，不改局部颜色/尺寸。

同时补齐现有ButtonContent和阅读动作的TextCommand insertNewline/insertLineBreak入口，直接复用Button Enter语义。它解决#3同一原生命令边界的遗漏，不创建第二状态机。实际平台是否投递该事件、物理键盘及VoiceOver仍需原生验收。

## 公共API与消费者证据

`reading_contract.rs`三项外部公共API测试，真实CoreText shaping：append+宽度/字号多次变化的逻辑锚点与选区，显式跟随、向上意图先于到达事件、手动底部不repin，Return释放捕获但不清空marked text。

独立composition-consumer使用原Rich Dialog的TextFieldContent，显式开启阅读，经过实际ContentHost Scroll路由、继续追加、Button TextCommand Return与活语义/AX Press动作；确认动作完成后控制器不改当前焦点，并验证普通未启用阅读的字段wheel也不拉回旧caret。消费者共18项通过（1 lib +17 scenarios）。AX Press本来会将焦点移到动作目标，这是宿主既有语义；follow_latest本身保留调用时的焦点。没有把该合成动作说成VoiceOver体验通过。

Gallery复用同一公开控制器，原两个Button和Tab停靠保留。Stream 4 updates是显式触发的有界消费方夹具：先到达一段，再由既有animate时钟投递三段，结束后不再调度；普通初始页面不持续追加。用户在中途读历史，余下到达不恢复跟随；失活时暂停夹具时钟。这是内容源模拟，不是组件内置定时器，更不是性能测试。

## GPU / Gold7 / 原生边界

真实Text与ScrollArea夹具共160帧：四主题、@1x/@2x、720默认字号/480大字号，initial/following/history/append-reflow/latest五阶段。每阶段断言选区保持；history到append-reflow断言锚点不漂移；following/latest断言offset等于实际range。32个latest案例额外检查焦点环在按钮右侧宿主gutter内确有像素，防止内容clip再次裁掉独立焦点。

`before-focus-clip/`与`gpu/`保留160对相同阶段图片及哈希；前态已使用公开控制器，唯一生产视觉差异是阅读动作使用旧clip。对应前态TextDemo源码副本单列保存。这不是旧远端bc3de79的全项目对比，也不是Gallery完整chrome/全页45组件验收。抽查浅色history与追加重排、深色IC大字号latest，必要边界/独立焦点、可读性和滚动裁剪可辨；G4未新增按压动效，G5/G6由实际阅读状态/原生命令模型与像素分别留证，G7指向同一公共API/当前清单。

物理鼠标/触控板、键盘焦点体验、系统IME与VoiceOver未在本批重新操作，按#12继续unverified；不因模型、语义树或GPU证据将组件成熟度晋级。#10当前是已实现待验收。

## 门禁与后续

控件/tokens/design-export默认298项测试通过；Gallery reading过滤8项回归及单独160帧GPU验证通过，后续reading过滤还包含新GPU测试（9项）；消费者18项、治理32项、默认workspace与token-audit严格Clippy、fmt、DS、Gold7机械、reference alignment、纯Rust和生成检查通过。ReadingController明确登记为现有Text/ScrollArea的公共辅助能力，类型反向覆盖181条，组件仍45项/269项状态要求。输入与选择族源绑定观测实际重跑612/2016例并另存，不修改旧收据；完整token依赖仍未核验，不将其推断为完成。

全部旧WIP/历史review按基线哈希保护，本批有意修改文件另存前态与源overlay。没有提交、推送、tag、关Issue、发布或性能测量。原始导入包与JSON设计真源不变。

下一项#11：在既有原子持久化适配器和真实编辑器上补齐公共身份、版本、不可变提交及确认协议；存储/网络仍归消费方。#7、#14完整矩阵与其余组件族继续在总账中排队；局部原生阻塞不停止其他实现工作。
