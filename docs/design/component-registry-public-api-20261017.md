# #8 当前组件登记：公开编译绑定与历史身份映射

2026-10-17。基于 `bc3de79466834e99062a1b4ef0872323e2860001` 加活跃 checkout 的全部未提交 WIP。GitHub 当前仍为 14 个开放 Issue、全部状态 0 个 PR；最新 CI 是旧 `bc3de79` 的失败，本批未推送，不能据本地结果声称远端 CI 恢复。#13 当前总览见 [整改当前状态](issue-remediation-current-20261017.md)。冻结源、编译/GPU/门禁结果和逐验收项状态见 `design/reviews/component-registry-api-20261017/receipt.json`。

## 根因与共享修复

根因属于公共组件契约与治理：旧清单用全仓同名 `pub` 声明证明 API，既会漏掉 ScrollArea 的真实 re-export alias，也可能接受 `tekes_ui_controls::widgets::Button` 这样的私有模块路径。17 项没有 API 字段，Gallery 又有自己的 renderer→API 回退表，导致清单、文档和实际消费入口不一致。

现在复用 `design/gallery/workbench.json` 为登记唯一入口，补齐当前 45 项的主 API 和 80 条模型/视图公开路径。Text、TextField、TextArea、SearchField 明确共享 TextSurface/TextInput 能力，按原变体配置使用；没有为消除登记缺项另建编辑器或 UI 类型。Surface、Icon、Skeleton 直接登记实际公共 View；Toolbar 只登记当前动作组模型/View，不冒充 #14 的窗口级注册与生命周期能力。

`scripts/generate-component-capabilities.py` 按模块与 re-export/alias 定位真实声明和当前行号，不再从全仓同名类型随意选文件。定位器保守支持当前明确的外部模块/扁平 use 组；它不是完整 Rust 解析器或可见性证明。生成的 `crates/tekes-ui-controls/tests/component_registry_api.rs` 才是独立的编译验证：在库外 integration-test 上下文使用全部 80 条实际类型路径，并逐项比较当前 JSON 的组件 ID、route 和 binding。API/route 改动而测试未重生成会失败；同名私有路径不能靠声明扫描通过。

生成的 JSON 继续写 `compiler-check-required`，不把“生成过测试”写成“已经通过编译”。本批通过结果由冻结源收据保存，未来源码变化必须重新编译。JSON 原有字段类型保持兼容，新编译信息为增量字段。

Gallery 的所有组件 API 引用改为读取同一登记；移除组件 renderer 的独立猜测表，缺项显式显示登记失败。旧文档对所有基础组件一律列出 button-hover 等通用 token，不能证明真实依赖，现改为明确未逐项审计；本批没有伪造完整 token 映射。

## 40 → 45 的显式映射

新增的 `design/gallery/historical-component-map.json` 完整解释原 40 个 ID 和当前 45 个 ID：

- 39 项沿用稳定 ID；这只表示身份延续，不表示旧提案的 API、状态和体验已经实现。
- 当前增加 Surface、Toolbar、Field、Select、NumberField、Skeleton，共 6 项，分别注明当前能力和例外。
- 原 SelectionSurface 与当前 TextSurface 有部分能力重叠；跨节点富文档选择仍未实现。明确记录部分重叠与缺口，不静默丢弃、不标作已合并完成。

缺少/重复历史 ID、未解释的当前新增项、未知目标、伪装为 same-id 的改名都会使生成失败。历史原包、40 项 planned 登记和原截图完全不改。

## 公共 API、真实接入与 Gold7

| 层级 | 证据及边界 |
| --- | --- |
| 公共 API | 当前 45 项、80 条真实模型/View 路径全部在外部 Rust integration test 中编译通过。另两个 workspace 外临时 Cargo 消费者分别引用 widgets::Button、extended::Button，实际编译均以 E0603 拒绝，完整日志保留。12 项 Python 治理测试覆盖 alias、错误模块、缺失 API/绑定、历史缺项/重复/未知 ID。 |
| 真实接入 | Gallery 逐组件检查 Usage 与同一 JSON API 一致；45 个真实 docs route × 4 主题 × 两组布局/倍率，输出 360 张真实 GPU 图。另回归既有 Gallery 页面 520 帧和独立组合场景 80 帧；组件数维持 45，没有扩充迁移。独立组合消费者 13 项回归通过。 |
| 最终体验 | 抽查 ScrollArea 普通浅色页和 TextArea 深色 IC、窄窗口大字号/@2x 文档页，确认真实 API 名称可读、按模板排版。非背景像素或编译成功都不是全页体验通过；没有把 960 张图写成逐图人工验收。G1–G3 沿用 JSON 视觉规则，未改 token 或组件外观；G4/G5/G6 的物理动效、候选窗、VoiceOver 不因本批晋级；G7 增强公开路径和可运行 Gallery 的一致性。 |

工作区最终 Rust 测试 897 passed、0 failed、1 原有 ignored；两个计时测试按用户范围显式过滤。fmt、严格 workspace Clippy、DS、纯 Rust、Gold7 mechanical、reference alignment 及生成清单检查通过。既有 CI 已运行该生成检查和 workspace tests，因此新编译绑定会进入同一门禁；无需另增一条只看声明存在的检查，也没有修改或绕过原门禁。

## 原 Issue 未完成项

#8 仍为部分完成：编译绑定、45 项主 API 补齐与完整历史身份说明已交付；全体 public/helper 类型的反向分类、每个组件必需状态的覆盖，以及完整 token 依赖映射仍未闭合。新增未登记类型仍需这一反向清单才能精确指出缺项，不能声称本批已经解决全部公开面治理。所有组件继续 experimental，体验状态未自动改为 pass。

后续继续原 Issue 的反向类型分类和状态/token 审计；#7 消费方品牌入口、#10 公共阅读控制器、#11 草稿协议仍按现有 WIP 补公共边界，不重做已有 Gallery 实现。#5/#12 的候选窗和 VoiceOver 验收也继续保留。未自动提交、推送、打 tag、关闭 Issue 或发布，原 WIP 与历史证据保持保护。
