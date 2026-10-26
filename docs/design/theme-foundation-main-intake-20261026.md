# Theme foundation intake on local main

Delivery entry: #136, dependency slice of #62 and the existing #6/#7/#55 Theme contracts. Baseline: `361f418612693069d6a6534aafff2cc79154f91b`. Root is the sole integrator. Source provenance is the hash-frozen 1509-file historical observed source plus the exact sixteen accepted #62 Theme interface increments; this is not a copy of current Gallery Root WIP.

This slice supplies the complete selected historical Theme schema/rules/instances, Palette ThemeDefinition identity, full generated Surface packet facade, and normal pure Rust source/artifact/independent `/1` lock validation. Python remains offline generation tooling. The existing main Toolbar 20pt base inputs and metadata are preserved, with accurate reviewed source pins; no historical 13pt rollback is included. Existing default/Gallery BrandConfig semantics and role alpha remain authoritative.

## Source compatibility and dependency closure

Palette now selects an explicit `GalleryBrand` or `ThemeDefinition` source. External `Palette { theme, brand }` literals and `.brand` field access migrate to `Palette::new`, `Palette::from`, `source()` and `brand_index()`. The repository's direct uses are included in this batch. The two motion cache keys retain `(Theme, PaletteSource)` so independent ThemeDefinition choices cannot alias brand zero. This is a source API migration; unchanged rendered default/brand behavior is an actual regression gate, not an assertion of struct-literal compatibility.

The independent public consumer selects blue/clay/default in all four appearance/contrast modes, checks unknown identity rejection, accent isolation, neutral/default compatibility, complete packet consumption and original alpha. A separate external-artifact QA consumer is required for normal independent-lock acceptance and rejection/recovery. The ordinary `/1` lock is retained; this batch does not claim the later provenance sidecar or full-root proof contract.

## Acceptance boundary

The final local-main checks cover generator freshness, complete Theme rules and negative cases, normal tokens/design-export tests, the necessary Palette call-site builds/regressions/strict checks, and real independent public consumption of valid external input, rejected missing/stale/tampered input or lock, and valid recovery. Evidence is stored on SSD under `artifacts/issue-62/main-361-theme-foundation-r1` with source/program/depinfo/OUT_DIR identities and separate failed rounds.

This dependency slice does not re-accept all #62 ContentTheme inheritance/override/clear, Button/Text/TextField mounting or visual migration. #104 Chart usages/focus have now completed local-main acceptance on base `3e989f1` plus the exact source receipt; see [Chart design and current evidence](chart-content-20261024.md). The original #115 geometry formulas and bare-reference records are preserved. #92 ContentHost remains a separate intake requiring action-preparation/reveal dependencies. Historical Root evidence retains its original identity. #113 now uses the practical theme-consumption acceptance contract below; all52 status coverage remains required, but complete source-closure certificates do not gate component delivery. This foundation alone does not mark any root complete. Native windows, physical IME, VoiceOver, whole-library Gold7 and public release are not claimed here.

## Shared rules versus component intake (2026-10-26)

#71 Progress rules (`dc7283f9`), #83 StatusBadge/Skeleton rules (`8fb1909`) and #93 Notice rules (`3e989f1`) are accepted on local main. The #72 Progress Content/Gallery migration was subsequently received in `3f13a1e`, as recorded below. The #76/#77/#79 component migrations remain to receive; original Gallery Root proofs are not transferred to main. #104 now connects its two shared purposes to the actual public ChartView and independent default/external consumer. It does not receive #92 ChartContent. Receive each complete component slice with its actual dependencies, without waiting for every shared rule or redevelopment of accepted parts.

## 当前验收范围（2026-10-26，用户明确授权替换审计 AC）

本次调整审计机制，不削减组件功能、主题支持、Gold7 或无障碍等产品承诺。旧证据保留原源码身份；新范围通过不表示旧闭包 AC 通过。

