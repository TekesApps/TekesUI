# Gallery AppHost route coverage — Issue #294

Baseline: `v0.3.30`, `fedb69b6f5b622f3fa393d0173da0999b3411d2d`.
Source inspection on 2026-11-16; this is a migration inventory, not passing runtime evidence.
Parent: #290. Design: [Gallery as an AppHost consumer](gallery-apphost-workbench-20261114.md).

70 registered routes now mount source candidates with applicable sections. Check380
passed the Split/Icon action regression (1 test), all-70 route construction (1 test)
and Gallery all-targets strict checks. Route construction is not 70 independent
behavior or native passes. Check381 built the release-mode candidate successfully;
it is not a public Release or Dock delivery. That frozen r3 program retains the
legacy default and runs with `--apphost`. The final source candidate now selects
AppHost for ordinary no-argument launch, retains `--legacy` and the original
snapshot/selftest/performance diagnostic paths, and uses workspace version 0.3.31.
Check382 passed the entry argument contract, all 11 locked manifests and Gallery strict; the final release build and native acceptance remain separate.
installed/Dock v0.3.30 remains unchanged.

The current CPU evidence below combines affected passing checks with unchanged
prior checks; earlier failed attempts remain in their original logs. Native-r1
(Check361) and native-r2 (Check368, fixed 33-route program) retain their own limited
evidence. Native-r3 for the Check381 candidate, signature prefix `d1120c4`, is in
progress and has no completed acceptance result here. No physical IME, VoiceOver
or performance acceptance is inferred from CPU tests or the candidate build.

Combo/Select now include the conditional independent refresh-status consumer
specimens (three Combo, two Select). Their original private components and Feedback
are created by shared consumer factories and mounted with distinct IDs in Gallery's
one Host; Scenario::new uses the same factories. Link and SegmentedControl now
reuse original StoryState models, formal Content and the shared original matrix
painters. The source-only default-entry candidate is separate from the frozen r3 program and installed application.


The sections column follows `Entry::supports_story` in `gallery/workbench.rs`,
including `basic_states::supports` and `extended::supports_story` with collection
and feedback helpers. `docs`/`playground` are accepted route suffixes even on
reference and launcher entries; that does not assert they render tab controls.
Original behavior remains the acceptance baseline, including supported state
matrices, edge fixtures, sizes and actions. The mounted Playground's few edge
buttons do not establish coverage of the original edge-case section.

