# Gallery as an AppHost consumer

## Current verification status — Check381 candidate

All 70 registry routes have source adapters. Check380 passed two focused tests:
the real Split/Icon actions and construction of all registered routes; Gallery
all-targets strict also passed. Construction does not prove every route's behavior.
Check381 release-mode candidate build succeeded, with native-r3 (signature prefix
`d1120c4`) in progress. This is not a public Release or Dock delivery; legacy remains
the default entry. Native-r2 retains only its fixed Check368 33-route program's
limited evidence. Physical IME, VoiceOver and performance are not newly accepted.

Scoped receipts now supersede the earlier source-only statements below: Check370/371
cover Text/Icon/Calibration/Quality; Check374 secondary pointer dispatch (1 test),
Check375 window requests (1), Check376 ordinary actions (1), collections (3) and
Brand (2). Check377 passed Tabs/Disclosure/Slider (1), Chart (1), collection history
(1), the remaining Brand action/focus checks (1 each) and four pattern checks.
Check378 passed Command (1), launcher (1), original matrices (2), shared Scroll
regressions (2), feedback actions/overlay (1 each), and original consumer checks
(retained/status 1 each; slider/command 4 each). Check379 passed the remaining
Danger and finite Dialog checks (1 each). AppHost strict passed Check378; Gallery
strict passed Check380. Earlier failures remain historical evidence, not passes.
See [route coverage](gallery-apphost-coverage-20261116.md) for current per-route
scope; receipts live in the coordinator's `work/gallery-migration-checkNNN/partial-results.json`
with stdout/stderr links. The dated sections below retain their original batch
context and do not describe the current verification status unless stated.

User authorization: 2026-11-14. Implementation starts after the read-only
assessment of v0.3.28, source `5bc621ba5240ac001c89681b6589c52a170e3bbb`.
The evidence-only successor `59bc128` does not change runtime behavior.

## Product contract

Gallery uses the production AppHost window. Nav retains the registered page
directory, filtering and history; Focus hosts the selected page's real component
examples. Others registers exactly two single-instance destinations: Documents
and Inspector. Documents is guidance for the selected component type; Inspector
shows the selected live specimen instance. Existing AppHost window geometry,
toolbar ownership, Settings, splitters and input contracts remain authoritative.
The existing AppHost demonstration remains a demonstration, not the new shell.

The first release of Inspector is read-only. No property editor, theme editor,
runtime plugin discovery or in-app Markdown authoring is included.

## Ownership and selection

AppHost owns native window services, lifecycle, container layout and cross-scope
input. Components own state, rendering and formal ContentHost contracts.
Gallery owns registered routes, specimen fixtures, selection and document IDs.
Documents and Inspector do not introduce a second input or animation owner.

Selection has three stable identities: page route, specimen key and component
documentation ID. Runtime ComponentId is scoped to its mounted host and is not
a persistent documentation key. Same-type specimens share documentation while
retaining distinct inspected values. Business selection inside a List or Combo
is not automatically the Gallery's inspected specimen selection.

A single-component page selects its component by default. Multi-component pages
retain a still-valid selection, otherwise show page guidance and an explicit
specimen chooser. Pointer/keyboard interaction may select a registered specimen
without swallowing input, changing focus or replaying an activation. Hover alone
does not change document selection. The chooser also reaches disabled/passive
examples. Unknown or retired IDs do not inspect a different live instance.
Back/Forward restores a page and its valid specimen selection; document-tab
selection does not create spurious page history entries.

## Documents

Reuse `design/gallery/workbench.json` routes/component IDs and existing public
API bindings. Maintain one Markdown source per component, with page guidance for
non-component routes. Reuse existing component documentation where present.
Required content: purpose, real public entry points, minimal example, important
properties/events/state reads, composition/layout/scroll ownership, input and
focus behavior, and honest limits. Do not infer completeness from an API type
inventory: it does not enumerate every method contract.

Bundle document sources with the application so offline content matches the
release. Reuse the existing selectable rich-document and scrolling components.
The supported Markdown subset is explicit; no browser or second rendering
framework is required. Example snippets should share or compile against actual
consumer paths. A missing document or stale registry binding is reported rather
than silently displaying another component's manual.

