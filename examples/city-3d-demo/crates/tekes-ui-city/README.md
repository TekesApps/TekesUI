# TekesUI City

Data-only district recipes and a deterministic clock; no wgpu, window, UI, glTF loader or per-frame simulation dependency. The JSON describes closed routes, actor placement/speed/phase, tree species, wind/waves and an optional construction site. Configuration is validated before the host mutates rendering state.

The demo adapter in `tekes-ui-3d/src/district.rs` maps these values to generic `tekes-ui-scene3d` instance batches. Assets contain reusable, separately named meshes. Cars follow separated lanes with phase-offset bounded acceleration; pedestrians have different speeds and small lateral offsets. This is an illustrative animation, without traffic lights, collision avoidance, pathfinding or crowd AI.

Construction is a finite, smooth floor-by-floor reveal of a prebuilt frame; it keeps each floor's dimensions and stops at its configured height. A separate facade follows the frame. Outside scaffold posts, walkways and braces grow with the floors, hold until enclosure is complete, and dismantle from the top down. The building remains intact. A separate crane mesh slews around a pivot. It is a visual construction sequence, not a structural or crane simulation.

`CityClock` is host-owned. Pause, inactive windows and Reduced Motion freeze time without catchup. Reset returns to the initial construction stage. CPU route interpolation is a tools/test reference only; the interactive renderer evaluates routes, limbs, foliage, water and construction on GPU.
