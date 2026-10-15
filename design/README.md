# 设计产出保存约定

本文规定 TekesUI 的设计产出（token、图标、应用图标）怎么保存、怎么从 Design 交付流转进仓库、怎么变成代码可用的东西。委托正文见 [icons-and-tokens-brief.md](../docs/design/icons-and-tokens-brief.md)，本文引用的"第 N 节""7.x"均指该文档。

背景：Tekes（Swift）的设计资产靠 Xcode 导入 Assets.xcassets。TekesUI 是纯 Rust，没有这套机制，所以由本文显式规定。这份约定要在 Design 交付回来之前定好。

> 状态：约定先行。下文提到的目录大多尚不存在，`crates/tekes-ui-tokens` 和 `cargo xtask design` 也还没建，分别由路线图的 F0、F2 建立，见 [roadmap.md](../docs/roadmap.md)。

## 目录结构

```
design/
  README.md                      本文
  tokens/
    tokens.json                  token 唯一真源
    tokens.schema.json           机器校验规则
  icons/
    svg/<slot-name>.svg          每个槽位一个最终稿
    index.toml                   槽位索引与原创性声明
  app-icon/                      应用图标母版
  handoffs/
    <YYYY-MM-DD>-<轮次>/         Design 交付原包归档
crates/
  tekes-ui-tokens/src/generated.rs   生成并提交的代码
```

| 路径 | 谁写 | 能否手改 | 代码是否引用 |
|---|---|---|---|
| `design/tokens/tokens.json` | Design 交付经导入写入 | 可手改（需通过 schema 校验）；也可被新一轮交付覆盖 | 不直接引用；由生成工具读取 |
| `design/tokens/tokens.schema.json` | 工具链维护者 | 可手改 | 不引用；校验和导入时读取 |
| `design/icons/svg/*.svg` | Design 交付经导入写入 | 不手改，要改走新一轮交付 | 引用：生成的代码用 `include_bytes!` 嵌入 |
| `design/icons/index.toml` | 导入时由工具和人工共同维护 | 可手改（备注、声明） | 生成工具读取槽位名与文件名 |
| `design/app-icon/` | Design 交付经导入写入 | 不手改 | 打包脚本读取，运行时代码不引用 |
| `design/handoffs/*` | 原样归档 | 只读，不改 | 不引用 |
| `crates/tekes-ui-tokens/src/generated.rs` | 工具生成 | 禁止手改 | 引用：它就是代码 |

### tokens/tokens.json

- token 的唯一真源，schema 为 `tekesui-tokens/0`。
- 只能手改，或由 Design 交付覆盖。
- brief 7.1 允许 Design 交 JSON 或 TOML。若交的是 TOML，导入时转成 JSON；仓库里只保留 JSON 这一个真源，不并存 TOML。

### tokens/tokens.schema.json

机器校验规则，导入和 CI 都用它。JSON Schema 本身只管结构，引用无环、对比度这类跨字段检查由导入工具在 schema 之外补做。合起来至少包括：

- 引用无环（颜色可引用另一角色加 alpha 档，引用链不能成环）；
- 四种外观齐全：light、dark、IC-light（increase-contrast-light）、IC-dark（increase-contrast-dark），或者按 brief 7.1 写明的推导规则从 light/dark 推导；
- brief 7.1 校验清单里的其他项，例如对比度达标、每个 token 带用途说明和禁区。

### icons/svg/&lt;slot-name&gt;.svg

- 一个槽位一个最终稿，备选稿不进这个目录。
- slot-name 用 kebab-case。是否保留 `tekes-` 前缀，以 Design 交付时的命名规范为准（brief 第 4 节第 5 条），导入时统一，不混用。

### icons/index.toml

记录槽位名、文件、12/16 尺寸下的可读性备注、原创性声明，对应 brief 7.2 的索引和 7.4 的声明。示例条目（槽位名和内容仅为格式示意）：

```toml
[[icon]]
slot = "list-remove"
file = "svg/list-remove.svg"
readability-12 = "示意：两笔可辨，间距足够"
readability-16 = "示意：清晰"
originality = "原创"   # 或 "重画（曾参照 SF 形）"
redraw-done = true     # 仅"重画"项有意义
note = ""
```

### app-icon/

应用图标母版。打包时生成 .icns（mac）和 .ico（Windows），生成物不提交。

### handoffs/&lt;YYYY-MM-DD&gt;-&lt;轮次&gt;/

Design 交付的原包原样归档，只读，不被代码引用。只有原创性声明通过、可以随 MIT 公开的交付才归档到这里，见"流转规则"。

### crates/tekes-ui-tokens/src/generated.rs

