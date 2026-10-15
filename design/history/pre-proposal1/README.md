# 设计基线与真源规则

本仓库当前设计权威是用户明确批准的 `a8ab930` 原版 Gallery，契约见 [DESIGN.md](../DESIGN.md)。这次是冻结已经看见并确认的设计，不是照未知私有 artifact 重画，也不是 DesignSystem 全量完成或发布。

## 保存结构

```text
design/
  README.md
  system.json                 DesignSystem 机器入口，引用各层规范与证据
  system.schema.json          项目入口结构 schema
  tokens.json                 唯一数值真源（正常构建输入）
  tokens.schema.json          独立 Rust 编译器生成的 token schema
  baseline/
    manifest.json             原版源码/token/二进制/24 图来源与 hash
    screenshots/              24 张原三页校准 PNG，保持原文件名
```

文件名 `<primitives|tokens|static-text>-<light|dark|light-ic|dark-ic>@<1|2>x.png`。1x 为 1200×800 像素，2x 为 2400×1600 像素；页面逻辑尺寸恒为 1200×800 pt。

24 图来自本轮首次 Rust 改造前生成的 `gallery-snapshots/`，仅包含原创夹具与公开 Gallery；没有私有 compare 参照、live 截图或后续 10 页的 80 图。复制时逐张完整 PNG 解码、尺寸校验与 SHA256 核对，原始来源不改。

## 真源与派生关系

- `a8ab9303ab2f528913f83cfc2c8da2a0eab8b8f3` 的原 token 值与批准的 24 图定义当前视觉契约；当前生产值源已迁移为 JSON，sidecar 保留与原值逐位等价的精确绑定。
- 现有静态 token 的历史临时标记保留，不因冻结而宣称跨平台动态值、图标或整个 DesignSystem 已全部完成。
- 私有 TekesUI Design System artifact、Swift Kit 或未来导出只能作为参考/增量提案。没有用户对具体视觉变化的确认，就不能覆盖当前基线；artifact 未取得不阻塞当前冻结。
- Gallery 与渲染器 Rust 源保持 `a8ab930` 原样。源码发生变化时需要核查影响；token 契约和 golden 被保护，checker 不提供自动接受差异或更新 golden 的开关。

## 核验与更新

```bash
python3 -I scripts/check-design-baseline.py --self-test
python3 -I scripts/check-design-baseline.py
python3 -I scripts/check-design-baseline.py --check-snapshots <候选目录>
```

token crate 漂移默认失败；本次精确 JSON 来源迁移仅通过独立且固定指纹的 source-migration 收据授权，不能靠候选截图覆盖其他 token 改动；Gallery/render/platform 等视觉相关源码变化时默认检查返回非零“需候选快照复核”。只有显式提供的 24 张候选图逐字节一致且 checker 环境匹配，才可通过候选核验；候选图是否确由相应源码/构建产生仍须执行者另行证明。CI 默认检查元数据、source 与 golden 完整性，不声称 macos-latest 已做跨环境像素验收。

checker 的精确范围以脚本与 manifest 为准。通过意味着数据/来源满足冻结契约，不能代替真实 GUI、像素差评审、用户确认或性能验收。不要以重生 24 图/更新 hash 的方式使差异自动通过。

提出视觉更新时提供旧/新并排证据、主题/倍率矩阵和实际差异，再取得用户确认；通过后仅更新获批范围并保留旧来源。无法确认的变更保持草稿，不推翻原版，不混入另一 worktree 的 10 页 WIP。任何提交或发布仍需独立授权。

## JSON → Rust → Gallery

`tokens.json` 是 token 唯一数值来源；正常 Cargo build 自动生成并编译 OUT_DIR Rust，无旧数值回退。`system.json` 关联 DS 其余规范层。原 a8 设计仍是视觉权威，JSON 来源迁移不改变样式；历史反向导出是建立本输入的过程。

```sh
cargo run --locked -p tekes-ui-design-export -- --validate design/tokens.json
cargo run --locked -p tekes-ui-design-export -- --generate design/tokens.json /tmp/tekes-ui-tokens.rs
cargo run --locked -p tekes-ui-design-export -- --check design/tokens.json /tmp/tekes-ui-tokens.rs
cargo run --locked -p tekes-ui-design-export -- --resolved design/tokens.json /tmp/tekes-ui-resolved.json
python3 -I scripts/check-design-baseline.py --check-generated-rust /tmp/tekes-ui-tokens.rs --check-resolved-contract /tmp/tekes-ui-resolved.json
```

schema 可用 `--write-schema design/tokens.json design/tokens.schema.json` 确定性生成。生产没有 checked-in generated Rust；`--check` 用于审查临时生成物的新鲜度。JSON 数值、API 声明、单位或引用不合法会拒绝构建。`source-migration.json` 固定原 508 f32 通道、原 manifest、完整 JSON/生成器/适配源码指纹，不自动 rebaseline。

格式为项目自有 schema，借鉴 DTCG `$type`/`$value`，保留逻辑 pt，不宣称完整兼容。token 表仅是 DS 一层；布局/组件/交互/主题/可访问性及证据由 system 入口关联。完整组件库、图标与其他平台仍未完成。
