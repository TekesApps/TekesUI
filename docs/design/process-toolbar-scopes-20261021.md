# ProcessComponent toolbar ownership and local placement

This phase follows the immutable `remediation-checkpoint-20261021` checkpoint (main `1294987dacabb33c0e411c13ee5ea544a1123f8e`). The following changes are newer WIP, not claims about that public tag.

## Root cause and shared repair

The existing service toolbar declaration carried a scope but had no bound logical window for headless consumers, and accepted View scope had no actual standard Component placement. A consumer could inspect the latest command list but could not retain its registration envelope to reject stale delayed actions. Frame invalidations also shared a bounded business-output queue, allowing redraw-only activity to exhaust service delivery.

The repair reuses ProcessComponent, ComponentEmitter, the existing Session/toolbar revision, the original ToolbarControl, ContentHost lifecycle, and the existing View transport. It adds no second toolbar state machine or ClientMessage variant. `spawn_in_window(options, window, wake)` binds a service-only component without allocating UI or GPU resources. Native View attachment already supplies the binding. `window_identity()` and `view_identity()` expose logical identities, not transport endpoints.

`toolbar_registration()` retains the complete ServerMessage while Ready; consumers retain its Session and toolbar revision with displayed actions. Delayed actions use `request_from(&registration.session, RequestBody::Invoke { ... })` with that revision. A different process instance, epoch, scope or registration generation rejects the action, even when a command ID is reused. Foreign-window clears cannot erase another valid declaration. Existing unbound headless spawn remains a compatibility path; isolated hosts must opt into the binding.

`ProcessContainer` forwards `drain_events`, `request`, `request_from` and `toolbar_registration` to that same process queue. Hiding does not create another service consumer or discard replies. View invalidations are coalesced redraw signals, consumed by `take_view_invalidated()` or container `prepare()`; they no longer occupy the 64 business-output slots. Clients that previously searched `drain_events()` for redraw events should use this existing invalidation API instead. Replies remain queued and bounded.

## Actual placement and resource ownership

A View-scoped toolbar is rendered inside the original ProcessContainer by the original ToolbarControl. Its measured row reserves space above the remote body, and the body uses its own layout/clip/input rectangle. Window scope removes that local row while preserving the child process, Session and content state. The containing host receives no duplicate business activation: the captured action sends exactly one RPC internally.

Mixed text/icon controls use the existing ToolbarControl action/image map. `set_toolbar_icon_images` supplies host-owned ImageIds by module icon name; the container borrows them and never unregisters them. Missing icon capability is an explicit recoverable error, with no silent replacement by text. Theme/brand values continue to come from JSON Design System roles.

Shared ToolbarControl fixes clear stale hit/focus geometry when command IDs change or registrations become empty, preserve matching command focus where valid, support clipped painting/hits, and request redraw for hover changes while leaving unhandled movement available to ancestors. No per-product toolbar painter is introduced.

Applies to optional-View ProcessModules, multiple logical windows, local Component placement and window-collected toolbars. Reasonable exceptions: original unbound headless compatibility and explicit legacy V1 opaque-primary input. This does not implement remote keyboard/IME/AX, geometry-dependent service actions, arbitrary caller-supplied toolbar Content slots, or per-action native semantic children. Native AppHost keyboard routing remains separately tracked.

## Layered evidence and remaining work

Evidence root: `design/reviews/process-toolbar-scopes-20261021/`.

- Public API: 15 runtime/Mach tests, unchanged legacy Start wire/exhaustive ClientMessage test, 9 shared toolbar tests, 3 public toolbar integration tests; strict runtime/protocol all-target Clippy passed (`local-strict.log`, `local-real-final.log`).
- Independent service/View consumers: simultaneous window11/window29 isolation, stale owner/generation rejection, same factory attached to another View identity, Window/View scope switching, 130 coalesced redraws with retained business replies, 16 actual GPU import cases, 4 queued V1 and 6 legacy cases (`final-verify.log`).
- Actual standard Component: `process_container_toolbar` uses public APIs, real ContentHost, CoreText and GPU; mouse and keyboard produce one RPC, local/window scope switch preserves ownership, 12 theme/brand renders, missing-icon recovery and borrowed image retention pass (`local-real-final.log`, `local-gpu-final/`). The original V1 Component consumer also passes. These are real GPU results, not native desktop evidence.
- Current token dependencies: all 18 real probes passed again under the existing build lock/cache, in `design/reviews/stage40-audit-refresh-20261021/probes-process-local-final/`. Their exclusions remain unchanged; they do not claim complete token or experience coverage.
- Visual inspection: local before/after and dark-theme GPU images show actual child text and local control painting. Native external-display and Gold7 final experience are pending; toolbar traffic-light sizing and artwork normalization remain a separate shared-design task.
- New lifecycle regression: `local-hidden-before.log` proves an old local pointer arm survives hide/show and fires on late release. Fixed by calling the original ProcessContainer cancellation path before visibility changes. Six actual consumer cases (pointer/Space across hide, disable and window inactivity, each followed by restore and late release) now pass using a service FIFO barrier rather than a timing sleep (`local-hidden-final.log`). Strict all-target Clippy also passes. No native/Gold7 status is promoted from the earlier passing cases.

No performance measurements are included. Old logs, before-source copies, fixed source packages and public tag remain intact.