## Inspector

Expose a small shared read-only snapshot contract for live component identity,
supported public properties, interaction state and measured layout. A snapshot
is observational: no setter, event dispatch, focus change or animation advance.
Gallery renders this contract using ordinary components; AppHost does not contain
per-component inspection logic. A component reports supported fields explicitly;
unavailable fields remain unavailable, never fabricated from a component name.

Token entries distinguish semantic name, known alias/source, effective resolved
value, current theme/state and explicit override when observable. Values must
come from the live resolution/rendering inputs. A list of tokens that might be
used by the component is documentation, not evidence of actual use. Declared
sizes/padding are distinct from measured bounds and clipping. Reading Inspector
must not turn inspection-panel focus into the specimen's observed focus.

## Migration boundaries

The current Gallery Host and Workbench are not an AppHost Content node. Reuse
the already mounted ContentHost stories and their models; migrate their outer
composition. For legacy painting, adapt the actual page contract deliberately:
Focus allocation, clipping, ScrollArea ownership, text resources, input/IME,
semantics, window activity and clock must have one owner. Do not nest the old
window, sidebar and event loop, flatten scenes as a fake component, or maintain
two competing native text/AX trees. Specialized image/3D and motion stories need
their real resource/lifecycle path preserved.

Ship in independently reviewable batches:

1. AppHost assembly and selection: one existing ContentHost component page plus
   one multi-component page, real Nav and two Others destinations. Keep the old
   default entry until migration coverage is complete.
2. Documents: offline catalogue, component manuals and actual selection binding.
3. Inspector: shared observational contract, live properties/layout/state/token
   bindings and readonly panel; document concrete coverage per component family.
4. Remaining registered pages: migrate by actual adaptation boundary, retain
   every route and applicable story, then switch the default entry.
5. Integrated acceptance and ordinary compatible release/Dock adoption after
   the required checks pass. Partial prototypes do not close the overall issue.

## Verification

Reuse unchanged component tests and historical evidence with original identities.
New verification covers selection isolation, stable IDs, real documentation
binding, observational snapshots, route/state restoration and changed host paths.
Integration covers representative editable text, nested scrolling, popup close,
animation/activity, AX scope and resource-backed stories under the new host.
Native visual/input evidence is separate from CPU/static results; GUI uses the
unique CU and a granted batch window. No VoiceOver or deferred performance work
is restored by this upgrade. Required CI runs for a stable compatible candidate,
not each small document or adapter change. WIP and historical failures survive.

## Current execution

Design approved through the user's implementation instruction. Child Issues and
the coordinator ledger track actual implementation, checks, gaps and delivery.
No new-host runtime, native or release pass is asserted by this design document.

### First candidate inspection boundary

The shared `inspection` Cargo feature enables observation; ordinary Controls
builds retain no observation cache or Palette tracing. Gallery opts in explicitly.
Only the selected mount records its normal paint. Property hooks are initially
provided for Button, TextFieldContent, SearchFieldContent and CheckboxContent.
Snapshot layout is unavailable after invalidation until the next valid render.
Cached paint is explicitly historical: actual Palette lookup values and emitted
quads are distinct; aliases, override provenance, glyph/image pixels and GPU
composition are not reconstructed. Disabled recording clears the cache.

