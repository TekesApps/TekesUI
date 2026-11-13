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

## Equal content insets (Issue #245, 2026-11-13)

Tabs measure their height from the title line box and, when present, the close
icon artboard, plus `space-relaxed` on both sides at the resolved UiBody scale.
The same inset applies at the leading and trailing edges. The title-to-close
separation uses `space-regular`; the close icon remains centered in its own
hit target. That target moves inward to reserve the trailing inset, and the
title slot ends before the target. Natural-title TabStrip lanes
therefore have equal content insets; equal-width Tabs retain their assigned
extra title-slot width. Glyph ink is not the measurement boundary. Compact
Tabs follow the same content-inset rule rather than the old fixed 30pt floor.
General Button geometry, Toolbar partitions and ScrollArea ownership are unchanged.

For the standard 16pt UiBody line box and 16pt close artboard, Tabs measure 40pt
at 1x and 80pt at 2x; the existing Toolbar rows are 42pt and 84pt respectively.
A custom title line box taller than 18pt times the typography scale needs more
height than that Toolbar row; this change does not silently shrink its insets.
The real AppHost renderer measures shaped line height, so native verification
must also inspect the actual title font metrics and clipping.

The Gallery Workspace headings are separate TextContent/Stack children, not
ListView group headers. Their inter-group and heading-to-list gaps both use
`Spacing::Regular`; ListView row metrics are unchanged.
