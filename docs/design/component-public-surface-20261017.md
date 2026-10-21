# #8 公开类型反向覆盖与漏登记门禁

2026-10-17，活跃 checkout `gallery-design-baseline/TekesUI`，基线 `bc3de79466834e99062a1b4ef0872323e2860001` 加全部未提交 WIP。重新读取 GitHub 后仍为 14 个开放 Issue、全部状态 0 个 PR；#14 更新内容保持 `2026-10-17T08:15:16Z`，不执行全库迁移或窗口级 Toolbar 实施。

本批收据、逐项验收状态、源文件哈希与覆盖层、实际日志和新 GPU 图位于 `design/reviews/component-public-surface-20261017/`。此前各批报告、截图和收据保留原样；[当前总览](issue-remediation-current-20261017.md)继续独立维护。

## 根因、共享修复与适用范围

根因属于公共组件契约治理：只有 registry→真实类型的正向绑定，仍无法发现新公开类型未登记。不能把所有公开 struct 自动当组件，也不能把未登记类型静默当 helper。没有增加运行时抽象、组件或 UI 框架。

复用 `scripts/generate-component-capabilities.py`、现有 `workbench.json`、`capabilities.json` 和外部 integration test。新增 `design/gallery/public-api-surface.json` 仅记录公开类型的显式分类、现有组件身份和理由；组件 ID、route、主 API 仍只由 workbench 维护，不建立第二套组件登记。capabilities 从两者生成实际源码、声明类别和关联 route。

核对 controls 的全部可达公开模块、显式 re-export 与 alias，共 180 条公开类型路径：77 条组件模型/View 路径、48 条组件专用辅助、29 条共享辅助、14 条组合协议/运行时、11 条既有组件组合适配器、1 条组件兼容别名。77 是去重路径数，原 80 条组件绑定包含四种文本页面共享 TextSurface；当前组件总数仍为 45。

适用组件为当前登记的全部 45 项；组合适配器关联现有 Button、Text、Stack、Grid、ScrollArea、List、Switch、TextField、Menu、Dialog 家族。CompoundButton 不新建组件 ID；DiffViewPresenter 对应既有 DiffViewRuntime；FilteredMenuView 是 Select、ComboBox、CommandPalette 的公共实现辅助。文档数据节点、几何结果、状态/事件、绘制配置、宿主接口不获得独立页面或“已迁移”状态。辅助类型仍是公开 API，并不因分类而成为 Rust 私有类型。

## 门禁行为与合理例外

- 新公开类型或新 alias 没有显式分类时，`--check` 报出完整缺失路径。删除、私有化或重命名后的旧分类也报出准确路径。
- 组件分类必须与其现有 registry binding、ID 和 route 对应；已登记组件不能改标成 helper 来规避覆盖。适配器/组件专用辅助必须关联已有组件；兼容别名必须指向关联组件的实际类型。
- 所有 180 条路径追加进现有库外 Rust integration test；trait 用泛型约束编译验证，不额外要求 dyn compatibility。类型可见性不依赖 Python 的源码判断。
- 源码索引屏蔽注释/普通、原始和字符字面量，仅枚举模块顶层声明；private/pub(crate)、局部类型和私有测试模块不会扩入公开面。
- 当前只支持明确的外部模块及扁平 re-export。公共 inline module、glob、模块别名、未知顶层宏/其他导出形式会明确失败，要求扩展索引；不静默跳过。它是保守索引，不是完整 Rust AST 或 cfg 解析器。
- 当前唯一顶层生成 include 是滚动 token 常量。已审阅 build.rs 只生成七个私有数值常量；例外绑定精确 include 语句和生成器 SHA-256。生成器变化会要求重新审计，不能借 include 隐藏新类型。

范围是 `tekes_ui_controls` 的 struct/enum/trait/type 路径；函数、常量及其他 crate 的基础类型不在本批类型审计中。新类型属于组件还是 helper 仍需语义审查，工具不能自动证明分类理由正确。现有 CI 的生成检查、Python tests 与 workspace tests会执行这些门禁；单独运行旧生成的 Cargo 测试并不能发现尚未重生成的新源码类型，必须配合 `--check`。

## 分层验证

| 层级 | 本批结果与边界 |
| --- | --- |
| 公共 API | 两项库外 integration tests 通过：原 45 项/80 条组件绑定保持，另 180 条公开类型路径全部编译可见。20 项 Python 治理测试通过，覆盖实际临时源树中新增类型、alias、private/局部/注释排除、辅助类型误标、丢失绑定/route、错误别名、未知导出形式及生成器变化。两个独立完整源码副本的 CLI 负例准确拒绝 MissingRegistryControl/MissingRegistryAlias；没有在活跃 WIP 注入负例。 |
| 真实接入 | 独立 composition-consumer 13 项通过；真实 Gallery 45 个 docs route × 四主题 × 两组布局/倍率输出 360 张真实 GPU 图。组件数、路由、运行时和迁移范围均保持。 |
| Gold7 体验 | 360 张新图与上一批全部逐字节一致；抽查窄窗口大字号/@2x、深色 IC TextArea 的真实 API 文档。G7 的可查性和契约可发现性增强；没有把类型可见、像素相同或 mechanical gate 当成全组件体验通过。G4/G5/G6 的物理输入、候选窗、VoiceOver 等既有未验证项不晋级。 |

fmt、controls all-targets 严格 Clippy、设计系统、纯 Rust、Gold7 mechanical、生成新鲜度、diff 检查通过。本批没有修改运行时 Rust 源码；按改动运行上述针对性验证，没有重跑全工作区测试或计时/性能测试。前批 897 项全工作区结果属于前批收据，不重复记作本批结果。

## 原 Issue 状态与下一步

#8 的“区分公开组件与辅助类型、真实符号/编译绑定及双向覆盖”已在当前 controls 公开面完成；#8 整体仍为部分完成。必需状态覆盖和完整逐组件 token 依赖未闭合，因此不能宣称 CI 已能检测所有遗漏状态。45 个组件仍 experimental，experience 不自动改为 pass。

下一批继续 #8 的状态/token 审计，复用现有 JSON 语义和共享实现；确有共性缺失时先修共享层，再统一处理适用组件。#7/#10/#11 公共边界随后按依赖推进；#5/#12 原生验收保留。无性能测量、提交、推送、打 tag、关单或发布。
