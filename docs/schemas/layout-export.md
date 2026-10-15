# 排版数据导出格式 `tekesui-layout/0`

状态：F1 随 tekes-ui-text 落地，版本 0。用途见 [comparison-method.md](../comparison-method.md) 第 2.2 节与第 4 节：参照侧和 TekesUI 各自导出同一结构，逐项比较。格式只描述排版结果，不依赖任何平台类型，也不含夹具文本。

机器可读的定义：[tekesui-layout-0.schema.json](tekesui-layout-0.schema.json)（JSON Schema 2020-12）。导出与校验的实现：`tekes-ui-text` 的 `export` 模块（`export_layout`、`validate_layout`）。

## 约定

- 文件是一个 JSON 对象，编码 UTF-8。
- **长度单位是 pt**（逻辑点），不是设备像素；`scale` 只是记录，坐标不乘它。
- **文本偏移是 UTF-16 偏移**，半开区间 `[start, end)`，与 NSString / CoreText 一致。需要字节偏移或字素索引的使用方自己换算。
- 坐标系：原点在段落左上角，x 向右，y 向下。
- 浮点数是 IEEE 双精度，写成 JSON 数字（最短往返表示，读回逐位相同）。比较时用 comparison-method 里的容差，不要求逐位相等。
- `leading` 可以为负，行高里取 `max(0, leading)`。

## 顶层字段

| 字段 | 类型 | 说明 |
|---|---|---|
| `schema` | string | 固定为 `"tekesui-layout/0"` |
| `fixture` | string | 夹具名（私有夹具用名字，不放原文） |
| `font` | object | 请求的字体，见下 |
| `width` | number 或 null | 换行宽度（pt）；`null` 表示不换行，只在硬换行处断行 |
| `scale` | number | backing scale，记录用，正数 |
| `line_spacing` | number | 每行之后追加的行间距（pt），≥ 0；没有段间距 |
| `exporter` | object | 导出方，见下 |
| `units` | object | 单位声明，见下；使用方应当检查 |
| `text_utf16_len` | integer | 全文的 UTF-16 长度 |
| `height` | number | 段落总高度（pt） |
| `lines` | array | 各行，见下 |

### `font`

| 字段 | 类型 | 说明 |
|---|---|---|
| `family` | string | `"system-ui"` 表示系统界面字体，否则是请求的字体名 |
| `post_script_name` | string | 实际解析出的基础字体的 PostScript 名，如 `.SFNS-Regular` |
| `size` | number | 字号（pt） |
| `weight` | integer | 字重，400 是常规 |

### `exporter`

| 字段 | 类型 | 说明 |
|---|---|---|
| `id` | string | 导出方标识，TekesUI 侧是 `tekes-ui-text/coretext` |
| `version` | string | 导出方版本（TekesUI 侧是 crate 版本） |
| `commit` | string 或 null | 导出方所依据的 git 提交。TekesUI 侧由编译期环境变量 `TEKES_UI_GIT_COMMIT` 提供（运行时的同名变量也行），没有则为 `null`。参照侧写它所依据的 TekesMetalTextKit 提交 |

### `units`

固定为：

```json
{ "length": "pt", "text_offsets": "utf16", "y_axis": "down", "glyph_origin": "paragraph-top-left" }
```

## 每行 `lines[i]`

| 字段 | 类型 | 说明 |
|---|---|---|
| `index` | integer | 行号，从 0 起，等于数组下标 |
| `utf16` | [start, end] | 这一行覆盖的文本，**含行尾的换行符**。各行首尾相接，覆盖全文 |
| `ascent` / `descent` / `leading` | number | 该行 `CTLineGetTypographicBounds` 给出的值，是行内所有字体的包络，所以回退字体可以把这一行撑高 |
| `height` | number | `ceil(ascent + descent + max(0, leading))` |
| `baseline_y` | number | 基线 y（段落坐标系），**不取整**。第一行 = `ascent`，之后每行 = 上一行顶 + 上一行高 + `line_spacing` + 本行 `ascent` |
| `width` | number | 排版宽度，**含**行尾空白 |
| `width_trimmed` | number | 不含行尾空白的宽度 = `width - CTLineGetTrailingWhitespaceWidth` |
| `glyphs` | array | 本行的字形，按 run 的顺序、run 内按字形顺序（视觉顺序） |