| Route | Renderer | Original accepted sections | AppHost source coverage | Adaptation boundary | CPU evidence for current source | Native evidence |
| --- | --- | --- | --- | --- | --- | --- |
| `introduction` | `docs` | docs, playground | full source composition | reference/foundation page contract | Check364 CPU / strict passed | native-r3 in progress; earlier fixed-program evidence only |
| `getting-started` | `docs` | docs, playground | full source composition | reference/foundation page contract | Check364 CPU / strict passed | native-r3 in progress; earlier fixed-program evidence only |
| `component-index` | `index` | docs, playground | full source composition | reference/foundation page contract | Check364 CPU / strict passed | native-r3 in progress; earlier fixed-program evidence only |
| `foundation-tokens` | `tokens` | docs, playground | full source composition | original foundation/Motion model and text/resource services | Check365 foundation CPU passed | native-r3 in progress; earlier fixed-program evidence only |
| `foundation-typography` | `typography` | docs, playground | full source composition | original foundation/Motion model and text/resource services | Check365 foundation CPU passed | native-r3 in progress; earlier fixed-program evidence only |
| `foundation-iconography` | `iconography` | docs, playground | full source composition | original foundation/Motion model and text/resource services | Check365 foundation CPU passed | native-r3 in progress; earlier fixed-program evidence only |
| `foundation-spacing` | `spacing` | docs, playground | full source composition | original foundation/Motion model and text/resource services | Check365 foundation CPU passed | native-r3 in progress; earlier fixed-program evidence only |
| `foundation-shape` | `shape` | docs, playground | full source composition | original foundation/Motion model and text/resource services | Check365 foundation CPU passed | native-r3 in progress; earlier fixed-program evidence only |
| `foundation-motion` | `motion` | docs, playground | full source composition | original foundation/Motion model and text/resource services | Check368 clock (1) and scoped Symbols GPU (1) passed; unchanged prior checks retained | native-r3 in progress; earlier fixed-program evidence only |
| `foundation-accessibility` | `accessibility` | docs, playground | full source composition | original foundation/Motion model and text/resource services | Check365 foundation CPU passed | native-r3 in progress; earlier fixed-program evidence only |
| `component-stack` | `stack` | docs, playground, variants | full source composition | StoryState / existing Content composition | Check364 CPU / strict passed | native-r3 in progress; earlier fixed-program evidence only |
| `component-grid` | `grid` | docs, playground, variants | full source composition | StoryState / existing Content composition | Check364 CPU / strict passed | native-r3 in progress; earlier fixed-program evidence only |
| `component-separator` | `separator` | docs, playground, variants | full source composition | StoryState / existing Content composition | Check364 CPU / strict passed | native-r3 in progress; earlier fixed-program evidence only |
| `component-split-view` | `split-view` | docs, playground, behaviors, variants | complete source candidate | original action/model via formal Content; launcher uses existing AppWindow open request | Check380 real action/divider regression passed (1 shared test) | native-r3 in progress; earlier fixed-program evidence only |
| `component-scroll-area` | `scroll` | docs, playground, behaviors | all applicable source sections | original TextDemo surfaces in finite live viewport; same Host native input/scroll | Check370 finite viewport, native-owner/edit and reading contracts passed (3 tests) | native-r3 in progress; earlier fixed-program evidence only |
| `component-disclosure` | `disclosure` | docs, playground, states, behaviors | complete source candidate | original Disclosure model / formal controlled Content; eight original matrix cases plus five independent status consumers | Check377 live consumers passed (1 shared test); Check378 original retained/status consumer tests passed (1 each) | native-r3 in progress; earlier fixed-program evidence only |
| `component-surface` | `surface` | docs, playground, behaviors, variants | full source composition | StoryState / existing Content composition | Check364 CPU / strict passed | native-r3 in progress; earlier fixed-program evidence only |
| `component-toolbar` | `toolbar` | docs, playground, states, behaviors | all applicable source sections | original StoryState / production Content or original stepper View; shared original matrix | Check376 real actions/bounds passed (1 shared test); original matrices Check370 passed | native-r3 in progress; earlier fixed-program evidence only |
| `component-text` | `text` | docs, playground, behaviors | all applicable source sections | original TextDemo surfaces in finite live viewport; same Host native input/scroll | Check370 finite viewport, native-owner/edit and reading contracts passed (3 tests) | native-r3 in progress; earlier fixed-program evidence only |
| `component-icon` | `icon` | docs, playground, behaviors | all applicable source sections | original catalogue/Search; Docs/Behaviors/Reset; shared live icon services | Check370 original catalogue (8), host checks (3) and scoped GPU (1) passed; remaining host fixture Check371 passed (1) | native-r3 in progress; earlier fixed-program evidence only |
| `component-button` | `button` | docs, playground, states, behaviors, edge-cases, variants | full source composition | StoryState / existing Content composition | Check364 CPU / strict passed | native-r3 in progress; earlier fixed-program evidence only |
| `component-icon-button` | `icon-button` | docs, playground, states, behaviors | complete source candidate | original action/model via formal Content; launcher uses existing AppWindow open request | Check380 real action/divider regression passed (1 shared test) | native-r3 in progress; earlier fixed-program evidence only |
| `component-link` | `link` | docs, playground, states, behaviors | all applicable sections | original model / formal Content / shared original matrix painter | Check367 CPU / strict passed | native-r3 in progress; earlier fixed-program evidence only |
| `component-text-field` | `field` | docs, playground, states, behaviors, edge-cases, variants | all applicable sections | retained original StoryState/Host; passive original TextSurface matrices | Check365 chapters / retention / Search passed | native-r3 in progress; earlier fixed-program evidence only |
| `component-text-area` | `area` | docs, playground, states, behaviors, edge-cases, variants | all applicable sections | retained original StoryState/Host; passive original TextSurface matrices | Check365 chapters / retention / Search passed | native-r3 in progress; earlier fixed-program evidence only |
| `component-search-field` | `search` | docs, playground, states, behaviors, edge-cases, variants | all applicable sections | retained original StoryState/Host; passive original TextSurface matrices | Check365 chapters / retention / Search passed | native-r3 in progress; earlier fixed-program evidence only |
| `component-combo-box` | `combo-box` | docs, playground, states, behaviors | all applicable source sections | original StoryState plus shared original consumer Content factories | Check365 chapters; Check366/367 consumer CPU / strict passed | native-r3 in progress; earlier fixed-program evidence only |
| `component-slider` | `slider` | docs, playground, states, behaviors | complete source candidate | original matrix plus shared independent range/editor/action subtree in the receiving Host | Check377 live consumer passed; Check378 original slider consumer target passed (4 tests) | native-r3 in progress; earlier fixed-program evidence only |
| `component-checkbox` | `checkbox` | docs, playground, states, behaviors, edge-cases | all applicable source sections | StoryState / existing Content composition | Check364 CPU / strict passed | native-r3 in progress; earlier fixed-program evidence only |
| `component-radio-group` | `radio-group` | docs, playground, states, behaviors | full source composition | StoryState / existing Content composition | Check364 CPU / strict passed | native-r3 in progress; earlier fixed-program evidence only |
| `component-switch` | `switch` | docs, playground, states, behaviors | full source composition | StoryState / existing Content composition | Check364 CPU / strict passed | native-r3 in progress; earlier fixed-program evidence only |
| `component-segmented-control` | `segmented-control` | docs, playground, states, behaviors | all applicable sections | original model / formal Content / shared original matrix painter | Check367 CPU / strict passed | native-r3 in progress; earlier fixed-program evidence only |
| `component-field` | `field-meta` | docs, playground, states, behaviors | complete source candidate | original models and applicable fixtures / receiving Host | Check377 pattern checks passed (4 shared tests); Check380 route construction passed | native-r3 in progress; earlier fixed-program evidence only |
| `component-select` | `select` | docs, playground, states, behaviors | all applicable source sections | original StoryState plus shared original consumer Content factories | Check365 chapters; Check366/367 consumer CPU / strict passed | native-r3 in progress; earlier fixed-program evidence only |
| `component-number-field` | `number-field` | docs, playground, states, behaviors, variants | all applicable source sections | original StoryState / production Content or original stepper View; shared original matrix | Check376 real actions/bounds passed (1 shared test); original matrices Check370 passed | native-r3 in progress; earlier fixed-program evidence only |
| `component-tabs` | `tabs` | docs, playground, states, behaviors | complete source candidate | original Tabs selection / formal Content; five original matrix cases plus five independent status consumers | Check377 live consumers passed (1 shared test); Check378 original retained/status consumer tests passed (1 each) | native-r3 in progress; earlier fixed-program evidence only |
| `component-breadcrumb` | `breadcrumb` | docs, playground, states, behaviors | all applicable source sections | original StoryState / production Content or original stepper View; shared original matrix | Check376 real actions/bounds passed (1 shared test); original matrices Check370 passed | native-r3 in progress; earlier fixed-program evidence only |
| `component-command-palette` | `command-palette` | docs, playground, states, behaviors | complete source candidate | original models and applicable fixtures / receiving Host | Check378 query/chapter/reopen contract (1) and original consumer target (4) passed | native-r3 in progress; earlier fixed-program evidence only |
| `component-list` | `list` | docs, playground, states, behaviors | complete source candidate | original models and applicable fixtures / receiving Host | Check376 collections passed (3); Check377 tree/table chapter retention passed (1 shared test) | native-r3 in progress; earlier fixed-program evidence only |
| `component-tree` | `tree` | docs, playground, states, behaviors | complete source candidate | original models and applicable fixtures / receiving Host | Check376 collections passed (3); Check377 tree/table chapter retention passed (1 shared test) | native-r3 in progress; earlier fixed-program evidence only |
| `component-data-table` | `data-table` | docs, playground, states, behaviors | complete source candidate | original models and applicable fixtures / receiving Host | Check376 collections passed (3); Check377 tree/table chapter retention passed (1 shared test) | native-r3 in progress; earlier fixed-program evidence only |
| `component-chart` | `chart` | docs, playground, states, behaviors | complete source candidate | original models and applicable fixtures / receiving Host | Check377 actual viewport/keyboard test passed (1) | native-r3 in progress; earlier fixed-program evidence only |
| `component-menu` | `menu` | docs, playground, states, behaviors | complete source candidate | original models and applicable fixtures / receiving Host | Check377 feedback check (1), Check378 real actions (1) and overlay (1) passed; shared Scroll regressions passed (2) | native-r3 in progress; earlier fixed-program evidence only |
| `component-tooltip` | `tooltip` | docs, playground, states, behaviors | complete source candidate | original models and applicable fixtures / receiving Host | Check377 feedback check (1), Check378 real actions (1) and overlay (1) passed; shared Scroll regressions passed (2) | native-r3 in progress; earlier fixed-program evidence only |
| `component-popover` | `popover` | docs, playground, states, behaviors | complete source candidate | original models and applicable fixtures / receiving Host | Check377 feedback check (1), Check378 real actions (1) and overlay (1) passed; shared Scroll regressions passed (2) | native-r3 in progress; earlier fixed-program evidence only |
| `component-dialog` | `dialog` | docs, playground, states, behaviors | complete source candidate | original models and applicable fixtures / receiving Host | Check379 real modal and visible LOG regression passed (1) | native-r3 in progress; earlier fixed-program evidence only |
| `component-toast` | `toast` | docs, playground, states, behaviors, variants | complete source candidate | original models and applicable fixtures / receiving Host | Check377 feedback check (1), Check378 real actions (1) and overlay (1) passed; shared Scroll regressions passed (2) | native-r3 in progress; earlier fixed-program evidence only |
| `component-banner` | `banner` | docs, playground, states, behaviors, variants | complete source candidate | original models and applicable fixtures / receiving Host | Check377 feedback check (1), Check378 real actions (1) and overlay (1) passed; shared Scroll regressions passed (2) | native-r3 in progress; earlier fixed-program evidence only |
| `component-status-badge` | `status-badge` | docs, playground, variants | full source composition | StoryState / existing Content composition | Check364 CPU / strict passed | native-r3 in progress; earlier fixed-program evidence only |
| `component-progress` | `progress` | docs, playground, behaviors, variants | full source composition | StoryState / existing Content composition | Check364 CPU / strict passed | native-r3 in progress; earlier fixed-program evidence only |
| `component-empty-state` | `empty-state` | docs, playground, states, variants | complete source candidate | original models and applicable fixtures / receiving Host | Check377 feedback check (1), Check378 real actions (1) and overlay (1) passed; shared Scroll regressions passed (2) | native-r3 in progress; earlier fixed-program evidence only |
| `component-skeleton` | `skeleton` | docs, playground, behaviors | full source composition | StoryState / existing Content composition | Check364 CPU / strict passed | native-r3 in progress; earlier fixed-program evidence only |
| `component-markdown` | `markdown` | docs, playground | full source composition | StoryState / existing Content composition | Check364 CPU / strict passed | native-r3 in progress; earlier fixed-program evidence only |
| `component-code-block` | `code-block` | docs, playground | full source composition | StoryState / existing Content composition | Check364 CPU / strict passed | native-r3 in progress; earlier fixed-program evidence only |
| `component-diff-view` | `diff-view` | docs, playground | full source composition | StoryState / existing Content composition | Check364 CPU / strict passed | native-r3 in progress; earlier fixed-program evidence only |
| `pattern-form` | `form` | docs, playground, behaviors, edge-cases | complete source candidate | original models and applicable fixtures / receiving Host | Check377 pattern checks passed (4 shared tests); Check380 route construction passed | native-r3 in progress; earlier fixed-program evidence only |
| `pattern-filter` | `filter` | docs, playground, behaviors | complete source candidate | original models and applicable fixtures / receiving Host | Check377 pattern checks passed (4 shared tests); Check380 route construction passed | native-r3 in progress; earlier fixed-program evidence only |
| `pattern-master` | `master` | docs, playground, behaviors | complete source candidate | original models and applicable fixtures / receiving Host | Check377 pattern checks passed (4 shared tests); Check380 route construction passed | native-r3 in progress; earlier fixed-program evidence only |
| `pattern-async` | `async` | docs, playground, behaviors, edge-cases | complete source candidate | original models and applicable fixtures / receiving Host | Check377 pattern checks passed (4 shared tests); Check380 route construction passed | native-r3 in progress; earlier fixed-program evidence only |
| `pattern-danger` | `danger` | docs, playground, behaviors | complete source candidate | original models and applicable fixtures / receiving Host | Check379 real finite dialog/confirmation diagnostic passed (1) | native-r3 in progress; earlier fixed-program evidence only |
| `example-settings` | `settings` | docs, playground, behaviors, edge-cases | complete source candidate | original models and applicable fixtures / receiving Host | Check377 pattern checks passed (4 shared tests); Check380 route construction passed | native-r3 in progress; earlier fixed-program evidence only |
| `lab-calibration` | `calibration` | docs, playground | all original calibration/source chapters | original StaticText + measurement-only resources; Host-registry visible shaping | Check370 font-registry (1), original painter (7) and scoped GPU (1) passed | native-r3 in progress; earlier fixed-program evidence only |
| `lab-geometry` | `geometry` | docs, playground | complete source candidate | original raster alignment/alpha reference with Host text; GPU calibration not claimed | Check370 original geometry test passed (1); Check380 route construction passed | native-r3 in progress; earlier fixed-program evidence only |
| `lab-consistency` | `consistency` | docs, playground | complete source candidate | original eight group actions and forty state illustrations; cached virtual child routes | Check370 original groups/state passed; Check371 real navigation/history passed (1) | native-r3 in progress; earlier fixed-program evidence only |
| `lab-matrix` | `matrix` | docs, playground | complete source candidate | original BrandPreview model; one shared AppWindow palette source | Check376 Brand tests passed (2); Check377 remaining action/focus tests passed (1 each) | native-r3 in progress; earlier fixed-program evidence only |
| `lab-a11y` | `a11y` | docs, playground | all original reference copy | original explanatory text only; not VO or performance acceptance | Check380 route construction passed; original explanatory copy only | native-r3 in progress; earlier fixed-program evidence only |
| `lab-stress` | `stress` | docs, playground | all original reference copy | original explanatory text only; not VO or performance acceptance | Check380 route construction passed; original explanatory copy only | native-r3 in progress; earlier fixed-program evidence only |
| `lab-internal-text` | `internal-text` | docs, playground | all applicable source sections | original TextDemo surfaces in finite live viewport; same Host native input/scroll | Check370 finite viewport, native-owner/edit and reading contracts passed (3 tests) | native-r3 in progress; earlier fixed-program evidence only |
| `component-scene3d` | `scene3d` | docs, playground | full source composition | Preview3D; shared GPU resource contract required | Check364 CPU / strict passed | native-r3 in progress; earlier fixed-program evidence only |
| `apphost` | `apphost` | docs, playground | complete source candidate | original action/model via formal Content; launcher uses existing AppWindow open request | Check375 window request (1); Check378 launcher queue (1) passed | native-r3 in progress; earlier fixed-program evidence only |

