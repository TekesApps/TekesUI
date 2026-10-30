# #12 B 修复：Master–detail 列表的完整单词宽度

状态 **NEEDS_COORDINATION**：隔离源码修复及定向回归准备完成，尚未获得新 Rust/GPU 窗口。
本批修复真实失败，不新增另一套整页矩阵。C `0749059c5e493a903a98653ab3336b156a5f709a`
及其5文件交接包冻结；B原候选/24红图/原日志不改写。

## 真实失败与源码根因

实际失败基线：集成 MAIN `aecd20e3254afff77895ecc7cbe73ac68b9887bd`＋原收据记录 WIP。
证据根为 `/Volumes/HP P900/DEV/TekesUI/artifacts/delivery/integration-executor-20261030-07/master12/`。
其 no-run、ignored GPU、master原行为及Gallery strict均exit0，但24原图逐图视觉FAIL，
不能以程序测试绿覆盖阅读失败。本人实际查看以下两张原图，均见 `Docu / ment / NN`：

- `pngs/pattern-master__playground__light-standard__720x512__type-150__top@1x.png`
- `pngs/pattern-master__playground__dark-increased__720x512__type-150__bottom@2x.png`

原 manifest 的720×512/type150样本：列表visible宽107.67999pt，detail x=436.67999pt、
宽254.32001pt；内容总宽374pt，原列宽=374×0.32=119.68pt。
源码再扣20pt和两侧focus padding，行按钮分配约89.68pt，Button随后还扣自身原token内边距。
原代码只用这个受限宽度测量行**高度**，因此可以把三行文字完整画出，却不保证完整单词可读。
这是本页父容器的宽度预算缺陷；没有证据需要修改共享CoreText断行算法。

## 最小修复与原设计同步

保留32%及240pt的原偏好，在该页使用原 `TextEngine::measure_button` / 生产Button测量
`Document` 在当前UiBody字号下的外框宽度，加回既有行预算和focus留白，作为列表列的最低宽度。
详情继续使用剩余空间；行布局、列表ScrollArea、logical focus、命中和详情来源不分叉。
完整标签仍为原 `Document 01` 至 `Document 80`，数字允许在完整单词之后自然换行。
不缩字号、不改文案/文字引擎/按钮token、不删行、不放宽视觉AC，也不新加状态机或布局框架。

对应原设计：`docs/gallery.md` 的Master–detail条目/共享几何约定、
`docs/comparison-method.md` §3 整体阅读与间距、Gold7 G1 whole-page-reading及G7真实组合。
这只是修复原完整单词阅读要求，不重写父#1/#55矩阵，不提升成熟度。
原 `master_reading_capture.rs` 24图入口原字节保留；待以新的真实源码身份重新采图，不覆盖红图。
原说明文案修复已在集成侧完成，本批没有再次携入或更改它。

## 受影响定向验证

新增 `master_detail_` 前缀回归使用实际CoreText paragraph和生产Button label frame，
检查窄/宽内容与100%/150%、1x/2x的单词容纳、80行逻辑序列和列表/详情边界。
原selection、nested scroll与focus reveal回归保持原断言，不能仅跑编译收口。
新增回归尚未实际执行；其旧源码失败预期须在受控窗口中以同一测试对旧分配重现，
修复后通过才记为真实红→绿。

专用集成会话接收本批精确生产hunk及测试后，在原MAIN/SSD cache的
`build_lock()` / `environment()` 和GPU占用guard内串行执行：

```text
cargo +1.95.0 test --locked --offline -p tekes-ui-gallery --bin tekes-ui-gallery master_detail_ --no-run

cargo +1.95.0 test --locked --offline -p tekes-ui-gallery --bin tekes-ui-gallery master_detail_ -- --nocapture --test-threads=1

TEKESUI_MASTER_READING_OUT=<new-fresh-directory> cargo +1.95.0 test --locked --offline -p tekes-ui-gallery --bin tekes-ui-gallery master_detail_narrow_reading_captures_real_page_and_nested_viewports -- --ignored --nocapture --test-threads=1

cargo +1.95.0 clippy --locked --offline -p tekes-ui-gallery --bin tekes-ui-gallery --tests -- -D warnings
```

旧分配红例只在新增测试断言上验证；不重跑旧24坏图来重复已知GPU失败。
新24原图必须全部逐张看：Document完整、数字可读、详情仍可读且有实际空间、列表滚动/thumb、
header与Event log顺序和顶/中/底可达性。任何失败保留原图/日志；测试绿不代替逐图阅读通过。
记录真实源/程序/命令/输出/PNG hash和guard释放事件，不触发独立PR或全库CI。
C只含Settings，未改变其相关renderer或状态路径；不因本页修复重跑A/C全矩阵。

## 本轮实际检查与未验

- 实际两张代表原始坏图及原manifest、四项exit0与visual FAIL收据核对：完成。
- scoped rustfmt、git diff/check、C专属源码/文档未改检查：通过。
- 新回归旧分配红例/修复后绿例、Rust编译、Clippy、新GPU及24图目测：未执行。
- 原生键鼠/物理显示/IME/VO/性能未验；不属于本局部修复的通过结论。

## Actual integration acceptance — 2026-10-30

The original visual failure was retained: all24 original frames split Document. The first new regression could not compile (borrowed map key), and its one-expression fix was returned by the original visual domain. The corrected test then actually failed on the old width allocation at row10002/360pt. Restoring the production width fix exposed a test-metric error: Paragraph.width includes trailing whitespace. The original domain supplied the exact per-line Line.width_trimmed assertion, matching the existing logical advance-width geometry without adding tolerance; whole Document, all80 logical rows and positive/non-overlapping detail assertions remain intact. Original13e9/eb/681/6d5 history and every raw failure are preserved.

The integration executor admitted and froze actual MAIN HEAD2af0290dad4bf4bf3d345c221a3261282ceed595 plus only the reviewed master story_docs copy hunk. The original visual domain then owned one complete serial window through the existing SSD/MAIN review-cache/build and GPU guards. Compilation, all3 actual master behavioral tests, the ignored capture test (one actual execution), and Gallery strict Clippy passed. The same archived test executable generated24 original PNGs across four themes,1x/2x,720x512/type150,top/mid/bottom. The domain B Worker individually inspected24, the domain lead independently inspected4, and the integration executor inspected4 additional distinct originals and verified all24 file hashes and the archived program. Document now stays whole with its numeric suffix on the next line; list/detail space and the empty event log remain readable and available.

All3494 input hashes and Git/index state were unchanged during that window; owned subprocesses exited and both original locks were independently reacquired/released. Actual program/source/commands and per-image conclusions are versioned in `design/reviews/master-word-readability-20261030/{execution.json,visual-review.json}`; immutable originals remain at SSD artifacts/delivery/integration-executor-20261030-09/visual-domain-window01/. This is finite initial-Document1 offscreen acceptance, not nested native input, physical display, IME, VoiceOver, performance or full Gold7. C64 and other unaffected passing evidence were reused, not rerun or relabelled.
