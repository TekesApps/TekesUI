# Panel：已有共享规则的宿主倍率漏接

根因属于宿主到公共组件的适配契约。`paint::surface_at_scale`、`SurfaceView::render_at_scale` 和 `ButtonTextRenderer::backing_scale` 原本已经存在且正确；若调用兼容入口 `surface`／`render`，则按1x处理。多个运行／示例调用点仍走该入口，普通@2的装饰线因此成为2个物理像素。

本轮没有修改共享controls库、设计token、公共API或状态协议。修复统一转发现成的renderer/text backing_scale，继续由同一个共享装饰边界解析决定颜色、线宽和IC例外，不在页面覆盖颜色或手工指定0.5pt。

适用调用覆盖Stack/Grid预览单元及variants、Disclosure展开内容、SplitView预览面板及variants、EmptyState状态／变体面板和测量探针、两个独立组合示例的私有Badge，以及测试夹具中的Disclosure面板。必要控件边界、独立焦点和可拖动SplitView分隔条不调细。旧Canvas示例没有装饰边界，保留兼容调用不会改变线宽。旧API默认1x的兼容行为保持原样。

验证分层：

- 真实接入：两项Gallery回归和一项独立消费者回归在修复前真实失败。当前Gallery／独立消费者／新API观测／原生宿主严格Clippy通过，169项消费者测试通过。两个真实组合工厂的16个CPU场景覆盖4主题、@1/@2；文本输入使用实际CoreText服务，CPU图标ID只用于几何测试，不冒充图标GPU验收。
- 实际Gallery GPU：7个布局展示路径和2个EmptyState路径，共4主题×3品牌×两倍率×9路径＝216场景、432当前帧。修复前场景／规则参考原样保留。当前每幅实际画面与此前仅按既有规则修正装饰边界的参考逐像素相同；54个普通@2场景改变，162个其余场景与原画面逐像素相同。布局、文本、真实祖先裁剪、必要边界和分隔条没有因本轮修复移动或调色。GPU窗口范围有界、字号为正常值，不能据此宣称所有完整页面、大字号或物理显示矩阵通过。
- 公共API及token：新增192个实际捕获，Stack几何96项零动态lookup与Surface绘制96项明确分开，子项依赖不归入布局组件。当前16份有效观测共6120有界场景，complete=false；7份输入字节未变的证据经当前哈希复核后复用，8份受影响证据实际重跑，新增1份。不是改写旧报告制造新通过。44项治理、8项组合、3项生成绑定通过。
- 库验证边界：本轮controls库源文件与Grid轮次的419项验证输入相同，已复核源码哈希；该419结果作为已有验证引用，不虚报为本轮重跑。新增API观测和调用方回归单列。
- 外接屏原生：沿用原异构List公开工厂和原私有Badge，在U2790B @2实际激活，四主题场景、源码／二进制身份与PNG分别留证。普通装饰Panel为0.5pt，IC仍为1pt。这个宿主只验绘制，未接管子控件输入／AX，不冒充VoiceOver或产品RunView验收。窗口已关闭，产物另留清理记录。

Gold7：G3的共享装饰／必要边界职责、调用方接入、前后图和有界原生绘制分别留证；主观物理清晰度、跨显示、整页及ClientKit同条件对照未完成。G7保留兼容API、真实消费者、可重建小宿主和文档。registry仍为experimental。

证据入口：`design/reviews/panel-scale-adaptation-20261021/progress.json`、`comparison.html`、`native-acceptance.json`、`current-observation-bindings.json`。当前观察身份 `f3997961265f2ff25b87e731ce6b314b5cecbe9f13b1b0eff0e5775e6d0c9a22`。owned-production.patch以本轮全部WIP前态为基准。

AgentKit冻结包2b16已经有surface_at_scale能力，已把本次漏接及现有接法同步给该会话；不要求替换冻结输入才能继续。旧冻结包未修改。集合旧候选继续使用其已保存的精确输入，不把本轮修改倒灌为旧通过。未测性能，未提交、推送或关闭Issue。
