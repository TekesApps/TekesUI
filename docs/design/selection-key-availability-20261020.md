# Checkbox / Switch：无关按键也观察当前可用性

本轮基于活跃 Gallery checkout 全部 WIP，属于八批计划的动作／选择族后续。原 #3/#8/#12 验收继续处理，不新增重复 Issue，不修改 AgentKit 的 formal431、公开61dd或本地532冻结源。

根因属于已有公共交互契约：Checkbox 只在 Space 时调用共享 Button.key；SwitchView 在检查 model.disabled 前先过滤非 Space 按键。按住 Space 或指针后，如果调用者禁用控件、随后控件收到 Enter／Escape／方向键／Tab，再重新启用，原来的释放可能复活旧操作。两个公开回归修复前均失败。

共享修复仅在 widgets.rs 和 selection.rs：Checkbox 所有按键先交已有 Button 同步可用性，仍仅 Space 激活；SwitchView 先复用 cancel 清理旧指针／Space，再保持非支持按键返回 Ignored，不吞掉父级导航。没有增加状态机、原语、token、样式或组件抽象。所有直接使用这两个公开控件的消费者都受益；现有 Gallery／SwitchContent 继续复用原实现。Checkbox 的内建 Content adapter 缺口没有被本轮修复掩盖。

合理例外：Switch 没有 loading 契约；Checkbox 保留已有 disabled/loading。无事件、无绘制观察的短暂外部状态变化不能由控件获知；不带模型参数的 pointer_move 也不是模型可用性观察入口。Enter 不因同步 Button 而变成 Checkbox 激活键。其他已有动作／选择实现先对照当前代码：Button、IconButton、Link、Slider 已先同步可用性；RadioGroup 先撤销失效捕获，SegmentedControl 先处理失效焦点项；没有为追求统一而修改它们。

公共 API：40组 Checkbox、20组 Switch 新事件组合通过；相关两目标12项及 controls 全部341项通过。真实消费者：复用现有 CoreText 测量和生产 View，四主题、三品牌、指针／Space共24场景通过，整个 scenarios 33项通过。测试中 CheckboxLayout.frame 是字段，Switch 命中须使用其真实 layout.frame；初次夹具的这两处错误及日志保留，没有冒充产品失败或成功。

controls／真实消费者严格 all-target Clippy、格式、42项能力治理测试、8项组合审计测试、45组件当前矩阵通过。源码变化后九组真实 token 探针全部重新采集，共4416个有界案例，complete=false。矩阵重新核对四个已固定输入和未变化锚点，没有提升组合容量或原生成熟度。557个无关源码输入字节保持不变。

原生窗口、物理按键及最终 Gold7 体验仍未验收。现有消费者的文字 draw 是空实现，因此这轮契约测试不称作 GPU 像素证据；未改变绘制规则，旧 GPU 收据保留原身份。Gold7逐项边界见本轮 gold7.json。此前等待激活的文本族小宿主字节未更改，也没有拿来替代本修复的原生证据。

已删除61个确认未使用的测试可执行文件，保留共享库／依赖缓存、源码、before快照、五文件 implementation.patch 和红绿日志。未测性能，未提交、推送、打tag、合入或关闭Issue。

证据入口：design/reviews/selection-key-availability-20261020/acceptance.json。

后续真实 GPU：`design/reviews/selection-key-gpu-20261020/acceptance.json`。独立两控件小宿主复用原 CoreText／raster／atlas 后端，1440帧／288场景通过；288次显式 cancel 逐像素相等、288次新选中值可见变化。四主题、三品牌、两倍率、两组宽度字号、指针／Space和三种可用性情形覆盖；Reduced Motion=true。保存648张PNG及散列，查看了四张窄窗大字前后图；不是整矩阵人工验收。宿主严格Clippy、格式及全部注册包与当前root lock身份一致，通过后清理该宿主package构建产物，保留公共缓存和证据。没有启动新原生窗口，不改变原生／最终Gold7待验状态。

最终原生收口：同一已签名候选在U2790B@2x、Light／640pt下，三组Space和三组指针链通过。用户分别确认“完成，一切正常”／“一切正常”；AppKit日志逐项核对真实Down→Escape→Up及后续Space、整个链活动状态、旧值保留、父级继续处理、新激活可见。旧进程66条日志及CUA串行导致的无效尝试明确排除。阶段性“原生未验”记录保留为历史；最终入口为 `design/reviews/selection-key-gpu-20261020/native/acceptance.json`。本项共享修复的限定Gold7验收通过，不升级整族或整页成熟度。CloseRequested/closed日志和PID消失已核实，原生runtime经lsof与哈希检查后删除；源码、操作确认、GPU及原生日志保留。VoiceOver本轮未开启，性能未测，无提交推送／Issue操作。
