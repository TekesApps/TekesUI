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

Python 是设计开发工具，不是原生 Gallery 的运行时依赖。按导入包固定依赖准备隔离环境：

```bash
python3 -m venv target/design-tools-venv
target/design-tools-venv/bin/python -m pip install -r design/imports/v0.3.0-proposal1/upstream/TekesUI-Design-System-v0.3.0-proposal1/requirements.txt
target/design-tools-venv/bin/python scripts/merge-design-system.py
target/design-tools-venv/bin/python scripts/merge-design-system.py --check
target/design-tools-venv/bin/python scripts/check-design-system.py
cargo run --locked -p tekes-ui-gallery
```

正常 Cargo 构建从合并 JSON 生成 `OUT_DIR` Rust 实现；UI、渲染器不加载 Python。包内其余 brand/context/adapter/generate/validate 工具完整保留，是否可用于原生产品以对应验收为准。不要在原样导入目录执行会写文件的 `generate`/`seal`/`--write-report`；需要复现时复制到自己拥有的临时目录。

包静态 schema/颜色检查与项目 Rust 测试、实际截图、人工目测、真实输入及性能分别验收。导入包 reports 不自动成为当前原生实现的通过证据。版本与发布规则仍适用，本次 Git 保存不创建 tag 或执行发布。
