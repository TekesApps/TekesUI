# #15 / #16 目标隔离与真实重排补证

2026-10-18，沿用全部 WIP。根因/共享实现不变，本批无公共实现、token、设计源或迁移范围变更。

## #15 生产目标登记

独立、可重放的 Rust 主线程探针使用当前消费快照，原生产逻辑不改，只增加 audit binary 和访问测试入口。构造两个真实 Shared 宿主，直接核验同一弱目标表的登记身份、finish 撤销、另一目标保持独立、19 个代理不保活宿主、Drop 撤销，以及关闭前 pending 回调在关闭后安全丢弃，共 32 个断言。无需 NSApplication、NSWindow 或桌面操作。

这比单独两个原子状态的隔离测试覆盖更多，但仍不是原生多窗口完整验收：活窗口事件投递/绘制/现场隔离尚未证明，#15 对应条件仍为“已实现待验收”。输入逐文件哈希、审计 instrumentation 与可运行完整源码分别保留。探针较小 workspace 的 lock 离线裁剪；所有保留 package 版本/source 与当前上游 lock 一致，不把改过的审计副本说成公开生产 consumer。

## #16 AgentKit 真实消费布局

扩展现有实际 Earlier 回归而非新造阅读算法。第一页600pt/100%，第二页前插同时400pt/150%，真实CoreText测得两页位置误差均0pt，保留UTF16逻辑位置、选区和Composer焦点4；A→B→A恢复原阅读位置。UI workspace 11项和strict Clippy再次通过，默认headless源码未变，76项结果沿用前一冻结批次。

## 验收边界与顺序

公共API与共享实现仍引用既有wake-contract、prepend-reading；实际AgentKit接入见p0-consumer-closeout。GPU/Gold7代表画面与Gallery原生分页证据不改写，不因本次确定性测试扩大视觉或现场结论。AgentKit原生窗口未启动：此前授权只有Gallery；物理IME/VoiceOver未测，性能暂不测。公开不可变pin仍因禁止提交/推送/tag而阻塞。Issue保持开放。

本批证据：design/reviews/wake-targets-20261018/receipt.json。复现：cargo run --offline --locked --manifest-path design/reviews/wake-targets-20261018/probe/Cargo.toml -p tekes-ui-platform-macos --bin wake-registry-audit（生成target应放外部临时目录以保持冻结证据不变）。

可继续执行项是#19：平台已有button/modifiers/click_count，ContentEvent丢失字段，TextFieldContent固定false/1。补齐公共传递链、保留简化事件兼容和主/次按钮规则，再复用TextSurface的词/段落选择、字素、capture/cancel。尚未修改这些共享模块，维持单一负责人；#17文本服务随后统一安排。
