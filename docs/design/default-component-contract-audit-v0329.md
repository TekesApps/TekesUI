# Component defaults and consumer consistency — v0.3.29

Audit parent: https://github.com/TekesApps/TekesUI/issues/298
Baseline: annotated v0.3.29, commit de3b28dac56b0917cbfe9463fc019b6c8f401b1b.
This is a fixed-source audit; later repairs retain separate candidate identities.

## Scope and ownership

Theme defines semantic roles; components own their geometry, state and lifecycle;
AppHost owns windows, regions and declared command routing; consumers own content,
models and business events. Different page/list/form spacing roles are legitimate.
Public Model/View and Content entry points both exist; a low-level API or numeric
literal is not by itself a defect.

Four read-only audits covered collection/layout/scroll, inputs/actions/selection,
AppHost/Gallery/Settings consumer boundaries, and Theme/remaining component
families. Root cross-checked the new findings against primary code. Root is the
only source writer; no GUI operator or native session was started.

The primary checkout retains historical WIP; the release checkout retains local
AGENTS/configuration edits. Neither was reset or edited. Audit source was admitted
clean at the exact requested tag in an isolated managed worktree. Current user
lifecycle instructions supersede the tag's historical fixed-chat requirement.

## Findings and independent batches

| ID | Finding and reachable effect | Owner / consumers | Tracking |
| --- | --- | --- | --- |
| F1 | Nonempty Search: Clear hover followed by movement into the editor/outside leaves Clear's cached hover. Blur/cancel also only cancel its press. | Shared Search compound routing; default Gallery Search Playground and AppHost Gallery filter | #299; first repair batch |
| F2 | Hidden nested ToolbarGroup is excluded from measured child size but still counted in gaps and arranged, moving/clipping visible siblings. | Shared ToolbarGroup; public nested composition is reachable. No current production AppHost/Gallery declaration of this shape was established. | #300; independent repair batch |
| F3 | Direct SettingsForm uses 560 × body typography scale; AppHost uses fixed 620. At 150%, wide allocations produce 840 vs 620. | Shared Settings layout / AppHost adapter; AppHost Gallery and app-host-demo Settings | #301; shared 560 + resolved body typography scale |
| F4 | Legacy Gallery directory Search still uses TextDemo, and action helpers shrink compact frames while two consumer measurement paths add the same 4pt back. | Gallery migration; no newly proved pixel failure from the compensation itself | Reuse #290/#294; do not create parallel migration tickets |

F1 sources: composition/search.rs:214–218,425–440,621–656;
composition.rs:1922–1959; widgets.rs:1365–1373,1431–1451.
F2 sources: composition/toolbar_content.rs:131–206; composition.rs:1255–1285.
The enclosing ToolbarContent recursive minimum-width calculation at
toolbar_content.rs:387–397 also counts inactive gaps; the same repair includes
this reachable constrained-allocation path.
These are under crates/tekes-ui-controls/src.
F3 sources: crates/tekes-ui-settings/src/form.rs:210–231;
crates/tekes-ui-app-host/src/window_settings.rs:128–139,1203–1255,1918–1939;
design/source/base.tokens.json:1498–1506.
F4 sources: crates/tekes-ui-gallery/src/gallery/mod.rs:224,718–723;
gallery/text_demo.rs:618–631,726–763;
gallery/workbench.rs:1595–1628,1658–1715;
gallery/workbench/extended/feedback_adapter.rs:1668–1688
(the latter paths are relative to crates/tekes-ui-gallery/src).

Each linked Issue contains the complete call path, failure condition, minimal
repair and necessary verification. #299–301 are actual GitHub sub-issues of #298;
#294 retains its existing parent #290 and is reused by reference.

## Proposed repairs and compatibility

F1 forwards the original Clear Button's outside primary Move and merges redraw
without stealing editor capture/actions. Full Search blur/cancel uses the original
Button blur contract, with no public API or global Button cancellation change.
Validate original Scene ink plus real Host query/focus/capture/action behavior.
Native follow-up also exposed a tall allocation stretching Clear while the
single-line editor stayed at its natural height. Search now bounds its auxiliary
button to the original Button's measured height (clamped to available height),
without changing the outer allocation or requiring consumer height patches.
The empty area below Clear must not activate it; the tall-allocation Host
regression preserves the observed failure separately from hover cleanup.

F2 measures and arranges the same active child set and counts only its gaps.
Retain hidden nodes and surviving mount identities; no consumer padding or
remove/reinsert workaround. Validate both axes, visibility transitions and
constrained geometry using public composition.

F3 uses the existing shared 560pt token scaled by resolved body typography
(840pt at 150%), capped by the available content allocation and centered.
`SettingsForm::body_rect` owns this operation for direct rendering and AppHost's
mounted original host. AppHost retains pane allocation, titlebar-safe insets and
outer padding. This narrows the former AppHost 620pt body at 100% and allows it to
grow with type size; no new token or row/control-specific compensation is added.
The former 620pt radio fixture retains its content-fit acceptance at the shared
scaled width, alongside direct/mounted geometry regression coverage.

