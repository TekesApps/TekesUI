# Reading stream consumer verification — 2026-10-19

Root owns this additive independent consumer fixture. Existing ReadingController, TextSurface, TextFieldContent, ContentHost and typed action delivery implement the behavior; no shared abstraction or new design token was added. The missing piece was a reproducible native consumer that supplies arrivals independently of the editor and exposes functional readback.

The API reading contract, all 136 independent consumer tests and strict all-target Clippy passed. Two new actual CoreText integration cases cover paused arrivals, historical anchors, text/width reflow, independent draft selection/focus, explicit return to latest and repeated typed actions.

The reading small host ran on U2790B external display at @2x, Dark. Native Metal screenshots and readback show append following the tail, history remaining at UTF16 1206 / within-line 10 across 150% text and 480pt width, and keyboard Shift+Tab/Return restoring the latest position. A second recorded stream retained draft focus 8802 while appended count increased 7→8→9 and pending fell 2→1→0, with the same historical anchor and draft content. The inactive action rejection is a separate negative check.

Evidence: `design/reviews/reading-stream-native-20261019/`, immutable build identity and cleanup: `design/reviews/component-runs/reading-stream-20261019/receipt.json`. The native close exited the owned process; the standard finish operation removed the round binaries/App and retained source, evidence and shared cache. No post-close AX-empty claim is made.

Gold7 review is limited to this reading scene: explicit latest action, preserved reading intent and draft, usable reflow and centered actions. No performance, VoiceOver, whole Gallery or #22 mixed-content transaction acceptance is claimed. #10 still needs exact delivered source identity and final independent review. Prior frozen audit and AgentKit inputs remain unchanged.

#11 separately passed three actual process executions using existing DraftJournal, the consumer file adapter and the actual RichDialog TextFieldContent/CoreText: failed save preserves file/editor, retry retains the frozen request, late/duplicate acknowledgements preserve a newer edit, and a new process restores it. See `design/reviews/draft-process-replay-20261019/receipt.json`; this is process/storage/editor evidence, not a native window replay.
