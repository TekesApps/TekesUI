# 三列 AppHost 标题栏导航原型

状态：2026-10-21 已接入并以原生窗口实际点击验证；仍为固定原生模块原型，不代表通用业务路由、完整 IME/AX 或公共 ProcessContainer 已全部验收。

最新行为见 [leading 懒启动与布局交换](lazy-leading-swap-20261021.md)：leading 默认不启动，首次点开才加载；右上交换只调整 center/trailing，不进入导航历史。以下 panel-layout、R 导航及五进程启动描述是此前验收基线，已由新方案取代。

## 使用

App 位于 `target/apphost-native/navigation-prototype/TekesAppHost.app`，名称 TekesAppHost Navigation。标题栏红绿灯右侧为窗口级前进/后退纯图标；三个内容模块的 toolbar 保持各自声明。首次空历史通过正常导航顺序展示布局 A/B/C，后退和前进可观察中文、英文、Unicode 三列实际换位。按 R 发起新的换位导航；在历史中间发起换位会按导航模块规则截断前进分支。

历史目录为 `~/Library/Application Support/TekesUI AppHost Prototype/navigation/`；nav.json 仍由无 View 导航进程独占写入。重开 App 恢复当前条目，不重新播种已有历史。

## 装配和职责

- `run_with_navigation(AppProfile, NavigationOptions)` 增加可选导航装配；原 `run(AppProfile)` 和单列入口保持兼容。固定 worker 入口在窗口/GPU 初始化前分流，因此导航 worker 不创建 Renderer、Scene、atlas、View 或共享纹理。
- 实际进程数为五：Host、三个 View worker、一个独立的导航 ProcessComponent。三个 View 此轮仍复用现有 ModuleProcess/ProcessControl 路径，未提前声称已迁入最终公共 ProcessContainer。
- `navigation_window.rs` 只承担 Host 请求/回执路由、模块 toolbar 挂载、实际布局发布和临时 receipt。导航模块拥有 entries/cursor/revision、事务与文件；Host 不写第二份历史。模块命令保留原 owner/session/window/registration/command 身份。
- TekesUI `ToolbarControl::set_icon_actions` 统一负责图标按钮的命中、禁用、单行测量、溢出及菜单标签；AppHost 只从公共 Glyph 目录解析 `tekes-back`/`tekes-forward` 并提供已缓存图片。没有 AppHost 私有的前后按钮布局或溢出实现。
- 本原型唯一演示路由为 `panel-layout`，参数使用固定模块 ID 排列。校验模块数量、重复和未知 ID；移动保留原进程、业务状态及纹理，展示文字仍走原生 TekesUI 文本组件。

## 发布和输入顺序

导航返回执行目标或恢复快照 → Host 关闭相关内容的新输入、取消旧 arm → 改变列排列并等待三个 View 当前几何画面 → Renderer 提交该窗口画面 → 后台 GPU 完成确认 → Host 返回 Applied/RecoveryApplied → 导航持久化 HistoryCommitted 或确认 RecoveryReady → Host 开放导航及相关内容输入。

该顺序对原型的“列排列”上下文成立；它没有扩称为多个业务模块内部状态的完整联合切换事务。UI 线程不等待磁盘、IPC 或 GPU；已有后台完成队列传递发布确认。导航忙碌/恢复期间按钮禁用，精确内容输入继续受当前几何约束，原生窗口激活与主按钮门控保留。

## 验收

记录见 [验收汇总](../../design/reviews/navigation-window-20261021/acceptance.json)、[五进程](../../design/reviews/navigation-window-20261021/five-processes.txt)、[后退快照](../../design/reviews/navigation-window-20261021/back-history.json)、[前进快照](../../design/reviews/navigation-window-20261021/forward-history.json)。

- 通过 CUA 从 Finder 正常双击激活后，实际点击标题栏后退/前进，窗口标题和中文/英文/Unicode 内容的列顺序随之改变；磁盘 cursor/revision 相应提交。后台点击的 active/key-window=false 被拒绝，未为自动化绕过门控。
- 正常退出并重开后恢复原 cursor/revision 和排列；三个 View 和导航均为独立 PID。
- Toolbar 四项定向测试通过，包括纯图标方形命中、原命令 ID、禁用与窄宽溢出；AppHost 几何/路由两项测试、严格 Clippy、第二个公共 API 消费者编译通过。
- 无 View 服务的 16 组故障场景重新运行通过，仍包括三组确认丢失/回收顺序/恢复中崩溃场景。这不表示已在原生窗口中重复注入全部进程与磁盘故障。
- 现有三列 GPU/IPC 回归单独通过。未提交、推送、发布或宣布公共容器整体迁移就绪。

## 左右列显隐 toolbar（2026-10-21）

标题栏左侧注册 leading on/off，右侧注册 trailing on/off；均使用共享 ToolbarControl 的纯图标和 selected 状态，窗口始终保留恢复入口。它们属于 Host 的物理槽位布局，不随内容换位，不进入导航历史。隐藏取消旧输入/toolbar arm，把焦点转到中间可见列，保留原模块 PID、业务状态、纹理和 splitter 比例；恢复后等待有效几何才接收精确输入。中间列填补隐藏区域，另一侧可见列保持 fixed。

原生实际点击验证顺序为左右可见 → 左隐藏 → 双隐藏 → 仅右隐藏 → 双可见，三个 View PID 始终相同，nav.json 前后字节一致。AppHost 三项、Toolbar 四项定向测试及 lib 严格 Clippy 通过；all-target Clippy 遇到其他 WIP 的 text_sharpness.rs collapsible_match，未改该文件。见 [显隐验收](../../design/reviews/panel-visibility-20261021/acceptance.json)。
