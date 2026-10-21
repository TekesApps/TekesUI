# City Block physical proportions and site clearance

User feedback identified two related inconsistencies: adult/car/tree dimensions were large relative to the apparent window stories, and the ground story/doors were disproportionately tall. The old atlas represented a window row every 0.3125 scene units (1.25 m under the district's four-metre authoring scale), while structural stories were about 3.2 m. Shrinking actors alone would preserve the architectural error.

The district now explicitly authors ordinary stories at 3.2 m and ground stories at 3.8 m. Upper facade boxes begin above the ground story and their UVs contain exactly one atlas window row per story. Whole horizontal window bays avoid cut-off windows at building corners. Ground-floor doors are 2.24 m high and glazing 2.6 m. Construction enclosure follows the same story dimensions. This is the district's authoring convention; no general renderer unit or importer behavior changed.

Exported GLB checks measured adults 1.63–1.78 m, passenger cars 4.17–4.54 m long and 1.43–1.56 m high, and street trees 3.24–5.37 m high. Cars retain indexed geometry and corrected inverse-scale normals after reshaping. Benches, sidewalk curb heights and river railing dimensions were adjusted to the same scale. People retain speed variation; gait frequency follows stride scale after resizing.

`construction-hoarding` contains the perimeter and stored supplies separately from `construction-lot`. It retires over two seconds following scaffold completion (63.8–65.8 s); the permanent foundation and completed building remain. Existing generic GPU Removal3D is reused, with no new shader or per-frame mesh/instance upload.

Validation in the current full-WIP checkout:

- `python3 crates/tekes-ui-3d/assets/check_city_block.py` passes actual exported positions/UVs/normals and physical size bounds. No Pillow runtime dependency for checking.
- `cargo +1.95.0 test -p tekes-ui-3d -- --include-ignored`: 7 passed, including real GPU pixel comparisons for hoarding dismantling/no remnants, completed building persistence, reset, shared composition and zero geometry/instance uploads across 120 frames.
- `cargo +1.95.0 clippy -p tekes-ui-3d --all-targets -- -D warnings` passes.
- Release package builds and strict codesign verification passes. Native Retina preview inspected for architectural/actor proportions; snapshots at 65 and 80 seconds inspected for site clearance.
- Logs, exported dimension receipt and snapshots: `/tmp/tekesui-city-proportions/`.

Current worktree and unrelated WIP retained; no commit created. This is targeted validation, not a full-workspace test claim or new CPU benchmark.