## Resource contract and remaining acceptance

The narrow two-phase ContainerContent resource contract is implemented. Scene3D
and Motion have scoped renderer/clock GPU evidence from Check368; Icons and
Calibration have scoped GPU evidence from Check370. Those checks do not replace
native interaction, asset reachability or visual acceptance on the Check381 program.

Current receipt sources are the coordinator's
`work/gallery-migration-check{368,370,371,374,375,376,377,378,379,380,381}/partial-results.json`
and each run's linked stdout/stderr. Check374 secondary-pointer dispatch passed
one CPU regression; Check375 one-shot window opening passed one. Check378 consumer
counts are retained/status 1 each, slider 4, command 4 (not four tests total).
Check378 AppHost strict and Check380 Gallery strict passed. Failed earlier runs
are retained as history, including Split Budget/Effects failures before Check380.

## Original implementation sequence (historical)

1. Complete original Field/Area/Combo/Select/Search sections in
   `workbench/apphost_story.rs`, retaining the original models and registered
   specimen identities. Add section selection to the retained page state;
   test Back/Forward restores both live edits and selected section. Replace old
   Search/Button compensations only once their consuming paths have migrated.
2. Migrate ordinary component/collection/feedback pages with the existing
   Content implementations and StoryState fixture data. Port actual supported
   sections and their controls, including disabled/passive chooser reachability;
   do not map every route to a generic specimen. Keep live model/action log
   ownership singular, with one AppHost clock and ContentHost input/AX tree.