The grouped Inspector consumer uses shared Stack and Grid with TextContent labels
and TextFieldContent::read_only_content values inside the existing ScrollContent. Identity, Properties, Layout, Observed tokens
and Emitted primitives have aligned label/value rows with wrapping values.
Inspector publishes selection, chooser and derived snapshot values only in
ContainerContent::prepare_for_render, called by LocalContainer immediately before
its normal Host render. Inspector with_host is a pure host visit: input and
native/AX queries use the last painted values and never invalidate their own
layout by publishing first. The short interval between Focus cancellation and
the next normal draw retains that painted version, not a claim that the live
source snapshot is still valid. If the source still lacks geometry at preparation,
the next draw explicitly displays Unavailable.
Within the same selected mount, groups with unchanged row-label sequences retain
their value owners through controlled publication. Only changed values reset
selection; focus and mount remain. Layout has fixed rows across invalidation.
Identity focus changes therefore do not retire an unchanged Page/Specimen value.
Groups whose row structure changes are replaced, and a selection or mount change
replaces all groups; the scroll owner is retained. Each value supports independent
selection/copy, not continuous selection across cells. Dynamic token/primitive
row-structure changes may reset selection; they have no stronger continuity promise.
No stale geometry is relabelled as current or used to suppress updates.
Value readers have no field background/border, grow with wrapped text and disable
internal scrolling; the outer ScrollContent remains the sole scroll owner.
No property editor, local paint
background or new property framework is introduced. Property hook coverage stays
at the four component types above. Missing snapshots, unsupported properties,
invalidated geometry and unrecorded paint remain explicit. Historical lookup
calls, resolved colors and emitted quads remain separate from current values and
from final pixels; unknown alias/override provenance is not inferred.
This source candidate still requires affected CPU checks and native visual/scroll
acceptance; the earlier plain-text Inspector evidence does not certify its layout.

The first accepted native slice projected Markdown into selectable plain text.
The subsequent Documents candidate uses the existing RichContent for heading/code
layout and continuous selection, inside the original ScrollContent. Local bundled
manual links are consumed after input through RichContent link actions. External
or absent targets report that they were not opened; no system URL opener is added.
Command-Enter uses the existing RichContent keyboard contract through shared
AppHost routing; native reachability is recorded separately from CPU checks.

A same-document specimen change retains the reader and selection. An explicit
specimen selection change or explicit chooser activation (Enter or accessibility Press)
after browsing another manual returns to that component manual. An ordinary click
on the already-selected List row does not create a selection-change event.
A document switch replaces the reader through Host lifecycle; keyboard focus
transfer waits for the new mount to have actual text layout, and is cancelled
if the user focuses another control. Supplemental Ticking Number documentation
has an independent supplemental identity, not a new component or Gallery route.
Check331 passed affected CPU checks; native-r4 observed rich typography, copy
and selection retention. Inline link labels were covered by the standalone Link
background painter after glyph painting. The shared RichContent repair limits
inline decoration to an underline; it does not change consumer palettes or text
layout. Link activation remains pending repair acceptance. The original four
Playground native-r3 acceptance keeps its prior program identity.

### Consumer-owned startup arguments

Gallery consumes its `--apphost` entry selector before invoking
`AppWindow::run_with_args`, passing argv[0] and the remaining arguments.
The ordinary `AppWindow::run` retains process-argument validation. Fixed-profile
worker/module admission and rejection of unknown trailing arguments are unchanged.
This prevents the observed pre-window rejection of Gallery's own selector; it
does not enable runtime module discovery or modify process-global argv.

## First native slice and manual catalogue — 2026-11-15

The opt-in AppHost candidate has passed bounded official-CU navigation, component
selection, Documents/Inspector switching and Back/Forward draft restoration.
Its seven bundled manuals and composition fixture retain their exact program
identity; they do not certify full story migration or rich document rendering.
The startup argument transport regression is fixed through AppWindow::run_with_args;
Gallery consumes only its own selector before passing remaining arguments.

The source catalogue now maps all 45 registered component IDs to offline Markdown
manuals. Catalogue coverage, referenced paths and paired fences were checked;
Check329 type-checked all 46 original snippets and compiled the complete bundle. The List
manual distinguishes the existing scrollable composition from the non-scrolling
group and keyed selection model, avoiding duplicate scroll ownership.

Original Field/Area/Combo/Select Playground migration is recorded below.
The original Gallery remains the default and Dock production remains v0.3.28.

## Original hosted Playground intake — 2026-11-15

Field, Area, Combo and Select now retain their original StoryState and component
Host. A Gallery-only root decorator adds the original reset, disabled/error and
applicable edge-case controls. AppHost visits that single Host; Gallery consumes
its control intents afterwards instead of replaying the legacy input/paint loop.
Field retains the explicitly labelled additional multi-instance fixture from the
first slice. Original States/Variants/Behaviors sections and other routes remain
unmigrated; this is a Playground intake, not complete four-route parity.

