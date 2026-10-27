# One mounted Toolbar host per window

`composition::ToolbarHost<B>` is the window's declaration/mount adapter over **one original ContentHost**. It does not add a second Button, pointer, focus, tooltip or menu state machine. Main, Settings and other areas declare original `ToolbarGroup` Content under the same host. Placement is local to the receiving `ToolbarRegion`, never a global screen coordinate.

## Minimal command group

The independent, buildable consumer is `examples/composition-consumer/src/toolbar_mount.rs`; its native window is `examples/composition-consumer/examples/toolbar_mount.rs`.

```rust
use tekes_ui_controls::composition::*;

// host already has a SETTINGS region. These are stable window-tree Content IDs.
let buttons = [(100, "Undo"), (101, "Redo")]
    .into_iter()
    .map(|(id, text)| ToolbarButtonDeclaration {
        id,
        icon: None, // or the window renderer's registered ImageId
        text: text.into(),
        hint: Some(format!("{text} settings edit")),
        enabled: true,
    })
    .collect();

host.register(ToolbarRegistration {
    region: SETTINGS,
    order: 0,
    group: Content::new(EDIT, ToolbarGroup::new(
        ToolbarPlacement::PrimaryAction,
        vec![toolbar_commands(EDIT_CONTROL, buttons, ToolbarGroupOverflow::Together)],
    )),
    bindings: [100, 101].into_iter().map(|id| ToolbarCommandBinding {
        target: EDIT_CONTROL,
        action: Some(id),
        command: id,
        source: settings_source.clone(), // ORIGINAL module/session/generation
    }).collect(),
})?;
```

No MouseMoved/Entered/Exited or overlay code is added for this group. Per window, translate native events once, call `host.event(...)` (or `event_with_text` for a text-service delivery), render once with `host.render(...)`, then consume `host.drain_commands()` once. `event_bubbled()` is the shared fallback/propagation decision. An invalidated or inactive host does not expose stale input to a background dispatcher. An open More menu blocks background pointer, Tab and AX through the existing shared overlay scope. Previously acquired capture retains the original ContentHost rules.

Each invocation contains the original group, command, owned `source`, window and delivery source. Do not replace `source` with a currently selected navigation module. The transport still validates its ProcessComponent session, registration and window generation. Replacing a registration unmounts its old subtree and revokes undrained commands for that group. Removal, visibility loss, context changes and window deactivation clean up pending UI state. Delivery data is recorded in the original `ContentActionPayload::ToolbarCommand`, before menu-close focus restoration; it is not inferred by polling later. Consumers matching the payload enum exhaustively must account for this variant.

## Any original component

A ToolbarGroup accepts `Vec<Content>`, not a closed button/title enum. Original Text, standard Button (including image/text/custom contents), progress, custom Components and interactive Content subtrees can be registered. They retain their own measure, paint, event, help, text-service and AX contracts. Never nest independently interactive children inside a single Button action; the ordinary Button slot validation still applies.

For a title or breadcrumb, declare the original `TextContent`, e.g. `Content::new(TITLE, TextContent::new("Workspace / Settings"))`. Toolbar applies the existing one-line label/ellipsis service while preserving full source and accessible names. The window's real text backend supplies label shaping. A legacy backend without label support clips an unwrapped row; it does not silently wrap. Editors retain their own editor contract.

The independent consumer includes `JobProgress`, a normal Component adapter around the existing `ProgressView` and `Progress` value. Toolbar has no progress-specific branch or new progress state machine. Existing public components already implementing Component mount directly; paint/view primitives need their ordinary Component adapter.

Custom activation owners bind `target: ORIGINAL_CONTENT_ID, action: None` to the original command/source. No second button or AX identity is introduced. `ToolbarCommandBinding` currently routes Activate records; other component-specific state updates remain on their original API. Read-only `content_host()` provides existing focused text/native IME geometry and semantic queries; it is not another mutable input owner.

## Regions, overflow and lifecycle

