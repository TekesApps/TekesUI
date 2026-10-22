# TekesUI native AppHost prototype

共享 Host library，负责 AppKit 统一标题栏、TekesUI toolbar/Panel/splitter、普通列与进程列混用、输入路由和共享纹理合成。进程列的业务工厂仅在 worker 中运行，普通列保留窗口进程中的原始 TekesUI ContentHost。

- `tekes-ui-app-protocol`：纯数据 IPC 消息与 toolbar 描述。
- `tekes-ui-module-runtime`：原生 `PanelModule` trait、固定模块工厂、子进程渲染及 transport。
- `tekes-ui-render` 的可选 `macos-shared-surface`：Metal/IOSurface 包装，无 Host 依赖。
- `examples/panel-module-demo`：原生中性模块消费者。
- `examples/app-host-demo`：固定装配的 App 消费者入口。
- 设计：`docs/design/app-host-architecture.md`；本次证据：`docs/handoffs/app-host-native-prototype-20261021.md`。

不再使用 WASM、VM、线性内存、WIT、Scene JSON 或动态库加载。模块作为 Rust crate 静态链接进消费 App；Host 与三个 worker 使用同一可执行文件，但分别在四个进程内运行。工厂只在对应子进程调用，各实例独立持有状态、Scene、字形图集及本地 GPU 对象。跨进程只发送消息及 OS 共享资源句柄。

```rust,ignore
let modules = vec![ModuleDefinition { id: "my-view", create: my_view_factory }];
tekes_ui_app_host::run(AppProfile { modules })?;
```

## 公共列接口

```rust,ignore
use tekes_ui_app_host::{AppWindow, Container, ProcessContainer};

AppWindow::from_columns(vec![
    Container::new("sidebar", sidebar_content)?.into(),
    ProcessContainer::new(document_module).into(),
    Container::new("inspector", inspector_content)?.into(),
])?
.settings(settings)
.run()?;
```

按 leading / center / trailing 顺序装配一至三列，每列可独立选择运行模式；ID 必须唯一。`Container::from_source` 可借用已有 ContentHost，`on_action` 处理原始动作，`toolbar` / `on_command` 声明 toolbar 与命令绑定。`ProcessContainer` 保留固定工厂、进程、IPC 和共享纹理通路。旧 `AppWindow::new(AppProfile { modules })` 兼容全进程列。

Settings 也是 `AppWindow::from_columns` 的消费者，装配两个普通 Container，目录/表单保留原始 ContentHost；保存、导航及独立 Undo/Redo 服务保持原职责。Main 仅为 sample。`examples/app-host-demo` 的 `mixed-columns` feature 演示两进程列加一本地可编辑备注列，不改变默认 `three-panels` 的全进程配置。

## 开发验证

在 TekesUI 根目录使用 Rust 1.95.0，无需 wasm 编译目标：

```sh
scripts/app-host-prototype.sh verify-single
scripts/app-host-prototype.sh verify-three
scripts/app-host-prototype.sh window-three
```

`bundle-single` / `bundle-three` 生成两个固定 profile 的本地 ad-hoc 签名 App，位于 `target/apphost-native/`，不打包模块字节码。模块工厂白名单与初始列数来自消费 App 编译结果。发行前仍需正式发布精确依赖版本和可验证构建 provenance；当前源码覆盖是显式开发路径依赖。

toolbar 由模块注册，Host 通过 typed IPC 返回 command ID。点击 content 控件在对应子进程内处理。Space 向聚焦模块发命令；R 循环换位，保持进程和模块状态。splitter 立即更新 Host 列边界，队列合并尾部过期 layout，模块按视口更新绘制；连续 resize 中，旧帧仅按原逻辑尺寸临时裁剪；已完成帧可在已显示视口与当前请求视口之间单调前进，seq 也必须前进。新点击与依赖几何的命令等待显示视口追上请求视口；已有 move/up/cancel 不被这道门丢弃。该 resize 预览策略不等于完整几何/capture 协议已实现。

GPU 路径：模块进程原生绘制 → 不可变共享 IOSurface → Mach right → Host 本地 GPU 导入/合成。生产/消费 GPU 等待在 worker/后台线程；UI 线程不等待子进程。验证器的像素读回只用于验收，不作为 IPC。每帧仍分配不可变资源；每列最多两个在途帧，允许下一帧准备与上一帧消费重叠，继续生产受消费 GPU 完成确认约束。尚未完成纹理池。

原生红黄绿按钮与三列 toolbar 同一顶部栏；TekesUI 呈现模块命令。当前读取原生顶部安全区并与已注册 toolbar 的测量高度统一；全屏仍待覆盖。真实中文 IME、无障碍、完整 capture/几何代次/联合切换/theme 及性能验收均未完成。原生模块是可信发布代码；进程隔离不等于权限沙箱。

消费布局使用两个独立 splitter 位置，仅相邻列改变宽度，无关列保持 fixed；最小列宽限制阻止越界。Host 工作区使用平面默认背景，无三列卡片边框、圆角和阴影，未修改全局组件 Style。示例正文现走 TekesUI 只读多行 TextFieldContent，状态走 TextContent；字号/行距使用默认语义角色，三列提供不同语言文本。

Toolbar 是否存在与动作清单由各 Panel 模块注册。Host 将声明交给共享 ToolbarControl（composition Component），由控件测量、单行排布并在窄列使用溢出菜单；Host 不实现这些策略。顶部栏高度取三列测量与原生安全区最大值，按 scale 对齐，统一 content 视口与坐标。固定 56 pt / 96 pt 仅是旧原型记录，当前已移除。构建脚本使用协调的 review-cache，apphost-native 保留为产物/证据目录；不切换 cache owner 或清理缓存。

设计更新：进程模块基础改为 ProcessComponent，View 可选；ProcessContainer 承载有 View 的模块。导航模块是独立无 View 进程，声明窗口级前进/后退 toolbar 并独占 nav.json；AppHost 仅装配和执行导航，收到真实切换成功后回报，由导航模块提交历史。完整持久化/重试/恢复规则见 `docs/design/process-component-navigation.md`。这是抽取目标，当前原型证据不能当作无 View 导航已实现。