Check329 passes 15 affected CPU regressions, Gallery strict and release build.
The 45 offline manuals contain 46 Rust snippets; all type-check against the
existing Check328 Controls/Tokens libraries. This does not prove snippet runtime
behavior. Existing native-r2 identity and results remain limited to the first
slice. Native-r3 subsequently passed the bounded four-Playground input/reset/
options/history and document-linkage checks; other story sections remain open.

## Navigation and registered search — 2026-11-15 candidate

The AppHost candidate reuses the original Gallery section headers, item names,
route identities and filtering. The fixed Search header and scrolling directory
share one ContentHost. Filtering replaces directory groups only, retaining the
Search mount, selection and native input owner; it does not add history visits.

Scrollable List owns its default `Spacing::Loose` content inset; an explicit
`content_padding` replaces that value. Internal `ListContent::new` groups remain
unpadded and list rows retain their existing Leading alignment. Nav's outer
container padding and root Stack gap are zero. A Stack header declares top/left/
right Loose padding and zero bottom padding via `into_content_with_padding`;
the List's own top inset separates its first group from Search. Stack padding
changes original child measurement/placement without adding a viewport or curve
capability, and existing Stack struct literals remain source-compatible. Empty
Search queries use the complete editor allocation; nonempty queries retain the
existing separate Clear action and spacing.

When a consumer allocates a minimum column width, it passes its content minimum
to the original `ScrollContent::outer_width_for_content`. ScrollContent owns the
same padding, focus-bleed and requested scrollbar calculation used by layout.
The existing master-detail story uses this measurement to keep its Document word
intact; it does not duplicate List inset values. The consumer supplies its known
vertical-overflow condition, while its column ratio and inter-column gap remain
composition policy.

`LocalContainer::search_entry(ComponentId)` declares the window's single local
Find destination. AppHost handles Cmd+F, resolves the container-scoped identity,
uses existing PresentationState operations to reveal hidden Nav/Others, lays out,
and admits focus through the original ContentHost before native text publication.
Unavailable, read-only or disabled targets do not fall back to another editor;
IME composition, modal/overlay input and ongoing navigation retain their gates.
No registration preserves ordinary shortcut handling. This adds no Gallery key
interceptor or second text event loop. Legacy horizontal assemblies retain their
existing visibility behavior; automatic reveal applies to the formal presentation
model used by this Gallery. Coordinated Check343/345/346 verify the registered
Find and stable input-owner paths; native-r10 verifies grouped navigation, wide
and narrow Search, Clear and hidden-Nav Cmd+F. Check348/349 verify List navigation,
minimum document width, horizontal Tabs, Search phase geometry and retained
historical rendering. Final palette appearance remains a separate native check;
these results do not certify physical IME, VoiceOver or performance.

## Remaining-route inventory and first full-section candidate — 2026-11-16

