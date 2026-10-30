# #12 Settings 整页阅读：已验证的有限批次

Refs #12；关联 #1 / #55。遵循 `docs/comparison-method.md` §3、`docs/gallery.md` 和
`design/behavior/gold7-alignment.json` 的 G1/whole-page-reading-composition、
G7/business-combinations-are-real；本文只解释这一个 fixture，不取代原验收表。

## 本轮实际结果

状态 **READY**，仅限本 fixture 的初始 Ready 页。原候选已精确应用至 MAIN 后，
在获分配的共享 build/GPU 窗口完成下述三项，退出码均 0：定向 no-run、真实 48 PNG 测试、
Gallery bin/tests strict Clippy。实际 MAIN HEAD 为 `77860b59df6eb8dc3432a96dd4bfd1730c1b215c`
加 AX/Disclosure/Settings 及保留 WIP；不以候选 commit 冒充该集成源码身份。
3469 个输入前后逐 hash 相同，输入清单 SHA256
`27ff7ba1cc1ff42f569c3314b35f89ba0279512f2e74d9d10e7e946005380e9a`。

领域负责人逐图查看窄窗 24 张；配置 B Scout 逐图查看宽窗 24 张，负责人另外独立抽查
四主题共 4 张宽图。逐图结论均为有限范围 pass，原始 manifest 的 unverified 保持原样；
实际人工结论另存 `visual-review.json`，不把生成计数当验收。窄窗真实 offset=0/201/402，
Event log 在 bottom 完整可见。Save/settings 在完整单词间分行；普通滚动导致 mid 顶部
Edge cases/Reset 部分在 viewport 外，不是组件自身裁切。宽窗 scroll range=0，
三位置是同一完整布局，不能作为可滚动内容的 bottom reachability 证明。

原日志、3469 输入清单、可执行文件、48 原图和锁事件保留于 SSD
`artifacts/issue-12/settings-main77860b5-20261030/run01/`；精确路径、hash 和程序身份见
`execution.json`。构建锁及 GPU 锁均已释放，无 MAIN 源码写入。

## 具体缺口与原图观察

现有 `implemented_component_state_and_variant_stories_render_at_each_viewport_position`
只接收 `kind=component` 的 states/variants，因而不包含 `example-settings/playground`。
`canonical_supported_stories_keep_distinct_actions_disjoint_at_large_text` 虽含 example，
但只检查 scene 非空和 action 几何，不是整页像素或阅读验收。

实际查看两张原图（SHA256、原源码比较见 `inspection.json`）：

- `icon-gold7-57/native-settings-narrow-150.jpg` 的页面标题实际为 Icon，Settings 是图标身份。
  不能从文件名推断 Settings 表单已验收；该根目录的图仍是 #57 README 明确保留的历史 v2，
  本批不更改 #57 最终 C 的结论或材料。
- `remaining-components-20261017/snapshot-final/example-settings-dark-ic@2x.png`
  的确是完整 Settings 初始页，能看到表单、Save 和 Event log。但其历史 workbench/text/mod
  源哈希均不同于本候选，不能将此旧宽页签为当前窄窗、大字或四主题通过。

#12 顶部现有 Banner、SplitView、Progress 修复和收据继续按原身份复用；没有重新实现它们。

## 候选与边界

基线 `d51a7ffd5693388fb82ae3c7575eec63ece18ff0`，隔离工作区
`/Users/frankqin/.codex/worktrees/issue12-page-visual/TekesUI`。
唯一 Rust 变更是 `state_capture.rs` 尾部新增的独立忽略测试；没有生产 hunk。
它复用现有 `VIEWPORTS`、`POSITIONS`、滚动稳定、真实 GPU 和 PNG 写入逻辑：

- 实际路由 `example-settings/playground`，同时断言当前 `Page::Workbench` 索引及 renderer 身份。
- 默认 brand 0，四主题，@1x/@2x；720×512/150% 与 1440×960/100%。
- 每组按真实 range 采集 top/mid/bottom，合计 48 原始 PNG；实际 offset/range/document height 写入 manifest。
- 必须提供不存在的 `TEKESUI_READING_PAGE_OUT` 目录；已存在则失败，不覆盖旧图。
- manifest 中 `visual_review=unverified`。scene/像素非空及滚动位置断言通过也不代表可读性通过。

本批仅初始 Ready 表单的阅读页面。不覆盖 Pending/Failed/Saved、真实键鼠/TextCommand、
焦点 reveal、物理显示、IME、VoiceOver、性能或全 Gold7，不提升 registry 成熟度。
宽大字、窄正常字号也不在这两个代表组合内。

## 协调者执行包

MAIN 当前 `gallery/text.rs` 存在其他领域 WIP，`inspection.json` 如实记录其与本基线的差别。
只接收本批白名单，不复制整个隔离树，不覆盖该 WIP。由协调者将候选接入所选冻结集成基线，
记录实际源码身份；不得用本基线 SHA 为包含其他 WIP 的程序作证。

下列命令已在前置 Disclosure 结束并确认释放锁之后，于分配给本聊天的窗口执行。
协调者在既有 `tekesui_build.build_lock()` / `environment()` 和 GPU occupancy guard 内，
沿现有已绑定 MAIN 的受控入口串行执行。不要为本批建立 target、rebind、clean 或改时间戳。
父输出目录应已存在，`<fresh-output>` 本身必须不存在。

```text
cargo +1.95.0 test --locked --offline -p tekes-ui-gallery --bin tekes-ui-gallery settings_whole_page_reading_matrix_uses_actual_route_and_scroll_positions --no-run

TEKESUI_READING_PAGE_OUT=<fresh-output> cargo +1.95.0 test --locked --offline -p tekes-ui-gallery --bin tekes-ui-gallery settings_whole_page_reading_matrix_uses_actual_route_and_scroll_positions -- --ignored --nocapture --test-threads=1

cargo +1.95.0 clippy --locked --offline -p tekes-ui-gallery --bin tekes-ui-gallery --tests -- -D warnings
```

保存实际测试可执行文件身份、源身份、原始命令/日志和 PNG hash；执行期间有相关输入变化则保留
该结果并标明变化，不重签。上面是定向命令参数，不授权绕过既有 guard 直接跑 shell。
无需原生窗口，本批只申请离屏 GPU；物理屏幕与输入检查继续保留给相应原 AC。

采集后逐张查看 48 张原尺度图，按原 AC 检查正文/说明的断行、header actions、label/helper
间距、表单和 Event log 顺序、边界层级及 bottom 可达性。记录每张 pass/fail/unverified，
不能由非空断言或 48 文件数量替代目测。发现真实缺陷继续复用 #12，保留原红图；
涉及生产共享 hunk 时先由协调者指定 writer。本步骤现已完成，逐图结论见 visual-review.json；本有限批次为 READY。

## 已执行与未执行

- `rustfmt +1.95.0 --edition 2024 --check .../state_capture.rs`：初次格式失败，局部修正后通过。
- `git diff --check`：通过。
- 原图实看、registry example 身份及 component-only 过滤源码核对：完成。
- Rust 定向 no-run、Gallery bin/tests strict Clippy、真实 GPU 48 PNG：均 exit 0。
- 48 原图均已逐图查看并记录有限视觉 pass；输入前后一致，原始日志保留。
- 物理屏幕/真实输入/焦点/IME/VO/性能以及其他页面和状态：未验，本批不覆盖。
- 未 push、新开 PR、触发全库 CI、修改 MAIN/保留树或旧原图。
