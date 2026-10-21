# Slider Component / numeric semantics · 2026-10-18

基于全部当前 WIP；原 #3/#5/#8/#12/#14 保留，不测性能、不提交/推送/关闭 Issue。

## 根因与共享修复

原 Slider/SliderView 已有完整数值、步进、几何及交互模型；缺的是 Component 接入、数值域和相对语义动作。新增 SliderContent 直接持有原模型/View，不复制其状态机或重写绘制。名称只作语义名称，视觉标签/说明继续由已有 TextContent/Stack 组合。测量复用 JSON 的 size-row-height，无新 token/状态/框架或组件身份。

Component.value_range 默认 None、ContentHost.value_range 查询当前活跃作用域；使用 std::ops::RangeInclusive，过滤非法数值域。已有 ContentSemantics/SemanticNode 字段与 struct literals 不变。SliderContent 发布 Slider 角色、当前字符串值、范围；Increment/Decrement 调原 Slider.key，SetValue 调原 Slider.set，保留原 finite/clamp/snap 规则。无效/非数值/非有限输入拒绝；到达边界仍可处理动作，changed=false。语义更新取消旧指针手势；禁用/取消/卸载仍复用既有 View/ContentHost 生命周期。

适用：现有 Slider 和需要可选数值域/相对动作的消费者组件；不要求所有组件成为范围控件，不迁移其他组件族。范围数据不是子 UI 槽位；普通 Button 里嵌入独立范围交互继续按既有诊断拒绝。

## 原生数值语义

macOS AccessibilityRole::Slider(AccessibilityRange) 携带有限上下界；描述符字段私有且构造时校验，保留现有 AccessibilityNode struct literals 与 Role Copy/Eq 约定。原生值为 NSNumber，上下界独立暴露，支持增减、数值 SetValue、焦点；disabled/read_only 拒绝修改。沿用现有快照差异/ValueChanged 通知路径，并对范围边界变化触发通知。独立原生消费者增加 slider 模式，投影同一 ContentHost，使用真实公开适配器。

增减动作需表达相对修改，不能用旧 SetValue 从可能滞后的快照算绝对值，否则连续请求可能丢步。Apple 的 Slider 契约要求值与增减方法，增减返回是否触发请求，且值变化须通知；这里由已有窗口快照更新发布通知。[Apple Slider protocol](https://developer.apple.com/documentation/appkit/nsaccessibilityslider?language=objc)、[increment](https://developer.apple.com/documentation/appkit/nsaccessibilityslider/accessibilityperformincrement%28%29?language=objc)、[minimum value](https://developer.apple.com/documentation/appkit/nsaccessibility-c.protocol/accessibilityminvalue?language=objc)。

兼容性：原 Slider/SliderView API 与快照字段不变，新增 trait 默认方法/host getter 为增量接口。ContentRole 已 non_exhaustive；experimental composition/platform AccessibilityAction 及平台 AccessibilityRole 新增 variants，外部穷举匹配需显式更新。已更新已知独立原生消费者并执行编译/Clippy；不能据此声称所有未知消费者源兼容。未加入广泛迁移或预设未来范围控件。

## 分层验证

独立消费者公开场景：同一 Stack 中 Text、SliderContent、TextFieldContent、ButtonContent。验证范围/名称/值、相对增减、有限设值/原步进、边界、无效输入、重排/移除/再挂载、焦点、真实拖动、语义更新及禁用释放捕获、嵌套动作诊断。TextField 使用 CoreText shaping，glyph painter 为 no-op；不替代 GPU。

48条四主题三品牌公共 SliderContent 实际 token 路径与原216条 SliderView 六态合并为264条。共享源码变化后重采全部8组观测，共4500条，complete=false；不是完整状态组合或 token 依赖穷尽声明。公开适配器归入原 tui.slider 的 component-adapter，组件总数仍45，131项公共状态/125项fixture缺口不变。

Gallery在 composition-review/test States 追加同一公开消费者场景，经增减/键盘后渲染，保留默认构建。GPU真实绘制与原生Gallery视觉检查分别留证；原生新AX角色/边界/动作桥的编译/快照验证不能替代实际AX调用或VoiceOver。收据 design/reviews/slider-composition-20261018/receipt.json。

尚未完成：将新公开消费者纳入Gallery真实原生AX操作验收；物理IME、VoiceOver、实时异步、完整动效及其他组件族统一验收。这里只把公开适配器/契约标为已实现，原生接受状态保持未验证。
