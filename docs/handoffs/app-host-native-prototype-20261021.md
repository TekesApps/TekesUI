# TekesUI native AppHost prototype migration

日期：2026-10-21。当前为本地开发原型，未发布稳定 SDK/IPC，也未做生产性能验收。旧 WASM 验证记录保留为历史，不作为本次原生验收证据。

## 实现与目录

- `crates/tekes-ui-app-protocol`：只依赖 serde 的数据契约，typed input、toolbar、协议版本和帧元数据；不依赖 UI/GPU/Host。
- `crates/tekes-ui-module-runtime`：`PanelModule`、`ModuleDefinition`、本地 Scene/atlas、独立进程绘制与 Mach transport。
- `crates/tekes-ui-app-host`：共享 library，统一窗口 chrome、三列/toolbar/splitter、子进程生命周期、输入队列、共享画面消费/合成。无业务模块依赖。
- `crates/tekes-ui-render/src/shared_surface_macos.rs`：可选 `macos-shared-surface` 特性，通用 Metal/IOSurface 创建和导入，无 Host/runtime 反向依赖。
- `examples/panel-module-demo`：三个中性原生模块工厂，复用 TekesUI Button/surface；每个进程独立持有状态。
- `examples/app-host-demo`：独立消费 workspace，固定 profile 装配与验收入口。夹具计数断言保留在此示例，未放入 Host/runtime。

移除了 wasmi、WASM 导出/线性内存访问、Scene JSON 编解码、wasm32 编译/打包脚本、旧 `examples/app-host-fixture` 和 Host 的示例 executable。首版无动态库/插件发现：模块 crate 静态链接进消费 App，Host 启动同一 executable 的 worker 入口，按编译期白名单 ID 创建实例。单列为 1 Host + 1 worker，三列为 1 Host + 3 workers。

跨进程仍使用版本化 IPC、viewport/seq 校验和真实共享 GPU 资源。Scene、Rust 对象及 GPU device 留在子进程；不经 Host 重建业务组件树。toolbar 贡献由模块经 IPC 注册、Host 用 TekesUI 绘制，command ID 返回模块。统一标题栏和原生红黄绿控件保留。

## 验证

以下均为本次原生实现证据，路径相对 TekesUI 仓库根目录。

- `scripts/app-host-prototype.sh verify-single`：Host 47603、worker 47621，10 个步骤覆盖 layout、down/up、toolbar command、cancel 后 up 不激活、move 和 2× resize；GPU 导入/合成后的像素符合预期。
- `verify-three`：Host 43736，三个不同 worker PID 43739 / 43740 / 43741，每个模块独立计数与像素变化。结果为 `NATIVE_GPU_IPC_VERIFIED`，见 `target/apphost-native/evidence/verify-{Single,Three}.json`。读回只用于验收，不作为 IPC。
- 原生三列窗口：Host 44417；模块 0/1/2 对应 44577 / 44579 / 44578。三列 toolbar 各从 0 → 1；splitter 调宽后 content 在原 PID 重新布局；中列 content 点击 + Space 得到 3；R 换位为 `[1,2,0]`，状态与 PID 保留。见 `window-Three.log`、`three-native-initial.png`、`three-native-resize-reorder.png`。
- 故障隔离：结束 worker 44577，换位后右列变为 unavailable；左列 worker 44579 继续从 3 → 4。见 `three-native-isolation.png`。当前空闲进程退出在下一次请求时发现，非完整主动监管/恢复策略。
- 单列原生窗口也已运行，toolbar + content 点击从 0 → 2；见 `single-native.png`。两种 profile 由同一 Host/runtime 实现提供，消费 App 未复制窗口/事件/渲染代码。
- 未注册模块、worker 参数缺失、任意 runtime 模块选择及错误协议版本均明确拒绝，见 `rejected-inputs.json`。
- 三个共享 crate 的 all-target clippy（warnings denied）、示例 App 的 all-target/all-feature clippy、独立 `composition-consumer` check 通过。
- `tekes-ui-render --test external_target --features macos-shared-surface`：1 个实际 GPU 测试通过。此独立构建曾发现 CFNumber 依赖仅被其他消费者间接启用，现已在 render 特性中显式声明并复测通过。
- Single/Three 本地 App bundle 均做 ad-hoc 签名与 strict verify；产物摘要见 `bundle-{Single,Three}.sha256`。Resources 不含模块字节码。

## 复现

```sh
scripts/app-host-prototype.sh verify-single
scripts/app-host-prototype.sh verify-three
scripts/app-host-prototype.sh bundle-single
scripts/app-host-prototype.sh window-three
```

输出位于 `target/apphost-native/`。Rust 1.95.0；不需要 wasm target。示例 Cargo path 依赖为显式开发覆盖，发行前须发布精确 pin 和构建 provenance。

## 保留边界与未完成项

