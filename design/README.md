# 设计产出保存约定

本文规定 TekesUI 的设计产出（token、图标、应用图标）怎么保存、怎么从设计源头流转进仓库、怎么变成代码可用的东西。需求来源与历史见 [icons-and-tokens-brief.md](../docs/design/icons-and-tokens-brief.md)；本文引用的"第 N 节"均指该文档。**交付格式以本文为准**，brief 7.1 的 schema 草案已作废。

## 背景与设计源头

设计源头已改为 **TekesUI Design System**：claude.ai 上的一个 Design System 类型 artifact，私有，需所有者授权访问。它的 base 取自 Tekes 的 tokens（TekesMetalTextKit，main@7ee9703 这一 commit），含 27 个颜色、4 个主题、6 个文字样式，另有 133 个图标（作为 asset）、品牌手册和封面。补真实增强对比度值、重画 20 个图标、处理对比度，都在这个 artifact 里做。

本文不写它的链接：外部读者打不开，链接对他们没有价值；公开仓库也不应暴露私有工作区的标识。维护者在自己的 artifact 列表里按名称即可找到。

之前设想的"委托 Design Agent 交付"流程由 artifact 导出取代；那份发送件不再使用，brief 文档保留为历史记录与需求来源。

Tekes（Swift）的设计资产靠 Xcode 导入 Assets.xcassets。TekesUI 是纯 Rust，没有这套机制，所以由本文显式规定。

> 状态：约定先行。下文提到的目录大多尚不存在，`crates/tekes-ui-tokens` 和 `cargo xtask design` 也还没建，分别由路线图的 F0、F2 建立，见 [roadmap.md](../docs/roadmap.md)。

## 目录结构

```
design/
  README.md                      本文
  tokens/
    tokens.json                  token 唯一真源（artifact 格式）
    tokens.schema.json           机器校验规则
    motion.toml                  动效时长（手写）
  icons/
    svg/<slot-name>.svg          每个槽位一个最终稿
    index.toml                   槽位索引与原创性声明
  app-icon/                      应用图标母版
  handoffs/
    <YYYY-MM-DD>-<轮次>/         artifact 导出原包归档
crates/
  tekes-ui-tokens/src/generated.rs   生成并提交的代码
```

| 路径 | 谁写 | 能否手改 | 代码是否引用 |
|---|---|---|---|
| `design/tokens/tokens.json` | artifact 导出经导入写入 | 可手改（需通过 schema 校验）；也可被新一轮导出覆盖 | 不直接引用；由生成工具读取 |
| `design/tokens/tokens.schema.json` | 工具链维护者 | 可手改 | 不引用；校验和导入时读取 |
| `design/tokens/motion.toml` | 手写 | 可手改 | 不直接引用；由生成工具读取 |
| `design/icons/svg/*.svg` | artifact 导出经导入写入 | 不手改，要改回 artifact 里改、再导出 | 引用：生成的代码用 `include_bytes!` 嵌入 |
| `design/icons/index.toml` | 导入时由工具和人工共同维护 | 可手改（备注、声明） | 生成工具读取槽位名与文件名 |
| `design/app-icon/` | artifact 导出经导入写入 | 不手改 | 打包脚本读取，运行时代码不引用 |
| `design/handoffs/*` | 原样归档 | 只读，不改 | 不引用 |
| `crates/tekes-ui-tokens/src/generated.rs` | 工具生成 | 禁止手改 | 引用：它就是代码 |

### tokens/tokens.json

token 的唯一真源，格式即 artifact 的 `project/tokens.json`，原样导入，不再另立 schema（旧 `tekesui-tokens/0` 草案作废）。只能手改，或由新一轮 artifact 导出覆盖。结构：

- 除 `type` 外，每个 family 都是 `{"tokens":[{"name","value","usage"}]}` 列表。
- `color` 带 `themes`，共四个：`light`、`dark`、`light-ic`、`dark-ic`。颜色值写成 `{"light":…,"dark":…,"light-ic":…,"dark-ic":…}`，或写成别名 `"{other-token}"`。
- `type` 的结构是 `fonts`、`families`、`groups[].styles[]`。
- 可选的其他 family：`spacing`、`radius`、`stroke`、`opacity`、`size`。
- `motion` 不能做成 family（artifact 格式不支持），见下。

