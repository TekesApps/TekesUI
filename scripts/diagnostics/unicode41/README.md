# #41 Unicode input comparison

This opt-in diagnostic compares a standard AppKit `NSTextField` with the SDK's
original `TextFieldContent`. It does not repair or compensate characters, change
input sources, retry, prime a context with a throwaway key, or enable production
text logging. Run only in the coordinator's allocated native window with the
existing occupancy/input guards. Raw logs contain the fixed diagnostic text.

## Programs and fixed inputs

- SDK: `examples/composition-consumer/examples/unicode_input_compare.rs`, through
  the existing registered build helper/cache on MAIN, not a candidate cache.
- AppKit: compile `AppKitBaseline.swift` using `xcrun swiftc -swift-version 5
  AppKitBaseline.swift -o AppKitBaseline` in that same allocated build window.
  This standalone example uses system AppKit; it is not a production dependency.
- `AppKitStockBaseline.swift` is the separate default-editor Pinyin dispatch
  observation control; it accepts `pid-hardware-key pinyin-first DISPLAY_ID`.
- `AppKitUntouchedStockBaseline.swift` is the manual default-editor outcome
  control. Compile with `xcrun swiftc -swift-version 5` and pass
  `SOURCE CASE DISPLAY_ID`; SOURCE is `physical-keyboard`, `cua-typeText` or
  `paste`. It has no key/editing callback hooks and records only presentation
  and an explicit final outcome. Its standard Edit/Paste menu uses target nil
  and Command-V. Preserve the historical no-Edit-menu program identity.
- SDK and instrumented `AppKitBaseline` arguments: `SOURCE CASE`.
- Sources: `cua-typeText`, `pid-hardware-key`, `local-key-characters`, `paste`,
  `physical-keyboard`. This argument is an operator declaration, not proof.