3. Migrate patterns and references through their real composition contracts;
   preserve draft/reset, nested scroll, reference selection and calibration.
   Integrate the separately owned resource contract for 3D and resource-backed
   references. Verify local override and bundled asset paths outside the checkout.
4. Only after every row reaches equivalent behavior, switch default entry while
   keeping an explicit diagnostic legacy entry. Root runs the integrated build,
   CPU regressions and granted native acceptance on the frozen source identity.

## Verification and acceptance handoff

The table records current scoped CPU evidence and 70 source candidates. Its initial
Checkbox-only inventory was subsequently extended in the batches below; historical
candidate descriptions are not the current test status. The default entry, public
Release and Dock delivery remain pending coordinator acceptance. No generated
metadata or version was changed by this documentation update.

Affected runtime verification after implementation must include the existing
`apphost_original_pages_preserve_edits_and_specimen_selection_on_history` and
`apphost_workbench_history_retains_each_visit_and_live_input` tests, plus real
section-switch/edit-restoration and every migrated route's actual component
presence/actions. Run through root's registered serial build wrapper using
`cargo +1.95.0 test --locked --offline -p tekes-ui-gallery apphost_` as a scoped
filter (not evidence that unselected tests passed). Native acceptance includes
editable IME, popup dismissal, narrow Focus clipping, scroll ownership,
activity/animation and real 3D orbit/zoom/resource resizing. VoiceOver and deferred
performance remain deferred. No route or unsupported field may be removed to
make coverage pass.