- 由工具生成并提交，禁止手改，文件头带"自动生成，勿手改"标记。
- 内容：`Icon::Xxx` 枚举、token 常量。
- 注意：这个 crate 目前还不存在，由 F0/F2 建立，本文先定约定。

## Xcode asset catalog 功能对照表

| 功能 | Xcode / Tekes 现状 | TekesUI 接替方式 | 落地阶段 |
|---|---|---|---|
| 导入 | Xcode 里拖入 Assets.xcassets，格式检查靠人 | `cargo xtask design import <handoff 目录>`，校验通过才拷进 `design/`（检查项见下） | F2 |
| 多外观 | catalog 的 Appearances 变体；Tekes 在运行时按系统外观解析 | tokens.json 提供四种外观的取值；运行时由平台层读取当前外观和增强对比度设置，选出对应的一套。mac 上外观变化时要重新解析，参照 Tekes 的"解析时机"原则 | F2 |
| 类型化访问 | 生成的资源符号，Tekes 里是 case 枚举 | 生成 `Icon::Xxx` 枚举和 token 常量 | F2 |
| 矢量 | catalog 的 vector/PDF 资源，系统光栅化 | SVG 用 `include_bytes!` 嵌入；运行时用 usvg/resvg 按 12、16、24 光栅化，按 backing scale 乘倍数，再放进 glyph atlas，与文字共用 atlas 机制 | F2（atlas 由 F1 提供） |
| 浏览 | Xcode 里看 catalog | Gallery 里的 Tokens 页和 Icons 页。Icons 页按 12、16、24 三档、light/dark/IC 展示 | F2 |
| 守卫 | Tekes 的设计守卫 | CI 检查组件代码里不出现颜色、尺寸字面量（白名单机制参照 Tekes 设计守卫的思路，只写原则），并检查 generated.rs 是否最新：重跑生成，diff 为空才算通过 | F2 |

### 导入的检查项

`cargo xtask design import` 对 SVG：

- viewBox 必须是 `0 0 24 24`；
- 只能用 `currentColor`，不能有填充色、渐变、滤镜、文字；
- 剥除所有 metadata，包括 C2PA 内容出处块、编辑器私有属性和注释；
- 校验失败就整体拒绝，并列出全部失败项，不做部分导入。

对 tokens：走 `tokens.schema.json` 校验。

### 为什么生成后提交，而不在 build.rs 里生成

- diff 可审：设计变更落到代码上的结果在 PR 里一目了然；
- rust-analyzer 可见：生成的类型和常量不依赖构建脚本运行就能跳转和补全；
- 不拖慢编译：每次构建不必重新解析 JSON 和 SVG。

代价：容易忘记重新生成。由上表的 CI 新鲜度守卫兜底。

## 流转规则

1. 委托件（如 SEND 发送件）留在私有仓库，不进本仓库。本仓库只有 `docs/design/` 下的委托草案。
2. 交付原件先原样放进 `design/handoffs/<日期>-<轮次>/`。此时只在工作区，不提交。
3. 跑 `cargo xtask design import`，做格式校验和元数据剥除。
4. 人工核对原创性声明和映射表（brief 7.3、7.4、第 9 节）。
5. 两者都通过后，才写进 `design/tokens/` 和 `design/icons/`，并重新生成 generated.rs。
6. 一次导入一个提交：`handoffs/` 里的原包和写进 `design/` 的结果放在同一个提交里，提交信息注明 handoff 目录。第 3 或第 4 步没通过的，整包从工作区删除、不提交，并退回 Design 重交。

硬规则：

- 来源不明或带 C2PA 元数据的资产，一律不进 `design/`。带 C2PA 的原件也不进 `handoffs/`，在本地拒收，并在委托里要求重交。
- 不能清掉元数据就当作合格。元数据只是触发核查的信号，资产本身必须经原创性核对。
- 品牌和第三方 logo 不进任何目录（见 brief 第 5 节）。
- `handoffs/` 在公开仓库里：只有原创性声明通过、可以随 MIT 公开的交付才归档；含私有内容的交付件留在私有仓库。

## 过渡期

Design 交付之前，tokens 先用 Tekes 现有数值做临时占位，在 tokens.json 里标记 `"provisional": true`，或用同等机制标记，使占位值可被机器识别和统一替换。

- 占位只能用设计词汇里的数值：间距、圆角等平台中立数值，以及 Tekes 已定的 light 校准色。
- 依赖系统语义色的角色，用平台层读系统色兜底，并标注为临时。
- 图标在 Design 交付前不进 `design/icons/`，Gallery 用占位方块。不得用 SF Symbols，也不得用 Tekes 原 SVG 充当，因为原 SVG 未经原创性核对。
- Design 交付并导入后，去掉对应 token 的 provisional 标记；CI 可在发布前检查是否仍有 provisional 项。
