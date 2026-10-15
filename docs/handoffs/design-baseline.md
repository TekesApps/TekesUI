# 原版设计基线冻结交接

> 本文保留原版冻结及 JSON 来源迁移的历史。当前提案整合优先级和验收见 [proposal1-integration.md](proposal1-integration.md) 与根 DESIGN.md；原黄金图不覆盖。

## 决定与隔离

用户在确认首次文字上下滚动版本后，要求以这个设计固定 DesignSystem，保留原版。本工作区 `/Users/frankqin/.codex/worktrees/gallery-design-baseline/TekesUI` 从 `a8ab9303ab2f528913f83cfc2c8da2a0eab8b8f3` 建立；另一 worktree 的 10 页 registry/模板/审计等 WIP 原样保留，不做恢复覆盖、不合并、不当作 golden。

本轮只新增冻结契约、24 张原图、来源 manifest 与检查机制，Gallery/渲染/token 的 Rust 源不改。DS v1 是用户批准的视觉基线名称，不是 SemVer tag，也不声称已发布 v0.1.0 或正式 DS 全量完成。

## 原版来源

既有 release binary SHA256：`9805be44bcb53e8ca925ae2cc1aade0cba76fc53e6398fc5132a6cfc7ecb2ebb`，6513216 bytes。稳定证据副本：

`/Users/frankqin/.codex/visualizations/2026/10/15/01a13ead-f79a-7ff2-9a15-1c4c8366e507/verification/original-gallery-preview/bin/tekes-ui-gallery-a8ab930`

该 binary 对应基线源码与当时仅原文档 WIP，不是从当前新工作区重新构建的产物。它已以 `TEKES_PERF=scroll:15` / mixed 单次视觉预览，用户亲验；3/7/11 秒原生截图及启动参数记录在同级 `preview-manifest.json` 与 `README.md`。没有把这次预览当新性能门禁。

24 张公开 golden 原图来自 `/Users/frankqin/.codex/visualizations/2026/10/15/01a13ead-f79a-7ff2-9a15-1c4c8366e507/gallery-snapshots/`，复制到 `design/baseline/screenshots/`。三页 × 四主题 × 两倍率，完整解码与 SHA 一致；不包含私有 compare/live 截图。复制校验记录在 `verification/design-baseline-freeze/asset-copy-verification.json`。

## 最初冻结验证记录（导出工具增量之前）

冻结机制由 core 编写，worker 为主要执行者串行核验。证据根目录：

`/Users/frankqin/.codex/visualizations/2026/10/15/01a13ead-f79a-7ff2-9a15-1c4c8366e507/verification/design-baseline-freeze/`

- 通过：24 图完整文件名矩阵、逐张完整 PNG 解码、1200×800 / 2400×1600 尺寸与复制 SHA；旧 binary 稳定副本 hash。记录 `asset-copy-verification.json`。
- 通过：冻结 checker 6 组正/负例；默认当前 source/token/24 golden 核验；显式核验原 `gallery-snapshots/` 候选的全部 24 图逐字节一致且 checker 环境匹配。记录 `design-checker-self-test.log`、`design-checker-default.log`、`design-checker-candidate.log`。候选是既存原图，不是本轮重新渲染；不能把环境匹配自动当成重构建来源证明。
- 通过：pure Rust guard 8 组正/负例及当前仓库 159 个受管/非忽略文件检查，记录 `pure-rust-self-test.log`、`pure-rust-default.log`。
- 通过：HEAD 精确为 a8，94 个受管 Rust/WGSL/Cargo/工具链文件逐字节与 a8 相同，没有额外 untracked Rust/WGSL；`git diff a8ab930 -- crates Cargo.toml Cargo.lock rust-toolchain.toml` 为空。所有差异限于批准的文档、基线资产、脚本和 CI，记录 `source-parity.json`。
- 未运行：cargo build/test/clippy、新 24 图渲染、GUI 或性能重测。当前必要证据为用户已经亲验的旧预览与冻结数据一致；没有拿 dirty draft binary 冒称新工作区构建，旧 261 项测试属于 10 页草稿，不套用到此工作区。

汇总见 `verification-results.json`。本次没有启动后台/GUI 进程，没有提交或发布。checker 默认对视觉源码漂移返回“需候选复核”非零，token 变化始终失败；不提供自动更新 golden 的开关。

## 保留的真实结果与缺口

- 原 `a8ab930` 基线 workspace tests 与 40 case 文字对比曾通过，报告 `/Users/frankqin/dev/TekesUI-compare/takeover-20261015/report.json`、`report.md`。这是原版已有证据，本轮没有重新跑构建/测试。
- 原 release 的三轮 10 秒 mixed 性能门禁未通过：目标时间戳代理最长连续丢刷新 1 / 2 / 1，脚本阈值 ≤ 1，第 2 轮失败。目标时间戳代理不等于物理上屏证明；原始证据保留在同视觉证据根目录 `f1-perf/scroll-run{1,2,3}.json/.log`。
- 60 秒空闲 CPU、物理无掉帧、IME、VoiceOver、跨平台字体/系统色和完整组件/图标设计仍未验证或未完成。视觉冻结不改变这些状态。

后续按 [DESIGN.md](../../DESIGN.md) 进行最小增量；任何可见改变都需旧/新并排证据和用户确认，不得自动更新 golden 绕过。

## JSON DesignSystem 反向重建（历史阶段，后续已完成真源迁移）

