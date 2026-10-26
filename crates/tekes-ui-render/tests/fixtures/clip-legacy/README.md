# Frozen unshaped rendering baseline

These ten source fixtures are byte-for-byte copies of the original #80 Phase 2
baseline PNGs, not regenerated from the candidate. They cover Light/Dark and
0.5/1/1.25/1.5/2 scaling, all original primitive paths in clip_legacy_fixture.rs.
The test decodes each original PNG and compares exact RGBA bytes on every checkout,
without relying on a directory named candidate. New captures use the configured
SSD target/output directory. Historical evidence keeps its original source identity.

Original evidence: design/reviews/panel-shared-clip-implementation-20261024/phase2/gpu-legacy.
