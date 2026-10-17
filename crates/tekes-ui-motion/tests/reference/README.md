# Apple numeric reference

Only numeric CSV / JSON fixtures are retained in this pure Rust source tree.
The original measurement script was preserved outside the project at
`/tmp/tekesui-apple-motion-reference/apple_oracle.swift`; it is not a build input.
The Swift compiler is not needed by the crate or its tests.

The 63 samples come from public SwiftUI Spring / UnitCurve APIs under macOS SDK
26.5, Swift 6.3.3. See docs/design/apple-motion-defaults.md for mappings and scope.
CSV is the zero-dependency Rust test fixture; JSON contains the same numbers for
inspection. A deleted external temporary file would not affect tests.
