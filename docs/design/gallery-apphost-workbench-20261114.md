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

The first Documents reader projects the supported Markdown subset through
RichDocument into selectable read-only content. Rich heading/code styling and
link opening remain unimplemented; this candidate does not claim those features.
