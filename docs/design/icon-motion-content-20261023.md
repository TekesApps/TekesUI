# 原图标不定状态与组件帧请求边界

## 2026-10-29 Main 接收

最终落点为本地 main；共享周期基础已提交 `f6d678c`，组件在该基线上接收。
原 IconContent、SVG 资源缓存、独立公共声明和完整 Icon 原生场景保留，使用 Main
已交付的 BitmapImageLease 退休机制，不新增 Renderer API。FrameTextAdapter
必须同时转发原 icon 服务和窗口动画许可；缺失服务仍显式失败，不清 Composer。

当前 Main 的未声明 Surface 按原 Button 标签/装饰颜色兼容路径工作，显式 Surface
才使用生成的完整 packet。接收测试同时覆盖这两条真实路径，保留全部状态、品牌、
原控件和精确颜色/单 owner 断言；不修改原 Button 行为来迎合旧候选的输入假设。
六个新公开类型归入原 `tui.icon` 及其支持 API；两个 SVG 服务按原 Cargo feature
编译绑定，不增加组件根或第二份 registry。七个活动消费者 lock 只补必要路径依赖，
原包版本保留，历史快照锁不改写。

实际通过：默认/SVG 控件严格检查、原库及布局回归、两种 API 编译绑定、独立
icon/状态/动效用例；真实 SVG GPU 的旋转/整周/clip/字号/1x2x、状态颜色 oracle、
资源复用/退休和 renderer 隔离。原生窗口使用完整原 Icon 混合树，经真实 pointer/AX
选择器及合成 NSTextInputClient 回调，6 次精确 UTF-16 查询和实际状态记录确认
Composer/mount/单次动作保持；Run/Stop、换 glyph/tone、隐藏恢复、窗口局部
Reduced Motion 输入、真实失活及关闭均有证据。CloseRequested 显式调用原
Window.close；两轮旧超时/采样及中途编译/断言失败保留，未改签通过。

原生仅一个实际 backing/Light palette，不冒称其覆盖全部主题；相应像素/几何
范围由独立真实 GPU 覆盖。物理键盘/Pinyin、VoiceOver、跨进程 #48、整页 Gold7
及性能仍未验证，不作为本票原 AC 外的新增关闭条件。原 Root native 文件中另有
未接收 Link 业务模式，本次只移植原默认 Icon 场景，Root 双模式原文件完整保留，
不宣称 Link 已交付。证据在 SSD
`artifacts/issue-47/main0933180-icon-intake-candidate-20261029-01/` 的
`main-cpu01..03`、`main-status04`、`main-gpu-native-build05..06`、
`main-native-close-build08`、`native06`、`native07-sampled`、`native08-close`。
旧 Theme1.5.3 缺 packet 的 ignored 用例仍是历史边界，不计当前正例通过。
远端不可变版本/必要 CI 的最终身份由本票及交付 PR 回填，尚不能用本地提交代替。

## 历史 Root 方案与分阶段证据


本阶段继续 #47，保留全部旧证据和原验收条件。不改 AppHost，不发布或关闭 Issue；用户延后原生听读，本轮只做后台验证。

## 根因与共享位置

静态 IconContent 和 checked 资源已存在，不新增状态附件组件或业务 ticker。原 LinearRotation、TransformedImageQuad、motion_context、request_animation_frame 可复用；此前缺少不定旋转的语义周期，以及当前 ContentHost 的窗口调度许可向子组件的传递。

在 authoritative design/source/motion.json 新增一个 continuous.indeterminate_rotation_period_seconds。默认沿用既有旋转 recipe 的一周周期，非按钮/浮层过渡时长；编译器拒绝缺失、非数值、非正数、超出 f32 表达范围及下溢。生成常量由原 tokens 包公开，渲染不内嵌第二份数值。

IconContent.set_indeterminate(bool) 默认为 false；开启时旋转原 canonical SVG，量度/资源/tint/AX 仍用原机制。标准 Button 可直接承载，装饰 glyph 没有第二个动作、焦点或 AX owner。组件仅在成功绘制、可见、活动、非 Reduced Motion 时通过原 renderer 申请下一帧。缺时钟、逆时钟和上传错误显式报告，错误不会制造自循环。

原 ButtonTextRenderer 增加 window-level animation_frames_allowed 查询，ContentHost 的现有 TextAdapter 合并真实 window_active 与上游许可；borrow、LabelRenderer 和 WithIconRenderer 均转发。嵌套适配不建立独立窗口状态，原 capture、焦点与 IME 路由不变。此共享门槛也阻止其他组件在失活窗口继续将帧请求转发给平台。

量度补查发现另一个原缺口：根/拉伸槽位给大 allocation 时，IconContent 以槽位宽度栅格化，放大了已经测成 Regular 的图案。冻结 metrics-red 用例明确复现；measure 与 paint 现在复用同一个 private natural_side 计算，原 IconView 负责居中及受限裁剪，不在消费者限定图标尺寸。

## 适用范围与例外

适用于同进程原 Content 的被动图标、glyph-only Button、混合按钮内容、Summary/Launcher 等业务声明；SDK 不认识 Agent/task/session。消费者只设置 canonical identity 与当前运行状态。资源服务按原 renderer/尺寸/tint 复用，旋转不创建每角度一张图片。

静态图标不要求时钟；活动不定图标缺少 motion_context 时明确 MissingMotionContext。Reduced Motion、禁用动画许可或窗口失活显示原静态图案，不请求旋转帧。恢复从当前时钟的新一轮开始，不补播后台时间。停止、隐藏及卸载结束组件工作；祖先不可见时原 paint traversal 不再申请帧。任意自定义组件仍须遵守原共享时钟和帧请求接口。

跨进程图像传输/资源能力尚未因此交付；#48 保持独立验收。公开精确 pin 与依赖闭包也未发布。

## 消费与验证

```rust
let mut icon = IconContent::new("tekes-back")?;
icon.set_indeterminate(is_running);
// 仍放进同一个 ContentHost、标准 Button 或其他原内容槽。
```

原独立 consumer 增加 Run/Stop 声明，使用原系统 SystemMotionChanged 更新偏好，原窗口动画事件驱动宿主时钟；没有消费者业务 ticker 或 SVG 引擎。原 Composer 合成 ni 标记、选区和 binding 在 Run/Stop、glyph 更新、selected/disabled/loading 与辅助动作后保持；真实物理输入仍需单独验收。

正式检查、冻结输入、失败前后证据、清理与可重放程序见 design/reviews/icon-motion-content-20261023。检查含 JSON→Rust、公共绑定、独立 consumer、原 SVG GPU、窗口许可/停止边界、默认/放大字号及窄裁剪，不测试帧率或性能。GPU 检查比较不同角度的实际像素、整周恢复、单一资源身份及原 clip 外的 ground；不以截图证明调度预算。

本轮不启动小宿主或 VoiceOver，不把构建、合成 marked、GPU 或普通对比度测试提升为原生、物理 IME、听读、完整 Gold7。#47 仍为部分完成；Theme 独立 icon 通道/原缓存检查按其精确源另行记录。#14 的整库矩阵、#8 旧采样、#13/#12 总验收均未借此盖章。