- Cases: `ascii` = `Ascii-A7`, `cjk` = `原生草稿保留`, `mixed` =
  `原生A草稿保留` (the exact original #41 mixed string), `pinyin-first` = `你好`.

Use a fresh process/context for every program/source/case. Package each exact
executable as a normal `.app` using the existing native launcher; archive executable
SHA, source/lock inputs, bundle identity, launch command and current input source.
Do not silently replace normal LaunchServices launch with a direct binary run.
Capture stdout/stderr completely. Every record starts with `UNICODE41 `, has a
monotonic sequence and includes the actual PID, value, UTF-16 selection, marked
range, active/focus state and submit count. Both programs expose **Diagnostic →
Record result and close**. Closing by that menu retains the final observed value
without injecting another editing key. No ready or result is a test failure.

## Source-specific procedure

1. Allocate the safe native window; capture the current input-source identifier
   without switching it. Launch one exact `.app` and verify its foreground PID.
   For the instrumented controls, wait for the single `ready` record: empty/
   unmarked, active and focused. The manual default stock control emits only
   `window-presented`; establish its actual empty/focused editor using retained
   native/operator evidence, without adding readiness or input-context hooks. No
   warmup key is sent. Preserve the first run even when it fails.
2. `cua-typeText`: call the actual CUA app `typeText` once with the exact sample.
   Save that tool invocation/output, foreground identity and before/after AX.
   Repeat on the other control with identical source/string. A different CUA
   API, clipboard call, Unicode CGEvent or `KeyCharacters` is a different source.
3. `paste`: use the actual CUA app paste operation with the same exact string;
   retain invocation/AX evidence separately. This proves clipboard insertion only.
   Official CU2 `cu2_paste_text` with USB Command-V is another explicitly named
   lane; compare it only with the same CU2 operation on the other control.
   The SDK diagnostic handles Command-V on the ordinary key route or a native
   `paste:` selector using the same host pasteboard/Commit contract as the normal
   native consumer. It rejects inactive, unfocused, disabled, read-only or marked
   editors before reading the clipboard. The platform offers a key with no text
   selector callback to the ordinary fallback only once; key-up never pastes.
   A `clipboard` diagnostic row identifies the host-read payload separately from
   observed AppKit `commit` callbacks. The following original `editor` row records
   the real selection replacement and result; no payload is invented for typeText
   and no source automatically falls back to paste.
4. `local-key-characters`: the fixtures send one local `NSEvent` down/up per
   Unicode scalar through `NSApplication.sendEvent`, keyCode 0, 100 ms between
   scalars, no modifiers. Both use identical fixed characters and close after
   1500 ms. This is only a synthetic AppKit input-context probe. It cannot
   reproduce or clear the original CUA `typeText` observation.
5. `pid-hardware-key` / `pinyin-first`: keep `com.apple.inputmethod.SCIM.ITABC`
   already selected, or record this lane as unavailable. Use the original
   AgentKit `MacInput.key` mechanism: CGEventCreateKeyboardEvent with no Unicode
   text, no flags, CGEventPostToPid to the verified foreground exact app, keycodes
   n/i/h/a/o = 45/34/4/0/31, then space = 49; down/up spacing 30 ms. Preserve its
   exact driver identity and TYPE/COMMIT scheduling. Existing source reference:
   TekesAgentKit `8734185`, `scripts/test-system-ime.py`,
   `scripts/native_ime_probe.py`. Do not run that AgentKit-specific launcher
   against this fixture: it expects different markers. The supplied
   `pid_pinyin.py --pid PID --program EXACT_EXECUTABLE --log STDERR_LOG
   --receipt FRESH_DRIVER_JSONL` verifies the already running foreground target,
   waits for its fresh ready record, posts the five original keys, and posts
   space after five actual keyDown spans finish. It keeps the original 30 ms
   down/up spacing. Its span-driven COMMIT trigger differs from AgentKit's
   host-driven marker; record that difference rather than claiming an exact
   historical timing replay. The driver does not close/kill/activate the app. In this explicit
   PID-key diagnostic mode only, each fixture schedules its result/normal close
   250 ms after the sixth original keyDown span (space) finishes, allowing its
   matching keyUp to arrive. This is final cleanup, not first-key readiness,
   input warmup or retry; incorrect first Commit and final text still fail. A guard failure leaves
   the app and first failure available for inspection.
6. `physical-keyboard`: human operator types the sample or Pinyin sequence.
   Record this separately from software keys; candidate UI/VoiceOver also remain
   separate. Nothing in this fixture automates or claims this lane.
7. Choose Record result and close. Do not rerun until a saved failure has been
   analyzed. For first-context Pinyin, first text callback must be Preedit; a
   first Commit is a failure even if later final text is correct.

## Comparison and attribution

Create a source receipt with `source`, `case`, and `runs.appkit` / `runs.sdk`:
`pid`, `program_sha256`, `log_sha256`, `source_evidence` (exact operator/driver
artifact references including actual injection method and parameters). `compare.py`
checks the log identity and rejects missing/incomplete runs, unbalanced context
spans, stale/nonempty/unfocused baselines, incorrect final values, unintended
submit and remaining marked text. Run:

```
python3 scripts/diagnostics/unicode41/compare.py --appkit appkit.log --sdk sdk.log \
  --source-receipt source.json --output comparison.json
```

The comparator retains ordered callbacks and native key characters. It cannot
certify the operator's source declaration; review its linked evidence. Both
passing only supports the recorded source/case. An AppKit pass and SDK failure
with equivalent observed keys requires SDK investigation; both failing or
unequal keys does not uniquely attribute the cause. Missing CJK in both native
key streams narrows the investigation upstream but does not prove which injection
implementation lost it. SDK Finished records expose the actual input-context
handled value and disposition. Standard AppKit editing calls superclass exactly
once; that superclass does not expose its internal `handleEvent` Boolean, so the
oracle records it as null rather than changing the standard path to manufacture
an equivalent value. Commit/Preedit/Unmark and final text are independently visible.

Reuse the unchanged paste/preedit/replacement/cancellation/restoration regressions;
run only their affected checks plus example strict/build, comparator tests and
Swift typecheck. Historical #41 failure and later success are independent evidence
and must never be overwritten by a passing new pair. Physical IME is not required
to deliver the software diagnostic; its execution status remains explicit.

## Recorded fixture corrections

The first `/private/tmp` stdio run proved normal AppKit launch and exact ASCII
value, with SDK blocked before ready by a missing declared diagnostic menu
command. That failed run and its 45-second timeout remain archived. The ready
command is now registered rather than bypassed through a hidden host callback.
For standard AppKit snapshots, `hasMarkedText()` determines whether marked is
null: NSTextView can return a zero-length markedRange even when it is unmarked.
Process verification uses file identity (`samefile`), preserving the exact
executable requirement across the observed `/DEV` versus `/dev` volume aliases.
None of these corrections changes the production SDK text-input chain.

## LaunchServices stdio and retained outcomes

The actual macOS kernel denied xpcproxy creation/read of stdout on the external
SSD before the app could start. Pre-creating the file did not solve it. Use a
fresh private `/private/tmp` directory (0700), one new stdout/stderr pair (0600),
and keep the exact executable/app/build and module caches on SSD. Tail only that
launch's stderr for PID injection. On every outcome, archive complete byte streams
back to SSD and verify SHA256 before cleaning only those new temporary files;
retain failed/timeout rendezvous files. This changes neither system permissions
nor the input method. The controlled artifact launcher and its exact per-run
versions are archived with the evidence package.

The three fixed local-NSEvent cases passed on both controls. The first software
Pinyin pair failed strictly: standard NSTextField reproduced first Commit(`n`)
and final `n i好`; SDK produced first Preedit(`n`) and final `你好`. Both saw the
same keys; neither control was repeated to obtain green. This is not an SDK-unique
reproduction or a stable Pinyin pass. Actual CUA typeText and physical IME remain
unverified. See `docs/design/unicode-input-comparison-20261030.md` for the complete
matrix, source boundaries and retained failure history.


## Recorded input-source disposition (2026-11-05, Issue 41)

The original Issue 41 permits documenting an automation-input limitation and
reproducible system-IME steps; fixing a third-party typing provider is not a
prerequisite to delivering that diagnostic result. Keep these source categories
separate:

- macOS CUA `App.typeText`, exact mixed sample: Stock051 and SDK085 both retained
  only `A`, selection `[1,1]`, marked null and actions zero. These are retained
  FAIL results. The SDK ingress recorded only `A`; no CJK payload reached that
  observed boundary. This tested lane is not qualified for full-Unicode input.
  The provider version and emitted payload are unknown. This does not establish
  a unique provider implementation bug, an intentional provider support policy,
  or a general inability of AppKit or TekesUI to accept Chinese.
- macOS CUA `App.paste(text, {format: "text"})` is a separate source, documented
  to restore the previous clipboard. The SDK diagnostic paste wiring has been
  repaired and built. The new CUA stock attempt returned unchanged AX and a
  placeholder image after one App.paste; the later finish action failed with
  Computer Use inactive and the run timed out without an original final result,
  so its paste outcome is NOT_PROVEN. A separate CU2 stock attempt
  completed a seven-character paste request and GUI+V; the retained image still
  showed the placeholder, but that run also timed out without a final result.
  Both older runs remain incomplete, not a valid matched-pair failure or pass.
  The old stock app lacked a standard Edit/Paste menu. The menu-only corrected
  stock source, with no input hooks or outcome changes, was built and signed as
  program SHA256 `d60f059c8c2c4c47c05440534c653ec8c04f33e6c3bdffcbf45305f122905cef`.
  One new official CU2 `cu2_paste_text` operation (USB Cmd+V) passed the finite
  default-NSTextView final-outcome oracle: `原生A草稿保留`, UTF-16 selection [7,7],
  marked null, actions 0; the exact process exited and guards were released.
  Callback provenance was unobserved; strict callback acceptance is null.
  The new SDK program SHA256
  `a9d112a34d7e493791fd92e7a19bd46b2dbb694dbc8a84d18b2a7160fe28a9fb`
  also passed one same-source CU2 paste: one clipboard record and one editor
  Commit, exact final text, selection [7,7], marked null and actions 0, active
  and focused. Its pre-close image showed the full text; its immediate after
  image was empty. No render-latency measurement follows from those images.
  Both runs completed normal cleanup. This is a finite final-outcome comparison,
  not strict callback equivalence: the default stock control observes no callbacks.
  CUA App.paste and CU2 cu2_paste_text are different sources and must not be
  merged into one comparison. Do not claim bytewise clipboard restoration,
  typeText repair, or a unique historical cause from this finite stock result.
  Earlier AgentKit paste success retains its original scope.
- Actual system Pinyin has separate successful finite input evidence. To
  reproduce it, use the admitted normal editor and exact recorded input source,
  confirm Simplified Pinyin (`com.apple.inputmethod.SCIM.ITABC`), then verify the
  original empty/unmarked focused baseline. Send N/I/H/A/O as five ordinary
  down/up key pairs; retain real marked/preedit and candidate observations.
  For `pinyin-first`, the first text callback must be Preedit, not Commit.
  Send Space once as the sixth key pair, then require final `你好` in the empty
  editor, UTF-16 selection `[2,2]`, marked null, no duplicate insertion or Submit.
  Preserve all native events and source/program identity; close the owned app
  normally. Existing native input cards/guards provide the launch and cleanup.
  No paste, synthetic preedit, input warmup or silent retry substitutes for this
  source. This is software/native keyboard evidence, not human physical IME.

A named provider repair or changed emission evidence is a useful trigger to
retest the failed typeText lane once, not a new condition for the original
limitation-documentation branch. Publish the exact source-qualified support
and failure evidence, retain original paste/preedit/cancel/restore regressions,
and state physical-IME execution status explicitly. Pending native paste work
must be settled before claiming that repaired diagnostic lane usable.

The unchanged `compare.py` requires the instrumented `UNICODE41` schema and
`control=appkit`, one ready and one final result. It is not a parser for either
`appkit-stock` or `UNICODE41_STOCK_MANUAL`; never normalize the default stock
records into instrumented callback evidence. The CU2 paste final-outcome comparison
uses each control's original result plus exact source/payload/PID/program and
operator records. Its accepted records are
`native-pair-cu2/stock-edit-menu/actual-run-001/ROOT-ACCEPTANCE.json` and
`native-pair-cu2/sdk/actual-run-001/ROOT-ACCEPTANCE.json` within the archived
Issue41 paste-comparison evidence. These finite results do not prove bytewise
clipboard restoration, CUA App.paste, typeText repair or human physical IME.
The SDK archive retains its original MAIN WIP compilation context; it is not
proof of a build from only the publicly delivered diagnostic files.
