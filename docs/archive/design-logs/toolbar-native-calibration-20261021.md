# Native window-toolbar calibration

This follows public main1294987/PR40 as newer WIP. It does not modify the published checkpoint or the older accepted AgentKit freeze.

## Root cause

AppKit's native toolbar allocation and its traffic-light alignment line are different measurements. The current Unified/full-size-hidden oracle reserves66pt but places the actual native controls at y26pt with14pt diameter. AppHost centered all contributions in66pt, putting them at33pt. Source measurement and an independent actual2200×1280 native screenshot both prove the7pt offset. A separate artwork measurement showed that a common24-unit SVG viewBox did not imply common visible ink: back/forward displayed at9.07pt, side-bar/panel at11.07pt, while the dots intentionally displayed at2pt.

## Shared repair and application

`Window::titlebar_control_center_y()` measures visible native button bounds converted into the existing top-left content view. Ordinary content returnsNone. `titlebar_safe_area()` retains its original tuple and now shares native-bound extraction. No AppKit view tree is added. The host's window layout keeps full native chrome allocation for body layout/input, but arranges all window contributions around the actual control center. Oversized measured content can grow the row instead of clipping. Inline toolbars remain caller-arranged.

`ToolbarControl::window()` selects JSON `toolbar-label` instead of changing every Body label. Actual system20pt capital height is14.09pt; the native control diameter is14pt. The existing inline constructor retains UiBody. Text/icon action hit frames share the original IconButton minimum, and larger content increases them together.

Independent native review of the first corrected candidate951423f exposed two further defects: its uniform32pt visible gray frame was over twice the native diameter, and baseline ellipsis ink was6.5pt below the common center. These findings are retained under `design/reviews/toolbar-calibration-consumer-20261021/`.

The original shared Button now supports optional `chrome_height` and reports `chrome_bounds()`. DefaultNone preserves ordinary Buttons. Window Toolbar uses JSON `size-toolbar-chrome`20pt; the original>=32pt layout/hit/semantic target remains, and measured larger labels can expand decorative height. Paint/focus stays in Button and text uses its original unclipped label slot. Gray-frame margins still accept a single action through the original clipped hit geometry; no Host style is painted.

Toolbar overflow can borrow an original resolved ImageId via `set_overflow_icon`. AppHost supplies canonical `tekes-more` through its existing image capability. Small dots remain in a fixed16pt slot with their own centered ink. No per-icon runtime auto-resize or duplicate dot geometry is introduced. Inline label-only consumers retain their compatible textual overflow when they do not supply an image. Popup labels keep original Menu typography.

At artwork/design time only, the four actually used regular window icons (back/forward/side-bar/side-panel) are corrected in their shared canonical SVGs to21units of vertical ink on the24-unit artboard, yielding14pt at the existing16pt slot. All177 catalog entries were measured read-only; semanticMore stays2pt. Remaining catalog-wide optical classification/verification is still required before claiming all regular artwork conforms. The original eight animated compatibility icons and published source import are untouched. Both Rust and existing Web entrypoints consume the same assets. A stale Web mapping for the already-existing Swap icon was discovered by real package tests, repaired through canonical catalog generation, and guarded in CI. Removing the Swap command from AppHost does not remove the public icon.

Applicable consumers are Window-collected module actions, navigation/visibility controls, shared Toolbar overflow, and any Button consumer explicitly selecting compact decorative chrome. Original inline toolbars, ordinary Button rendering, and semantic small artwork are the documented exceptions. Per-action AX, host-wide keyboard traversal, remote ProcessContainer keyboard/IME/AX and catalog-wide optical acceptance remain separate work.

## Evidence boundary

Evidence directory: `design/reviews/toolbar-native-calibration-20261021/`.

- Native geometry oracle and platform contract use real NSWindow/NSToolbar/control conversion without showing a window; this is API/object evidence, not native interaction.
- Public Toolbar consumer checks window versus inline typography through original ContentHost. Shared tests check equal text/icon hit and visible geometry, clicking outside decorative chrome but inside the larger target, and original cancel/disabled/overflow behavior.
- The actual independent ProcessContainer/CoreText/GPU consumer retains single-RPC mouse/keyboard activation, twelve theme/brand renders, resource ownership and restore/late-release isolation.
- The new actual token probe includes both inline/window profiles, six states, four themes and three brands (144cases); the other17 probes are rerun as well. Token probes exclude actual font/icon pixels and native/Gold7 acceptance.
- Candidate75688ce (`retained/TekesAppHost.chrome`) includes the20pt visible frame and originalMore image. Independent AppHost native review matched this input and its separately signed binary: actual screenshot measurements show14pt traffic/icon/capital ink,20pt visible chrome, and2ptMore ink centered at26pt (capital raster center26.5pt). A click outside gray chrome but inside the retained hit target activated exactly once; overflow action and side visibility also passed. Evidence: `design/reviews/toolbar-chrome-consumer-20261021/{identity.json,acceptance.json,pixel-check.json}`. This proves the current small App scope, not global keyboard/AX, full catalog, physical-display perception or complete Gold7 acceptance. Earlier951423f findings remain historical failures, not transplanted final receipts.
- Design package/import/historical golden integrity still passes. No performance test, OS setting change, new UI framework or Issue closure is included.

## Public source compatibility

The new window constructor, optional Button chrome and optional overflow image preserve old defaults. The generated TextRole enum gainsToolbarLabel: exhaustive external Rust matches must handle the new case (or their intentional wildcard). This is a potential source-breaking change despite current independent consumers compiling. It must be called out in the next exact-pin migration/release; no accepted frozen package or older tag is rewritten. A broad semver-compatibility claim is not made.
