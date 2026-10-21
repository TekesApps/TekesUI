# Portable three-process draft replay

Run on macOS with the repository toolchain. All Cargo path dependencies resolve relative to this export; no original checkout or user directory is required.

Use one build cache and compact development settings as in docs/component-review.md. Build with `cargo build --manifest-path design/reviews/draft-process-portable-20261019/consumer-source/Cargo.toml --locked --offline`. Run the resulting `tekes-ui-draft-process-replay` executable three separate times with mode `prepare`, `retry-ack`, `verify`, each followed by the same fresh caller-owned JSON file path. The parent data directory must exist. Retain all three JSON outputs as evidence; remove only the harness executable after completion.

The test exercises failed save, immutable retry, late/duplicate acknowledgement and editor reconstruction in three processes. It uses the actual existing RichDialog TextFieldContent and CoreText shaping with a no-op draw backend. This is storage/public-editor evidence, not native window or GPU acceptance. Historical absolute-path harness and receipts remain unchanged.
