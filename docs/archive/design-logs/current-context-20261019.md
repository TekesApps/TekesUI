# 当前能力 context 入口

根因属于 #8 的机器入口契约：implementation inventory 已有当前公开 API、路由、状态和证据，
但 integration.json 的 tools.context 仍指向不可变导入包。该工具生成的是历史提案的40项计划数据，
不能作为活跃 Rust 实现的当前能力判断。

复用现有 generate-component-capabilities.py 增加只读 --context 模式。component-context.py 仅把
默认执行转到该模式，所有源码解析、双向公开 API 检查、状态/token证据及清单 freshness 检查
继续由原生成器负责，没有第二套注册表、token解析或成熟度推断。全部公开组件均适用。

```sh
python3 scripts/component-context.py
python3 scripts/component-context.py --component tui.button
python3 scripts/component-context.py --component component-button
```

默认输出已核对的完整 capabilities JSON；可按稳定ID或Gallery route选取一个组件，并携带完整清单
的SHA-256和覆盖边界。未知组件、旧清单、旧编译绑定、变化源码或缺失公开API分类均非零退出，
stdout为空；不会修改产物以使查询通过。修复来源后，显式运行原生成命令更新清单，查询入口本身不修复。
组件的experimental、未完成状态、runtime-unverified和experience-unverified保持原样。

integration.json 的 tools.context 指向新只读入口；原工具改列 tools.historical_context。
原ZIP、导入工具、历史生成context与提案规范不修改。effective_specifications.components 仍表达
不可变提案契约，并明确指向 supplement.current_capability_inventory 作为实际实现状态来源。
工具入口、integration和navigation也纳入能力清单的内容哈希，入口变化必须显式重新生成。

验证范围：新的CLI红例先在未改生成器上失败；隔离源码验证新只读查询、稳定ID/route选择、
未知项拒绝、inventory/binding/source三类过期拒绝和文件内容/mtime不变。
完整能力治理回归、真实CLI与整合引用检查的结果位于 design/reviews/current-context-20261019/。
不从Python门禁或context输出推断公共Rust行为、原生体验、Gold7、IME或VoiceOver通过。

活跃整棵工作树目前还有并行新增的TickingNumber/TickingNumberError未分类；新context按设计拒绝输出，
不会拿隔离候选的成功输出冒充当前WIP完整通过，也不会把新组件自动扩入本轮体验验收。
Field的203d38ff冻结包和运行中的小宿主未改，继续等待其活动原生验收；没有性能测试或Git发布动作。
