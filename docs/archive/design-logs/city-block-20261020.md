# Living City Block — 2026-10-20

The current Gallery worktree preserves its unrelated WIP. The development results below describe the full working tree before scope separation; isolated commit validation is recorded in `city-block-commit-validation-20261020.json`. Health-cabin assets, host integration and BLEND support are excluded from this commit.

## Scope and boundaries

- Data/clock: `tekes-ui-city`, serde only, no rendering dependency.
- Host adapter: `tekes-ui-3d/src/district.rs`; standalone `TekesUI City Block.app`.
- Generic rendering: named-node GPU instances, closed route progress, wind/waves, pivot swing, construction reveal, GPU mipmaps. No city policy in the renderer.
- Offline original asset generator: four finished buildings, one construction lot/frame, slewing tower crane, 10 cars, 28 pedestrians, 18 trees across three leaf/crown types, waterfront and pond. GLB is a reusable mesh library; adjacent JSON supplies placement/motion. Runtime needs neither Pillow nor third-party model assets.
- Construction reveals floors smoothly about every eight seconds, stops at full height, and restarts on Reset. It clips a prebuilt frame; no watertight moving cap or structural simulation. Crane hook clears the completed roof.
- Scaled-down actors/trees, individually varied pedestrian speeds and phase-offset bounded vehicle acceleration. No crowd/traffic collision solver.
- GPU mipmaps are generated once, linear-light, with trilinear and 8x anisotropic sampling; foliage is alpha MASK. Four-sample MSAA remains opt-in for this demo. No real-time shadows/PBR.

## Validation

`cargo +1.95.0 test -p tekes-ui-city -p tekes-ui-scene3d -p tekes-ui-3d -- --include-ignored --nocapture`: 25 passed, including real wgpu tests.

`cargo +1.95.0 test -p tekes-ui-gallery scene3d -- --include-ignored`: 6 passed.

`cargo +1.95.0 clippy -p tekes-ui-city -p tekes-ui-scene3d -p tekes-ui-3d --all-targets -- -D warnings`: passed.

Isolated actual image differences: car 6625 pixels, pedestrian 2170, tree 4303, water 20968, construction 3460, crane boom 1584. Construction additionally produces identical images at 120/240 seconds when other animations are disabled, proving growth stops. City GPU geometry residency: 2,959,040 bytes (excludes materials/targets/instances). One instance/route configuration uploads 22,336 bytes; the isolation test configures 14 times, totaling 312,704 bytes. The subsequent 120 frames add zero vertex, pose or instance uploads and reuse buffers. Dynamic frames update the 16-byte environment uniform and 128-byte camera uniform. This is a buffer-reuse check, not a sustained frame-rate or native CPU benchmark.

Regression coverage includes GPU skin/morph and numerical normals, glass alpha, reversed-Z, MSAA resize, pause/inactive/reduced-motion clock policy, reset, stable ImageId across four demos and Gallery lifecycle/adaptive-resolution paths.

Release app built, ad-hoc signature verified, and the updated native Retina window visually inspected. Offscreen snapshots show distinct initial/24-second construction stages. Native sustained animation throughput was not measured.

Baseline hash comparison preserved protected 3D/health-cabin WIP; two controls files changed externally during this run (`composition.rs`, `owned_paint_contract.rs`) and were left untouched. No global capability governance changes were made.


## Scaffold completion extension

Independent structural frame, enclosing facade and outside scaffold nodes now cover the full finite sequence: grow floors, finish the enclosure, hold, then dismantle scaffold from the top down. The building remains intact. Default recipe starts dismantling around 49 seconds and finishes around 63 seconds; 46/55/80-second GPU snapshots exercise completed-with-scaffold, partial removal and fully removed states. `StagedReveal` is a generic GPU lifecycle with no per-frame geometry/instance upload and unchanged 112-byte instance records. New real GPU tests compare removal with an explicitly hidden scaffold and verify shared-frame equivalence at all stages. Current geometry residency is 3,414,640 bytes; each instance/route configuration uploads 22,560 bytes; the full isolation test configures 19 times. These replace the older asset counts above.
