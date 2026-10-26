# Theme/Surface API rules 1.8.0

Additive to rules 1.7.0: all original 32 uses, states and compiled color roles remain identical.
New rest-only status-badge-info/running/succeeded/failed use their matching status-* solid and c-status-badge-foreground text/icon. Boundary/focus/marker/helper null. Solid text/icon require 4.5 normal, 7 increased; fill/parent >=3.
Skeleton/rest uses surface-2, primary-text text/icon, border boundary, other channels null. Boundary/fill and boundary/parent >=3.
All seven channels and exact bindings are required. Other states reject. Existing Palette/resolve_surface/SurfaceColors API is unchanged. Surface vocabulary follows source rules; independent consumers cannot alter shared rules.
No new token, runtime clock, Python build invocation or component migration. Offline generation and pure Rust verify the same policy. Rules/schema/contract/source/output locks are synchronized.

The complete prior [1.7.0 contract](../progress-shared-purpose-20261024/api-v1.7.0.md) remains in force, including progress-track/rest and all preceding guards. This document adds only the five passive purposes. The seven core surfaces are required; explicitly authored additive surfaces remain supported by the source compiler. Unknown resolver combinations and consumer attempts to substitute shared Surface rules reject.