示意片段（token 名与数值仅为格式示意，不是真值；`type` 只给骨架）：

```json
{
  "color": {
    "themes": ["light", "dark", "light-ic", "dark-ic"],
    "tokens": [
      {
        "name": "row-selection",
        "value": { "light": "#EEF0F0", "dark": "#2A2C2C", "light-ic": "#DDE0E0", "dark-ic": "#3A3D3D" },
        "usage": "持久行选中底"
      },
      { "name": "list-selected", "value": "{row-selection}", "usage": "别名示例" }
    ]
  },
  "spacing": {
    "tokens": [ { "name": "gap-sm", "value": 8, "usage": "相邻控件间距" } ]
  },
  "type": {
    "fonts": [],
    "families": [],
    "groups": [ { "styles": [] } ]
  }
}
```

**motion 时长**放在 `design/tokens/motion.toml`，不放 tokens.json。现值（秒）：

| 名称 | 时长 |
|---|---|
| instant | 0 |
| stateChange | 0.12 |
| geometry | 0.18 |
| listExpansion | 0.3 |
| expansion | 0.5 |

选 `motion.toml` 而不是在 `tekes-ui-tokens` 里手写常量，理由：generated.rs 禁止手改，手写常量只能放在它之外的另一个文件，设计数值就分散在 `design/` 和 crate 两处；放进 `design/tokens/` 后，所有设计数值都在 `design/` 下，由同一个生成工具读取并写进 generated.rs，Gallery 也能统一展示。以后 artifact 若支持 motion family，再并回 tokens.json。

### tokens/tokens.schema.json

机器校验规则，导入和 CI 都用它，校验上述 artifact 格式；旧 `tekesui-tokens/0` 草案作废。JSON Schema 本身只管结构：

- family 列表结构（每项带 name、value、usage）；
- `color` 的四个主题 light、dark、light-ic、dark-ic 齐全；
- 别名 `{other-token}` 可解析且无环；
- name 在 family 内唯一。

对比度达标这类跨字段检查由导入工具在 schema 之外补做。

### icons/svg/&lt;slot-name&gt;.svg

- 一个槽位一个最终稿，备选稿不进这个目录。
- slot-name 用 kebab-case。是否保留 `tekes-` 前缀，以 artifact 里的命名规范为准（brief 第 4 节第 5 条），导入时统一，不混用。
- 现有 SVG 的颜色写死 `#000`，没有用 currentColor。导入时统一转换成 `currentColor`，仓库里只存这一种规范形态：Gallery 和生成代码不必区分，usvg 渲染时按 currentColor 着色即可，运行时也不依赖"当作 mask"的隐式约定。
- 待重画的 20 个槽位（15 个只有 SF 回退，5 个因 C2PA 作废）在 artifact 里完成，导入前不进 `design/icons/`。
- artifact 中现有的 133 个图标，base 来自 Tekes 现有自绘图标；它们进入 `design/` 前仍要过原创性核对。

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

artifact 导出（`project/tokens.json` 和 `assets/Icons/*.svg`）的原包原样归档，只读，不被代码引用。只有原创性核对通过、可以随 MIT 公开的导出才归档到这里，见"流转规则"。

### crates/tekes-ui-tokens/src/generated.rs

- 由工具生成并提交，禁止手改，文件头带"自动生成，勿手改"标记。
- 内容：`Icon::Xxx` 枚举、token 常量。
- 注意：这个 crate 目前还不存在，由 F0/F2 建立，本文先定约定。

## Xcode asset catalog 功能对照表

