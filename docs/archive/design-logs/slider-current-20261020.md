# Slider 当前源码分层复验

本轮基于全部 WIP，复用 SliderView、SliderContent、ContentHost 与已有消费者，未修改生产实现或设计 token。发现的缺口是旧证据未覆盖当前版本的原生禁用、移除／恢复和真实 selector 焦点交接，而非必须新增共享抽象。

现有五项公共消费者测试重跑通过；四主题、三品牌、最小／中间／最大值、六种状态、窄宽与 @1/@2 共 432 组、864 帧真实 GPU 核验通过。Slider 为既有静态绘制，普通与 Reduced Motion 图像一致；不据此声称其他组件的动态效果通过。

外接屏 U2790B@2 的最终签名宿主通过 26 个原生检查点，实际事件区间 11–74，无失活中断。覆盖键盘步进及边界、AX SetValue 63 量化至 65、禁用键鼠隔离、恢复后无旧焦点、移除／重新挂载保留原值、真实 TextCommand insertBacktab: 的一次按键焦点交接、相邻文本框方向键隔离、真实鼠标拖动和捕获释放。所有检查点均绑定输入序号、原生后状态、进程与二进制哈希。

Gold7 范围为本场景已有的值、焦点环、布局和交互隔离体验，未宣称整组件／整页全部通过。尚缺原生 AX Increment/Decrement selector、人声 VoiceOver，以及本轮未覆盖的最小化恢复／跨应用生命周期。公共 API 层的 Increment/Decrement 已测，不冒充原生 selector 或听读。VoiceOver 按用户要求保持关闭，性能不测。

窗口与进程关闭后，经未使用检查清理运行目录、测试可执行文件和宿主包；共享依赖缓存保留。源码、GPU 图像、日志和有界输入快照保留，快照不是完整可发布源码闭包。未提交、推送或关闭 Issue。

证据位于 design/reviews/slider-current-20261020/，以 acceptance.json、native/result.json、native/evidence-manifest.json、cleanup.json、protection.json 和 final-verification.json 为入口。