- Supply the real window viewport, window identity and `ToolbarRegion { id, available, native_safe_area, scope }`. Multiple regions can be in separate rows or container segments. Within each, Navigation/Principal/PrimaryAction placement uses that region's original ToolbarContent layout.
- Command groups default to Together through the declaration helper: all buttons remain in the row or use one original More menu. Split explicitly allows a visible prefix. Each command group owns its existing More; the host owns traversal and menu priority across all groups. There is no additional global menu state machine.
- Reserve original command inspection triggers before allocating remaining width to titles/custom content. Each group's allocation stays within its region. If even the inspection-trigger minima exceed the region, the shared allocator clips proportionally to the actual available width; it never invents width outside the consumer allocation. Custom content remains in the row and clips; it is never silently converted into a menu item. Consumers needing an alternate compact representation declare that original component explicitly.
- `register(...)` mounts/replaces a declaration through original ContentHost insert/remove lifecycle. Unchanged groups keep their mounts. Use `update_component::<T>(group, target, ...)` for value/availability updates, not structural child replacement. Use register/remove for structural declarations so mount/unmount callbacks run.
- `set_visible`, `move_group` and `remove` own cleanup. Moving uses `ContentHost::reparent` on the original instance, preserving its help declarations and mount identity while cancelling old interaction geometry. A layout revision invalidates old published AX targets even when the underlying mount survives.
- Native AX adapters retain the opaque `ToolbarAccessibilityTarget` provided for owners and parts. Never invoke a naked recycled Content ID. Republish current geometry after resize, font/backing-scale changes or declarations. While a successful render is pending, old targets and events are fenced. Window deactivation revokes input and clears focus/capture/hints/menus.

## Migration and remaining window responsibilities

1. Create one ToolbarHost for the window; declare all Main/Settings/other regions.
2. Move existing original ToolbarGroups/ToolbarControls/custom Content into registrations. Bind original module command sources; register image handles with the window renderer as before.
3. Replace per-group event loops, focus/menu fields and hint/overlay passes with one host event/render path. Consume each owned command drain once.
4. Route state changes through registrations/value updates and notify the host of visibility, removal, region and window activity changes.
5. Project the host's current semantic nodes/parts to the native bridge using their stamped targets. Forward the window's real keyboard/text/IME callbacks using the existing ContentHost/native adapters where applicable.

The consumer still owns the native window, renderer and text services, screen placement, real available regions and native titlebar safety measurements, application font zoom input, image resource registration, native AX projection and business/ProcessComponent transport. It does not own per-group hover, pressed/capture, hint timers/paint, Tab/focus or menus. This mount adapter does not reposition native traffic lights or change AppHost. AppHost migration is performed by its consumer owner.

## Verification

Historical Gallery Root evidence is retained under `toolbar-mount-lifecycle-20261022`; it is not current-main acceptance. [Issue #69](https://github.com/TekesApps/TekesUI/issues/69) records the separate local-main intake and its exact source/binary receipts. Public API/consumer, real GPU, native keyboard/pointer/AX, user hover review and scoped visual review are kept separate. No performance, VoiceOver or whole-system Gold7 completion is implied.

## Ownership during container visibility transitions

Registration selects a region with explicit ownership. `ToolbarRegion::new` has
`ToolbarRegionScope::Container`: original Content follows that container's supplied
allocation and disappears when it is hidden/clipped or removed. A window-owned
region is declared with `ToolbarRegion::window(id, window_band,
ToolbarWindowEdge::Leading | ToolbarWindowEdge::Trailing)`. Register persistent
buttons, editors or arbitrary Content into that region. Keep its original region
and group IDs stable while container visibility changes; do not migrate the
registration to whichever container is currently visible.

ToolbarHost measures original Content once per layout, reserves the fixed window
edge space from same-row container contributions, shares a narrow-window budget
between the edges, and applies native safe-area exclusions. The consumer supplies
the window band and ownership declaration, never per-button compensating positions.
Moving/removing a container invalidates its inputs, not an unchanged window-owned
editor's focus/session. Updating one button cancels its stale input and pointer
capture without blurring its unrelated sibling editor. Window resize, window
identity change and deactivation retain their full invalidation rules.

Public regression `examples/composition-consumer/tests/toolbar_scope.rs` checks all
samples of leading/trailing collapse/reopen and reversals at three typography sizes:
one original image per button, stable frame/mount, native exclusion, editor draft
and focus, disappearing container content, and narrow-window non-overlap.

## Restoring the window's logical focus owner

A window adapter may retain its existing logical toolbar owner while a temporary input barrier cancels the active gesture. After validating that original source/mount and restoring valid geometry, obtain a current owner target from `semantics()` and call `restore_focus(target, visible)`. It delegates to the same ContentHost focus/reveal/IME admission and preserves pointer versus keyboard focus visibility; it does not synthesize a click or replay an armed key. Only registered original owners are accepted. Semantic parts, stale targets, inactive/not-ready windows, replaced mounts and refused focus transfers return false. An accepted already-focused owner returns true. The adapter must not restore an old owner after a newer user focus choice.
