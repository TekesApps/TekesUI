# Design System 入口与生成链

[system.json](system.json) 是完整系统入口，[integration.json](integration.json) 索引生效的提案规范、代码与现有补集。冲突时用户提供的 v0.3.0-proposal1 优先；没有对应定义的旧角色和已有校准夹具保留。

## 文件归属

- `imports/v0.3.0-proposal1/source.zip` 与 `upstream/`：完整原始交付，禁止就地修改或重封以掩盖差异。`import.json` 记录 134 个文件及原 ZIP 的 SHA256。
- `source/`：生效的基础定义、保留补集、Rust 映射和动效 JSON；`brand/`：显式消费者配置和统一品牌策略。
- `tokens.json`：由作者输入生成的合并 token 投影，不手改；`tokens.schema.json` 描述机器格式。
- `baseline/`、`source-migration.json`、`history/`：此前用户确认画面与来源迁移的历史证据，不自动改成新提案的验收记录。

## 品牌是一套生成规则

消费者修改品牌配置的 seed 或带理由的显式覆盖；离线品牌编译器结合 base 与 policy 生成 light、dark、light-ic、dark-ic。强调、主操作、hover/pressed、浅底和配对前景联动，对比度、状态差异与品牌依赖隔离一并校验。正文、中性表面、焦点、选区、状态、语法、图表不因品牌输入而全部染色。

默认 Gallery 使用提案登记的 neutral 参考夹具；blue/clay 是示例，不是系统强制品牌。输入或 policy 改动后重新生成；陈旧投影应被检查器和正常 Cargo 构建拒绝。

## 本地工具与运行

Python 是设计开发工具，不是 token 颜色消费的运行时依赖。离线工具按导入包固定 requirements 使用已有隔离环境；`TEKESUI_PYTHON` 指向该环境的解释器。受管协作环境复用当前 SSD 输出、共享锁和源码身份保护，不在仓库内另建 target/venv，不因下面列出命令而授权安装依赖、重生成或启动窗口。本轮架构审计未执行这些生成/构建命令。

```bash
"$TEKESUI_PYTHON" -B scripts/merge-design-system.py --check
"$TEKESUI_PYTHON" -B scripts/check-design-system.py
```

正常 Cargo 构建从合并 JSON 生成 `OUT_DIR` Rust 实现；UI、渲染器不加载 Python。包内其余 brand/context/adapter/generate/validate 工具完整保留，是否可用于原生产品以对应验收为准。不要在原样导入目录执行会写文件的 `generate`/`seal`/`--write-report`；需要复现时复制到自己拥有的临时目录。

Theme 实例由 `scripts/generate-theme-system.py` 从定义/规则产生；普通 Rust Theme verifier 在 design-export，tokens 的 build.rs 校验后产生 typed token 与品牌/Theme 表。它们是同一链的不同阶段，须由单一共享 owner 同步输入、版本、锁和输出，不再建第二套引擎。默认、显式外部 Theme＋独立锁、legacy 行为分别遵守既有接口契约；不能通过“简化架构”放松 freshness、来源或拒绝条件。具体消费差距及 #55/#136 的职责见 [架构与平台边界](../docs/platform-boundary.md)。

本地接收的 rules 1.7.0 / Surface schema 1.3.0 在原 31 个用途上增加 `progress-track/rest`：轨道背景、必要边界和进度 marker 来自完整共享包，focus/helper 为空；rest 不替代业务 loading/error/success/indeterminate 状态。Python 与正常 Rust 校验同守旧用途状态和必要对比，组件接入仍由 #72 按原范围验收。

#62 的既有 solid-status 修复与 #71 共享生成链一并接收：`blue-inputs` 1.0.1 仅修正 light 的实心状态标签前景；四状态标签对底色遵守普通 4.5、增强 7.0。普通 Surface 前景不能替代实心标签检查。生成产物和来源锁必须由原工具产生；外部 Theme 的独立锁、自洽语义拒绝与合法恢复仍是验证要求。此次接收不代表 StatusBadge/Progress 组件迁移、原生无障碍或完整 Design System 已完成，交付记录入口为 #136。

包静态 schema/颜色检查与项目 Rust 测试、实际截图、人工目测、真实输入及性能分别验收。导入包 reports 不自动成为当前原生实现的通过证据。版本与发布规则仍适用；Design System 提案编号不是项目 SemVer。Git 提交、阶段标签与正式发布分别由用户授权。

## 主线历史约定的承接

此前主线的私有 artifact 导出、motion.toml 和手动 generated.rs 方案保留在 Git 历史中；当前实现以本文的作者 JSON → 合并投影 → Cargo OUT_DIR Rust 生成链为准，不维护第二套 token 真源。主线要求的单一 registry、统一 Story 模板、可重复审计与稳定寻址继续适用，见 [章程](../docs/charter.md)。