## Historical Checkbox candidate

`workbench/apphost_basic.rs` owns the original `StoryState` fixture and one
ContentHost. Playground and Edge cases use the production `CheckboxContent`;
property commands preserve its mount, while section changes remount the view
from the retained original model. The legacy matrix and new passive reference
nodes share the same 16 state definitions. Passive nodes delegate measure,
arrange and paint to the actual Checkbox adapter, expose no input owner, and
remain reachable through the Documents/Inspector chooser. No legacy Scene is
flattened and no second native tree or clock is created. CPU regression covers
Mixed activation exactly once, disabled rejection without remount, retiring the
interactive instance on matrix entry, section/history state restoration and
Reset preserving the chosen section. Native matrix appearance and narrow-label
wrapping remain unverified. At that stage, no completion claim was made for the other 69 routes.

## Ordinary-page batch and resource boundary

The shared basic-story container now also hosts Button (all six sections), Stack,
Grid and Separator (Docs/Playground/Variants). Static Button references use the
original sized renderer; live buttons use the existing Button Content. Layout
references measure and arrange original models in real Content nodes. The three
Overview pages reuse original copy and component registry; index actions enter
the same navigation/history service. Added regressions cover Button loading
rejection/once-only activation across sections and one index-to-layout navigation.
No new runtime acceptance is claimed before the unified root check.

