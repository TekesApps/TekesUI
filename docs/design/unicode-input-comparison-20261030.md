# #41 native Unicode input provenance

Issue: https://github.com/TekesApps/TekesUI/issues/41

The original CUA `typeText("原生A草稿保留")` observation retained only `A`;
paste retained the full string. Separately, AgentKit on SDK `50ec49f` observed a
fresh-context Pinyin first-key Commit instead of Preedit and final `n i好`.
The unchanged program subsequently passed. These are different input sources
and different observations; neither establishes a unique SDK production defect.
The first failure, previous-program success and repeat success remain separate.

The diagnostic batch adds an independent standard NSTextField/field-editor oracle
and an SDK example using the original TextFieldContent, ContentHost, public
text-service state and existing native key/pointer adapters. It introduces no
production platform or Controls changes. Both programs record fixed ASCII/CJK/
mixed and fresh-context Pinyin input, ordered native key spans and callbacks,
current value, UTF-16 ranges, focus/active state and unintended action count.
Standard AppKit calls its original superclass once. Its opaque internal input
context Boolean is explicitly unavailable; SDK's actual handled/disposition is
retained. Logging exists only in these explicitly launched diagnostic programs.

Hypotheses to distinguish: injected event stream lacks CJK; matching native input
is consumed without text callbacks in both controls; standard AppKit succeeds
while SDK loses a callback/value; new-context first key differs from subsequent
keys. Fixed source-matched pairs and complete event order are required before
attribution. No fallback characters, timers to warm up input, dropped first key,
automatic retry or alternate IME implementation is authorized by this evidence.

Execution/source procedure and limits are in
`scripts/diagnostics/unicode41/README.md`. A declared source is never inferred
from the result text. Synthetic NSEvents do not certify CUA typeText, software
Pinyin does not certify physical IME, and AX does not certify VoiceOver.


## Software comparison results, 2026-10-30

All runs used normal LaunchServices apps, fresh process/editor contexts, the
existing helper/GPU locks, verified occupancy and HID idle >=60 before each
launch, and the original bounded 45-second launch guard. Current input source
remained `com.apple.inputmethod.SCIM.ITABC`; no settings or permissions changed.
Programs were compiled from `53da5d8` plus the integrated #25 inputs and #41 R5
fixture. Later #25 module/export ordering and metadata changes do not relabel
those original compiled inputs. Exact source diffs, programs, signatures,
arguments, stdio routing and raw callbacks remain archived with each run.

| Recorded source and case | Standard NSTextField | SDK TextFieldContent | Strict pair |
| --- | --- | --- | --- |
| Local NSEvent / ASCII | `Ascii-A7`, caret 8 | `Ascii-A7`, caret 8 | Pass |
| Local NSEvent / CJK | `原生草稿保留`, caret 6 | `原生草稿保留`, caret 6 | Pass |
| Local NSEvent / original mixed string | `原生A草稿保留`, caret 7 | `原生A草稿保留`, caret 7 | Pass |
| PID software Pinyin / new context | first Commit(`n`), final `n i好` | first Preedit(`n`), final `你好` | **Fail retained** |

Both controls received the same key characters for every pair. The three local
NSEvent cases had matching ordered commits, exact final UTF-16 selection, no
remaining marked text and no Submit. The Pinyin source used the original
`MacInput` implementation from TekesAgentKit `8734185` byte-for-byte, with keycodes
45/34/4/0/31 then 49 and 30 ms down/up spacing. Each key verified the exact
foreground PID/program file identity and current input source. Only the COMMIT
trigger differs from the historical AgentKit host marker: this fixture waits
for five completed native key spans. This is not a complete timing replay.

The standard-control Pinyin sequence was Commit(`n`) -> Preedit(`i`) ->
Preedit(`i h`) -> Preedit(`i ha`) -> Preedit(`i hao`) -> Commit(` i好`). Its
strict result failed; the first run was not repeated. SDK's separate fresh
process produced Preedit(`n`) through Preedit(`ni hao`) then Commit(`你好`).
AppKit ran first and SDK second, once each. These observations establish that
the historical failure shape can occur without SDK editing code. They do not
uniquely distinguish software injection, AppKit/system-context initialization,
or fixture timing/observation effects, and do not establish stable Pinyin input.
No production platform/editor change or character/delay compensation was made.

The original CUA `typeText` source was **not executed** in this software window.
Local NSEvents cannot clear that original observation. Physical IME and
VoiceOver are also not_run. Existing paste/preedit/replacement/cancel/restore
records retain their original source/program identities; this diagnostic does
not replace them or claim a new CUA paste/physical-IME result. Root owns the
remaining source-specific acceptance and integration CI.

