# Symbol Effects：官方范围与 TekesUI 实现

核对日期：2026-10-16。来源为 Apple 官方 Symbols / SwiftUI 文档，版本读取其文档 JSON 的 `metadata.platforms.introducedAt`。

## 版本与效果族

| 起始 iOS | 效果 | 官方文档 |
| --- | --- | --- |
| 17 | Bounce，短暂弹跳/缩放 | [BounceSymbolEffect](https://developer.apple.com/documentation/symbols/bouncesymboleffect) |
| 17 | Pulse，整体或分层透明度脉冲 | [PulseSymbolEffect](https://developer.apple.com/documentation/symbols/pulsesymboleffect) |
| 17 | Variable Color，按层改变透明度 | [VariableColorSymbolEffect](https://developer.apple.com/documentation/symbols/variablecolorsymboleffect) |
| 17 | Scale，状态缩放 | [ScaleSymbolEffect](https://developer.apple.com/documentation/symbols/scalesymboleffect) |
| 17 | Appear / Disappear，入场/离场 | [Appear](https://developer.apple.com/documentation/symbols/appearsymboleffect), [Disappear](https://developer.apple.com/documentation/symbols/disappearsymboleffect) |
| 17 | Replace，内容替换 | [ReplaceSymbolEffect](https://developer.apple.com/documentation/symbols/replacesymboleffect) |
| 18 | Breathe，柔和呼吸；plain / pulse | [BreatheSymbolEffect](https://developer.apple.com/documentation/symbols/breathesymboleffect) |
| 18 | Wiggle，方向性或角度摇摆 | [WiggleSymbolEffect](https://developer.apple.com/documentation/symbols/wigglesymboleffect) |
| 18 | Rotate，顺时针/逆时针旋转 | [RotateSymbolEffect](https://developer.apple.com/documentation/symbols/rotatesymboleffect) |
| 18 | Magic Replace，关联符号保留共同元素 | [MagicReplace](https://developer.apple.com/documentation/symbols/replacesymboleffect/magicreplace) |
| 26 | Draw On / Draw Off，按标注路径绘入/绘出 | [Draw On](https://developer.apple.com/documentation/symbols/drawonsymboleffect), [Draw Off](https://developer.apple.com/documentation/symbols/drawoffsymboleffect) |
| 26 | Variable Draw，数值驱动已绘制长度 | [SymbolVariableValueMode.draw](https://developer.apple.com/documentation/swiftui/symbolvariablevaluemode/draw) |

iOS 16 已有 [variableValue 图标](https://developer.apple.com/documentation/swiftui/image/init(systemname:variablevalue:))，它让参数控制图层显示；本表所列完整 Symbol Effects 动画 API 从 iOS 17 开始。

前 7 项的 macOS 起始版本为 14，接下来 4 项为 15，最后 3 项为 26。Variable Draw 是渲染模式，Gallery 为观察效果而给它添加时间轴。Automatic 是按上下文选择转场的策略，不是额外的一套几何动画。

Apple 官方介绍：[SF Symbols](https://developer.apple.com/sf-symbols/)、[WWDC24 SF Symbols 6](https://developer.apple.com/videos/play/wwdc2024/10188/)。

## 独立模型

`tekes_ui_motion::symbols` 零依赖、无时钟、无平台字体/图标绑定：

- `SymbolEffect`：14 项效果族；`ios_version()` 提供官方起始版本元数据。
- `sample(effect, progress, layer, layers, options, reduced_motion)`：输入连续进度与稳定层序号，输出缩放、旋转、二维位移、透明度、有序笔画弧长区间、替换权重。
- `SymbolScope`：Whole / ByLayer / Individually 三种组合。Apple 的 Individually 选项对应 Draw；其他效果的逐层列是 TekesUI 组合实验，不表示 Apple 原生提供相同选项。
- `SymbolOptions`：强度、有限循环、归一化错层间隔、方向、角度或平移摇摆、呼吸叠加脉冲、变量层累计/迭代、隐藏/淡化非活动层、往返变量值、正序/反序绘出。组件宿主将语义方向映射为实际方向。
- `SymbolCapabilities` / `resolve`：缺少共同元素对应关系时 MagicReplace → Replace；缺少有序路径时 DrawOn → Appear、DrawOff → Disappear、VariableDraw → VariableColor。宿主先选择实际效果，不能只看效果名称而假设素材具备能力。
- `RecipePlayer::new(RecipeKind::Symbol(effect))`：复用现有外部时间驱动进度轨道，支持暂停、继续、反向、有限播放完成和 Reduce Motion。不在后台自启无限循环；持续状态由宿主在可见且活动时安排下一轮。

重复计数针对每层的本地进度。修改时长/目标保持当前进度位置，Tween 的速度按新轨迹重新计算；修改强度、循环次数、错层间隔会立即重采样，可能改变当前姿态，不承诺所有效果属性或角速度连续。暂停期间修改参数也可改变姿态，但不会恢复帧调度。

Reduce Motion 关闭弹跳、呼吸、摇摆、旋转、缩放运动，图标仍保留语义状态；可见性/替换由 RecipePlayer 即时完成，描绘用静态端点。Variable Color 保留完整可读图标。

## Gallery

Motion → **Symbols / 图标**，独立于 22 个 Recipes 和两组基础预设。15 个选项（14 个效果族与 TekesUI 匀速旋转），每项三列并排比较整体、错层、逐层；仍用同一套目标 A、暂停、目标 B、重播、中途改目标和继续操作。

默认强度 0.6、错层比例 0.12；时长与有限循环经过分别调整：

| 预设 | 整轮秒数 | 循环次数 |
| --- | --- | --- |
| Bounce | 0.7 | 1 |
| Pulse | 1.6 | 2 |
| Breathe | 2.4 | 1 |
| Wiggle | 0.9 | 3 |
| Rotate | 1.6 | 1 |
| Scale / Appear / Disappear | 0.6 | 1 |
| Replace / Magic Replace | 0.8 | 1 |
| Variable Color | 1.8 | 1 |
| Draw On / Draw Off / Variable Draw | 1.2 | 1 |

`SymbolEffect::preview_defaults()` 是代码内的单一预设来源。可调范围分别为 0.3–4 秒、0–1、0–0.6、1–8 次；不适用的参数禁用。**这些都是 TekesUI 实验值，不是 Apple 官方常量。** 图标动画内部的完整曲线和每个符号的编舞并未由上述文档公开，不能套用基础 Spring 的默认值来宣称逐帧一致。

预览直接使用已确认的 8 个图标：Settings、NewGoal、GoalCompleted、Memory、Think、Router、Provider、Appearance。图标选择按钮位于预设下面，播放/暂停/反向按钮位于选择区下面，正常窗口中无需滚到底部操作。

`tekes-ui-icons::animation::Layer` 提供稳定层 ID、24pt 坐标锚点、原 SVG 元素/子路径引用、明确绘制顺序和共享元素键。SVG 仍是唯一几何源；圆、圆弧、圆角矩形、复合路径均由现有 usvg 解析成规范化路径。所有原始子路径必须恰好被引用一次，测试检查遗漏和重复。

描边层按累计弧长裁切，曲线在部分绘制时细分为折线；完整绘制仍用原始 Bézier 路径。实心点和区域按层淡入保留填充，不将填充边界伪装为中心线笔画。路由图标拆分为主干、上下分支和两个箭头；静止/完成时恢复原 SVG 的合并绘制，避免交叉边缘重复 alpha 叠加。

NewGoal ↔ GoalCompleted 的 `goal-ring` 是经过路径拓扑检查的共享圆环，关联替换插值圆心/轮廓并淡出非共享部分、淡入目标勾选。其他 6 个图标没有关联语义，Magic Replace 明确降级为 Replace，界面提示降级。

Gallery 适配器初始化时解析 SVG 和注释，缓存每个图标/主题/倍率的原图、1.5 倍采样原图和分层图集。几何/透明度效果通过 `TransformedImageQuad` 在 GPU 上旋转、缩放、位移与混合；播放、改参及反向复用相同纹理。原图完成态使用原生分辨率纹理，保持与原 SVG 像素一致；运动中采用采样图集，覆盖模型最大 1.4 倍缩放，纹理插值不声明与每帧矢量重绘逐像素一致。

Draw On / Off / Variable Draw 与具备共享轮廓的 Magic Replace 保留 CPU 矢量路径，三张动态纹理通过 `Renderer::update_image` 原位上传。部分描绘复用弧长/细分信息，完整路径仍用原 Bézier；无对应关系的 Magic Replace 使用普通交叉淡化缓存路径。图集增加常驻显存，以减少几何动画播放中的 CPU 重绘和上传。切换图标/主题/倍率释放不再使用的缓存，暂停后不再安排动画帧。

**Linear rotation / 匀速旋转** 是独立的 TekesUI 模型，不标为 Apple 内置效果。`linear_rotation::LinearRotation` 用外部 `Duration` 驱动连续未包裹圈数；速度调整、反向保留当前角度，暂停时间不计入进度，无加速或缓动。默认一圈 2 秒，参数为周期、层相位和每周期圈数，强度禁用。Reduce Motion 暂停旋转；取消后由用户继续。Recipes 原有 22 项数量不变。

## 能力边界

已实现纯 Rust 效果模型、现有 8 个图标的动画注释，以及 Gallery 可交互的真实 SVG 预览。未调用 Apple 的 Symbol Effects 运行时，也没有提取 SF Symbols 字体。Apple 的复杂非刚性变形、逐符号手工编舞、全部 Replace 变体和视觉逐帧等价不在完成声明内。

本轮验证包含：26 项 Gallery Motion 测试；8 图标 × 4 主题 × 2 倍率的完成态与原 SVG 像素严格相等；8 图标 × 14 效果的实际光栅化；路径引用完整性、替换端点和明确降级；复用纹理的真实 GPU 更新测试。原生窗口已实际操作呼吸、摇摆、路径描绘、关联替换、暂停继续与中途反向。原生交互只检查了这些代表状态；全部组合的原生视觉、物理输入及 120Hz 性能未做完整验收。


## GPU 缓存路径验证（2026-10-16）

内核 57 项集成测试与 1 doctest 通过。Gallery Motion 31 项、perf 8 项通过；严格 Clippy 已通过。真实 GPU 图像测试 18 项、离屏场景测试 23 项通过，包含旋转、图集 UV 及零旋转与旧图元逐像素一致。64 个静态完成态（8 图标 × 4 主题 × 2 倍率）与原 SVG 严格相等；120 个运动组合的覆盖面积差低于 3.5%、重心偏差低于 0.6 设备像素。缓存计数测试确认纯几何/透明度播放与参数变化不产生新光栅化或上传，路径效果仍走明确的动态路径。

原生 120Hz/CPU 前后测量独立保存于 `design/reviews/symbols-performance/` 与 `design/reviews/symbols-gpu-cache/`；测试通过不等于实际呈现零掉帧，最终以实际 presented 收据及完整结果表为准。
