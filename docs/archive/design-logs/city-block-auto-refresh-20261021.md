# Viewer automatic display cadence — 2026-10-21

The Viewer previously defaulted to a fixed 60 FPS city animation budget despite the platform requesting display-synchronized updates up to 120 Hz. It now defaults to consuming every native display-link callback. No additional timer is used, and callback timestamps still drive animation time independently of draw rate. H toggles a 60 FPS city animation / automatic rotation cap; a second press restores automatic cadence. The existing --benchmark-120hz CLI remains a fixed 120 Hz test budget.

Native HUD observations (short functional checks, not a fresh 60-second benchmark): built-in Retina 120.0 FPS by default; H cap 60.0; external U2790B 30.0 in the current display/system configuration; returning to built-in restored 120.0 without changing mode. These are submitted drawable frame counts, not proof of display scanout or external display maximum refresh specification. Screenshots and logs are under target/verification-receipts/city-block-auto-refresh-20261021. Background benchmark mode was enabled only for observations and then disabled; window returned to built-in.

Validation: four cadence/FPS tests, all-targets strict Clippy, native Release build. Tests cover successive native callback streams at 120/60/90/120 Hz and cap-to-auto transitions. No shared platform source or unrelated WIP changed.

Application SHA-256: 5eaa04402ceb11b545f67335b72753e2472f04b7ec2b5a32cd03bd7c557502bd.

Cleanup: 29.60 MiB unused Viewer incremental cache removed after target lock and open-file checks; source, snapshots, logs, screenshots, executable and dependency lock retained.
