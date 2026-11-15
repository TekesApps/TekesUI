# Gallery as an AppHost consumer

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
