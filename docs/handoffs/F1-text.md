# F1 文字层记录：tekes-ui-text

日期：2026-10-15。机器：macOS 26.6.2，系统语言列表 `en-JP, ja-JP`。分支 `claude/f0-foundation-06c986`，未推送。本文只记 tekes-ui-text 一块（排版、光栅化、导出、对照），atlas 与渲染管线见 render / scene 的文档，F1 整体交接以后另写。

## 做了什么

| 内容 | 位置 |
|---|---|
| 三个 trait：`FontFallback`、`Shaper`、`Rasterizer`；数据模型 `Paragraph` / `Line` / `ShapedRun` / `ShapedGlyph`；`TextStyle`（系统界面字体或按名字，字号 pt，字重） | `src/traits.rs`、`model.rs`、`style.rs` |
| CoreText 实现：字体登记表（`CoreTextFonts`，字体到 `FontKey`）、排版器 `CoreTextShaper`（同时实现 `Shaper` 和 `FontFallback`）、光栅化器 `CoreTextRasterizer` | `src/coretext/` |
| `push_paragraph`：每行一个 `GlyphRun` 编进 `Scene` | `src/push.rs` |
| 布局导出 `tekesui-layout/0`：`export_layout`、`validate_layout`；文档与 JSON Schema | `src/export.rs`、[docs/schemas/layout-export.md](../schemas/layout-export.md) |
| 像素对比 `compare::diff_rgba8`（通道阈值、包围盒比例、8 连通域、热图 RGBA8） | `src/compare.rs` |
| 对照图（oracle）`render_oracle`、`framesetter_line_ranges` | `src/coretext/oracle.rs` |
| 测试 | `tests/shape.rs`、`raster.rs`、`export.rs`、`e2e_oracle.rs`，加各模块的单元测试 |

新增依赖 objc2-core-text、serde、serde_json 已记入 [dependencies.md](../dependencies.md)。serde_json 打开了 `float_roundtrip`，导出的数值读回逐位相同。

## 与参照实现行为的对照

依据的是 `metaltextview-text-pipeline.md`（私有仓库的行为描述）。"匹配"指本机实测与该描述一致或按该描述实现。

### 排版

| 项 | 参照实现 | 本实现 | 结论 |
|---|---|---|---|
| 字体回退 | 完全依赖 CoreText 自动级联，没有 `CTFontCreateForString`，没有级联列表 | 同；属性字符串只设基础字体 | 匹配 |
| 断行 | `CTTypesetterSuggestLineBreak` + `CTTypesetterCreateLine` 逐行；宽度 `max(1, w)` | 同 | 匹配 |
| 断行与 `CTFramesetter` | 未知 | 拉丁、CJK、混排各 7 个宽度（30 到 2000pt），断行与 `CTFramesetter` 逐行相同 | 一致，无差异 |
| 兜底前进 | `length <= 0` 时强制 1 个 UTF-16 单元，可能切开代理对 | 前进一个**字素簇**（`CFStringGetRangeOfComposedCharacterSequence`） | **有意差异**。实测 `SuggestLineBreak` 本身从不返回 0：宽度小到 0.5pt 时 CoreText 自己每行前进一个簇（emoji、国旗、ZWJ 序列、组合字符都不会被切开），所以兜底路径只是防御，没被触发过；单元测试直接测了簇长 |
| 行度量 | 每行 `CTLineGetTypographicBounds`，含回退字体的包络；仅当 `ascent + descent <= 0` 才退回基础字体 | 同 | 匹配 |
| 行高 | `ceil(ascent + descent + max(0, leading))` | 同 | 匹配 |
| 基线 | `cursorY + ascent`，不取整 | 同，系统字体 14pt 的 ascent 是 13.535，基线保持小数 | 匹配 |
| 行间距 | 每行之后加，下限 0，无段间距 | 同；负值按 0 | 匹配 |
| 文末 `\n` | 补一个基础字体行高的空行；空文本也是一个空行 | 同；只认 `\n`（`\r`、U+2029 结尾不补行，与参照实现一致） | 匹配 |
| 总高度 | `max(defaultLineHeight, cursorY - lineSpacing)` | 同 | 匹配 |
| 默认字体 | `CTFontCreateUIFontForLanguage(.system, 14)`，失败回退 `.AppleSystemUIFont` | 同，字号是样式参数 | 匹配 |

