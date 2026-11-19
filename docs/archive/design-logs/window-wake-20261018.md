# WindowWake：后台通知与窗口生命周期

对应 #15 / TKUI-001，基于全部 WIP，未提交。根因是平台边界只有 UI 线程绘制控制，缺少后台到目标窗口的通知；不是组件布局或 Agent 协议问题。

## 公共契约与共享实现

在 UI 线程调用 `Window::wake_proxy()`，得到 `Send + Sync + Clone` 的 `WindowWake`。后台先通过调用者自己的 Mutex/atomic/channel 发布 snapshot，再 `wake()`；返回 Queued / Coalesced / Closed。Queued/Coalesced 不保证请求返回前已处理，窗口关闭可以取消已排队通知。代理不承载业务数据，也不要求 Window 成为 Send 或接入 Tokio。

平台沿用主 dispatch queue 把 `Event::Wake` 交给原 Shared 事件队列；宿主收到后读取最新 snapshot、标脏并按需 request_redraw。没有定时轮询、显示链接或自动帧泵；无数据时不会因持有代理而自动产生通知。宿主仍可为真实动效使用既有动画接口。

代理只拥有 Arc 原子状态；UI 线程私有的弱目标表持有 Weak<Shared>，后台及 dispatch context 均不持有 NSWindow/Shared/Rc。Arc 身份在队列任务完成前存活，避免地址复用命中另一个窗口。finish、windowWillClose 和 Shared Drop 关闭状态并移除弱目标；已排队任务和所有克隆均不能复活窗口。

pending 在调用宿主前清除。此后到达的后台请求会再次排队，即使前一 handler 尚未结束，也不会被 handler 末尾清标志覆盖。原事件队列保证同步 AppKit 重入只入队，handler 之间没有跨回调 RefCell 借用。若后台在清除前看到 PENDING，可合并，因为合并请求也执行带 Release 的原子 RMW，宿主清 pending 的 Acquire 后读取同步 snapshot；同步数据所有权仍由调用者保证。

适用所有采用本平台 Window 的异步宿主，不只 AgentKit。没有后台状态的窗口无需获取代理；图形组件、token、绘制和状态机不迁移。Event 已 non_exhaustive；Window 新方法、独立新类型为增量公开 API，现有宿主可以继续忽略 Wake。

```rust,ignore
// UI thread, once per window:
let wake = window.wake_proxy();
// Worker: publish through caller-owned synchronized storage, then notify.
std::thread::spawn(move || {
    publish_snapshot();
    let _ = wake.wake();
});
// WindowHandler, Event::Wake:
read_latest_snapshot();
window.request_redraw();
```

## 分层证据与边界

- 公共 API：平台确定性测试证明 Send/Sync/Clone、8000 次并发合并、清 pending 后再次请求、关闭取消和两个状态相互隔离，以及 4000 次并发发布/清除后最终 snapshot 必达。独立 consumer 的 examples/wake.rs 只导入公开平台 API，跨线程发布/通知并在 App::run 返回后断言克隆 Closed；已编译/Clippy，未操作其桌面窗口。
- 真实接入：composition-review 的可选 Gallery 验收模式使用同一公开代理，保留普通 Renderer/Scene。仅有限工作线程发布 1 次初始值、8×8 次累加，最后一次明确审计 checkpoint；没有业务轮询。窗口标题实际显示“后台快照 65”，事件记录包括零输入、零 AnimationFrame、通知合并与最后值。退出后在真实 Host Drop 内再次 wake 验证 Closed。
- Gold7：G1–G3 保留原 JSON DS、画面与渲染，无颜色/布局变更；本轮仅检查原生 spacing 页面仍正常。G4 不适用新动效，无动效参数变化；G5 验证后台通知不依赖输入/不自动接管编辑焦点，组字/草稿组合未测；G6 验证关闭与恢复边界，不声称多窗口现场验收；G7 独立公开 API consumer、实际 Gallery 和指南分别留证。
- 自动测试、真实原生窗口、物理中文 IME、VoiceOver、性能分别记录。原生标题的 AX 可读不等于组件 AX/VoiceOver 验收。性能暂不测；事件/帧计数仅检查通知正确性和没有轮询，不测 FPS/CPU/延迟。

收据：design/reviews/wake-contract-20261018/receipt.json。仍保留：AgentKit 移除持续 frame pump 后的流式/取消/连接状态现场联调，第二原生 consumer 实跑、物理 IME/VoiceOver、公开不可变 commit/tag。正式 pin 受用户明确禁止 Git 发布操作约束，不能将 WIP 哈希当作正式 pin。下一 P0 为 #16 的历史前插阅读锚点。