| 功能 | Xcode / Tekes 现状 | TekesUI 接替方式 | 落地阶段 |
|---|---|---|---|
| 导入 | Xcode 里拖入 Assets.xcassets，格式检查靠人 | `cargo xtask design import <handoff 目录>`，对 artifact 导出的 tokens.json 和 SVG 做校验，通过才拷进 `design/`（检查项见下） | F2 |
| 多外观 | catalog 的 Appearances 变体；Tekes 在运行时按系统外观解析 | tokens.json 的 `color` 提供 light、dark、light-ic、dark-ic 四个主题的取值；运行时由平台层读取当前外观和增强对比度设置，选出对应的一套。mac 上外观变化时要重新解析，参照 Tekes 的"解析时机"原则 | F2 |
| 类型化访问 | 生成的资源符号，Tekes 里是 case 枚举 | 生成 `Icon::Xxx` 枚举和 token 常量 | F2 |
| 矢量 | catalog 的 vector/PDF 资源，系统光栅化 | SVG 用 `include_bytes!` 嵌入；运行时用 usvg/resvg 按 12、16、24 光栅化，按 backing scale 乘倍数，再放进 glyph atlas，与文字共用 atlas 机制 | F2（atlas 由 F1 提供） |
| 浏览 | Xcode 里看 catalog | Gallery 里的 Tokens 页和 Icons 页。Icons 页按 12、16、24 三档、light/dark/light-ic/dark-ic 展示 | F2 |
| 守卫 | Tekes 的设计守卫 | CI 检查组件代码里不出现颜色、尺寸字面量（白名单机制参照 Tekes 设计守卫的思路，只写原则），并检查 generated.rs 是否最新：重跑生成，diff 为空才算通过 | F2 |

### 导入的检查项

`cargo xtask design import` 对 SVG：

- viewBox 必须是 `0 0 24 24`；
- 允许的颜色只有 `#000`/`black`（填充与描边，导入时统一转换成 `currentColor`）和 `currentColor`；其他颜色、渐变、滤镜、文字一律拒绝；
- 剥除所有 metadata，包括 C2PA 内容出处块、编辑器私有属性和注释；
- 校验失败就整体拒绝，并列出全部失败项，不做部分导入。

对 tokens：走 `tokens.schema.json` 校验，再做对比度等跨字段检查。

### 为什么生成后提交，而不在 build.rs 里生成

- diff 可审：设计变更落到代码上的结果在 PR 里一目了然；
- rust-analyzer 可见：生成的类型和常量不依赖构建脚本运行就能跳转和补全；
- 不拖慢编译：每次构建不必重新解析 JSON 和 SVG。

代价：容易忘记重新生成。由上表的 CI 新鲜度守卫兜底。

## 流转规则

1. 在 TekesUI Design System artifact 里完成设计，导出 `project/tokens.json` 和 `assets/Icons/*.svg`。
2. 导出原样放进 `design/handoffs/<日期>-<轮次>/`。此时只在工作区，不提交。
3. 跑 `cargo xtask design import`，做格式校验、`currentColor` 转换和元数据剥除。
4. 人工核对原创性（brief 7.3、7.4、第 9 节）。核对对象是从 artifact 导出的图标，尤其是 20 个重画槽位。
5. 两者都通过后，才写进 `design/tokens/` 和 `design/icons/`，并重新生成 generated.rs。
6. 一次导入一个提交：`handoffs/` 里的原包和写进 `design/` 的结果放在同一个提交里，提交信息注明 handoff 目录。第 3 或第 4 步没通过的，整包从工作区删除、不提交，回 artifact 里修改后重新导出。

硬规则：

- 来源不明或带 C2PA 元数据的资产，一律不进 `design/`。带 C2PA 的原件也不进 `handoffs/`，在本地拒收，回 artifact 里重做后重新导出。
- 不能清掉元数据就当作合格。元数据只是触发核查的信号，资产本身必须经原创性核对。
- 品牌和第三方 logo 不进任何目录（见 brief 第 5 节）。
- `handoffs/` 在公开仓库里：只有原创性核对通过、可以随 MIT 公开的导出才归档；含私有内容的导出件留在私有仓库。

## 过渡期

- 在 artifact 补齐真实增强对比度值之前，light-ic / dark-ic 的临时值以 artifact 当前值为准，并以 artifact 中的临时标记为准识别；导入工具把仍带临时标记的 token 列出来，CI 可在发布前检查是否仍有临时项。
- 依赖系统语义色的角色，用平台层读系统色兜底，并标注为临时。
- 图标在导出并通过核对之前不进 `design/icons/`，Gallery 用占位方块。不得用 SF Symbols，也不得绕过原创性核对直接用 Tekes 原 SVG。
- 补齐真实值并导入后，去掉对应的临时标记。
