# #31 标签行数与尾部省略：真实缺口与复用边界

本轮完成实施前核验与机制实验，没有新增公开运行时布局接口，不将 #31 或八批目标算作完成。源码与证据见 `design/reviews/label-layout-assessment-20261021/`。

## 对照与根因

已读取最新原 Issue #31，并分别核对公开 8cf0840790334ba224afc0595897c74231c712e6、全部当前 WIP 和实际 ClientKit。ClientKit SummarySectionHeaderButton 使用单行、尾部省略，完整原标题作为 AX label。TekesUI 的 TextContent／ButtonContent 无等价 opt-in；ShapeOptions 的宽度和不换行不是行数限制或省略策略。

根因属于公共标签排版契约及适配缺口，不是缺少另一个字体引擎。不复制 Summary/Agent 状态或视图，不在消费者用字符数截字符串，不把编辑器当被动标签。旧自然多行行为仍是需要保留的兼容契约。

## 实际验证

两个新的独立测试目标通过且严格 Clippy 通过：

- `examples/composition-consumer/tests/label_layout_baseline.rs` 使用真实 TextContent、ButtonContent、CompoundButton 及 ContentHost，四主题、三种字号倍率、窄／宽窗口，共72个基线场景。窄窗自然增高，所有原始 AX 名称完整；CompoundButton 子内容保持装饰属性。文字由 CoreText 实际测量，glyph sink 是 no-op，没有 GPU 或窗口验收。
- `crates/tekes-ui-text/tests/label_native_probe.rs` 对243个原生 CoreText 场景观察 CTLine 尾部截断，覆盖 CJK、emoji 家庭／旗帜／肤色、组合字素、RTL与混排、硬换行、空文本和极窄宽度。保留原始 native runs、索引、坐标、合成标记及源字素边界。

观察到105个合成省略符 run，其起点在本数据集均为源字素边界；不能因此声称所有 Unicode 或完整截断 API 已通过。原生 End 截断已处理所测 RTL 排序，适合复用；不是消费者自己重排。

原语仍需共享库封装：90个 raw 调用返回空值，其中包含极窄的空文本情况；直接创建 CTLine 也会把硬换行内容作为一个行对象处理，不能直接代替 max-lines。需要库层的“是否真正截断”、极窄确定回退、硬换行／最后可见行，以及原文与合成 glyph 来源约定。

## 下一实施单元

优先复用现有 CoreText Typeset／FontFallback／Paragraph／Glyph 输出和文本服务；新输入仅表达现有接口确实无法表达的行数／overflow。保持 ShapeOptions、TextContent、ButtonContent 既有公开 struct literal 和默认路径兼容。

拟优先采用现有 Content 的 additive opt-in 属性，在原有 TextContent／ButtonContent／CompoundButton 原对象上接入，不再创建一套组件状态机。独立 Button 的必要入口与同一布局实现共享。`Component::requires_text_services` 已存在，不应再发明一层通用组件协议。

测量与绘制必须消费同一份不可变、具有真实字体和可见 glyph 的布局结果；不能分别在 measure/draw 里重新猜截断字符串。只读标签布局必须说明原文、被省略范围和合成省略符，不冒充编辑器的 caret／selection 段落。宿主缺少布局服务时明确诊断，不能静默伪装为已支持。

接口名及最后可见行算法尚未落地。仍需支持完整原 Issue 的 opt-in 输入、确定 tiny-width fallback、RTL/字素/fallback/裁剪、真实控件接入、三倍率和默认兼容，以及 GPU／外接屏／AX／匹配体验。公开版本、CI和独立消费者验收另按授权完成。

本轮只构建上述两个小测试目标。无性能测试、无新 UI 框架、无系统设置或桌面操作；测试可执行文件在确认不用后已清理，保留源码、输入哈希、机制数据及公共缓存。没有提交、推送、关闭 Issue，未改动 ClientKit、Kernel 或任何 AgentKit 冻结包。
