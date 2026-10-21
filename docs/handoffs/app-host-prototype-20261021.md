> Historical WASM prototype evidence, superseded by the native module migration. Current architecture and validation: `app-host-native-prototype-20261021.md`. Results below do not establish native-module acceptance.

# TekesUI AppHost 基础通路原型记录

日期：2026-10-21。范围：macOS 本机开发原型；不代表完整框架、稳定 SDK 或发布验收。

## 源码与迁移

- 在现有 Gallery worktree 中保留全部原有 WIP，增加 `crates/tekes-ui-app-host`、`examples/app-host-fixture` 和 `scripts/app-host-prototype.sh`。
- Host 是 library + 原型 binary。消费者可调用 `tekes_ui_app_host::run(AppProfile { modules })`，无需复制窗口/事件循环；`examples/fixed_app.rs` 验证公共入口。
- 共享渲染包增加外部纹理绘制及导入接口，并增加 `external_target` 集成测试；未修改 Agent UI 或 Kernel。
- 当前设计在 `docs/design/app-host-architecture.md`。旧独立仓库保留公开历史及迁移说明，本地 `dev/TekesAppHost` 已按用户要求删除，未删除远端仓库。
- 依赖全部为 workspace 内相对路径或带 Cargo.lock 的 registry 依赖。当前 worktree 含既有未发布组件 WIP，不能将此原型称为已发布精确版本的消费验收。

## 已跑通的通路

WASM 中实际编译并运行 TekesUI controls/scene/tokens，Button 的布局、绘制贡献和点击逻辑发生在 WASM。原生模块运行器只解析受限 Scene 图元并调用 TekesUI/wgpu 绘制；Host 不重建业务控件。模块无文件/网络 WASI 导入，设置每次调用 fuel 和 32 MiB guest memory 上限。

普通 NSKeyedArchiver 无法传输本次 Metal 共享句柄；原型改用 IOSurface Mach send right。各进程在本地 Metal device 上构造纹理并由 wgpu HAL 导入，验证同 GPU registry ID。没有 CPU 图像经 IPC 传输；验证器的像素读回发生在 Host 导入并合成之后，仅用于断言。

生产完成由模块进程等待 GPU 后再通知；消费完成在后台线程等待并确认，UI 线程不等待模块或 GPU。共享 surface 每帧不可变，避免 Host 继续显示旧帧时生产者覆盖；缓冲复用池和跨 GPU 队列共享事件优化尚未实现。

## 验证结果

| 证据层 | 实际结果 |
|---|---|
| 共享渲染公开接口 | `cargo test -p tekes-ui-render --test external_target` 通过，外部 target 绘制再 GPU 采样，像素符合预期 |
| 原有 GPU 图像接口 | `cargo test -p tekes-ui-render --test gpu_images -- --ignored` 三项通过，覆盖颜色、裁剪、替换、生命周期、不同上下文和同 encoder 顺序 |
| 静态检查 | AppHost `clippy --all-targets --no-deps -- -D warnings` 通过；构建脚本语法检查通过 |
| 独立公共 API 消费者 | `cargo check --manifest-path examples/composition-consumer/Cargo.toml --tests` 通过；AppHost 的 `fixed_app` 示例随 all-targets 检查 |
| 单列实际进程 | 真正 Host + WASM 模块进程；验证按钮 down/up、取消后 up 不激活、toolbar action、counter 变化及 resize/2× backing scale 的导入画面采样 |
| 单列原生窗口 | `.app` 中原生 toolbar 点击从 14 到 15，content WASM Button 点击从 15 到 16，画面/注册标题随之更新；通过 CUA 发送原生窗口事件，非真人输入或 IME 验收 |
| 三列拓扑 | 一个 Host、三个不同 PID、三个不同 SHA-256 的 WASM 产物；各实例独立状态，分别导入纹理 |
| 三列原生交互 | 中间 toolbar 单独递增；右侧 content 点击和 Space 单独递增；R 换位后计数随 Panel 保留；splitter 拖动后新的 viewport 帧收敛 |
| 单模块故障 | 终止一个中性模块后，在下一次请求检测到退出并只禁用该列；其他列仍可递增与绘制。不是自动监控/重启验收 |