Root's shared `ContainerRenderContext` provides the existing renderer, root frame
and clip, backing scale, same-frame motion and palette. Optional resource hooks
run `prepare_resources`, exactly one normal Host render, then `finish_resources`
before whole-window submission. The second hook can consume actual child bounds
recorded during that frame's arrange; it does not infer them from a parent frame
or perform another layout. Default hooks do nothing. Effective window/container
activity is projected into the visited Host. Gallery must deactivate the departing
cached route through its existing host borrow, avoiding a recursive RefCell borrow.
Resources and native/clock ownership remain singular. The shared API is pending
its combined checks and resource-backed consumer integration.

## Selection and Scene3D candidate

Switch, Slider and RadioGroup now reuse the original models in their existing
Content adapters for Docs/Playground/States/Behaviors. Original extended matrix
state definitions and fixture painting are shared with the legacy reference.
Gallery reads changed model values after the formal Host visit, covering Slider
drags without replaying toggles. Passive matrices borrow real text/resources
with the original frozen motion policy, and do not request animation frames.

Scene3D's root-owned adapter is connected through both resource hooks and the
normal ContentHost. Its specimen has no registered component document ID;
Documents shows the real page guidance while Inspector can observe the instance.
When navigation changes a cached route, Focus deactivates the departing Host
inside its existing borrow. CPU fixtures now run through real LocalContainer
activity projection rather than manually setting Host activity.

