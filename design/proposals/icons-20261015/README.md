# 三应用图标增补与优化提案

当前产品按 Tekes、TekesDXF、TekesDrama 评估。本轮交付 51 个可编辑 SVG 提案：38 个新增、8 个旧稿优化、5 个已作废槽位的独立重画。其中 8 个优化稿已于 2026-10-16 获用户批准，提升为 [tekes-ui-icons](../../../crates/tekes-ui-icons/README.md) 的默认资源；其余 43 个新增/重绘仍是候选。生产应用尚未更新，旧私有资产未复制进此目录。

## 需求与优先级

| 产品 | 本轮数量 | 功能依据与预见需求 |
|---|---:|---|
| 三应用共用 / Tekes | 10 新增 | 放大、缩小、外部打开、附件、筛选、排序、撤销、重做、差异批注、关联文件。优先补现有 IconRole / SF 回退和文档工作流缺口；不再给已有搜索、任务、文件、插件等另造近义图标。 |
| TekesDXF | 14 新增 | 图层、尺寸、捕捉、Window/Crossing 两种选择、直线、多段线、圆弧、块、布局、PDF 导入、CAD 导出、几何对比、表格提取。优先图层/尺寸/选择/导入导出/表格；实体工具图标作为后续功能储备，图标存在不表示对应 UI 已实现。 |
| TekesDrama | 14 新增 | 10 个现有工作区：Brief / Characters / Script / Directing / Production / Editing / Delivery / Audience Lab / References / Collaboration；另准备镜头、分镜板、声音轨、字幕。优先工作区导航，后四项用于预见的对象编辑和时间轴，不宣称业务已经实现。 |
| 共用旧稿 | 8 优化 | Settings、New Goal、Goal Completed、Memory、Think、Router、Provider、Appearance。详见下方问题与取舍。 |
| 作废槽位 | 5 独立重画 | 批注、只读、工作区写入、完整访问、电脑控制；使用新资源名，不恢复被排除的旧原稿。 |

功能依据是当前 TekesMetalTextKit 的 IconRole、TekesDXF README/工具栏和 TekesDrama README/WorkspaceStage，不是对未来路线图的承诺。

## 旧集审查结论

旧集 138 个母版的根描边均为 1.5 pt。主要问题不是混用线宽，因此保留整体圆头、单色、轻量轮廓，不进行无理由的全面重画。现有 133 项可用底稿中的大部分继续保留。

| 旧图标 | 具体问题 | 本轮提案与取舍 |
|---|---|---|
| Settings | 同心圆加四向短线容易读成准星；既有 brief 也记录过这一问题 | 独立构造六齿齿轮和清晰中心孔，优先语义可辨。 |
| New Goal | 与普通目标几乎同形，新增动作缺少加号 | 保留目标主体，把加号放在右下并留出空间。 |
| Goal Completed | 多层圆环挤压中心小勾 | 简化为外环加大勾；读作完成更直接，但弱化了靶心特征，需结合目标列表确认。 |
| Memory | 12 个引脚和小内框在 12 pt 偏密 | 减至 8 个引脚并增大负空间；保留芯片轮廓。芯片仍可能被理解为计算/硬件，暂不擅自换成长记忆书本语义。 |
| Think | 云团及下垂小点在小尺寸下视觉重心偏低 | 小幅缩短尾部并留边，保持原有思考泡识别，不作新风格。 |
| Router | 三个节点闭合成三角形，容易读成分享 | 改为一入两出的方向路径。 |
| Provider | 叠层形与 CAD Layers 发生语义冲突 | 改为服务器抽屉，CAD 图层继续使用叠层。 |
| Appearance | 调色盘与绘图/着色工具混淆，小圆点较密 | 改为明暗分区。此稿强调主题外观；若功能扩展为自定义配色，调色盘仍可作为独立“颜色”槽位。 |

`plugin/plugins` 等同形资源目前是同一语义的别名；没有为了增加数量而全部画成不同图案。合法的 selected/filled 状态也没有一律改成空心。

## 统一规则

- `viewBox="0 0 24 24"`，单色 `currentColor`，1.5 pt 圆头圆角描边。
- 12 / 16 / 24 pt 为实际使用尺寸，48 pt 只用于放大检查；同时比较深浅色，不以放大后好看代替小尺寸可读性。
- 以可辨认的几何物体/动作表达功能；减少密集细节、重叠圈层和过小徽标。
- 同一动作跨应用共用同名资源，领域对象通过 `cad-` / `drama-` 区分。保留语义名，不把绘制实现细节写入资源名。
- 5 个作废原稿不作为绘制或预览参照，新稿独立构形。新资源分别为 `annotate`、`access-read`、`access-workspace`、`access-full`、`desktop-control`；正式接入需更新资源映射，而不是移除旧排除清单。

## 文件与本机预览

`svg/` 是本轮新绘矢量稿；`manifest.json` 给出名称、中文语义、用途、分组、变更类型与优化理由。生成与校验不依赖旧私有图标：

```sh
python3 design/proposals/icons-20261015/tools/generate.py
python3 design/proposals/icons-20261015/tools/validate.py
```

旧新对比工作台和 Gallery 试装目录仅在本机生成，保留来源 SHA256，不提交旧资产：

```sh
python3 design/proposals/icons-20261015/tools/preview.py \
  --legacy '<本机 TekesIcons 目录>' \
  --out target/icon-studio-20261015
python3 -m http.server 8741 --bind 127.0.0.1 --directory target/icon-studio-20261015
```

本机页面提供共用/CAD/Drama/优化/独立重画/原完整集分组、名称及用途包含搜索、深浅色切换、旧新同尺寸对比。原生 Gallery 已直接使用正式默认资源，并从外部旧目录读取 before 版本供并排对比。以下旧试装命令仅用于导出预览，会覆盖副本中的旧稿，不能作为原生对比源： `TEKESUI_ICON_DIR=target/icon-studio-20261015/gallery-svg`，该目录为 176 项 SVG（133 旧集 + 38 新增 + 5 新重画，8 优化稿按同名覆盖本机副本），加 10 基础项共 186 项；生产源目录保持原样。

已通过 XML/样式/引用校验和 resvg 多尺寸非空光栅化；本机浏览器已检查分组和对比布局。证据及具体边界在忽略目录 `target/icon-studio-20261015/`。正式 AppKit/Metal 消费映射、用户视觉选择、VoiceOver 文案与生产发布未在本轮执行。
