# 输入控件族：TextSurface 状态与真实 token 核验

2026-10-18，基于活跃 `gallery-design-baseline/TekesUI` 的 `bc3de79466834e99062a1b4ef0872323e2860001` 加全部 WIP。GitHub 刷新仍为14个开放 Issue、全部状态0个PR，#14更新时间仍为 `2026-10-17T08:15:16Z`。本批推进 #8，关联 #1/#12；不新增组件或扩大组合迁移。证据入口：`design/reviews/input-family-20261018/receipt.json`。

## 根因、修复位置与适用范围

根因一属于共享基础交互呈现：TextSurface 的边界/文字已处理 disabled，但插入光标只检查 focused、read-only、blink 和空选区，遗漏 disabled。公共字段更新和宿主失焦之间可能短暂保留 focused，导致禁用输入仍显示可编辑光标。现直接在同一绘制条件中检查 disabled；没有新增状态机、token或抽象层。TextField、TextArea、SearchField 及既有 TextFieldContent 都通过这条共享路径受益。read-only 继续允许焦点、复制和选区，但不显示插入光标；disabled 不接受编辑或新的指针选区。disabled 不清空已有草稿/选区，错误边界和中性焦点各自保留原契约。

根因二属于真实接入与验收夹具：三个状态页只有 rest/focus/disabled/error，遗漏已有 read-only/preedit/selection 场景；而且曾将 disabled 同时作为 TextDemo 构造器的 read_only 参数传入。现分开两种状态，并以现有 TextInput::select_range/preedit 和 TextSurface.clauses 填充实际场景；多行沿用相同路径和滚动/裁剪。没有局部传色或页面专属视觉补丁。

合理例外：TextArea 保留多行排版/换行和纵向滚动；TextField/SearchField 沿用当前单行 TextSurface 别名接入，没有新增 Search 专属组件或清除按钮。读写限制和选区/IME 公共事务统一复用。设计 input profile 虽列出 hover/pressed，但现有输入契约没有定义独立的悬停/按压呈现规则；本批确认命中、指针选择/拖选/取消已有能力，不套用 Button 表现，也不把这些操作冒充两项独立状态已验收。三个组件的 hover/pressed 六条记录继续 unverified，待 #1/#8 明确规则。

## 公共 API、消费者与真实 token

`crates/tekes-ui-controls/tests/input_contract.rs` 通过库外公共 API 驱动生产 TextSurface、CoreText shaping、指针和 TextInput 编辑事务，共324例：三个输入别名×四主题×三个既有品牌×九项状态/组合。验证必要边界/错误 token、稳定 frame、渲染不改草稿与选区、disabled/read-only 编辑拦截、闪烁光标抑制、UTF-16 grapheme 归一化、空 preedit 取消恢复原选区、IME commit/undo 和指针取消。探针的 glyph renderer 不绘制像素，真实字形/GPU证据单列。

独立 composition-consumer 在原 Rich Dialog 中复用 TextFieldContent，补单行/多行实际接入：原生事件模型的 preedit/clauses、取消恢复、AX 选区、read-only/disabled 拦截和真实 CoreText 调用。已有异构 List、Dialog、焦点、IME 和 AX 场景全部保留；消费者共15项通过（1项lib+14项scenarios）。它是实际消费者事件路由验证，不能替代系统输入法或VoiceOver体验。

新增审计入口复用已有 token-audit collector，不进入默认构建：

```sh
python3 scripts/audit-input-tokens.py --output /tmp/new-input-observation.json
```

输出路径必须全新，工具执行公共探针并校验前后源码哈希一致。三个别名各108例、各12个真实查询 ID；共324例，绑定57项输入。包括必需边界/错误/焦点、正文/禁用/选中文字、选区与输入/只读底色及输入圆角。观测含 layout/draw 与编辑事务，`complete=false`；常量（如现有 Stroke::Rule/padding）、已编译展开alias、TextRole style、字体后端和未执行路径不在覆盖范围。完整依赖仍为 runtime-unverified、tokens=[]，有界查询另列 runtime_observation，不能以12个查询ID当完整依赖表。

共享源码变化后，旧选择族观测按门禁应失效。本批实际重跑2016例选择族探针，在新目录保存新观测并更新当前引用；历史 selection-family 证据原样保留，没有只改哈希“续期”。生成检查新增输入案例判定，精确按公开探针的状态名核对四主题，不将 disabled-focus 算作 focus 或晋级 hover/pressed/native。治理测试32项通过。

当前45组件、269项要求不变：夹具已源码核对62项，待核对207项；公共API核验43项，其中输入族21项、选择族22项。全部 experience 仍 unverified，成熟度未晋级。待核验记录并不表示对应功能未实现。

## Gold7 与分层证据

| 范围 | 结果与证据边界 |
| --- | --- |
| G1/G2/G3 | 复用既有中性底色、必要边界、错误色和独立焦点；read-only 与 disabled 不混淆。真实状态/variant整页前后各288帧，覆盖三页、四主题、720×512大字号/1440×960默认字号、top/mid/bottom及@1x/@2x。抽查浅色TextField整页前后、深色IC大字号TextArea、浅色IC大字号SearchField，选区、组字下划线与focus可区分，内容仍服从视口裁剪。不是全品牌人工体验通过。 |
| G3/G5/G6 | 复用已有选区/插入光标GPU用例，留64张实际字形图；四主题×单/多行×两倍率，disabled/read-only在两种blink输入下逐像素相同，草稿保持。抽查disabled单行@2x及深色IC多行选区。物理屏幕与系统辅助技术未重新验收。 |
| G4 | 沿用已有滚动和光标时钟，无新增过渡；不把静态GPU或合成输入计为物理动效验收。 |
| G5/G7 | 复用现有IME、选区与真实消费者；公共契约/代码接入/像素证据分开。物理拼音/日文候选窗、VoiceOver、跨屏与完整原生矩阵继续待验收。 |

控件/tokens/design-export默认测试294项、独立消费者15项、Gallery文本模型/布局回归29项及单独GPU光标用例通过；新增公共状态夹具测试包含在29项中。workspace严格Clippy和显式token-audit严格Clippy、fmt、DS、Gold7机械检查、reference alignment、纯Rust与能力生成检查通过。全workspace测试未重跑；没有发起性能基准或产出性能验收结论。报告与收据不将测试框架耗时作为性能证据。

## 后续与保护

#8/#1/#12仍部分完成。先明确输入hover/pressed的共享呈现契约，再按现有组件族继续核实状态与真实token依赖；物理IME、VoiceOver分别补原生证据。#14的公共组合最小场景和全部既有WIP保持，不因本批扩大迁移。原Issue未满足条件继续原Issue处理。

开始时9548项工作区文件均记录哈希；本批有意修改9项既有文件，其余9539项逐项保持，包括旧review证据。收据另存本批源码清单、前态副本和完整源码overlay。没有提交、推送、tag、创建/关闭Issue或发布。