New affected checks: `apphost_`, `scene3d_`,
`state_matrix_reserves_wrapped_text_rows_and_focus_gutter_at_large_type`, the
previous original matrix layout/clock filters, and Gallery strict. Root owns the
scoped ignored Scene3D GPU run. Native acceptance remains separate and pending.

## Display/reading batch and native fidelity repair

The next seven routes are Markdown, CodeBlock, DiffView, Surface, Skeleton,
StatusBadge and Progress. Readers retain their original models and View layout
algorithms. A metric-only borrowed text adapter lets those existing Views return
natural document height; their real paint runs in the current Host with its
palette and text services. No legacy scene is captured or replayed. Surface uses
PanelContent with an independent nested Button; progress retains the original
bounded 10% reducer and passive ProgressContent. Loading stripes and all status
variants retain their original painters.

Check361 CU exposed two migration fidelity issues in the preceding program:
BasicStory section commands formed a vertical column, and titles lost hierarchy.
The new source uses one horizontal Stack inside the existing horizontal
ScrollContent. A private parent derives finite row height from the actual button
metrics; scrolling and focus reveal stay with ScrollContent. The shared
`story_heading` uses the existing proposal UiHeading role. Legacy window chrome
used BodyEmphasized, so this is not a claim of pixel identity with that old role.
No token values or AppHost window geometry changed. New CPU regressions cover
280/740pt widths at 1.5 text scale, reaching Reset, nested action ownership,
progress clamping, and original reader/footer layout at narrow width. All await
the next root check; native evidence remains attached to Check361's frozen binary.


Coverage correction during the Tabs/Disclosure dependency audit: the original
Slider States route appends `slider_content::build` under test/composition-review
(extended.rs, following its ordinary matrix). That independent range/editor/action
subtree was not in the earlier migration. Prior matrix/CPU passes remain valid for
what they exercised, but do not establish complete Slider story coverage. The gap
was tracked explicitly as partial. The current source now mounts that original
consumer through a shared factory in one Host; verification of this addition is
still pending, and the earlier broad coverage claim remains corrected history.

## Quality reference navigation candidate

