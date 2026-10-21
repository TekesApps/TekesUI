# Issue23 CI closeout - local proposal, not published

Source identity: 53228e613cbd4c5da0d398a45c512c10fdab93a81419497aa2515eef11d6a53a
Base public commit: 61ddf7c68fccd819e20a01ba3104a8a0f8d7e64f
8-file followup;20 cumulative files relative to431 (6 new file paths beyond prior14file authorization).

Fix1: restore the existing generated public binding for TextSurfacePresentation and update its public-input hash. The exact accepted2f090 generator output is reused; coverage assertion is preserved.
Fix2: import only5 already-published PR24 CI dependency files: lockfile dev edge, Gallery dev dependency, isolated historical language-profile test, independent CTLine shape metrics test and reference-language JSON. No live UI font policy change, global settings writes or golden PNG replacement.
Fix3: make CI workspace Clippy use --locked, matching the tested deterministic command.

923 workspace tests passed,1existing ignored,3timing tests explicitly skipped.134consumer tests and both strict all-target Clippy passes. Reproduced the prior431CI exact pixel652,145 failure under an English process; the published reference-profile fix passes all24 original golden comparisons. Patch replay from public61ddf7c reproduces every source file exactly.

All public production libraries and native consumer source/lock are unchanged from the accepted122 candidate. Original8active external-native receipts retain their original source identity. No repeated hardware/IME/VO claim or performance measurement.

See delta.json for all8files, followup.patch, checks-with-ci.json, English before/after logs, ci-dependency-provenance.json and manifest.json. No new followup commit/push, main merge, tag or Issue closure has occurred. Publication beyond14files awaits explicit authorization.