### 光栅化

| 项 | 参照实现 | 本实现 | 结论 |
|---|---|---|---|
| 边界 | `CTFontGetBoundingRectsForGlyphs`；null 矩形是零墨迹，不是失败 | 同；零墨迹返回 `None`（atlas 会缓存"没有位图"，不再重复调用） | 匹配 |
| 边距 | 墨迹边界乘 scale 向外取整，每边加 2 设备像素 | 同；测试验证外圈 2 像素无墨迹，边距不超过 4 | 匹配 |
| 位图 | DeviceRGB、8 位、`premultipliedFirst + byteOrder32Little`（BGRA8 预乘） | 同 | 匹配 |
| 抗锯齿 | 开 | 开 | 匹配 |
| 字体平滑 | 单色关，仅彩色字体开 | 同（见下） | 匹配 |
| 填充色 | 白色，颜色在着色器里乘 | 同 | 匹配 |
| 绘制 | `CTFontDrawGlyphs`，原点是位图里的整数点，不烘焙亚像素 | 同；先在设备像素里平移到整数原点再缩放，原点必为整数像素 | 匹配 |
| 亚像素 bin | 无 | 无（键里只有字体、字形、scale） | 匹配 |
| 字体身份 | PostScript 名、字号、符号特征、矩阵、变体轴 | 同 | 匹配 |

**字体平滑的实测（回答"对彩色 emoji 意味着什么"）：**

- 单色字形：开平滑后 'A'、'j'、'中' 的 alpha 总量变大 30% 到 48%（1x 与 2x 都是），说明开平滑会让字形变粗（字体膨胀）。关掉与"灰度抗锯齿"的粗细一致，符合参照实现的注释。默认关，测试里断言了这一点。
- Apple Color Emoji（sbix 位图字形）：平滑开或关，输出逐字节相同。所以"彩色字体才开平滑"对 emoji 没有可见效果；保留参照实现的做法（彩色字体开），不影响结果。若以后遇到 COLR 矢量彩色字体，平滑才可能有影响，到时再测。
- 彩色字形颜色：位图自带颜色，预乘；`RasterFormat::Color`。通过 `kCTFontTraitColorGlyphs` 判定。

### 字体回退的实测（影响对比）

- 系统界面字体（`.SFNS-Regular`）的 CJK 回退是 **Interface 变体**：`.HiraKakuInterface-W4`、`.HiraginoSansGBInterface-W3`、`.AppleSDGothicNeoI-Regular`，度量与 SF 一致。所以**系统字体下 CJK 行与 Latin 行同高**（14pt：17pt），回答了调研笔记第 10 节第 3 条的一半：行高不变，但回退字体照常出现在 `fonts` 里。
- 主字体不是系统界面字体时，回退是 `HiraginoSans-W3`，leading 约 7pt，**所在行变高**：Menlo 14pt 纯 Latin 17pt，纯 CJK 22pt，中英混排 24pt；Helvetica、Courier、Times New Roman、Arial 同样（22 到 23pt）。这与调研笔记里"HiraginoSansGB 比主字体多约 6pt leading"一致（本机是 HiraginoSans-W3，leading 7）。测试 `non_system_font_cjk_fallback_makes_the_line_taller` 断言这一点，同一段落里纯 Latin 行高度不变，行高按行计算。
- **同一段汉字用哪个回退字体取决于系统语言列表**：本机语言是 `en-JP, ja-JP`，不含简体中文偏好，纯简体字句子落到 `HiraginoSansGBInterface`，而"支持 CJK 混排"这类混排句里的汉字落到 `HiraKakuInterface`（日文字形）。参照侧和 TekesUI 在同一台机器、同一语言设置下才可比；对比报告里应记录语言列表（`defaults read -g AppleLanguages`），否则同一个夹具在不同机器上 glyph id 可能不同。
- `CTFontCreateWithName` 不认 "SF Mono" 这样的家族名（得到 Helvetica）。要等宽系统字体得用 `.AppleSystemUIFontMonospaced-Regular`（参照实现的做法）；`FontFamily` 还没有专门的等宽变体，F2 需要时再加。

