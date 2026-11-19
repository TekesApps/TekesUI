# City Block variety and runtime boundary

Current-worktree follow-up; previous unfinished changes and unrelated WIP retained, no commit created.

## Result

- Four existing buildings have deterministic independent HVAC placement/count/dimensions/orientation, with parapet and service gaps. Counts are 3/2/1/2. Door and awning palettes differ between all four buildings.
- Original streamlined electric sedan with tapered body, sloped glazing, dark panoramic roof and flush handles; no brand assets/logos. Police, ambulance, fire and refuse vehicles use independent indexed prototypes, retaining route-scale physical dimensions. Four service types plus six passenger cars share two traffic routes.
- Eight volumetric pedestrian prototypes use rounded torsos/shoulders/heads and thick limbs. Skin, clothing and hair are vertex colors in the models, rather than a single tint applied to the entire person. Children have separate smaller placement scales; seniors use grey hair and slower strolling.
- Walking, strolling, listening with actual headphone geometry, and jogging use different speeds, stride amplitudes, arm geometry and poses. Jogging uses bent arms and a static forward torso lean composed from existing Swing primitives. Each actor retains individual phase/speed/variation. Current mean speeds span approximately 0.73–3.29 m/s, plus bounded variation.

## Ownership

`tekes-ui-scene3d` is the lightweight reusable GPU runtime: buffers, indexed rendering, skin/morph, routes, swing, waves, float/trail and finite reveal/removal. This change adds no city-specific shader or renderer behavior.

`tekes-ui-city` is data-only: clock, recipe/placement, appearance index and Activity. The host adapter `tekes-ui-3d::district` translates these values into existing generic GPU instances once. The offline GLB authoring pipeline owns roofs, palettes and visual prototypes. CPU handles data/configuration and UI policy; GPU evaluates continuous geometric motion. Full physical dynamics, collision avoidance and pathfinding are separate future capabilities, not claims of the current route demo.

## Validation

- Actual exported GLB physical-scale/normals audit passes, covering all five vehicle types, all eight people, children/adult separation, volumetric depth, complete window stories, non-overlapping rooftop units and distinct palettes. Generator repeat produces byte-identical GLB/JSON.
- `cargo +1.95.0 test -p tekes-ui-city -p tekes-ui-3d -- --include-ignored`: 9 passed. GPU tests independently isolate all four service vehicle types and strolling/listening/jogging; service motion is observed through multiple route times to account for building occlusion. Existing reset, scaffold/hoarding/crane removal and 120-frame zero vertex/pose/instance-upload assertions pass.
- `cargo +1.95.0 clippy -p tekes-ui-city -p tekes-ui-3d --all-targets -- -D warnings`: passed.
- Release package rebuilt, strict signature verification passed. Native Retina window inspected after restart. Separate people/vehicle model views inspected along with the full city scene.
- GLB: 4,345,512 bytes. Models are resident/shared; no per-frame geometry generation or new shader pipeline was introduced. No new CPU percentage or full-workspace test claim.
- Logs, dimension audit and previews: `/tmp/tekesui-city-variety/`.