The compiler/fixture failures remain retained: Swift global-binding/name errors;
two pre-input LaunchServices EPERM attempts; and one SDK fixture missing-ready
command timeout. Kernel evidence located EPERM at xpcproxy opening external-volume
stdio, not at input dispatch. A fresh `/private/tmp` stdio rendezvous allowed
normal LaunchServices launch while programs, apps and build/module caches stayed
on SSD. Complete streams were copied back byte-for-byte with SHA verification
before temporary cleanup; failed-run streams were retained. The missing ready
command and observed volume-path alias comparison were fixed in diagnostics only.

Evidence package: `artifacts/issue-41/unicode-comparison-20261030-r1` on the
registered SSD, including `software-comparison-matrix.json`, original logs,
per-run launcher/source/program receipts, exact launcher sources and the formal
handoff. All eight final-run fixture PIDs were confirmed absent; the earlier
fixture-timeout PID was separately verified and stopped. No window or lock is
retained by this domain.


## Source-qualified disposition, 2026-11-05 (Issue41)

The earlier table and failures above are historical, not overwritten. Later
source-bound CUA `App.typeText` mixed-input controls (stock051 and SDK085)
both retained only `A`; SDK ingress also contained only `A`. The tested method
is not qualified for full Unicode. This does not prove a unique provider bug,
an intentional ASCII-only policy, or an SDK loss after receiving Chinese.
Issue41 explicitly permits documenting an automation-source limitation and
reproducible system-IME acceptance steps instead of requiring a third-party fix.
The README now states those steps and source boundaries. Separate genuine Pinyin
preedit/Chinese-commit evidence retains its original identity and finite scope;
it does not repair typeText or certify human physical IME.

The diagnostic SDK consumer now reuses the ordinary native clipboard contract:
host-owned pasteboard read, active focused writable/unmarked editor, original
Commit transaction, Command-V fallback or `paste:` selector dispatched once.
No production platform/editor API changed. The separate manual default AppKit
control adds only a standard target-nil Edit/Paste menu; it introduces no key,
editing callback or input-context hooks. Historical menu-less stock identity and
two incomplete paste attempts remain retained, with no unique-cause claim.

One new official CU2 `cu2_paste_text`/USB Command-V operation per program passed
the final-outcome comparison: stock program `d60f059c8c2c4c47c05440534c653ec8c04f33e6c3bdffcbf45305f122905cef`,
SDK `a9d112a34d7e493791fd92e7a19bd46b2dbb694dbc8a84d18b2a7160fe28a9fb`.
Both ended with exact `原生A草稿保留`, UTF-16 selection [7,7], marked null and
zero actions. SDK recorded one clipboard input and one editor Commit; stock
callback provenance is unobserved, strict callback acceptance null. This is not
strict callback equivalence or a measured render-latency result. Both exact
processes and original resource guards were released. Primary accepted records
are the stock-edit-menu and SDK `actual-run-001/ROOT-ACCEPTANCE.json` under the
archived `native-pair-cu2` evidence; README explains the distinct log schemas.

CU2 paste is distinct from CUA App.paste, failed typeText, synthetic local
NSEvent and human physical input. Bytewise clipboard restoration and human
physical IME/VoiceOver remain unverified here. A provider fix/version is a
future trigger for meaningful typeText re-verification, not an added prerequisite
to the original documented-limitation branch. Root retains final Issue acceptance.
The SDK archive contains its actual MAIN WIP compilation context; passing that
archive does not verify a clean public baseline containing only this diagnostic
closure. Exclude unrelated candidate-geometry platform WIP from this delivery;
public source strict/build acceptance must name its actual integrated baseline.


## Pure Rust publication boundary correction (PR232 / CI199)

CI199 rejected the three newly tracked Swift control sources under the existing
Pure Rust source boundary. That failure remains retained. Issue41 requires an
independent standard-control comparison and reproducible system-IME procedure;
it does not require foreign source inside the TekesUI repository. The standard
AppKit controls therefore retain their existing independent external artifact
identities. Their exact source hashes and archived locations are listed in the
README; all original sources, binaries, failures and accepted input results are
preserved. The Rust SDK diagnostic and Python comparison/procedure remain public.

The three Swift paths are removed only from this PR's public tree. No boundary
exception, extension disguise, embedded source, foreign compiler invocation or
production API change is introduced. Local MAIN WIP is not removed. Exact external
source is retrievable through the README's permanent per-file links to published
commit `05830cb7d54694431ced01565db40446c4b304eb`, with individual SHA256 values.
Fetch into a separate diagnostic directory outside the managed repository and
verify the bytes before any separately allocated external compilation. Cargo/CI
never fetches or compiles this source. The historical commit failed the source
boundary and serves only as immutable comparison material, not a validated or
supported product build baseline. The corrected public commit must pass its own
static checks/CI; CI199 and all old typeText failures retain their identities.
