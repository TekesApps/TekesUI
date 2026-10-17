# 下一批主流动画配方候选

核对日期：2026-10-16。已有四个 Recipes 保留，本轮为官方资料研究与候选整理，尚未新增下列 Gallery 实现。“主流”指主流设计系统或官方框架提供该能力，不表示做过产品使用率排名。来源中的示例值、组件默认值、平台 token 与 TekesUI 建议值分开列出。

## 建议的十种场景

| 顺序 | 配方 | 可以实际演示什么 | 可调参数与中断方式 | 官方依据 |
| --- | --- | --- | --- | --- |
| 1 | 列表增删与重排 | 插入一项，其余让位；排序时同一项移动；删除后收拢空位 | 行距、入/出时长、placement spring；连续排序按稳定 ID 重定向，删除中撤销 | [Android Compose Item animations](https://developer.android.com/develop/ui/compose/lists#item-animations) |
| 2 | Shared Axis 方向导航 | 步骤前进/后退沿同一 X/Y 轴滑动并渐隐，进入层级可用 Z 轴缩放 | 轴向、距离、时长、淡出/入分界；途中连续前进、后退，从当前呈现状态接管 | [Material 官方工程文档：Shared axis](https://github.com/material-components/material-components-android/blob/master/docs/theming/Motion.md#shared-axis) |
| 3 | Fade Through 内容切换 | 不具空间关系的 tab / 内容块先淡出、再淡入，减少两段文字叠在一起 | 总时长、交接比例、缩放幅度；快速 A→B→C，只保留最新目标 | [Material 官方工程文档：Fade through](https://github.com/material-components/material-components-android/blob/master/docs/theming/Motion.md#fade-through) |
| 4 | 拖拽松手、惯性与吸附 | 卡片跟手、松开后按速度滑行并停在最近或预测吸附点 | 速度、减速、吸附间距、边界与弹簧；滑行途中再次按下立即夺回控制 | [Microsoft InteractionTracker / InertiaModifiers](https://learn.microsoft.com/en-us/windows/apps/develop/composition/inertia-modifiers) |
| 5 | Stagger 交错入场 | 列表按顺序出现，网格可从中心向外展开，退出可反向 | 每项间隔、起始位置、单项时长、总时间上限；入场中退出，取消尚未启动项 | [Motion 官方 stagger](https://motion.dev/docs/stagger)、[Carbon choreography](https://www.carbondesignsystem.com/building-blocks/foundations/motion/choreography) |
| 6 | 锚定菜单 / Popover 出入场 | 围绕触发按钮展开，位置、缩放、透明度与层级共同表达来源；打开与关闭独立配置 | transform origin、位移、scale、enter/exit 时长；指针快速离开/返回或再次开关可反向 | [Fluent 2 Enter/Exit 与 Elevation](https://fluent2.microsoft.design/motion)；锚定位置与可反向接管为 TekesUI 配方设计 |
| 7 | 按压反馈与空间/效果分轨 | 按下时形状或尺寸回弹，颜色/透明度平稳切换；松手回位 | spatial / effects 分别选 fast/default/slow，按压幅度；按下→拖出→拖回→松开 | [M3 Expressive motion physics](https://m3.material.io/styles/motion/overview/how-it-works) |
| 8 | Skeleton → 实际内容 | 保持内容轮廓，真实数据到达后局部替换；可对比 shimmer 与静态占位 | 模拟到达时刻、淡入时长、shimmer 周期；取消加载或重载不闪回旧内容 | [Fluent Wait UX](https://fluent2.microsoft.design/wait-ux)、[Fluent Android Shimmer](https://fluent2.microsoft.design/components/android/core/shimmer/usage)；替换转场由 TekesUI 设计，不声称 Fluent 所有 Skeleton 都内建 |
| 9 | 数字内容转场 | 计数、价格或指标随增减方向过渡，位数变化时保持对齐 | 更新频率、方向、时长、小数位、对齐；上一轮没结束又收到新值时重定向 | [Apple numericText(value:)](https://developer.apple.com/documentation/swiftui/contenttransition/numerictext(value:))；具体逐字映射和动画外观需独立设计/验证 |
| 10 | 顶层页面切换 | 整体栏目/工作区切换，内容轻微缩放与淡入，工具栏层级清楚 | 前后页面时序、scale、位移、持久工具栏；连续切页保留最新目标与焦点 | [Fluent 2 Top level](https://fluent2.microsoft.design/motion) |

优先实现 1–6：它们补齐数据列表、导航、手势、群组编排和锚定浮层，和已有四个示例的功能差异较大。7 用于改善现有多轨策略；8–10 可随后补齐加载与数据反馈。

## 较新的基础：Material 3 Expressive

Google 官方说明在 2025 年 5 月引入 motion physics system，分 expressive / standard 两套方案，每套都有 spatial / effects × fast / default / slow。Spatial 用于位置、尺寸、旋转和圆角；effects 用于颜色、透明度，避免这些属性出现越界回弹。这是已有多属性面板可以采用的分轨策略，不是又一种必须单独发明的曲线。

来源：[官方说明](https://m3.material.io/styles/motion/overview/how-it-works)，[MDC Android 当前源码文档](https://github.com/material-components/material-components-android/blob/master/docs/theming/Motion.md)。以下是后者列出的六个 **Android 默认 attribute**，不能称为跨设备、跨框架的统一默认值。`damping` 此处为阻尼比 ζ，不是力学阻尼系数 c。

| 类型 | Fast：ζ / stiffness | Default：ζ / stiffness | Slow：ζ / stiffness |
| --- | --- | --- | --- |
| Spatial | 0.9 / 1400 | 0.9 / 700 | 0.9 / 300 |
| Effects | 1 / 3800 | 1 / 1600 | 1 / 800 |

转换到 Rust 时，若采用单位质量模型：`frequencyHz = sqrt(stiffness) / (2π)`，阻尼比直接使用 ζ。不得把 ζ 直接传给 `Spring::from_physical` 的 damping 参数；该参数接受 `c = 2ζ sqrt(mk)`。实施时应锁定官方源码版本，再做数值 oracle 与中断测试。

## 数值使用规则

- Motion 文档的 `stagger(0.05)` 是每项 50ms 的**示例**，不是所有产品的默认值。可作为 Gallery 起点，再限制总延迟，避免长列表拖慢操作。
- Atlassian 的 [StaggeredEntrance 文档](https://atlassian.design/components/motion/staggered-entrance/code) 列出 `delayStep=50ms`，但其 [Entering motion](https://atlassian.design/components/motion/entering-motion/code) 有迁移到新 Motion component 的提示，因此仅保留为既有 API 参考，不当作新项目的唯一推荐。
- Android 列表文档给出的 `fadeIn=250ms / fadeOut=100ms` 是**自定义代码示例**，不是 `animateItem()` 默认常量。
- Fluent 的说明强调根据距离/尺寸选时长，本轮没有找到一套能直接套在所有这些配方上的官方数值。候选阶段不填猜测值。

## 对现有模型的影响

列表重排需要稳定元素 ID 与每项的呈现位置；导航需要前后内容的生命周期与方向；手势需要直接操作→松手速度→惯性/吸附的状态机。它们可以复用现有 Spring/Tween，但不是给单一进度换一个 easing 就够了。

所有候选进入 Gallery 时，应提供：默认配置来源、可调参数、重播、暂停/继续、快速反向/换目标、Reduce Motion，以及停止后的无帧请求。此轮没有启动原生动画窗口或 GPU 性能测量。

## 实现状态更新

以上十种已于本日加入 Gallery Recipes（总数 14）。具体已实现范围、操作和简化处以 [motion-recipes.md](motion-recipes.md) 的扩展章节为准；原生视觉与性能验收状态见 [handoff](../handoffs/motion-preview.md)。