- 架构文档的完整几何代次、上下文联合切换失败/取消、文本与几何分别发布、持续 capture 和迟到 grant 契约全部保留；当前只实现 viewport/seq 与基本输入，不能将设计要求当作已验收功能。
- 当前 toolbar 只展示夹具的一个注册按钮；完整分组、图标、溢出和快捷键规则未完成。
- 正式 IME/候选窗、无障碍、主题/环境更新、窗口动态安全区/全屏、隐藏恢复、超时/退避重启、关闭保存协调仍未覆盖。
- 每帧不可变 IOSurface + 生产 GPU 等待 + 后台消费确认仍为原型方案，未完成有界纹理池/共享事件优化和持续拖动延迟/CPU/GPU/内存基准；不因移除 VM 宣称无卡顿。
- 原生模块是可信编译代码，独立进程不等同于权限沙箱；未实现 OS 权限隔离或 CPU/内存配额。
- 未修改 ClientKit Agent UI 或 Kernel。保留其他 WIP；迁移期间观察到 Gallery `main.rs` 及 3D viewer/GPU 文件发生外部并行修改，未覆盖或回退。

最终重新打开的健康三列窗口：Host 54236；workers 54255, 54256, 54257。进程命令与父子关系见 `final-processes.json`，画面见 `three-native-final.png`。

## Text reflow fixture

The rounded button now contains module/width/line-count/shaping-time labels; the body contains long mixed Chinese/English text. Each worker shapes at its available width, caches unchanged-width paragraphs, draws visible lines into its own atlas and exports the rendered shared surface. Runtime starts an atlas frame before each module update. Displayed milliseconds measure shaping CPU time only, not full input/IPC/GPU/presentation latency. The GPU verifier also checks visible text pixels in the imported frame. Native drag screenshots are recorded as `three-text-wide.png` and `three-text-narrow.png`.

实际窗口验证：第一列从 366 pt / 97 行拖到 259 pt / 137 行，相邻列扩展到 419 pt / 81 行，文字换行相应变化。截图中的单次排版时间约 13–16 ms（debug 原型、本次运行观察，不代表完整响应时间或性能基准）。三进程 GPU 验证均检测到 body 内的实际文字像素；输入/cancel/2× resize 检查和示例 App clippy 通过。

## 后续修正：fixed splitter、平面工作区与真实文本组件

两个 splitter 现在是独立位置，仅相邻列 resize，无关列固定。移除 Host 和模块的三列卡片框、圆角/阴影与三色背景，使用缺省 Palette。每列最多两个不可变在途帧；连续 resize 接受单调前进的已完成预览，避免严格只收最新请求帧导致停画。新点击/依赖几何命令等待视口匹配，已有 move/up/cancel 继续路由；完整联合切换与 capture 协议仍未实现。

早先的手工 DisplayParagraph 正文与按钮文字叠画已经替换：正文为 `TextFieldContent::read_only_content`，状态为 `TextContent`，计数按钮由 ButtonTextRenderer 绘制真实 label。字体适配器使用默认 UiBody 字号、字重和行高，以及完整 Paragraph/caret 数据，不再使用显示专用快路径。平台适配器实现 TextSurfaceHost；TextSurface 持有 wrap、缓存、选择与裁剪几何。当前内容只读，未声称实现编辑或跨进程 IME。

三列分别为中文长文、英文长文、Unicode 混排，各重复四次。混排包含 emoji/ZWJ/肤色、组合音符、日/韩、阿拉伯语/希伯来语与长连续标识符。窗口已用这些真实组件检查不同宽度换行。并行存在用户操作，不把整段窗口日志归为自动化控制。

验证：`text-components-verify.log` 三个独立 worker 的 GPU 导入/实际文字像素、toolbar、计数、取消与 2x resize 全通过；`plain_text_content` 三项组件测试通过；模块示例 clippy -D warnings 通过。三列 App 已重新打包并运行。证据位于 `target/apphost-native/evidence/`，窗口截图 `text-components-before.png` / `text-components-after.png`，运行日志 `window-text-components.log`。

实际组件全量 caret 排版在本次 debug 窗口中约数毫秒至十余毫秒，依宽度和负载变化；这是组件 layout CPU 时长，不是端到端响应。此前显示专用路径的低耗时不能作为当前路径性能结论，尚未完成正式连续拖动性能基准。

## 真实 Text 组件排版优化（2026-10-21 后续）

继续使用 `TextFieldContent::read_only_content` / `TextContent`，正文仍保留全部行、字形和 caret 数据，未切回 DisplayParagraph。默认语义字号/行高及三个不同文档保持不变。

- 共享 `TextSurface` 为 intrinsic measure 保存最多一份完整段落，按文本、角色、像素对齐 wrap 宽度与 type scale 匹配；arrange 复用它，避免 Grow 的测量/布局重复排版。相同条件下测量可直接复用已布局段落。测量不发布或改变当前编辑、选择、滚动或 IME 几何；文本/角色/宽度/scale/wrap 不匹配时重新排版。缓存不持有内部 borrow 调用外部字体服务。
- `PreparedText` 复用原 CoreText 字素边界；精确 UTF-16 行范围的 primary caret 坐标可跨宽度复用，仍逐个提供完整 caret。缓存受最多 `max(4 * utf16_len, 64)` 个 caret 条目预算约束，FIFO 回收；新文档/字体准备对象独立。行顶、baseline、换行与范围仍每次计算，未跳过 TextSurface。
- 字体注册表先匹配已经强引用保留的 CTFont 对象，命中时避免逐行重复解析 PostScript/variation 身份；不同对象继续使用完整身份比较。glyph cluster 下一边界在已排序列表中二分查找。