## 行尾空白：CTLine 宽度是否包含

**包含。** `CTLineGetTypographicBounds` 返回的宽度包括行尾空白；`CTLineGetTrailingWhitespaceWidth` 单独给出这部分。实测（系统字体 14pt）：

| 文本 | 宽度 `width` | 不含空白 `width_trimmed` | 说明 |
|---|---|---|---|
| `Hello` + 3 个空格 | 44.352 | 32.990 | trimmed 与不带空格的 `Hello` 的宽度逐位相同（测试断言 1e-6 内） |
| 换行后的 `Hello `（宽度 60） | 36.777 | 32.990 | 行尾空格留在上一行 |
| 宽度 120 下的 `Hello world, this is ` | 121.700 | 117.913 | 尾部空白可以"挂"在换行宽度之外：`width` 超过 120，`width_trimmed` 不超 |

所以断行判定用的是不含行尾空白的宽度：每一行的 `width_trimmed <= 换行宽度`（测试对 120pt 的拉丁句断言了这一点），`width` 可以超出。导出格式两个值都有，与 comparison-method 第 2.2 节"含尾部空白与不含各一个"对应。

## 对照图（oracle）与 atlas 路径的对比

测试 `tests/e2e_oracle.rs`。条件：系统字体 14pt，换行宽度 160pt，前景 #202430，背景 #F6F5F0，原点 (8, 8.3)，每个例子再用 8.37 的小数 x 原点重复一遍；1x 与 2x；atlas 页 1024，渲染走 `Renderer::render_to_rgba8`（wgpu，Metal 后端，sRGB 空间 alpha 混合）。门槛按 comparison-method 第 2.1 节：通道差 24/255，超阈值比例 ≤ 0.5%（1x）/ 0.3%（2x），任何连通差异域不超过一个字形面积。

### 门禁对照图：逐字形 `CTFontDrawGlyphs` 画在 `round(原点 × scale)` 的整数像素上

| 句子 | scale | 超阈值比例 | 最大连通域 | 最大单像素差 |
|---|---|---|---|---|
| 中文句（两行，含标点） | 1x | 0.0000% | 0 | 1 到 2 / 255 |
| 中文句 | 2x | 0.0000% | 0 | 1 到 2 / 255 |
| 混排句（TekesUI + CJK + Latin + 数字） | 1x | 0.0000% | 0 | 1 / 255 |
| 混排句 | 2x | 0.0000% | 0 | 1 / 255 |
| 含彩色 emoji 的句子 | 1x、2x | 0.0000% | 0 | 1 / 255 |

差到最多 2/255 来自 GPU 混合与 CPU 混合的舍入。这条路径独立于 atlas 和 wgpu（CoreText 直接画进位图），所以它检验的是：位图偏移（left/top）、2 像素边距、取整规则、atlas 打包与上传、sRGB 空间混合、预乘颜色（emoji）。它通过，说明这几项没有偏差，**不需要放宽任何阈值**。反向校验：把对照图平移 1 个像素，比例 17.4%、连通域 282 像素，门禁失败（测试 `a_one_pixel_position_error_is_detected`），说明比较确实对位置敏感。第二帧不再有 atlas 未命中，且两帧结果逐字节相同。

### `CTLineDraw` 对照图：与 atlas 路径**不能**当像素门禁

任务书设想用 `CTLineDraw` 画每一行当对照。实测它与 atlas 路径差异远超容差（混排句；系统字体 14pt）：