| 旧要求 | 处理决定与理由 | 新验收 |
|---|---|---|
| 全库递归图、完整 owned-reference closure、逐路径无未知边 | 取消强制；非统一设计值与组件消费的必要前提 | 审核公共语义消费入口、受管设计值及具体状态差异 |
| 标准库、第三方内部、原生 ABI 全面证明 | 取消强制；超出本次产品目标 | 具体边界出现产品风险时定向验证 |
| 每根来源证书、完整事件依赖图、条件全排列 | 取消强制；共享结果可复用 | 真实公共组件/Gallery 代表性路径、必要状态与对比、功能回归 |
| 通用解析器、图引擎、证明框架扩建 | 停止本轮新增；已有成果后置可选 | 复用现有生成器、registry 和稳定验收入口 |
| 契约有效性、实际主题消费与全52根覆盖 | 保留 | 以下实用 AC；未验/缺陷如实记录 |

1. **契约正确**：Schema、类型、必要字段、引用、默认值有效；生成输出匹配当前输入，正常构建拒绝明确损坏/过期输入。复用已有生成器及有效正负例。
2. **消费正确**：Theme/Palette/Surface 或等价公共语义接口管理颜色、字号、间距、圆角等设计值；普通布局、索引、几何数字无需全 token 化，合理例外说明原因。
3. **行为正确**：真实公共组件与适用 Gallery 路径验证主题切换、必要普通/悬停/按下/禁用/焦点等状态、必要文字/背景对比和原功能回归；代表性组合覆盖不同实现路径，不重复无独立风险的全排列。
4. **覆盖明确**：复用 `design/gallery/workbench.json` 的52个登记根；在已有 `docs/design/component-registry-public-api-20261017.md` 维护一份消费入口、已验状态、例外、缺口、证据与 Issue 清单，不另建登记系统，不要求52份闭包证书。

共享生成器、规则、文字/渲染基础能力只在输入变化或新增具体风险时补验。每次新增检查注明 AC/风险，重跑注明失效输入；仅服务于取消证明目标的门禁明确退出强制验收，不静默跳过后称旧 AC 已通过。历史生成、源码及失败记录完整保留。

## 执行与退出条件

唯一协调/接收负责人：root；共享 schema、生成器、规则仍由 root 单点维护。组件实施按实际缺口在私有候选交可整合差量；主工作树仅 root 整合。通过范围必须区分私有候选、Gallery Root、本地 main 与远端；partial 不标 complete。

构建沿原共享锁、SSD 和占用保护；不改源码/mtime/flags 强迫重编，不创建新 target、不清空缓存、不打断任务。只暂存明确归属文件/hunks，本地必要验证后提交；不 push/远端合并/发布。原功能、原生/a11y及公开版本交付的未完项保留在对应原票，不附加为任意单项 Theme 票的关闭前提。

## First practical Button intake (2026-10-26)

Base `3d7ff9e` plus one test-only widgets.rs hunk; no production API or behavior change. Existing ButtonContent -> Button::render -> button_variant_with_interaction -> Palette is a valid semantic consumption chain. Animation interpolation bounds 0/1 are algorithmic constants; no forced Surface packet migration is required.

On final local-main source under the original shared lock and SSD cache: 20 Button lib regressions, one cross-theme emphasis test, one focus-ring test, two public composition tests and Controls lib strict check passed. The new test checks actual Scene fill and transition snap when Theme/PaletteSource changes during hover/press animation. Receipt: SSD `artifacts/issue-113/practical-theme-audit-scope-20261026/run-20261026T133948.070000Z/run-receipt.json`. Generator/config/GPU inputs did not change and their checks were not rerun. Candidate89 evidence retains its own source identity.

At that intake: real Gallery theme-switch/state evidence and necessary contrast evidence were pending; the following two batches resolve the representative checks. Remaining #60 product obligations stay separate. This does not establish native acceptance or the superseded closure AC; #113 remains open. Single-file format check reports two pre-existing differences outside this hunk (motion_palette type and checkbox test); no unrelated formatting was applied.

## Button public contrast and real Gallery follow-up (2026-10-26)

Public contrast commit `cef7b7f` captures actual ButtonContent text ink and Scene fill: Quiet/Primary/Panel, rest/hover/pressed, four modes and two representative brands meet the existing unrounded 4.5 standard / 7 increased-contrast threshold. Disabled text is excluded by the existing contrast-pairs contract. Three integration tests and the targeted strict check passed.

On base `cef7b7f` plus one Gallery test hunk, the real `component-button/playground` route receives pointer and appearance/contrast/brand commands. Mid-hover Light -> Dark Increased and mid-press brand 0 -> 1 repaint actual Button quads and text glyph runs with the new Palette, finish pending transitions and retain layout. The new test, existing pointer activation/cancel regression and disabled-button tab-sequence regression passed, as did Gallery test strict checks. No synthetic replacement component or production getter was introduced.