F4 continues existing route/story migration, including real Button layout at
normal/compact density. Removing compensations before adopting the shared owner
would risk changing the intended appearance. Do not replace missing stories with
lookalikes or reopen the already delivered Field/Area/Combo/master-detail work.

## Already repaired / reasonable differences

- #269 and #270–277 are closed finite deliveries, verified against current issue
  state; clipboard, wheel metadata, Toolbar/Menu lifecycle, text snapshots and
  Gallery Field/Area/Combo/master-detail changes are present. Do not reimplement.
- List scrollable defaults own Loose viewport padding and Leading alignment;
  nested ListContent::new is organization only. Existing hover-background and
  stable selection fixes are not new findings.
- Empty Search reserves no Clear width/gap and paints no Clear semantic part.
- ScrollContent's documented barless default differs deliberately from the
  ordinary scrolling List's hover scrollbar. Tree/Table may use an outer viewport.
- Settings row anatomy, host pane/titlebar allocation, and explicit zero outer
  padding paired with content-owned padding have legitimate single owners.
- Finite string/index Select and Combo adapters publish immutable selection
  payloads; they do not promise arbitrary domain-ID storage. Keyed List supplies
  stable selected_key and intents.
- Command+F correctly resolves an explicitly declared search_entry. AppHost
  Gallery registers FILTER. Legacy Gallery and collapsed Settings do not register;
  enabling Find there is a product/capability decision, not a proved routing bug.
- Panel, feedback, chart, media and reading paths inspected use semantic Palette/
  Surface/typography roles. Ordinary component-specific geometry is not a token
  violation. Dialog's inherited child Surface has an explicit existing test and
  was not reclassified as a defect without a visible failure.

## Coverage and limits

The registry contains 45 entries. Source-level paths were inspected by family:
Stack/Grid/Separator/Split/Scroll/Disclosure/Panel; Toolbar/actions/navigation;
Field/Area/Search/Combo/Select/selection; List/Tree/Table; Chart/media/readers;
feedback/passive components. This is family-based source coverage, not exhaustive
execution of every state, branch or public type.

All ten current example manifests/entry-point sets were inventoried:
app-host-demo, brand-consumer, composition-consumer, host-reveal-consumer,
icon-catalog-consumer, media-viewer-content-consumer, panel-content-consumer,
panel-module-demo, process-content-consumer, theme-system-consumer.
They are independent local development consumers, not all workspace members.
Focused boundaries were traced; the 203 top-level integration-test source files
were not all deeply audited or executed. Historical design/reviews harnesses and
external repositories are outside current-product consumer acceptance.

Coverage reconciliation also inspected Grid/SplitView's retained layout and pane
lifecycle; Checkbox/Radio/Switch/Segmented/Slider/Number/Icon/Link's original
state and public value/event paths; and the separate legacy MarkdownView,
CodeBlockView and DiffViewRuntime actually used by Gallery. The latter are
documented static previews: RichContent supplies continuous selection/copy.
No new reachable defect was established in those paths. Similar press-only
cancellation in NumberField was not equated with Search's visible hover failure:
its step buttons deliberately paint transparent chrome. Gallery's lower-level
Segmented and Link Views remain valid public APIs, with Content migration and
its acceptance separately tracked by #294.

## Evidence dimensions

- Baseline source: local and remote tag peeled to de3b28d; isolated checkout clean
  at admission. Evidence and assignment ledger: outputs/default-contract-audit-v0329/.
- Baseline generation: generate-component-capabilities.py --check PASS.
- Baseline composition audit: check-composition-audit.py PASS for 45 entries.
  These are metadata/source-binding checks, not CPU behavior or native proof.
- Runtime repairs / CPU: initially pending; subsequent receipts record exact inputs.
- Native appearance/input, physical IME, VoiceOver, GPU and performance: no new run.
- Public delivery and downstream adoption: no new claim from this audit or Issue creation.

## First repair candidate

F1 and F2 are repaired in the shared components, with no AppHost/Gallery runtime
patch and no new public API. Old Search Scene output demonstrably retained its
hover fill after moving into the editor; the real ContentHost reproduced the
same failure. Old ToolbarGroup measured 119pt where the equivalent visible tree
measured 115pt. Initial test compile errors are retained separately and are not
counted as behavioral red evidence.

After repair, 48 scoped tests passed: 4 button/chrome, 10 List/Scroll/real-text-Host,
3 Toolbar unit, 25 independent public Search consumer, and 6 public Toolbar
consumer checks. The Toolbar regression covers both axes, hidden first/middle/last,
empty active sets, visibility restoration, surviving mounts, visible clips and
an enclosing toolbar with a competing sibling. Affected library/test Clippy
passed with warnings denied. Exact commands, durations and source hashes are in
outputs/default-contract-audit-v0329/green-1-receipt.json.

The tests use deterministic immediate chrome rendering and CPU CoreText shaping;
they do not certify animated GPU pixels or native window interaction. No GUI was
started, installed application replaced, release published or adoption inferred.
F3 remains a width-policy decision; F4 remains the existing migration's work.
