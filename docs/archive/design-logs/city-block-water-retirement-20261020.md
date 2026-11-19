# City Block: waves, navigation and crane retirement

Current-worktree follow-up to b8999fe; no commit created. Existing unrelated WIP is retained.

- Original procedural motorboat and indexed wake geometry; GLB is 1,290,956 bytes (+28,864).
- River wave amplitude increases from 0.032 to 0.065, with stronger crest shading. River-bed top is -0.325, below the lowest possible trough (-0.2275), removing exposed dark patches.
- Generic GPU Float follows world-wave height/slope rigidly; SurfaceTrail samples earlier route arc lengths per vertex and follows the same wave field. No CPU buoyancy simulation or per-frame mesh uploads. These are analytic visual effects, not a fluid simulation.
- Generic Removal3D composes independently with crane swing. Scaffold removal finishes first; crane retirement runs approximately 64.8–67.8 seconds. Completed building remains. Instance layout becomes 128 bytes (+16 bytes per instance); all configuration stays immutable during animation.
- Existing shared clock handles pause/reset/inactivity/Reduced Motion.

Validation in the active full-WIP checkout, Cargo 1.95.0:

- Render/city/scene3d/3d tests: 113 passed (112 before the added navigation-boundary test, then 29 motion tests rerun). Gallery scene3d: 6 passed. No claim of a workspace-wide test run.
- Real GPU pixel comparisons isolate water, boat, wake and crane retirement. The retired crane matches an explicitly omitted crane; scaffold removal has the same check. Reset reproduces the initial image. Existing 120-frame assertions retain zero vertex/pose/instance uploads and cached draw reuse.
- Navigation route covers repeated loops with hull clearance from river boundaries; invalid boat route/speed and invalid removal timing rejected.
- Clippy on render/city/scene3d/3d with all targets and warnings denied passed.
- Release app package and strict codesign verification passed. Native Retina window inspected after relaunch: moving boat/wake, flowing waves, no dark river-bed intersections. Offline snapshots at 5/66/80 seconds also inspected.
- Logs and snapshots: /tmp/tekesui-city-water/. No new CPU percentage or benchmark reduction claimed.