在冻结原版后新增独立 `tekes-ui-design-export` 工具，调用既有 token Rust API 生成 `design/tokens.json` 与 schema；`design/system.json` 为机器入口，链接 token 数值层、原版校准布局/组件、交互、四主题、可访问性缺口及基线证据。固定画布/列宽/夹具几何是 specs，不混作通用 token。

本阶段是 Rust→JSON 的可检查设计快照（`generated-candidate`）；自有 schema 借鉴 DTCG 语法且保留逻辑 pt，不宣称完整兼容，也不宣称 JSON→Rust 已闭环或已成为唯一真源。原 UI、token 值、24 golden 与 manifest 保持原样。

本阶段证据目录：

`/Users/frankqin/.codex/visualizations/2026/10/15/01a13ead-f79a-7ff2-9a15-1c4c8366e507/verification/json-design-system/`

- 通过：offline/locked metadata、全 workspace/all-targets check、fmt、workspace tests 258 项；导出工具 lint 清理后单独 5 项测试重跑通过；全 workspace/all-targets Clippy `-D warnings` 通过。初次仅两项 collapsible_if lint 失败，原日志保留，最小折叠后复验，没有放宽门禁。
- 通过：运行时每个导出数值的 f32 位级回读、稳定 ID/枚举覆盖、确定性和 schema/文件同源测试；CLI 两次导出与仓内 tokens/schema 逐字节相同，改动数值的候选被拒绝。记录 `exporter-cli-validation.json` 与 `exporter-test-final.log`。
- 通过：system 项目 schema 实际使用的关键字检查、错误 format 负例及 242 个本地文件/JSON Pointer 引用。现有 Python 环境没有第三方 jsonschema，未联网安装；不声称已用通用完整 JSON Schema 引擎验证。token schema 与运行时一致性由 exporter 自有校验器和测试核验。
- 通过：pure Rust guard 8 组与仓库检查、Design baseline checker 6 组；默认检查如实退出 2（Cargo.lock 漂移待候选复核），新候选同环境核验退出 0。没有刷新 manifest/golden 来使默认漂移消失。
- 通过：在此工作区实际构建 dev Gallery 并新渲染 24 张候选，全部与批准 golden 逐字节一致。新 binary SHA256 为 `eaa66ed96a6f1a88fc29dd78619917df9a43a4e87910354db4df61379877a96e`，构建日志 `snapshot.log` 与 `source-binary-provenance.json` 保存实际来源；不是复用另一 dirty worktree 的 binary。
- 通过：UI/render/scene/text/platform/tokens 与 a8 源码保持相同，根 Cargo.toml 不变；Cargo.lock 仅新增导出工具本地包 8 行，没有重解或改变第三方包，记录 `cargo-lock.diff`。manifest/token/golden 没有更新。

本阶段没有额外跑 GUI、性能或 60 秒 idle；JSON→Rust 真源切换仍未实现。汇总 `verification-results.json`，没有自启进程残留或提交发布。

## JSON 唯一真源迁移（当前阶段）

按用户后续授权，`design/tokens.json` 现为生产 token 唯一数值来源。独立纯 Rust 编译器不再依赖 token crate，token build.rs 反向 build-depend 编译器；正常 Cargo build 从 JSON 生成 OUT_DIR Rust 并 include，无旧值回退。JSON/生成器/schema、system 入口和本轮迁移收据组成可核验链路。此节取代上节“尚未切换”现状描述，上节保留为历史。

原 manifest 与 24 golden 不变。`design/source-migration.json` 只授权精确 JSON/生成器/适配源码和 Cargo.lock 来源迁移，由 checker 外置固定 SHA 绑定；不允许其他 token 数值或 UI 改变。默认检查与 CI 完整生成核验均可通过，不以永久非零作为完成态。

证据根：`/Users/frankqin/.codex/visualizations/2026/10/15/01a13ead-f79a-7ff2-9a15-1c4c8366e507/verification/json-source-migration/`。

- 通过：实际从 git a8 源编译原 getter probe，508 f32 通道与当前解析 JSON 逐字节一致（`legacy-a8-probe/`、`channels.txt`）。旧 fixture 排序差异已修，未修改任何 path/bit。
- 通过：隔离副本正常 Cargo 两次构建，中间只把 JSON micro 间距 2→3 pt，未 clean；第二次明确 tokens.json changed，getter bits `40000000→40400000`（`cargo-json-probe/`），生产 JSON 未改。探针副本早于最终 clippy 的函数位置整理，仅 resolved_document 定义位置不同，无语义差异，已由独立复核确认。
- 通过：确定性生成、陈旧生成物拒绝、错误单位/缺失/额外 token 拒绝；262 项 workspace tests、fmt、strict clippy。首轮 fixture 排序失败与 clippy 函数位置失败日志保留，修后通过；无放宽门禁。
- 通过：pure Rust 8 组、design guard 11 组；default、生成 Rust+解析 508 契约核验、候选同环境核验均 exit 0。
- 通过：当前工作树新构建 dev Gallery 输出 24 张候选 PNG，全部与批准 golden 逐字节一致。UI/render/text/scene/platform 源未改；二进制 SHA256 `37456f1703b3b764f27a9dd3dc8ae845000d3644f469e61eaacb8d9c4522d350`。详见 `verification-results.json` 完整 source/JSON/binary/PNG 清单。
- 通过：system JSON 解析、必要根字段/状态及本地引用核对；无第三方完整 jsonschema 引擎，未声称通用 schema 验证。

本轮无额外 GUI、性能或 60 秒 idle 验收；历史性能失败和无障碍/完整组件库缺口保留。没有提交/推送/tag/发布，没有自启进程残留。