Consistency retains its eight original group destinations as private virtual child
routes, outside the 70-entry directory. Each retains the original ShowcaseState
and painter in one cached Host. Back/Forward restores that Host and specimen
selection; directory selection stays on Consistency. Documents uses the original
group component manuals; Inspector labels these static reference illustrations,
not live-control acceptance. Geometry reuses original raster calibration geometry
and Host text. The new routing regression
`apphost_quality_navigation_retains_state_and_consistency_directory` and owner
quality CPU test are pending Check369; no native or pixel result is asserted.

## Tabs / Disclosure / independent Slider candidate

Tabs and Disclosure now use their formal Content owners, retain original story
selection/expanded state across chapters, and keep controlled Disclosure intent
confirmation in the receiving Gallery Host. Their original passive matrix
painters are shared with the legacy entry. The original five conditional status
cases per page use the same private Header/Panel/Feedback/editor subtree as the
independent consumer Scenario, with distinct IDs and no nested Host. Disclosure
icons have one painter: embedded Content for Gallery, original external painter
for Scenario. Slider adds the original range/editor/action fixture at value 60
(the original Increment then Right initialization), with a separate retained draft.

`apphost_tabs_disclosure_and_slider_keep_original_live_consumers` covers real AX
selection/expansion, all independent header actions, range change and untouched
draft, chapter return, and Reset. It uses a CPU resource service, not pixel or
native evidence. Stepper's missing rectangular input declaration is repaired;
its existing bounded AX regression still awaits the next check. No Cargo/GUI run
was performed by this source owner.

## Integrated owner candidates awaiting the first combined check

Collections (four), Patterns (six), Field metadata, and Feedback (seven) now
consume their owning adapters through the shared routing. Collection typed actions
are drained and applied before sorting replaces the table snapshot; the entire
live body is retained across Docs/States/Behaviors, with Reset rebuilding it.
The integration regression checks tree collapse and table sorting across those
transitions. Field metadata States retain original emphasized FieldView metadata
and passive Studio input fixtures; the live page shares the original pattern
transport. Pattern/feedback owner tests remain pending, not established by wiring.

CommandPalette mounts the formal Content with the original query editor, nine
visual cases and three original conditional lookup consumers. The consumer
Scenario and Gallery share one factory. The new real Scroll Host regression
checks trigger AX, query editing and chapter retention; the shared popup reveal
contract is under the coordinator's review. No consumer bypass is used.

Check371 passed the shared Breadcrumb Scroll regression, Quality history, corrected
Icon fixture and strict checks for that earlier source. Stepper was the remaining
red case then; its geometry declaration and all newer integrations await checking.
SplitView and IconButton remain explicitly unimplemented in AppHost in this source.

Brand matrix now shares one `Rc<Cell<Palette>>` with the cached page factory and
AppWindow.palette_source. Apply updates that source; Back/Forward retains the
same candidate/model. The window remains authoritative for the selected theme.
Brand owner tests and the new AppHost source contract remain pending this check.

Final source additions: IconButton uses the formal named Button with canonical
plus artwork, preserving the original seven state fixtures and once-only action.
SplitView uses formal SplitViewContent with original pane configuration, finite
viewport, constrained pane choices, divider owner and three original ratio
variants. AppHost launcher queues one request per real Button activation; the
existing window loop opens the original demonstration with the current native
theme and shared SettingsOwner. Repaint/navigation never enqueue a window.
The legacy default entry remains unchanged pending acceptance.

CommandPalette reopening intentionally clears the query in the original model.
The corrected regression checks editor text and external baseline after chapter
return *before* reopening, then checks that real reopening starts a blank lookup.
No production behavior was changed to satisfy the erroneous prior expectation.

## Same-Host dialog composition — final source correction

Feedback Dialog and Danger Zone reuse the Gallery-private `dialog_viewport`
composition: the scrolling content and formal Dialog are siblings arranged in the
same finite allocated viewport. A Dialog is not measured as a vertically flowing
row inside an unbounded ScrollArea. The existing Host owns modal input, focus and
retirement. Check379 verifies finite visible modal geometry, feedback log visibility
and closing/removing the old dialog on section changes. The earlier oversized
modal failure is retained in Check378; no public Dialog contract was weakened.
