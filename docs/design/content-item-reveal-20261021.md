# Issue28 — original stable item navigation

Current status: implemented in WIP; public and real mixed-tree CoreText, GPU and scoped external native evidence are recorded below. Scoped external native and Gold7 regression now passed on producer43ea; independent exact-input / matched product-page acceptance remain separate. Root owns the shared implementation; AgentKit owns Agent presentation and business initial/follow deduplication. No commit, push, tag or Issue closure in this round.

## Shared cause and implementation

Existing ReadingDocumentVersion, Content mount identity, Content/ScrollContent traversal, ScrollArea and ProgressTransition were sufficient foundations. The missing contract was an explicit stable item command that could express placement and motion without using focus as a scrolling surrogate. The shared viewport prerequisite from the previous round remains in place.

`ContentHost::reveal_scroll_reading_item(owner, &expected, item, options, text)` accepts the original reading owner, retained attachment/revision token, declared stable ComponentId, existing ScrollRevealOptions and the current original ButtonTextRenderer, as event_with_text already does. The consumer passes no offsets or target rectangles. It requires an initial successful render and requests an ordinary render before native text/AX/pixels are published.

The command validates attachment/revision before layout, then declaration, original per-revision mount identity and layout. A→B→A, another owner, old revision, removed/replaced or unlaid-out targets have typed rejections. Reading transactions reject before calling the backend. Accepted new revisions reset item mount baselines. A failed render makes geometry ineligible until a valid render. Unmount creates a fresh attachment and clears item baselines and temporary navigation intent; the remount regression failed before that cleanup was added and is retained separately.

The library plans only the target's actual ScrollContent ancestor path within current modal scope. It reuses ScrollArea nearest/centered placement, existing Insets and clamping. The original tree is arranged at the proposed endpoint to verify actual clipping; only library-owned ScrollArea/reading metadata is saved/restored. Editors, actions, children and application components are never cloned. Layout must be deterministic for unchanged inputs; arbitrary application layout mutations are retained, as with normal render. Fully clipped or unreachable targets are rejected; ordinary offscreen targets can be revealed.

The existing render measure/arrange/text-service checks are reused through one private layout_frame helper, including Issue27 post-layout guards. There is no second layout engine, synthetic focus/click, IME command or replacement editor.

Accepted navigation latches reading-history intent and fences old momentum, including an already-visible target. Anchor compensation is suppressed only while the corresponding explicit navigation owns that reading viewport, so it cannot counteract nested navigation as if it were document reflow. It is released on completion, interruption, revision change or unmount. Fresh wheel input wins; no timer reasserts a target after manual browsing.

Original shared motion handles animation and Reduced Motion. An unclocked backend completes immediately. Private process-unique reveal generations plus original component mounts prevent an older command from cancelling newer direct navigation, even after public scroll-value replacement. The latest result reports Animating, Completed, Interrupted or Rejected; a new accepted command supersedes the old one, while invalid commands leave it intact. This is not a business queue.

## Applicability and boundaries

Applies to the original declared reading items under existing ScrollContent, including text, Panel/group and original controls, with nested scroll ancestors. Tokens, sets and API token values remain unchanged. The three new public types are composition support; no registered component was added.

Normal valid navigation preserves original composer focus, editor identity, text, directed selection, marked range and undo. Missing text/label resources retain the existing render failure/cancellation contract; callers must repair services and render successfully before retrying. Application arbitrary side effects during layout are not transactionally rolled back.

The test consumer uses a production SurfaceView for the Panel and original TextContent, ButtonContent, GridContent, ScrollContent and TextFieldContent. Its navigate method passes IDs/options/backend only. Native review buttons use original preserving_focus actions. The private clipping/font/width/Reduced Motion switches are test controls, not product presentation or system-setting changes.

## Verification

- 16 independent public CoreText tree tests: nested nearest/centered, edge clamping, unrelated branch isolation, original composer preedit/baseline/undo, ABA/revision/foreign/removed/replaced/unlaid-out/clipped targets, clipping recovery, Reduced Motion/unclocked completion, wheel/momentum ownership, geometry interruption, transaction rejection, newer direct navigation and owner remount/resource recovery.
- 426 control tests and 208 independent consumer tests, strict controls/Gallery/consumer Clippy. A stale generated public type binding was regenerated using the existing generator; no test or gate was removed. Full capability validation follows fresh runtime observations.
- Final GPU host: 48 cases / 240 real GPU frames over four themes, three font scales, nearest/centered and 360@1 / 520@2. Immediate, animated and Reduced Motion endpoints match; real target pixels change and original preedit baselines remain identical. CPU tests separately verify placement and rejection semantics. The final host uses the same existing root inset as the native consumer, retaining the whole composer focus ring; earlier no-gutter images are preserved as fixture diagnostics.
- 17 canonical actual token probes / 6,432 bounded cases, complete=false; these do not establish exhaustive state/token coverage.

Evidence: `design/reviews/content-item-reveal-20261021/`. Final native, Gold7 scope, cleanup and isolated delivery receipts must be read separately; a build or GPU result does not imply native/VoiceOver/product-page acceptance. ClientKit RunChrome/indicator ownership was compared read-only; no Swift or Agent UI state was copied here. Matched ClientKit product-page appearance and receiver integration remain independent requirements.

No performance measurement. VoiceOver is not enabled for this round. Immutable545 and292f inputs remain untouched; this implementation needs its own later candidate identity and receiver acceptance.

## Final producer native round

Exact producer identity43ea9e62734840927f121eb76d91bf8895bfa9169b3bc6705d6df6b236dac079:22 valid active U2790B@2 checkpoints and7 recorded actual animation-phase frames. Real element actions,coordinate pointer,wheel,keyboard,clipped rejection/recovery,Reduced Motion and400pt/1.5type-scale context preserve the original composer editor ID,focus and reverse selection. Its original undo/redo history still works. An unrelated render after manual browsing does not reassert navigation. Full focus outline and original centered button text were visually reviewed.

Raw29observations retain background entry,initial empty selection,mis-targeted coordinates andduplicate-read diagnostics; those are explicitly excluded from the22accepted checks. Native n/i typed ordinary characters,not marked composition: no physical/system-candidateIME claim. Public and48-case GPU tests separately preserve real original marked/preedit state. VoiceOver stayed off. App/process/owned outputs closed/unused-checked/cleaned; source/evidence/common framework cache retained. verified-inputs.json reuses unchanged immutable older evidence by hash-checked references and copies only20changed/new files(1.49MB),avoiding another full WIP snapshot copy.

Native acceptance does not imply the later isolated candidate or matched ClientKit product-page has passed.

## Same-extent geometry follow-up

The historical producer43ea and superseded c44 package did not detect a target moving inside an unchanged document extent. A retained red/green regression now covers this case. ContentHost checks the current original scroll ancestor path and each target document rect against the accepted command, projecting inner viewports to planned endpoints so ordinary animation stays valid. A changed layout interrupts the old command without replacing the composer or starting another command. This reuses the existing private navigation metadata and adds no public type or token value.

Current follow-up evidence is in `design/reviews/content-item-geometry-20261021`: 17 public item tests, 426 controls, 209 consumer tests, 48 GPU cases/240 frames and 17 bounded token probes. Exact-source native, immutable delivery and matched product-page acceptance remain separate; historical43ea receipts are not rebound to this fix.