实际三列窗口验证中，Host PID 为 24022，模块 PID 为 24047/24049/24048。换位与 splitter 拖动时 PID 保持，计数从 `[0,1,2]` 随 Panel 移为 `[1,2,0]`；终止 24047 后，另一列仍从 1 递增到 2。PID 仅为这次运行证据，不是固定配置。

本地生成证据位于 `target/apphost-prototype/evidence/`：`verify-Three.json`、`bundle-Single.sha256`、`bundle-Three.sha256`、`window-Three.log`、`single-native.png`、`three-native-resize.png`、`three-native-fault.png`。缓存及截图不作为源码提交；重跑脚本会更新生成记录。

## 运行

```sh
scripts/app-host-prototype.sh verify-single
scripts/app-host-prototype.sh window-single
scripts/app-host-prototype.sh verify-three
scripts/app-host-prototype.sh window-three
```

脚本分别构建 Single/Three 固定 profile 的 `.app`，WASM 随包交付，正常启动不接收运行时模块选择。`--verify/--window` 是显式开发入口。三列完整拓扑不是三个单列窗口拼接。

## 尚未完成的设计验收

原型目前只覆盖受限夹具图元与基础 pointer/key/action 通路。尚未实现或验收正式的 module SDK/WIT、通用图标与 toolbar 贡献协议、跨列业务上下文联合发布、主题/几何环境同步、分布式 press/capture grant 全部状态机、中文组字及无障碍桥接、模块超时监控/自动恢复、持久化及关闭协调。

没有测得输入到画面的延迟分布、120 Hz 帧预算、重负载 resize 收敛或低 CPU 指标。静态分组/主题规则与上述同步设计继续作为后续要求；不能因本次三列可显示并交互，就把完整设计矩阵标为通过。

## Unified macOS titlebar correction

- Added opt-in `WindowConfig::unified_titlebar` in the shared macOS platform layer. Default windows retain their existing configuration. AppKit supplies a unified native toolbar chrome, hidden duplicate title and full-size content view; the toolbar commands themselves remain TekesUI controls. Native traffic lights are retained, not drawn replicas.
- Host draws one top band, with content below it; the first physical slot reserves 96 pt for window controls. Module-owned toolbar registration and actions stay unchanged. Empty toolbar regions route the current native mouse-down to AppKit's window-drag API. Prototype geometry currently uses a 56 pt band and fixed 96 pt reservation; dynamic system inset/fullscreen/accessibility coverage remains unfinished.
- Native validation: Host PID 33413; WASM workers 33425 / 33427 / 33426. All three toolbar counters changed 0 -> 1 through clicks in the unified band. First splitter moved from about 367 pt to 300 pt; toolbar and content boundaries moved together and counters stayed 1. Evidence: `target/apphost-prototype/evidence/window-unified-final.log` and `three-native-unified.png`.
- Fixed local bundle refresh by atomically replacing the executable and ad-hoc signing/verifying the complete bundle after resources are installed. Added a monotonic per-process endpoint suffix after observing concurrent module endpoint names collide at the same clock timestamp.
- This validates the current native layout and basic input, not the complete reviewed capture/IME/theme/performance contract.

Validation after this correction: AppHost clippy with warnings denied, macOS platform all-target check, and the separate composition consumer check all passed. The rebuilt single-column bundle also displayed the unified band and its toolbar click changed 0 -> 1 (`single-native-unified.png`). After the three-column resize, a content click in the middle column changed 1 -> 2. The three-column window remains running. Existing unrelated tracked WIP hashes were unchanged.
