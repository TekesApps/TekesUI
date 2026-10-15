# fixtures/text：原创文本夹具

本目录所有文本为 TekesUI 项目原创，按 MIT 许可发布，不含第三方文本。

内容为日常、无害的虚构文字，不含个人信息，不含真实公司或品牌名；网址只用 example.com、example.org 这类保留域名。

这套夹具对应 [comparison-method](../../docs/comparison-method.md) 第 5 节“并行的原创夹具”：让公开 CI 和外部贡献者始终有可用的回归夹具。它只在结构上对应私有参照夹具（句子数、行数范围、长度范围、必含的标点和数字），内容不同。F1（静态文字对比）使用。

## 文件

| 文件 | 内容 | 行数 |
|---|---|---|
| `sentences-a.txt` | 100 个中文单句，每行 35–48 个码点 | 100 |
| `sentences-b.txt` | 另外 100 个中文单句，规则同上 | 100 |
| `mixed-cjk-latin.txt` | 中文夹英文单词、版本号、单位、示例网址、括号、引号、代码片段 | 30 |
| `kinsoku.txt` | CJK 避头尾；逐行说明见 `kinsoku.md` | 32 |
| `kinsoku.md` | `kinsoku.txt` 每行想压测什么 | |
| `emoji.txt` | 中英文中的 emoji | 20 |
| `combining.txt` | 组合字符 | 15 |
| `rtl.txt` | 阿拉伯文、希伯来文与中英文混排 | 10 |
| `long-paragraph.txt` | 单行长段落，约 1100 个字符 | 1 |
| `manifest.json` | 每个夹具的名称、类别、默认样式、测试宽度、倍率与备注 | |
| `check.py` | 校验脚本 | |

约定：UTF-8、LF 换行、无 BOM、无行尾空白、无空行。**一行就是一个段落**（长段落文件只有一行）。文件末尾有一个换行。

`sentences-a/b` 每行是一个中文句子：含至少一个全角逗号“，”和一个 ASCII 数字，以全角句末标点（。！？）结尾，不含拉丁字母、emoji 和首尾空格；两个文件共 200 行，互不相同。

## 与 comparison-method 第 1 节类别的对应

| 类别 | 文件 |
|---|---|
| 句子列表（对应私有 HarnessStubSentences 的 A、B 两半，F1 起） | `sentences-a.txt`、`sentences-b.txt` |
| 中英混排 | `mixed-cjk-latin.txt` |
| 长段落 | `long-paragraph.txt` |
| 避头尾 | `kinsoku.txt` |
| emoji | `emoji.txt` |
| 组合字符 | `combining.txt` |
| RTL | `rtl.txt` |

代码块、表格、diff 属于 F3，本目录不含。

## manifest.json

顶层有 `"schema": "tekesui-fixtures/0"` 和 `"license": "MIT"`。`fixtures` 数组每项含：

- `name`、`file`、`category`、`notes`
- `style`：默认样式 `{"font": "system-ui", "size": 14, "line_spacing": 0}`
- `widths`：要测的排版宽度，句子列表只测 `[700]`，其余为 `[700, 500, 320]`
- `scales`：backing scale，`[1, 2]`

## 需要特别说明的码点

**以下行含有分解形式（不是 NFC），读取和比较时不得做规范化。**列出的是预期的码点序列，U+XXXX 表示单个码点。

emoji.txt（行号为文件内顺序）：

- 第 5 行 家庭：U+1F468 U+200D U+1F469 U+200D U+1F467 U+200D U+1F466；U+1F469 U+200D U+1F469 U+200D U+1F467。
- 第 6 行 职业：U+1F469 U+200D U+1F4BB；U+1F468 U+200D U+1F373；U+1F9D1 U+200D U+1F3EB。
- 第 4、18 行 肤色：U+1F44D U+1F3FB；U+1F44D U+1F3FF；U+1F44B U+1F3FD。
- 第 7 行 国旗：U+1F1E8 U+1F1F3、U+1F1EF U+1F1F5、U+1F1FA U+1F1F8、U+1F1E7 U+1F1F7。
- 第 8 行 keycap：U+0031 U+FE0F U+20E3，同样的 2、3，以及 U+0023 U+FE0F U+20E3。
- 第 9、10 行 VS16：U+2764 U+FE0F 与 U+2764，U+2600 U+FE0F 与 U+2600。
- 第 14 行：U+1F3F3 U+FE0F U+200D U+1F308（彩虹旗）。
- 第 15 行：U+1F469 U+200D U+2764 U+FE0F U+200D U+1F468。
- 第 19 行：U+0035 U+FE0F U+20E3 之外的不完整 keycap，即“5”后只有 U+FE0F U+20E3。

combining.txt：

- 第 1 行：a U+0301 U+0323；e U+0302 U+0301 U+0323；o U+0303 U+0308；u U+0308 U+0304 U+0301。
- 第 2 行：e U+0301；n U+0303；u U+0308（与预组合 é ñ ü 对照）。
- 第 3 行：cafe U+0301（与预组合 café 对照）。
- 第 4 行：越南语预组合（NFC）。
- 第 5 行：越南语分解：e U+0302 U+0301；e U+0323 U+0302；a U+0301；u U+0301；a U+0323；o U+0323；a U+0300；e U+0303。
- 第 6 行：行内以孤立的 U+0301 开头（没有基字符）。
- 第 7 行：x 后接 U+0300 至 U+030B 共 12 个组合标记，再接 y。
- 第 8–10 行：天城文，含 क्ष（U+0915 U+094D U+0937）、ज्ञ（U+091C U+094D U+091E）、त्र（U+0924 U+094D U+0930）、श्र（U+0936 U+094D U+0930）。
- 第 11–13 行：泰文，含声调符号与上下元音的叠加。
- 第 14 行：α U+0301 与预组合 U+03AC 对照。
- 第 15 行：o U+20DD（组合圈）；a/i/u U+0304；a U+0301、U+030C、U+0300 与预组合对照。

## 校验

在任意目录运行：

```
python3 fixtures/text/check.py
```

只用 Python 3 标准库。它检查：编码、BOM、换行、行尾空白、空行；两个句子文件的行数、长度范围、必含的全角逗号、ASCII 数字、末尾标点、无拉丁字母、无 emoji、200 行互不相同；其余夹具的行数范围和内容特征（例如 emoji 文件含 ZWJ、VS16、肤色、国旗、keycap，组合字符文件含非 NFC 行）；`manifest.json` 的 schema、license 以及所引用的文件是否都存在。通过时退出码为 0。

## 修改夹具

夹具内容一旦被黄金文件引用，改动就会让黄金文件失效。修改前先确认影响，改完重新运行 `check.py`，并重新生成对应的黄金文件。