文本以 `\n` 结尾，或文本为空时，最后多一行空行：`utf16` 为 `[len, len]`，`glyphs` 为空，`ascent/descent/leading` 取基础字体，`height` 是基础字体的行高，`width` 为 0。

## 每个字形 `glyphs[j]`

| 字段 | 类型 | 说明 |
|---|---|---|
| `id` | integer | 字形编号（所在字体里的编号，0..65535） |
| `font` | string | 该字形所在字体的 PostScript 名（回退字体会和基础字体不同） |
| `x`, `y` | number | **字形原点**（基线上的点）在段落坐标系里的位置（pt）。`y` 对普通字形等于所在行的 `baseline_y` |
| `cluster` | [start, end] | 该字形对应的 UTF-16 范围：从它的字符索引到同一 run 里下一个不同的索引。连字、组合字符会让几个字符落在同一个簇里 |

## 示例

文本 `Hi 你好`（原文不进文件），系统字体 14pt，不换行，2x：

```json
{
  "schema": "tekesui-layout/0",
  "fixture": "example/hi",
  "font": { "family": "system-ui", "post_script_name": ".SFNS-Regular", "size": 14.0, "weight": 400 },
  "width": null,
  "scale": 2.0,
  "line_spacing": 0.0,
  "exporter": { "id": "tekes-ui-text/coretext", "version": "0.1.0", "commit": "0123abc" },
  "units": { "length": "pt", "text_offsets": "utf16", "y_axis": "down", "glyph_origin": "paragraph-top-left" },
  "text_utf16_len": 5,
  "height": 17.0,
  "lines": [
    {
      "index": 0, "utf16": [0, 5],
      "ascent": 13.53515625, "descent": 2.953125, "leading": 0.0, "height": 17.0,
      "baseline_y": 13.53515625, "width": 43.19004645660203, "width_trimmed": 43.19004645660203,
      "glyphs": [
        { "id": 112,   "font": ".SFNS-Regular",         "x": 0.0,                "y": 13.53515625, "cluster": [0, 1] },
        { "id": 14316, "font": ".HiraKakuInterface-W4", "x": 17.3359375,         "y": 13.53515625, "cluster": [3, 4] }
      ]
    }
  ]
}
```

（示例里省略了中间的字形。）

## 与 comparison-method 第 2.2 节的对应

| 比较项 | 字段 |
|---|---|
| 断行位置 | `lines[i].utf16`（完全一致） |
| 行高与行基线 | `ascent`、`descent`、`leading`、`height`、`baseline_y` |
| 行宽（含与不含尾部空白） | `width`、`width_trimmed` |
| 字形位置 | `glyphs[j].id`、`x`、`y` |

caret 矩形、选区矩形、块级几何属于 F3 和 F5，不在版本 0 里；加字段时升 schema 的次版本（`tekesui-layout/0` 的字段只增不减，不兼容的变化才升到 `/1`）。

## 参照侧导出时要注意

- 参照实现（`MetalTextShaper.shape`）把 `baselineOrigin` 表示成 y 向下的视图坐标，原点由调用方传入；导出时让原点为 (0, 0)，使 `x`、`y` 落在段落坐标系里。
- 参照实现的字形簇是 `rangeOfComposedCharacterSequence` 给的 grapheme 级范围，TekesUI 的是 CTRun 字符索引到下一个索引；两者在连字、组合字符处可能不同。`cluster` 不是门禁项，位置与 id 才是。
- 字体名两边都用 PostScript 名。
