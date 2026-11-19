# Select 独立后台刷新消费者

基于bc3de79加全部WIP。GitHub当前14个开放Issue、0个全状态PR，更新时间与账本一致；继续原 #3／#8／#12／#14，不关闭原Issue。

## 根因与共享契约

独立消费者要让Select菜单进入已有ContentHost的overlay绘制阶段；原SelectView只有合并render，无法在基础内容和反馈之后单独绘制菜单。新增render_trigger／render_options，原render保留并委托同一实现，仍由原Select/Menu/Button持有状态、事件和生命周期。没有新增UI框架、异步状态类型、token或通用协议。新分阶段入口在overlay前选项被清空时也按当前labels/count取消，不绘制空菜单；这一回归修复前已编译运行失败，修复后通过。

公共兼容回归覆盖四主题×三品牌×打开/关闭24条路径，比较合并与分阶段入口的Scene、布局、选中值和打开状态；choice_contract共16项通过。原入口继续可用；仅需要组合弹层的调用方采用阶段入口，不展开全库迁移。

## 独立消费者与合理例外

examples/composition-consumer新增choice_status场景，用原SelectView和既有Component／ContentHost／Stack实现接入；复用既有应用Status、ProgressView、BannerView及Feedback，没有库级异步Select协议。Feedback的Error尺寸改用现有BannerView.measure，Loading按其既有56点绘制几何测量；原Tabs/Disclosure面板仍按其原面板布局接入。

后台刷新有缓存选项时，Loading／Error／Ready切换保留同一个组件、选中值、打开菜单、候选和键盘焦点，缓存选项仍可操作。错误在独立Banner节点中表达，选择字段本身不标为invalid；输入验证继续使用原set_error，两者不混同。

初次/空数据加载仍没有可选值，触发器不可用；数据到达不主动抢焦点。示例用唯一标签作为领域身份：重排时映射原选中值，遗漏当前值或重复值的响应原子拒绝并保留缓存。实际替换数据会复用原cancel结束旧菜单和按压，旧坐标释放不得激活新条目；仅状态变化则保留候选。这是示例的数据策略，不宣称字符串标签是通用稳定ID协议。调用方仍需检查响应接受结果。

消费者触发器采用既有Button固有测量宽度，反馈占可用宽度；原Gallery固定宽度样例保持原布局。这是调用方布局选择，不改共享视觉样式。确定性场景设置Reduced Motion；完整动画体验未验收。

## 验证证据

- 独立消费者2项新增测试证明上述状态流转、语义区分、真实选择、候选/焦点保留、数据身份重排、拒绝更新和旧按压隔离；还断言原ContentHost把Select菜单画在Banner之后。消费者总计36项通过。
- 控件/token/design-export共345项通过；Gallery原Select真实回归4项、完整36/76确认回归、窄宽正常/大字号矩阵通过。根和消费者all-targets严格Clippy、两套fmt检查通过。
- 默认构建首次发现审查样例引用未加既有feature门禁；已按composition-review/test边界接入，默认cargo build与审查cargo build分别通过。Gallery新增两个样例只在composition-review或测试构建中出现，默认Gallery不自动扩入这一实验消费者。原生证据明确使用审查构建。
- 当前7组token观测重跑：228/120/192/576/612/2016/408条。228由原Select/ComboBox的204条与独立Select后台加载/失败24条组成；所有源输入哈希匹配当前WIP。仍complete=false，不把实际读取当完整依赖提取。公共测试使用CoreText排版、空字形绘制，不当像素验收。
- GPU四主题×三品牌×两种比例，共144张当前Select整页；与select-availability冻结的同组件144张较早参考逐张对照均变化，原因是增加独立加载/失败样例和测量重排。不是本批即时旧基线重跑。代表性Light窄宽150%及Dark IC品牌2@2x核对层级、反馈、焦点和裁剪。
- 原生只操作带composition-review的Gallery，观察fill宽度及应用内420点/150%预览下的加载、失败样例；保留签名后二进制与源输入哈希。它证明审查构建真实绘制接入，不能替代原生实时异步交互。AX仅窗口/菜单，未验收控件AX、VoiceOver或物理IME。Gallery已退出，不改系统设置、不测性能。
- 能力门禁37项、组合门禁6项、reference、Gold7机械检查、pureRust通过。真实体验与机械门禁分别记载。

公共状态121项，fixture缺口135项，required269不变。选择profile剩余4项：ComboBox loading/error、CommandPalette loading/error；Select原生实时异步交互、完整动效和统一AX/VoiceOver体验仍保留未完成。下一步复用现有TextFieldContent/TextSurface为可编辑选择消费者补齐查询/IME与异步反馈接入，不复制输入状态机。

源差异、输入哈希、测试/GPU/原生证据及WIP保护见design/reviews/select-async-20261018/receipt.json。历史冻结证据保持不变，不提交、推送、打tag、关闭Issue或发布。
