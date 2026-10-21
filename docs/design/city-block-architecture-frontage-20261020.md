# Original City Block architecture, frontage and ground-floor detail

Current worktree remains on b8999fe with all earlier and unrelated WIP retained. No commit created. Game-derived experiment assets remain outside the repository.

## Model changes

- `city_architecture.py` independently authors three original 512×512 facade atlases, relief window frames/sills, belts, cornices, quoins, apartment balconies and courtyard/service windows. Office/hotel/shops have real upper-storey setbacks. Story heights stay 3.2 m above 3.8 m ground floors.
- `city_shopfronts.py` independently authors structural cores and four aperture walls. Doors and displays sit behind the exterior plane with real recesses. Each street frontage has detailed door leaves, handles, frames, transoms, canopies, brackets, sign fascia and paving. Cafe/office/hotel/market/lobby recipes select different shallow displays and furniture. Textures and signage are original procedural assets.
- The west apartment, north office, east hotel, south shops and west construction lobby face their assigned roads; their canopy, door and shopfront share the same local frontage frame. Footways connect to corresponding sidewalks. Non-entry elevations have recessed windows rather than extra doors.
- Office/hotel garages are separate 3.04 m wide × 2.4 m high apertures, parking signs and sloped connections to the road edge. This is visual geometry; the current traffic route does not drive cars through garages.
- Sedan body corners, curved windscreen/roof and wheels use more segments and smooth normals. Vehicle dimensions are unchanged. The native district already uses backing-pixel resolution and 4× MSAA; this change refines the model rather than multiplying render-target resolution. Ambulance crosses are red on light panels.

## Validation and cost

- Standard-library exported GLB audit passes: metric dimensions, full facade stories/bays across stepped segments, entrance directions, recessed door geometry, road connections, garage dimensions, setbacks, unit normals and medical-cross color.
- Doorway probes intersect actual indexed triangles and verify that the visible door is behind the exterior wall plane, with no solid wall sealing the aperture.
- Generator repeat produces byte-identical GLB and recipe. Current GLB: 11,417,356 bytes, 207,770 vertices, 108,162 indexed triangles. Additional geometry is immutable and resident; this change adds no per-frame vertex/instance upload.
- Three facade textures consume approximately 4 MiB as RGBA8 with mipmaps, versus approximately 8 MiB for the previous three 512×1024 atlases. Masonry, signage, foliage, other textures, geometry and render targets are additional; this is not a total-VRAM claim.
- Cargo 1.95.0 targeted 3d suite: 8 passed, including real GPU motion, reset, staged site clearance, shared composition and 120-frame zero geometry/instance uploads. Clippy with all targets and warnings denied passed. No full-workspace or new CPU benchmark claim.
- Release app rebuilt, strict signature verified. Native full-district window refreshed. Independent native storefront preview uses the same original authoring functions and permits close inspection of first-floor geometry. The preview's roof slab only closes the inspection module; upper stories remain in the full district.
- Logs/audit/snapshots and preview app: `/tmp/tekesui-city-frontage/`.
