# Tabs controlled close

Issue: [#229](https://github.com/TekesApps/TekesUI/issues/229),
[#245](https://github.com/TekesApps/TekesUI/issues/245).

`TabItem::with_closable(true)` opts a tab into closing; the default remains
false. The close button has independent geometry and an accessible name
`Close <tab name>`, with Button role parented by the original Tab part. It
does not select the tab. Close and ordinary selection use the original
ContentHost delivery queue and distinguish their owned payloads:

```rust,ignore
ContentActionPayload::TabCloseRequest { index, label, panel }
```

`index` is the original component index at the time of the request, including
closed/hidden entries; it is not an index into a filtered visible list. `label`
and `panel` are the original Content IDs. A later reorder cannot rewrite a
queued request. Consumers that defer approval across a model reorder resolve
the retained identity rather than interpreting the old index as a new position.

The request leaves ownership, selection and content untouched. The host may
approve it with `TabsContent::set_tab_open(index, false)`. Closing an inactive
entry keeps the selected panel. Closing the selected entry selects the nearest
enabled open entry to its right, then to its left. Closing all entries leaves
an empty presentation. `set_tab_open(index, true)` restores the same Content
pair, mount and state; it does not replay a constructor or lifecycle. Closed
entries do not contribute labels, semantic parts, panel input or focus targets.
The generic component retains fixed slots, so applications can restore a closed
entry at its original index. AppHost separately maps its retained container
identities and does not close the aggregate List tab.

Disabled/inactive tabs cannot request closing. A composing selected panel must
first finish or cancel composition before its close can be approved. Reorder,
hide, cancellation and window deactivation revoke old close captures, so a
late pointer-up cannot close a different tab. AX close Press creates the same
request; AX close Focus admits the independent close button for Enter/Space
through the original ButtonModel keyboard behavior. Selecting a tab retains the
existing roving-selection contract and composition barrier.

TabList AX admission is checked explicitly at the original ContentHost entry.
Tabs are not part of the existing Checkbox/Radio/Segment transaction
classification, and that global classification remains unchanged. Ordinary
Tab parts admit only Focus/Press, with a visible enabled TabList owner and no
expanded/text/value-domain capabilities. An explicit Component capability
additionally admits Focus/Press on visible, enabled close Button parts with
a visible enabled original Tab parent. The parent has no expanded state,
text layout or numeric value range. Other selection roles retain their
existing checks. TabsContent declares that capability
only for its real close parts. No process/window protocol field is added.

The close affordance uses the existing `tekes-clear-group` circle-and-cross asset,
centered at the existing standard icon size. Its original Button hit region,
identity and semantics remain unchanged. Closable Tabs require the original icon
service and propagate resource failures; nonclosable Tabs retain their existing
service contract. In AppHost, the Others Tabs contribute to the same Toolbar row;
selection and close payloads pass through the original Toolbar action queue.

Selected tab chrome uses the shared ButtonModel fill-only selection option.
It retains Surface-derived fill/text/icon states and the independent visible
keyboard focus ring; ordinary Quiet buttons keep their existing boundary.

Regression entry: `examples/composition-consumer/tests/tabs_close.rs` covers
original pointer/AX/keyboard requests, delayed identity, controlled fallback,
all-close/reopen, IME, disabled state, old captures and invalid auxiliary AX
parts. Native/GPU visual acceptance and test execution are recorded by the
coordinator against the frozen integrated source.

## Toolbar Tabs outer spacing (Issue #245, 2026-11-13)

The requested equality is between the selected tab background and the surrounding
Toolbar/partition boundaries, not between glyph ink and the tab background.
Ordinary Tabs retain their previous height, title inset and centered close target.
The Gallery Workspace group-heading spacing correction is independent and retained.

The Others Toolbar region opts out of the ordinary horizontal edge inset through
`ToolbarHost::set_region_horizontal_inset`; all other regions retain their default.
`ToolbarRegion` keeps its original public fields for full-literal source compatibility.
The setter preserves its value across allocation updates; removing and recreating
a region restores the default. Repeating the same value leaves live input intact. Its horizontal ScrollContent owns one outer
padding budget P: `max(space-regular * UiBody scale, FocusRingMetrics.reserved_band)`.
The existing toolbar presentation propagation applies this policy to the original
TabStrip and its containing horizontal viewport on every measure/arrange, including
Settings typography changes. Explicit non-toolbar scroll padding is unchanged.
A larger explicit ScrollContent content padding takes precedence over the automatic
Toolbar budget. Such a deliberate constraint can clip content and is outside the
default equal-spacing contract; it is never silently reduced.
No AppHost drawing offset, replacement wrapper or ScrollArea policy is introduced.

Toolbar TabStrip height is the Toolbar row minus 2P. Background, content placement,
close target and accessibility use that same exact height. Close artwork and its
slot scale with the actual typography in this presentation only. At 100/110/200/300%
P is 8/8.8/16/24pt and height is 26/28.6/52/78pt. At the supported 50% lower bound,
full focus coverage sets P to 5pt at 1x backing or 4.5pt at 2x, giving heights 11/12pt;
the close icon is 8pt. These floors reuse the existing focus geometry, including
its physical-pixel antialias guard, rather than clipping the focus decoration.

Titles retain natural widths. Toolbar inter-tab gaps equal P and the document
extent omits the last lane's gap. First/last explicit reveal includes outer padding,
so an overflowing document revealed to either end retains that spacing. A short
document may leave unused horizontal space; it does not stretch tabs. The create
button stays the viewport's sibling and right-side controls retain their allocation.
Historical internal-padding and general ScrollContent gutter candidates, with
266/267 evidence identities, are preserved in the task artifact directory.
