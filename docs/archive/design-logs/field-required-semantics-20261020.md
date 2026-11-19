# Field 必填语义共享整改

当前活跃 WIP 已实现，原生窗口／VoiceOver 验收仍待完成。证据入口：`design/reviews/field-required-primary-20261020/acceptance.json`。此前独立候选7b744e2c保留原身份；主工作区应用记录单列，不改旧候选。

根因属于公共组件语义及平台快照：Field.required 已有可见标记，但没有经过 ContentSemantics、AccessibilityNode 进入 AppKit。现在复用这两个既有结构转发 required 布尔值，真实 TextFieldContent、Native/Gallery 发布器及 macOS `isAccessibilityRequired` getter 接通；变化进入原有 layout-change 通知分类。现有失效节点／关闭所有者保护继续生效，AX setter 不允许修改业务约束。没有新增 token、状态机或抽象层，JSON 设计样式未改变，Field.required 仍由原模型负责。

适用于已有 Field／TextFieldContent 消费者，包括单行、多行、只读和无表单装饰呈现；无装饰模式允许明确提供语义元数据，不因此重加可见装饰。合理例外：未声明必填的控件默认 false；裸 TextSurface／View 宿主仍从自己的表单模型发布元数据。required 与 disabled、read-only、invalid 独立，不自动制造验证错误或改变编辑权限。

这是公开 Rust 结构字段的源码迁移：直接使用 ContentSemantics／AccessibilityNode 字面量的消费者需补 required 字段；ContentSemantics::new 继续默认 false。现有生产发布器和当前小宿主源码已迁移。未改变 AgentKit formal431、公开61dd或532后续冻结源；这个后续#5改进不是#23新增关闭条件，未来明确升级包含本修改的版本时再迁移消费者。

验证分别记录：

- 公共 API：Field required3项测试／32组正交组合，以及原 metadata3项／48个调色板样例通过。
- 真实接入：normal/error/disabled 的公共 Field 消费者通过，生产 Native 示例／Gallery／当前小宿主类型检查通过。
- 平台对象：主线程36项真实 AppKit getter／失效对象检查通过，没有创建 App／窗口或操作桌面；不能等同于活动窗口或 VoiceOver 听读。
- 当前源码门禁：旧 token 证据确实因修改被拒绝，九组探针实际重跑4416个有界样例后重绑定；42项治理、8项组合审计和相关包全targets严格Clippy通过。纯Rust检查及本轮文件格式检查通过。全root格式检查仍有其他既有Gallery WIP差异，原始日志保留，未格式化无关文件，也不声称完整root CI已绿。

清理43项测试可执行文件，保留共享库／依赖缓存、源码及证据。当前外接屏旧输入宿主保留其原身份等待激活；新版本重建后仍需实际窗口 AXRequired／适用 VoiceOver 验收。旧GPU／Gold7收据不改身份，本轮无新增视觉规则，不宣称重新完成全部体验。性能延期；未提交、推送、打tag、合入或写关闭Issue。

官方属性定义：https://developer.apple.com/documentation/appkit/nsaccessibilityprotocol/isaccessibilityrequired()

后续格式门禁收口：另一个独立格式整理轮次已核对并规范6个既有Gallery文件；全工作区 cargo fmt --all --check 已通过，当前能力投影和45行组合审计均匹配源码，其他Rust文件哈希保持。上述较早格式阻塞日志保留为历史。该轮未执行功能、GPU、原生或性能复测，不改变其验收状态；详见 design/reviews/gallery-format-gate-20261020/acceptance.json。

真实小宿主发布链补验：`design/reviews/input-native-projection-20261020/acceptance.json`。原 Window 发布代码与测试共用同一个私有映射函数；TextField／TextArea／SearchField 的96组必填、禁用、只读、错误、活动窗口组合及动态更新通过，宿主严格 Clippy／格式通过。没有新增库抽象或视觉规则；旧运行 App 字节未改变，这不是新版本活动窗口／VoiceOver 或最终 Gold7 验收。测试可执行文件已清理。

最新原生消费者已另轮重建：`design/reviews/input-family-final-native-20261020/acceptance.json`。同一小宿主使用原生Review菜单重置四种既有测试状态，前两输入必填、SearchField可选；保留真实输入事件和状态历史。3项公开消费者及96组真实发布器组合、严格Clippy通过，U2790B@2x已启动；初始非活动画面可见正确必填标记，但活动窗口、AXRequired／VoiceOver仍未验，不用标记截图代替语义验收。旧原生进程已退出，旧收据只作历史；最新副本等待用户激活，缓存中宿主构建产物已清理。

### 本轮真实活动窗口与宿主适配

`input-family-final-native-20261020/native-attempt1/acceptance.json` 记录 U2790B@2x 活动窗口中三种别名的正常／错误编辑、只读拒绝删除、禁用焦点和编辑隔离、正反 Tab。必填可见标记与发布器测试不替代 VoiceOver 必填播报验收。

现场发现小宿主未接入 Command+Z；共享 TextSurface 与正式消费者已有能力。仅复用正式消费者的无 Edit 菜单快捷键适配，补入此宿主。四项 API／发布器测试、严格 Clippy、构建与格式检查通过；新进程原生撤销／重做及 VoiceOver 仍待验证，旧证据不冒充新源版本的快捷键通过。无共享库或 token 新增。

### 最终必填配置 GPU 补验

前一轮 GPU 夹具三个字段均为 optional，不能证明最终 native 的 true/true/false 配置。本轮在同一独立消费者和真实 CoreText/GPU 后端上补128帧前后对照：四主题、四状态、280pt/1.5与640pt/1.0两组配置、@1x/@2x。64组必填标签可见变化，可选 SearchField 的输入区域逐像素保持一致，192组字段正文／选区／marked状态保持。抽查浅色标准和深色IC窄窗大字错误态的前后各一张。无库、token或native编译源变化。证据 `design/reviews/input-family-required-gpu-20261020/acceptance.json`；本包构建产物已清理，128张图与源码保留。原生撤销／重做和 VoiceOver 尚待，不替代整库Gold7验收。

### 新版原生撤销／重做补验

用户将33109进程置于外接屏前台后，三种真实输入别名的混合字形选区删除→Command+Z恢复→Shift+Command+Z重做全部通过；只读夹具三种别名均保留选区并拒绝Backspace。`verify-shortcuts.py` 从活动窗口事件序列及绘制后状态交叉核实，证据见 `input-family-final-native-20261020/native/acceptance.json`。这里是工具驱动原生按键和AX选区，不是物理IME或VoiceOver；Reset会重建夹具，因此不声称验证了带既有历史的只读Undo。必填／可选VoiceOver播报仍待；窗口为此保留，VoiceOver尚未开启。
