# #12 Batch B: Master–detail 窄窗阅读与实际说明缺口

状态 **NEEDS_COORDINATION**。本批基线为已冻结的 Settings batch A 完整候选
`3a711f17dd641fd62069e25ec5513ee247a9adb6`。A 的源码测试、48 PNG、原始 manifest、
逐图结论和输入/程序身份均不改变；本批只在 A 后增加独立 commit，不重新签 A 的结果。
遵循原 comparison-method/gallery 与 Gold7 G1 whole-page、G7 business-combinations，
复用 #12，不增加第二验收矩阵或新 Issue。

## 已核实的两个具体缺口

1. `pattern-master` 是 kind=pattern，现有 component-only state/variant PNG 入口排除它；
   `draft_capture.rs` 只捕获 form 协议阶段。已有 master 行滚动与焦点测试是程序几何/行为，
   不代替当前 720×512/type150 的四主题双倍率整页阅读。
2. 当前 registry 对 master 的 docs 仍说显式 Resolve、首次失败、retry；`story_docs` 只覆盖 filter，
   因而 Playground 显示这段文案。实际 master renderer 是 80 文档行与独立只读详情，
   不提供 Resolve/retry 控件。已有 `behavior_copy("master")` 已准确描述实际能力，可直接复用。
   这是源码确认的说明与实现不符，尚未运行本批新图，不将源码观察冒称当前 GPU 观察。

已实际查看 `design/reviews/workbench/13-master-narrow.png`。它确为 Master–detail 原生历史图，
文档行和详情可见；其 base=6ca8295dc398f68ba4d5d1fc333346f40fafee2b 加历史 WIP，
不是本批新基线，不能补签目标窄窗大字矩阵。原图 hash 和边界见 inspection.json。

## 独立候选

- `state_capture.rs` 尾部只增加一个 test module 声明，原 A 文件字节完整保留为前缀。
- `master_reading_capture.rs`：单一忽略测试，真实 pattern-master/playground，720×512/type150，
  四主题×双倍率×页面实际 top/mid/bottom，共24原PNG。复用 A 之前已存在的滚动/GPU/PNG helpers；
  不修改 A 的测试。记录 route/index/renderer、实际显示 description、offset/range/document height、
  visible list/detail frame 与初始 selected Document 1。输出必须不存在，旧图不覆盖。
- `proposed-master-copy.patch`：只供共享 Gallery 写者接收的最小2行生产提案，
  让 story_docs 的 master 分支复用现有 behavior_copy。**未应用**到任何源码树，
  未改 registry/generator/tokens；不能把提案文件当成已实现修复。

scope 只含初始文档，未执行内部列表滚动、文档切换、详情滚动、真实输入、焦点、IME/VO/性能。
看图必须检查 header 文案、whole-word 断行、list/detail 真实可见空间和页面 Event log；
若出现空/不可读 pane，要按原 AC 记录失败，不以非空 scene 判通过。

## 接收与执行依赖

MAIN 唯一写者/构建排期为共享渲染集成会话。本聊天未获 B 的新 build/GPU 窗口；
只请求一次完整 B 执行窗口，不请求逐命令转发。请该写者审查并接入必要的 master copy hunk、
本批专属 fixture，然后以该实际源码身份沿既有 build_lock/environment、SSD MAIN cache 与 GPU guard
执行，不另建 target/cache/rebind/clean，不动其它 WIP，不使用本隔离基线 SHA 代签 MAIN。

```text
# 由 MAIN 写者接收前，可检查提案精确上下文；本聊天只做了 --check：
git apply --check design/reviews/master-reading-20261030/proposed-master-copy.patch

cargo +1.95.0 test --locked --offline -p tekes-ui-gallery --bin tekes-ui-gallery master_detail_narrow_reading_captures_real_page_and_nested_viewports --no-run

TEKESUI_MASTER_READING_OUT=<fresh-absolute-output> cargo +1.95.0 test --locked --offline -p tekes-ui-gallery --bin tekes-ui-gallery master_detail_narrow_reading_captures_real_page_and_nested_viewports -- --ignored --nocapture --test-threads=1

cargo +1.95.0 test --locked --offline -p tekes-ui-gallery --bin tekes-ui-gallery master_detail_ -- --nocapture --test-threads=1

cargo +1.95.0 clippy --locked --offline -p tekes-ui-gallery --bin tekes-ui-gallery --tests -- -D warnings
```

A 不应为 B 资源等待而延期交付。若 B 修正文案，仅 master 路由输入改变，不重跑 Settings
48图；相关 Gallery strict/已有 master 行几何与焦点程序回归按上述范围执行。
24原图必须逐张阅读并给结论，manifest 仍保持 visual_review=unverified，目测结果另存。
留存实际输入清单、程序、原日志、PNG hash 与 guard 释放事件，不在本批触发全库CI。

## 真实检查

- scoped rustfmt（含嵌套 test module 路径解析）通过。
- git diff --check、生产提案 git apply --check 通过。
- A state_capture.rs 源字节前缀不变检查通过。
- B 编译/Clippy/程序回归/GPU/24原图目测均未执行，不能标 READY。