[The Issue #294 coverage table](gallery-apphost-coverage-20261116.md) tracks all
70 registered routes and their original supported sections. Navigation presence
and bundled manuals do not count as story migration. Checkbox is the first
candidate to compose every original section under AppHost, using the original
model, production Content adapter and shared passive state fixtures. Its Check356 CPU checks passed; native acceptance remains pending. The old default entry remains until the full
table is behaviorally equivalent. GPU-resource preparation for the 3D route
requires a separately owned shared host contract; the Gallery consumer does not
introduce a renderer or flatten a legacy scene to bypass that boundary.

The next ordinary-page source batch extends the same story container to Button,
Stack, Grid and Separator, plus the original Overview copy and component index.
The index routes through the same history service. The optional shared resource
boundary is now two-phase: prepare resources, normal Host render once, finish
resources before renderer submission. The finish phase consumes actual child
geometry from that render's arrange, so a resource consumer need not duplicate
Stack/Scroll layout or use stale child bounds. Context carries only the existing
renderer, root frame/clip, scale, motion and palette. Activity is projected by
LocalContainer onto each visited Host; Gallery deactivates a departing cached
page using the already borrowed Host. Resource pages still require integration
and acceptance; this API alone does not establish their coverage.

The selection-page batch retains original Switch, Slider and RadioGroup models
in their formal Content adapters, including complete original section sets and
shared static matrix definitions. Matrix text rendering borrows the window's
services with the original reduced-motion fixture policy; it cannot schedule a
second animation loop. The Scene3D consumer is now wired to both resource phases
and records actual arranged preview geometry. A cached departing page is stopped
within its existing Host borrow. Test fixtures use LocalContainer's real activity
projection. These new paths await combined CPU/GPU checks and native acceptance;
Check358 evidence applies only to the preceding ordinary-page source batch.

The subsequent display/reading batch adds the original Markdown/CodeBlock/Diff
Views, Surface's real nested action, Skeleton, status variants and bounded
Progress. Original View measurement uses a metric-only borrow of the receiving
text renderer, with normal painting performed by the actual Content owner.
Check361 native review also identified the vertically stacked section commands
and weak title hierarchy in that frozen program. The new shared row uses
horizontal Stack/Scroll ownership and measured height; `story_heading` uses the
existing UiHeading role without token changes. These source repairs and new pages
are separate from the frozen program's ongoing native evidence.

### Input chapters and reference/resource routing (2026-11-16, source candidate)

Field, Area and Search now compose all applicable original chapters. Search uses
that same StoryState root factory as Field/Area/Combo; the legacy caller passes
identity, so its native transport remains unchanged. Section navigation reuses
the previously verified horizontal SectionBar. Before leaving a live chapter,
the original input blurs while mounted; the Host removes and retains the whole
Content subtree, including the real editing model. Passive chapters use that
same Host. Returning to Playground or Edge reinserts the retained subtree;
Reset rebuilds the original fixture and preserves the selected chapter. No second
native or event owner is created. The passive text matrix uses the production
TextSurface layout/paint with original selection/preedit fixtures and the receiving
text service, without advertising those fixtures as editable native inputs.

Combo/Select additionally expose their original nine navigation-view fixtures;
conditional consumer refresh-status rows remain explicit coverage gaps. Six
foundation routes and Motion are wired to their owning adapters. Typography and
Iconography preserve their original reference text; component-icon remains a
separate unresolved route. Foundation and Motion selections use real page guidance
rather than invented registered component manuals. This candidate has formatting
and diff checks only; affected CPU/strict/native evidence is pending coordination.

### Consumer state fixtures and first navigation group (source candidate)

The five Combo/Select conditional refresh-status specimens now use small factories
in the existing independent consumer modules. Both their original Scenario::new
and Gallery instantiate the same private Query/Choice and Feedback components.
Explicit root/control/feedback IDs let Gallery put all specimens directly in its
one Host. Combo's deterministic initial Com/Enter selection uses the original
private query and menu reducer; validation remains independent of refresh status.
There is no nested Scenario Host or alternate event loop.

Link and SegmentedControl now use original StoryState models through their formal
Content. Model changes are read once after Host dispatch; link activation logs once
without replaying the action. Their original matrix painters and measurement are
shared by legacy and AppHost paths. Docs/Playground/States/Behaviors, Disabled,
Reset, and selection persistence are retained. The new same-Host consumer draft
isolation and navigation behavior regressions are written but not yet executed.

### Breadcrumb, Toolbar and NumberField admission (original source batch)

The 36-route source includes all original chapters of these three pages. Breadcrumb
uses its original model and formal Content, and preserves its ancestor-only action
payload. Toolbar uses the formal ToolbarControl with original Create/Rename/Remove
commands and its built-in keyboard/overflow owner. NumberField retains the original
non-editable NumberField and NumberFieldView through a private Component; substituting
the newer editable NumberFieldContent would change the original story contract.
The fixed 1–32 stepper supplies its original pointer/key reducer, two semantic action
parts and no text-input owner. Its value remains in StoryState across chapters.
Their six/five/seven passive matrices share the original case data and painters.

The admitted source is not the native-r2 program: r2 runs the frozen Check368
33-route package. Their real actions/bounds CPU test subsequently passed Check376;
original matrix tests passed Check370. Current native-r3 acceptance remains in progress.

### Calibration resource boundary (coordinator-approved contract)

Calibration may retain its original measurement-only TextEngine/offscreen atlas and
use the same Renderer for readback. This is an offscreen measurement service, not
a second window text/input/accessibility owner. Visible calibration text must be
reshaped by the receiving Host font registry at the same raw 14 pt size and crop;
a measured Paragraph must never cross registries with different FontKey tables.
The existing prepare_resources hook is sufficient; no new GPU framework is needed.
The Host weighted final-size shaping seam preserves original metadata/header font
weight. Its default implementation explicitly rejects unsupported non-400 weight;
it must not silently render requested weighted text as normal. Existing verified
Motion shaping behavior remains unchanged. Check370 subsequently passed the font-registry,
original painter and scoped GPU checks; current native acceptance remains pending.

### Consistency child routes and geometry reference

The original eight Consistency group actions open cached private
`components-{category}` routes using the original ShowcaseState and painters.
They are not additional registry entries: directory selection remains Consistency,
and the existing visit history retains their Host and inspection selection.
Documents resolves the original group component manuals. These forty state
illustrations remain static references, not substitutes for migrated live controls.
Geometry consumes the original raster alignment/alpha drawing with the receiving
Host text service; mounting this reference does not establish GPU calibration.

### Navigation stories and independent conditional consumers

Tabs uses formal TabsContent; Disclosure confirms the formal controlled intent
against its original StoryState only after Host action delivery. Original passive
state painters are shared with the legacy Gallery. Conditional content-status and
Slider examples expose their original Content factory so Gallery mounts their
private model, feedback and draft under its existing Host. Scenario/build use the
same factories; no nested Host or separate text/input/event owner is introduced.
The conditional Disclosure header assigns glyph painting to either embedded
Content or its original external Scenario painter, never both.

### Collection, pattern and feedback integration

The shared router delegates to their original domain adapters. Collection action
records are consumed before snapshot-changing sort projection; section navigation
retains the complete live body so Tree expansion and table order remain in their
formal Content owners. Reset alone rebuilds deterministic state. Patterns reuse
original submission/revision/delivery state and mount their original editor in
the receiving Host; Field metadata reuses the emphasized metadata view for passive
States. CommandPalette consumes the formal trigger/query/overlay owner and shares
its independent lookup consumer factory with the original Scenario. Its popup
query reveal geometry must satisfy the Host contract; Gallery does not bypass it.

### Final action, split and launcher consumers

The icon action uses one formal Button and a passive canonical plus IconContent;
the legacy fallback renderer remains intact. SplitView owns its original formal
divider and finite panes under the receiving Host; the outer viewport contributes
only geometry and conditional pane-choice layout. The three ratio illustrations
remain passive and do not change the live ratio.

The AppHost route is a launcher page. Real button actions enqueue window requests;
`take_window_open_request(theme)` consumes each once and delegates to the original
AppHost demonstration's `open_request`, sharing the Gallery SettingsOwner. AppHost
supplies the current native theme and opens via its existing event loop. Merely
visiting or rendering the launcher cannot open another window. Default Gallery
selection is unchanged until the combined source and native acceptance complete.

### Final default-entry candidate (#294, 2026-11-16)

Ordinary no-argument launch selects AppHost after the complete route migration.
`--apphost` remains accepted for existing commands; `--legacy` retains the original
Gallery diagnostic renderer. `--snapshot` and explicit legacy selftest/performance
environment entry points retain their original implementation and evidence scope.
The source candidate version is 0.3.31; this does not replace the installed v0.3.30
or assert completion of native acceptance, required CI, Release or Dock delivery.

### English consumer manuals and shared source coloring (#308, 2026-11-16)

All 45 registered component manuals and the supplemental Ticking Number manual use English consumer guidance with supported Markdown headings, paragraphs, bullets, simple links and fenced examples. Original Rust examples are preserved. Validation receipts and delivery bookkeeping belong to design/Issue history rather than reader prose; the prior manuals are archived in the coordination artifact documents/original-manuals.json.

Documents retains the registry → build bundle → ScrollContent(RichContent(RichDocument::from_markdown)) path. RichContent supplies syntax coloring through existing shared tokens and retained glyph geometry. Gallery owns neither a lexer nor a second text layout. Unknown languages remain plain. This change does not add Markdown projection editing (#132). Native observation and CI remain separate acceptance evidence.