Evidence: SSD `artifacts/issue-113/button-public-contrast-main2a-20261026-01/commit-receipt.json` and `artifacts/issue-113/button-gallery-main-cef-20261026-01/` (run receipt and commit receipt). These tests establish representative practical theme consumption for the current public label Button and its actual Gallery path. They are not pixel/GPU/native/VoiceOver certification, full #60 arbitrary-content/Host delivery, or full52 acceptance. No new whole-library prerequisites or all-combinations proof is implied. Producer/config/render inputs were unchanged; prior valid shared evidence was retained without rerun.

## Composition Surface inheritance intake (2026-10-26)

On base `bcea992`, the #62 shared composition contract is now received: ContentTheme carries a canonical Surface identity; Content can override or clear it; components may declare validated defaults; ContentHost sets the inherited Surface. Body, overlay and retained Dialog traversal use the actual ancestor chain. Explicit node Surface wins, clearing restores default/ancestor inheritance, and invalid defaults fail mount. A targeted real CoreText/public-Host test verifies nested Dialog inheritance, ancestor changes, override/clear and host fallback; a second verifies declared defaults and invalid mount rejection.

`ContentTheme` struct literals migrate to `ContentTheme::from(palette)` and optional `.with_surface(name)?`; the new field is private so callers cannot bypass canonicalization. Existing public Palette and rendering semantics remain intact. Two previously delivered #97 public retained-exit/half-ULP regression cases, missing from the older combined patch, are now included without changing production fragment behavior.

Actual verification: Controls lib (261), public Button alignment/contrast (3), original fragment consumer (6), Surface/Dialog consumer (2), Controls/Gallery all-target strict and targeted consumer strict passed under the original shared cache/lock. Evidence: SSD `artifacts/issue-136/surface62-dialog-main-bcea-20261026-01/`. No generator/config/render implementation changed; their earlier valid evidence was not rerun.

At this Surface/Dialog intake, #72 ProgressContent remained a separate private candidate: its full consumer required text snapshot/binding and native content-accessibility bridge APIs not yet received. The subsequent Progress batch below resolved those dependencies. Both failed attempts and all candidate files are retained in SSD `artifacts/issue-136/progress72-surface62-main-bcea-20261026-01/withheld-progress-source/`. No required test was deleted to claim it passed. This shared-interface commit is not Progress main acceptance, native/VoiceOver acceptance, or full #62 component migration. The remaining #81/#98 and #72 dependencies keep their original delivery obligations.

## Progress component intake on local main (2026-10-26)

The previously withheld #72 candidate is now verified on base `63214be` plus the exact intake delta: original Progress painter/View and ordinary ProgressContent consume the shared progress-track/rest packet; Surface inheritance and explicit overrides preserve the original model, passive Group semantics, Composer binding/marked/selection/focus and reading anchor. Shared read-only Content text queries and the opt-in macOS `content-accessibility` adapter are included as necessary dependencies, without claiming full #5 acceptance.

Evidence is in SSD `artifacts/issue-136/progress72-main-63214be-intake/`: round02 public consumer/API; round03 platform action/remount/text projection, Gallery progress and production strict; gpu-round04 128 default plus 256 external-theme GPU readbacks, external public consumer and 48 actual Gallery captures; strict-round05 targeted consumer strict. Failures remain in their original rounds. The final GPU test only received an equivalent nested-condition cleanup and an explicit opt-in ignore annotation afterward; production inputs did not change and GPU evidence retains its original source identity. Run this test explicitly with `--ignored --exact final_progress_actual_gpu_readback`, original locks and fresh SSD output/environment; ordinary test runs must not require capture paths.

The 0% boundary and View/Content pixel parity are verified, not inferred from screenshot counts. Two narrow large-text Gallery captures were visually inspected: progress track/fill/unknown states appear; navigation labels still wrap. The introductory copy incorrectly described indeterminate as absent at capture time; `6c7f55e` corrected that copy without changing rendering inputs. The remaining layout gap is retained under #12, not silently certified as full-page experience. Native windows, dedicated ProgressIndicator role, VoiceOver, physical IME, performance and full Gold7 remain unverified.
