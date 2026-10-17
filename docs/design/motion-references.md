# 动画参考与下一层模型

研究日期：2026-10-15。Apple defaults 已实现；2026-10-16 已新增四个可交互 Gallery 配方，见 [实现与操作](motion-recipes.md)。下表仍区分公开事实与设计建议，未对品牌产品做逐帧测量。公开发布演示不等同于真实产品每个版本的行为，未公开的时长、曲线、弹簧常量均标记未知。

## 官方参考

| 来源 | 已确认的内容 | 对 TekesUI 的启发与边界 |
| --- | --- | --- |
| [Figma Motion，2026-06-24](https://www.figma.com/blog/introducing-figma-motion/) | 同画布时间线、位置/缩放/旋转/透明度独立关键帧；fade/move/scale 样式可并行或串联；Dev Mode 导出 | 值得新增多轨 Timeline 与配方组合。它是近期的新工具能力，不代表出现了全新的物理定律；品牌统一默认数值未知。 |
| [Figma Spring 工程文章，2022](https://www.figma.com/blog/how-we-built-spring-animations/) | 介绍弹簧动画的设计与工程实现 | 参数预览与实际轨迹一起看；已有 spring 内核可复用。不能把任何常见网页弹簧参数称为 Figma 官方预设。 |
| [Figma 动画原则](https://www.figma.com/blog/principles-in-motion/) | anticipation、overshoot、follow-through、hold、settle，以及通过节奏组织叙事 | 多阶段反馈可以表达预备、主体动作、落定。品牌视频的叙事节奏不直接照搬到高频控件。 |
| [Airbnb 2025 夏季发布](https://news.airbnb.com/product-releases/airbnb-2025-summer-release) | Homes / Experiences / Services 组成重设计后的 App，官方页面提供产品视觉材料 | 用作跨入口导航、服务卡片与详情层级的视觉参考；本轮未逐帧观看并测量完整 App 转场，不能声称其确切动画模型/数值。 |
| [Airbnb Lottie 官方仓库](https://github.com/airbnb/lottie-web) | After Effects 导出动画的运行时，支持播放、分段、进度与速度控制 | 适合图标、成功反馈、引导插画等资产动画。它不是整套 Airbnb UI 的弹簧默认值，也不是新增通用依赖的理由。 |
| [Claude Design，2026-04-17](https://www.anthropic.com/news/claude-design-anthropic-labs) | 对话、直接编辑、细粒度旋钮与互动原型；可沿用团队设计系统 | 值得借鉴“在实际内容上调参数”的预览方式。本文没有公布 Claude 自身 UI 的动画规格；不推断其等待指示器、面板或文字流的时长。 |
| [Claude Artifacts 官方发布，2024](https://www.anthropic.com/news/artifacts) | 在对话之外呈现可交互产物 | 为聊天→预览、局部刷新与阅读稳定性提供场景。它是早期产品参考，不当作 2026 当前 UI 的逐帧证据。 |
| [Cursor Canvases，2026-04-15](https://cursor.com/blog/canvas) 与 [Design Mode，2026-06-05](https://cursor.com/blog/design-mode) | 交互画布与已有工具并置；指向元素做局部修改，在运行产品里迭代 | 值得测试焦点高亮、局部状态变化、面板切换时的上下文连续性。官方没有给出对应动效的 easing / duration / spring 常量。 |
| [Apple Phase / Keyframe 动画](https://developer.apple.com/documentation/swiftui/controlling-the-timing-and-movements-of-your-animations) 与 [WWDC23](https://developer.apple.com/videos/play/wwdc2023/10157/) | 离散阶段与多轨关键帧组织更复杂动画 | 这是成熟可复用的基础，适合作为 Rust 模型下一层；并非 2026 新发布的 API。 |

## 建议的模型与 Gallery 配方

以下参数均为 **TekesUI 待试验起点**，不是上述公司的内部常量。先通过 Gallery 比较，再决定是否纳入项目设计 token。

| 优先级 / 模型 | Gallery 场景 | 起始配置 | 中途改目标策略 |
| --- | --- | --- | --- |
| P0 共享元素 / 布局转场 | 卡片→详情、选中胶囊跨 tab | 现有 Apple Smooth：duration 0.5s / bounce 0；同一进度驱动 bounds、圆角，内容独立淡入 | 从当前呈现几何继续；稳定 identity，避免重复元素与闪现 |
| P0 多属性配方 | 面板展开、菜单、toast | 项目自定义 opacity 160ms；位移/缩放用 Apple Snappy 0.5s / bounce 0.15；支持同时或延后执行 | 每轨可取消；弹簧保留速度，透明度不越界 |
| P1 阶段状态机 | 保存→成功、轻量确认反馈 | 自定义 80ms 预备、160ms 主动作、120ms 落定；不得阻塞实际操作 | 新状态取消旧阶段，异步完成触发状态变化，不靠固定时长假装任务结束 |
| P1 Keyframe / Timeline | 图标微动、提示与引导 | 自定义 0→1 时间轴，分别编辑位置/旋转/缩放/透明度 | 明确 seek / restart / cancel；关键帧默认不承诺任意打断的速度连续 |
| P1 内容更新稳定性 | AI 文字流、列表插入、工具状态展开 | 自定义 120ms 局部 opacity；已读内容锚点固定；用户滚动优先 | 最新事件取代过时反馈，内容与滚动不相互争夺控制 |
| P2 资产播放通道 | 成功图标、空状态插画 | 资源自带帧率/时长，显示 progress / segment / loop，不编造统一值 | 切段/暂停策略明确；由宿主时钟推动，仅活跃时请求帧 |

这些能力分三层：`Spring/Tween` 负责随时间求值；`Timeline/Phase/Transition` 负责组合、身份和生命周期；组件只把选中、展开、成功等语义状态映射到配方。基础模型与组件耦合低，布局/身份转场与组件树交叉，适合复用但不能完全脱离组件系统。

所有配方需覆盖：快速反向、重复触发、卸载、失焦、Reduce Motion，以及停止后的无空转。面板或卡片的“高级感”应来自连续性和层级清晰，不能只靠增加 bounce。

## 2026-10-16 扩展候选

新增 Material / Fluent / Carbon / Motion / Apple 官方资料核对与十个候选场景，见 [下一批主流配方](motion-recipe-candidates.md)。其中包含 M3 新物理系统的空间/效果分轨、平台默认 spring attribute，以及示例参数与默认参数的区别。本轮仅研究整理，尚未新增这些候选的 Gallery 代码。
