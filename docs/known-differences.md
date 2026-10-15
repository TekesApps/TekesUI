# 已知差异

按 [comparison-method.md](comparison-method.md) 第 2.2 节维护：TekesUI 与参照实现（TekesMetalTextKit）之间经确认来自参照实现自身怪癖、或来自有意设计选择的差异，在这里登记原因，而不是悄悄放宽门槛。只用文字和数字描述，不含参照图，也不含私有夹具的原文。

## 当前状态（2026-10-15，F1 文字）

对原创夹具 8 个、40 个用例（夹具 × 宽度 × 倍率）的逐项比较，**没有任何超出门槛的差异**：断行、行度量、行宽、字形 id 与原点、浅色与深色两种外观的像素差，全部与参照逐位相同（最大差 0）。所以没有需要登记的数值差异。详见 [handoffs/F1-compare.md](handoffs/F1-compare.md)。

下面登记的是**行为上存在、但在现有夹具上没有显现**的差异，以及比较条件上的限制。

| 编号 | 范围 | 差异 | 数值 | 原因 | 状态 |
|---|---|---|---|---|---|
| KD-1 | 断行兜底 | 断行函数返回 0 时，参照实现强制前进 1 个 UTF-16 单元，可能切开代理对或组合字符；TekesUI 前进一个字素簇 | 现有夹具与宽度上 0 行受影响（CoreText 的 `SuggestLineBreak` 自己总是至少前进一个簇，兜底从未触发） | 有意设计：不把 emoji、国旗、组合字符切开 | 有意差异，未显现 |
| KD-2 | 比较条件 | 汉字用哪个回退字体取决于系统语言列表（`defaults read -g AppleLanguages`） | 本机 `en-JP, ja-JP`：纯简体字落到 `.HiraginoSansGBInterface-W3`，混排里的汉字落到 `.HiraKakuInterface-W4`；参照侧在同一台机器上得到同样的选择 | 回退由 CoreText 决定，两边都不自建级联 | 比较只在同一台机器、同一语言设置下有效；报告里记录语言列表和实际解析出的字体名 |
| KD-3 | 参照侧导出 | 参照实现不暴露每行的 ascent / descent / leading / 宽度，对比工具用参照给出的行区间重建 CTLine 取得；`font.weight` 写死 400 | 行高、基线已与参照数值交叉核对（`consistency_warnings` 为空） | 参照实现的 `MetalShapedText` 没有这些字段 | 导出方法限制，不是渲染差异 |
| KD-4 | 门禁对象 | `CTLineDraw` 的对照图不能当像素门禁 | 与整数原点路径超阈值 17% 到 21%，总墨量重约 13% | 见 [handoffs/F1-text.md](handoffs/F1-text.md)；参照实现自己用逐字形 `CTFontDrawGlyphs` 加整数原点，与 TekesUI 一致 | 说明性条目 |

## 未覆盖

- 其他机器、其他系统版本、其他语言列表：没有跑。
- 3x：comparison-method 暂不比较。
- 参照侧的窗口路径（视图居中的 `verticalOffset`、backing 相位）：比较条件是原点 (0, 0)、无偏移、无相位；Gallery 把段落放在整数 pt 原点上。
