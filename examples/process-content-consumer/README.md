# Independent Process Content consumer (#48)

Source baseline for this guide: public main
`1bc847726138e60804b3068dd800d7e081f29941` (PR213, 2026-11-01).
Check out the complete repository at that revision. This standalone Cargo workspace
uses its checked-in `Cargo.lock` and `../../crates` path dependencies; retain the
repository build scripts and token/JSON inputs. Copying this directory alone does
not supply the dependency closure. This is source delivery, not a formal release.

## Execution and capability matrix

| Mode | Public entry | Boundary |
| --- | --- | --- |
| Service-only subprocess | `ProcessComponent` without a View | Business services and lifecycle; no mounted editor or native interaction claim. |
| Legacy V1 View | `ProcessContainer::new_v1` | Primary pointer only through explicit opaque opt-in; no remote keyboard, scroll, IME or AX. |
| Negotiated Content View | `ProcessComponentOptions::with_view`, `ProcessContainer::new_view`, child `ProcessModule::content` | Same original child `ContentHost` and text backend; require the exact capabilities needed by the module. `ViewCapabilities::supports` checks required flags; unsupported capabilities reject. `content_pointer` disposition is distinct from legacy `primary_pointer`. |
| Native mixed Reader | Consumer `--native-scroll` on macOS | Three actual subprocess Readers, original editors/buttons/history and platform text/AX bridge. Native behavior and physical system IME remain separately accepted. |
| Local Content | Original `ContentHost`/`Content` in the host | Same component types and business inputs may be composed locally; this consumer does not offer an in-process `ProcessContainer` execution mode or certify local/remote equivalence. Keep subprocess PID/session receipts separate. |

This consumer's real execution requires macOS/CoreText and its rendering/IPC
backends. The non-macOS `main` only prints an unsupported-platform message;
its successful exit is not consumer acceptance. No Linux/Windows runtime support
is established by that stub.

For migration, retain business `Content` and state inputs in the child module,
provide its existing text backend through `ProcessModule::content`, declare the
module's actual `view_capabilities`, and attach the View with the required flags.
Mount `ProcessContainer::new_view` through the original Content lifecycle. Forward
platform events/text queries through the existing bridge and publication adapter;
do not duplicate text, focus, scroll or IPC state in the host. Keep the existing
session/View/viewport/mount and text-binding fences. See the
[negotiated View contract](../../docs/design/process-component-view-v2-20261021.md)
and the actual module/host examples in `src/main.rs` and `src/consumer/`.

## Normal execution map

Use the coordinator's existing source/cache/build-lock environment and Rust
1.95.0. These are entry points for an allocated batch, not a new execution grant.

```sh
cargo +1.95.0 clippy --locked --offline --manifest-path examples/process-content-consumer/Cargo.toml --all-targets -- -D warnings
cargo +1.95.0 build --locked --offline --manifest-path examples/process-content-consumer/Cargo.toml --message-format=json-render-diagnostics
```

Archive the actual non-test compiler artifact with its source inputs and hash
before another link. Invoke that program with no argument for the original default
mounted scenario, or one of the following separate non-window modes:

`--accessibility-only`, `--projection-only`, `--query-wake-only`,
`--view-context-only`, `--three-surfaces-only`, `--overlay-only`, `--scroll-only`,
`--reading-anchor-only`, `--reading-lifecycle-only`.

The existing required CI step **Independent Process Content consumer (Issue 48,
no native windows)** runs all-target strict, the Runtime
`process_component_services` example, default and these nine modes: eleven runtime
invocations total. Inspect each actual outcome and raw failure; a skipped step is
not success, and a failed aggregate step does not mean every mode failed. The
normal local `077078d` five-case receipt and old hosted CI failures retain their
own identities; no services-timeout root cause or stability claim follows.

## Original acceptance still open

The published manual journal path is `--native-scroll` with only
`TEKESUI_PROCESS_NATIVE_MANUAL_EVIDENCE` selecting a fresh evidence directory.
Unset the selftest/B2/reading script flags. The existing acceptance menu arms the
next real preedit for the original reveal/background-update scenario; it does not
synthesize composition. Actual physical Pinyin input, no-submit/focus/marked-state
continuation and candidate-window move/resize observations must be recorded in an
allocated native window with matching source/program identity. Programmatic
preedit and text queries do not certify the system candidate interaction.

The one-call native ACK-drain repair first passed strict/build at `0aec8897`;
that allocated native run was refused by HID admission and then withdrawn. The
original startup timeout and later Native063 first-input failure remain failed
historical results. PR213 published the subsequent input-clock adapter correction
at the public revision above. The design's
[scoped verification section](../../docs/design/process-component-view-v2-20261021.md#scoped-verification-and-remaining-observations-2026-11-01)
records CPU069/Native070 and the exact archived cohort; the
[original six-AC map](../../docs/design/process-component-view-v2-20261021.md#original-issue-48-acceptance-map-after-pr213-2026-11-01)
records remaining scope. That archived native result includes diagnostic WIP and
must not be relabeled as a clean run of this public pin or later Core libraries.
The late AX samples are a publication-sampling evidence gap, not a demonstrated
defect or a new final-state gate. These results remain separate from the Table
logical AX-focus repair and old services timeouts; no repeat run or speculative
fix is requested by this documentation update.

#48 original AC3 remains open. Existing native wheel/Tab/AX and headless
anchor/lifecycle proofs keep their exact scopes; current-main native execution is
not inferred from source publication or CI builds. VoiceOver and external AgentKit
adoption are not additional #48 closing gates. #82 retains its own applicable
physical IME, candidate geometry, VoiceOver and performance requirements.
