# 选择控件族：error 公共契约与真实 token 观测

2026-10-18，活跃 `gallery-design-baseline/TekesUI`，`bc3de79466834e99062a1b4ef0872323e2860001` 加全部 WIP。重新读取 GitHub 后仍有14个开放 Issue、全部状态0个PR，#14更新时间仍为 `2026-10-17T08:15:16Z`。本批继续 #8 的组件族核验，修复关联 #1/#5 的实际接入缺口；没有创建新 Issue或扩展组件迁移。证据入口是 `design/reviews/selection-family-20261018/receipt.json`。

## 根因、公共修复和适用范围

根因属于公共组件契约和共享呈现接入。设计要求 Checkbox、RadioGroup、Switch 都支持 error，已有 `ControlState.error` 和失败色 token。Checkbox 已接入；RadioGroupView、SwitchView 没有公共入口，低层 radio/switch painter 也忽略该字段。只补状态页不能解决消费者缺失的能力。

在现有 `selection.rs` 中为两个 View 增加 `set_error`/`error`，将验证状态传到已有 ControlState；公开 RadioGroup/Switch model 字段及其 struct literal 保持兼容。`extended.rs` 复用既有 `status-failed` 标记必要边界，焦点仍用独立的中性 ring；错误不改布局、选择值或启用规则。Switch 的错误标记放在 track，thumb 保持自己的值/禁用呈现。与 Checkbox 一致，disabled 期间保留验证错误，但继续阻止操作。清除错误、失焦或重新布局都不会无故清空选择值。

实际 token 观测还确认 Radio 的必要边界曾写死 `1.`。现改用已存在的 `stroke-rule`；当前 JSON 值对应的正常外观保持，后续共享值变更能够进入 Radio。没有新增状态枚举、token、视觉体系或 UI 抽象层。

适用组件为当前三个选择控件，以及已存在的 SwitchContent 组合适配器。SwitchContent 复用现有 `ContentSemantics.invalid` 导出验证状态；原生桥、VoiceOver及其他组件族不因这项修改自动通过。解释性错误消息仍由宿主/既有 Field 提供，View 不复制表单校验器。Checkbox 原有公共 error 能力保留，仅补齐真实状态夹具和统一回归。没有为 Radio 新增组合适配器或扩大迁移。

## 公共 API 与真实接入

`crates/tekes-ui-controls/tests/selection_contract.rs` 是库外公共 API 探针：生产 View、真实事件方法和 Scene 图元断言，共2016个场景，覆盖四主题、三个既有品牌、值/混合态、error、disabled、focus、hover/pressed组合。对比正常与错误布局，验证必要边界角色、独立焦点、值保留；另外核对 Space/方向键激活、禁用拦截、blur及清除错误。探针文字测量为确定性宿主，不冒充 CoreText/物理输入验收。

独立 composition-consumer 增加实际 SwitchContent 场景：错误语义进入活树，仍可切换，disabled 后拦截，清除错误不丢值。消费者共14项通过。控件、tokens和design-export 默认构建合计293项测试通过；既有45项/80条组件绑定、180条类型路径及契约测试均保留。

Gallery 两条既有 state matrix 路径统一接入新公共 API，为三组件增加 error 与值、focus、disabled 的真实 View 组合，没有页面局部样式。三页×四主题×两个窗口/字号组合×top/mid/bottom×@1x/@2x，修复前144帧、修复后144帧，均来自实际 Gallery/CoreText 场景的GPU渲染。完整图和配对哈希单独保存；默认捕获范围不变，本批仅通过测试参数选择这三个已登记组件与倍率。

## 有界运行时 token 观测

为获得实际调用证据，在 tokens 生成器既有 color/metric/text 查询函数加入显式 `token-audit` 编译条件。默认构建不编译 collector/hook；显式审计时，用同步、线程局部的 scope 记录真实语义 ID，不记录值、计时或网络数据。嵌套、其他线程和 panic 不污染后续 scope。没有另建 token 值表或改设计真源。

可复核入口：

```sh
python3 scripts/audit-selection-tokens.py --output /tmp/new-selection-observation.json
```

工具调用同一公共 API 测试，校验测试前后源哈希一致，再独占创建新的结果文件，拒绝覆盖旧证据。本批 observation绑定57项相关源码/设计输入，合计2016个场景；Checkbox 864例/20个ID，RadioGroup 576例/17个ID，Switch 576例/17个ID。scope包含基准与最终生产render，token列表是这些真实调用的并集。

它是 **lookup观测，不是完整依赖图**：常量、已编译展开的alias/文字样式、motion/layout访问器、真实宿主字体后端及未执行分支不在覆盖范围。`complete=false`，既有 `token_dependencies.tokens` 和完整依赖状态没有伪装成已完成；有界结果单列在 `runtime_observation`，统一Docs展示其scope与ID。

生成门禁检查观测的源码哈希、实际状态的四主题案例和canonical token ID。修改公共实现而未重新取证、缺少某主题error案例或出现未知运行时token，都会精确失败。31项治理测试通过；独立完整源码副本的三个CLI负例也分别拒绝这些问题。不能仅手改source hash来代替重新执行探针。

## Gold7 与验收边界

| 项 | 设计/代码结果 | 本批体验证据及边界 |
| --- | --- | --- |
| G1/G2 | 复用中性体系与已有失败色；没有重绘表面/整页强调 | 浅色整页前后对照抽查；新增强调只用于验证错误，不代表所有品牌体验已亲验 |
| G3 | 必要边界与独立焦点同时保留，Radio接入共享stroke | 抽查浅色Switch、浅色大字号Checkbox与深色IC大字号Radio；四主题、两倍率真实GPU全量留图，错误+焦点仍可区分、禁用保留值标记；物理外接屏未测 |
| G4 | 验证状态不接管Switch thumb动效或选择状态机 | 既有控件回归保留；本批不把静态GPU或合成事件当物理动效通过 |
| G5/G6 | error不阻断可修正操作；disabled继续拦截；组合invalid复用已有语义 | 公共API、独立消费者及活语义树通过；真实键鼠、VoiceOver和错误播报未重新验收 |
| G7 | 公共方法、实际状态页、源码绑定的token观测一致 | 当前22项选择族状态有公共API断言，53项状态夹具已源码核对；216项其他夹具仍待核验，不等于216项未实现 |

默认workspace严格Clippy、显式审计构建严格Clippy、tokens审计测试、fmt、DS、Gold7 mechanical、reference alignment、纯Rust与生成检查通过。没有性能测量。没有重跑全workspace测试，不把历史897项结果记成本批。

## 当前状态与下一步

#8继续部分完成；#1/#5/#12的全系统及原生验收条件保留。当前45组件、269项设计要求不变，选择族22项明确是 `public-api-verified`，experience仍为unverified；没有将合成API断言晋级为native/人工体验通过。历史包、旧报告与全部截图保留。

下一步进入已有 TextField/TextArea/SearchField 族，复用 TextSurface及现有IME/选区消费者场景，核验hover/pressed、read-only、composing、selection的真实能力与夹具，再按根因修共享层或补证据。完整运行时依赖与其他组件族继续核对；不新增组件、不测性能、不提交、推送、tag、关闭Issue或发布。
