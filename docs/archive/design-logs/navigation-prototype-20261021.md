# 最小导航原型：ProcessComponent 服务与持久化

状态：已实现并通过真实子进程/文件系统故障验收；本页记录服务侧最小原型。后续三列标题栏导航的原生点击与恢复验证已另见 [窗口接入验收](navigation-window-20261021.md)，仍不代表通用业务联合发布、完整 IME/AX 或公共容器已全部验收。

## 实现边界

- `crates/tekes-ui-app-host/src/navigation/`：独立导航模块、磁盘模型和 Host 执行回执适配。直接实现 TekesUI 的 `ProcessModule`，由公共 `ProcessComponent` 启动；没有另一套运行器、WASM 或 View/GPU 初始化。
- `definition()` 提供固定模块入口；消费者应先调用 `run_component_worker_from_args`，再创建窗口/GPU。通过 `navigation` topic 传输序列化 `Command`/`Reply`。
- `Open` 由 Host 提供独立 history 目录、namespace 和获准的非零 window ID；模块注册窗口作用域前进/后退 toolbar。使用公共图标 `tekes-back` / `tekes-forward`，标题为空，提供可访问名称/提示；注册只在可用状态变化时更新。按钮布局、溢出和绘制仍属于 TekesUI Toolbar，服务原型没有另写工具栏。
- 导航模块独占 entries/cursor/revision 和文件；Host 只执行目标、回报发布后的 context，并保留事务目标/操作及成功或失败终态，不维护第二份历史。`NavigationHost` 的发布回调是实际路由适配的接缝：本验收回调只发布内存中的测试目标，不把它当成三 View 的真实联合发布。
- `nav.lock` 使用 OS 独占文件锁；`nav.pending.json` 在执行之前持久化意图，成功 receipt 写入日志后才允许替换 `nav.json`。同目录临时文件 → 文件 sync → rename → 目录 sync；替换后失败冻结历史，不反向覆盖旧快照。持久性声明限于这些 OS/filesystem sync 操作，未验证掉电或硬件缓存保证。
- 恢复与提交在导航工作进程同一协调器串行执行；Host UI 不执行写盘。先收敛 journal/候选，再 RestoreOffer → RecoveryApplied → RecoveryReady；旧 epoch/实例/基线不能开门。未知执行结果保留待对账，不重放。
- 磁盘已确认提交后，pending 清理失败不反向改报“提交失败”；保留待清理状态并在恢复时幂等清理。执行失败也缓存终态，重复请求不能再次调用发布回调。

## 验收证据

入口：`crates/tekes-ui-app-host/examples/navigation_prototype.rs`。专用验收 factory 才处理故障控制消息；正常 `navigation::definition()` 不开放这些测试 topic。

最终运行及每个场景的身份、revision、门控/对账结果见 [acceptance.json](../../design/reviews/navigation-prototype-20261021/verified-final/acceptance.json)。构建、Clippy、回归及源码/二进制身份见 [验收汇总](../../design/reviews/navigation-prototype-20261021/acceptance.json)。

该入口使用真实 ProcessComponent 子进程及隔离的合成数据，覆盖：

1. 正常 push/back/forward、前进分支截断、窗口纯图标声明和原 command ID、注册代次、独占写入、重复/冲突事务。
2. 丢弃已生成的 RecoveryReady，不交给 Host 适配器；同会话重发 Applied 可取得 Ready，丢失期间不开门，提交后旧恢复基线不可复活。
3. HistoryCommitted 未交给 Host 适配器、pending 已删除后终止导航进程；以及 Host 已回收 receipt、pending 未清理时终止导航进程。恢复不重复执行/追加或等待已回收记录。
4. Host 发布成功后、导航记录成功前重启导航；通过新实例受限对账查询 Host receipt，拒绝旧普通回执。
5. 替换前失败、rename 后目录同步失败；未确认时磁盘可见状态和内存提交状态分别校验，重启后仅补交一次。
6. Applied/Ready 之间在 Ready 生成前、生成但未交给适配器后分别终止导航进程；另启动实际测试 Host 进程，在这两个位置直接 kill，再用新 Host epoch 恢复。
7. 双方丢失易失 receipt 后明确保持未知/门控；失败终态与重复成功回执对账；坏 JSON/未来格式保留原字节。
8. 暂停导航工作进程时，Host 侧可继续轮询状态/事件并取消、回收工作进程。这是 API 非阻塞证据，未代替原生窗口响应或物理输入验收。

复现必须使用新的空目录；不会清理或复用现有用户历史：

```sh
# 首先按 scripts/tekesui_build.py 的 build_lock()/environment() 构建：
# cargo +1.95.0 build --offline -p tekes-ui-app-host --example navigation_prototype
# 固定使用共享 target/review-cache，不调用 prepare_cache 或 cargo clean。
target/review-cache/debug/examples/navigation_prototype /absolute/path/to/new-empty-fixture
```

## 后续接入边界

现有三 View 只完成共享运行器 API 升级的兼容接线：ReceivedFrame 自动释放未导入纹理能力，ProcessControl 负责取消/回收，等待 GPU ack 的后台线程可退出。三 View GPU 回归单独记录，不代表与导航进程同时装配成五进程窗口。

三列标题栏已通过可选导航装配和 panel-layout 演示路由接入，见 [窗口接入验收](navigation-window-20261021.md)。完整公共 ProcessContainer 迁移、通用业务路由的联合状态发布、IME/AX 和全部原生故障验收仍待后续；服务侧故障证据与窗口实际点击证据分别记录。