`examples/panel-module-demo/examples/component_reflow_bench.rs` 通过模块真实 update 路径测量三类文档，包含组件测量/布局、完整 caret、Scene/字形准备。宽度从 240 pt 开始以 5 pt 前进再返回，共 120 次；初始 20 次不计入统计，分别报告首次宽度与返回方向。相同输入序列下，消除重复测量后的首次宽度中位数为中文 1.667 / 英文 2.869 / 混排 2.157 ms；再加原生文字缓存后为 0.994 / 1.271 / 1.195 ms，p95 为 1.344 / 1.848 / 1.483 ms。证据 `text-perf/component-before-native-cache.log` 和 `component-after-native-cache.log`；最终重跑见 `component-final.log`。这些是本机 debug 单进程组件 CPU 耗时，不能当作 IPC/GPU/屏幕呈现延迟或稳定刷新率保证。

原生窗口一次拖动后的读数为 0.56 / 0.96 / 0.68 ms；随后重新出现的窗口/操作亦观察到 5–12 ms 读数，未据此宣称稳定帧率。验证时有用户并行操作/重启/换位，因此窗口证据不用于独立推导可重复端到端延迟。截图 `text-perf/native-before.png` / `native-after.png`；应继续用实际拖动体验评估。Host 仍有两帧上限和单调 resize 预览，未以缩放文字替代组件重排。

验证：text 13 个 lib + 15 个 shape 测试、controls 248 个 lib + 3 个 plain_text_content 测试通过；新增测量/布局复用和失效测试，以及跨多宽度/缓存回收的 CJK、ligature、emoji、组合字符、RTL caret 坐标逐一与原 CoreText 调用比对测试。第二公开 API 消费者 composition-consumer tests 编译通过；text/controls clippy -D warnings 通过；三个 worker 的 GPU 导入/正文像素/toolbar/输入/cancel/2x resize 验证通过。新证据均在 `target/apphost-native/evidence/text-perf/`。未改 Agent/Kernel，也未将全套设计协议或全量工作区测试视为已完成。

## 模块声明 Toolbar / 自适配统一顶部栏

模块继续通过 ModuleState.toolbar 声明本 Panel 的动作、ID 与可用状态；示例现在声明 Increment / Reset count 两个动作用于溢出演示。Host 不创建业务动作，只将声明适配为共享 `ToolbarControl`。控件内部负责固有尺寸测量、单行排布、禁用状态、窄列 `…` 菜单与菜单激活；标签换行折为空白。所有动作仍按原注册 ID 返回对应模块进程。

ToolbarControl 是 composition Component，复用实际 ButtonContent 的按压/键盘行为与 MenuView 的 popup、禁用项、输入处理。AppHost 保留 ContentHost 传播、primary pointer、活动窗口、原 native down 拖窗 handoff 以及原 arm 取消语义；没有再建立 Host 按钮状态机。相同动作 ID 的标签更新保留逻辑焦点，几何/声明变化撤销旧按压；菜单 overlay 在兄弟 content 之上最后合成。

Host 仅取三个已注册 toolbar 的测量高度与平台实际 chrome 安全区最大值，再按 scale 对齐。固定 56 pt / 左侧 96 pt 已移除；当前窗口记录 `TOOLBAR_HEIGHT height=66 native=66 leading=79`。三列 content origin/height、IPC viewport 与 down/move/up 坐标共同使用动态 toolbar_height。文字仍走真实 TekesUI 组件。

验收分层：共享控件新增测量/窄列不增高、溢出原 ID/禁用项、resize 后迟到 up、真实按钮 Drag 冒泡及 ContentHost 内注册更新保持焦点与键盘激活等测试；Host 新增多列/多顶部栏高度的 content 坐标测试。native 窗口已呈现统一顶部栏和两个模块动作；CUA 后台点击/drag 被活动窗口门禁挡住，未触发实际模块输入，因此普通点击、菜单操作和真实桌面拖动不能标为已验收。既有三进程 GPU verifier 通过，但不替代窗口操作验证。

构建复用 `target/review-cache`，通过 scripts/tekesui_build.py 的 build_lock()/environment()，不调用 prepare_cache 或 cargo clean。`target/apphost-native` 留作既有 App/可执行快照/证据位置，未删除历史缓存与证据。共享锁被占用时脚本明确失败，不竞争写入。

普通 Container / ProcessContainer 的分工已写入设计：进程边界可以抽取成通用能力，但本轮未增加 subprocess 布尔属性或执行进程容器迁移。
