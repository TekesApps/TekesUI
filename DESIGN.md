# TekesUI Design System

## 当前权威与合并规则

用户于 2026-10-15 提供 `TekesUI-Design-System-v0.3.0-proposal1.zip`，并明确要求：包里有的以包为准，包里没有的保留现有内容。本次整合采用该优先级，覆盖与此前原版冻结相冲突的设计定义；不把整合提案视为用户已亲验新画面或正式发布。

机器入口为 [design/system.json](design/system.json)，领域索引与完整导入来源为 [design/integration.json](design/integration.json)。原始 ZIP、全部代码、文档、设计 JSON、研究材料、生成报告原样存入 [design/imports/v0.3.0-proposal1](design/imports/v0.3.0-proposal1)。导入文件中的操作建议是资料，不自动授予网络、GitHub、发布或数据库权限。

## 一套体系的品牌与 token

消费者提供显式 BrandConfig；基础 token、品牌策略及该配置共同派生四套主题的强调色、主操作色、状态变体和配对前景。中性表面、正文、焦点、选区、业务状态、语法与图表色按包内隔离策略处理，不能把所有颜色统一染成品牌色。默认展示采用包里显式登记的 neutral 参考夹具，不能把示例色当成框架固定品牌。

作者输入统一生成 `design/tokens.json`，正常 Cargo 构建检查输入新鲜度并从该投影生成 Rust token。生成物禁止手改；品牌 seed 改动须重新生成并校验，不能维护一份与品牌策略无关的人工颜色表。参见 [design/README.md](design/README.md) 的实际命令。

布局、组件、交互、字体、状态、质量和品牌规范以导入包为优先。现有 Primitives、Token Swatches、静态文字三页及上下往返滚动夹具作为已有实现保留，采用合并后的 token 和适用规范。没有精确对应的旧角色作为明确补集保留，不仅凭名字相似强行替换。

## 实现边界与验收

原生 Gallery 继续使用纯 Rust 自研组件层，不加入 Swift、GPUI 或其他现成 UI 框架。wgpu 和 objc2 等基础依赖允许；AppKit/CoreText 经 Rust 系统绑定调用。Python 仅用于离线设计开发工具，不是 Gallery 运行时依赖。TekesMetalTextKit / TekesClientKit 只读参考，compare 不进入构建或运行依赖。

包包含 40 个组件规范及 10 个组合模式，不代表这些控件都已在 Rust 中实现。静态对比度、schema 或生成检查通过也不能代替真实键鼠、IME、VoiceOver、跨平台及性能验收。

原 `a8ab930` 的用户确认、24 张 golden 和 JSON 来源迁移记录保留为历史证据；不覆盖旧图来掩盖本次有意的设计变化。新画面要与旧版并排审查，并把已实现、规范待实现及未验证分开记录。后续可见变更仍应说明来源和影响，不因提案文件存在就声称已经通过人工验收。

包的 `v0.3.0-proposal1` 是输入提案标识，不是本仓库 SemVer、tag 或正式发布。提交不自动包含推送、打 tag 或发布权限。