| scale | `subpixel` 关 | `subpixel` 开 |
|---|---|---|
| 1x | 超阈值 20.5%，最大连通域 99，最大单像素差 240 | 约 20.8%，170，173 |
| 2x | 超阈值 18.5%，最大连通域 319，最大单像素差 242 | 约 17.2%，260，145 |

根因（实测定位，不是猜的）：

1. **字形位置**。`CTLineDraw` 里字形落在 CoreGraphics 自己的亚像素网格上（不论 `setShouldSubpixelPositionFonts` 开关），每个字形最多偏半个像素；本项目和参照实现都把每个字形原点取整到设备像素（键里没有亚像素 bin）。同样用 `CTFontDrawGlyphs` 逐字形画，放在小数位置与放在整数位置也相差 7% 到 15%（超阈值）。基线 y 和整体包围盒对得上（x0、x1、y0、y1 相差不超过 1 到 2 像素），所以不是基线或 left/top 的错。
2. **字重**。同样关闭字体平滑，`CTLineDraw` 的总墨量比 `CTFontDrawGlyphs` 重约 13%（1x、2x 都是 12.8% 左右），与是否亚像素无关；逐字形 `CTFontDrawGlyphs` 放在小数位置的总墨量与 atlas 路径相差不到 0.3%。也就是说 `CTLineDraw` 有自己的加粗，`setShouldSmoothFonts(false)` 管不住。

这两项都是 CoreGraphics 绘制路径的属性，而不是 atlas 路径的缺陷：参照实现用的就是逐字形 `CTFontDrawGlyphs` 加整数原点（调研笔记第 5 节）。因此：对参照实现的像素对比（私有伴随仓库）应当拿参照实现自己的输出，不是 `CTLineDraw`；公开仓库里的自测用 `GlyphsRounded` 对照图。`CTLineDraw` 对照图保留在 `render_oracle` 里（`Placement::Line`），测试 `ctlinedraw_differs_only_by_glyph_position_not_baseline_or_rasterization` 记录并断言了上面的结论，防止以后有人拿它当门禁。

## 需要主控决定的事

1. **取整规则里 y 的取整**：参照实现在 `MetalTextView` 里先对 `verticalOffset` 取 floor 到整数 pt（视图居中），再对 quad 边取整。本层只给出段落内的基线（不取整），竖直居中和视图内偏移属于调用方。F1 的 Gallery 页对齐参照图时，调用方需要决定原点 y 是否先取整到整数 pt。
2. **`Placement::Line` 对照图是否保留**：我保留了它作为测量工具；如果不想让 API 里留一个"不能当门禁"的选项，可以删掉，结论文档不变。
3. **语言列表**：见上，建议对比报告和夹具说明里记录系统语言。
4. 零墨迹字形（空格）光栅化器返回 `None`，由 atlas 缓存；`push_paragraph` 因此不会为空格生成图元。
5. 字体登记表（`CoreTextFonts`）不是 `Send`（objc2 的 CTFont 未标 `Send`/`Sync`）。排版和光栅化要在同一线程上共享一个登记表。如果以后要把光栅化放到后台线程，需要包一层并核实 CTFont 的线程安全性（Apple 文档说字体对象不可变、线程安全）。

## 没验证的

- 与参照实现（Swift）的实际对比：本机没有 TekesUI-compare，没有运行参照侧。这里只在 TekesUI 内部自洽（atlas 路径对 CoreText 直绘）。
- 1x 只在 2x 屏幕的离屏渲染里测（`render_to_rgba8` 与屏幕无关，scale 参数决定一切），没有在真正的 1x 显示器上看。
- 120Hz 帧时间、空闲 CPU：本层没有测，归 render 与 Gallery。
- RTL、竖排：只在 `font_runs` 里出现了阿拉伯文字体，没有写断行和字形顺序的断言；夹具在 `fixtures/text/`。
- 字重（`weight != 400`）只验证了会得到不同的字体身份，没有对比渲染。
