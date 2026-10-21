# App protocol (experimental)

Data-only, serde messages for Host/module processes. No UI, native pointer, closure, trait object or wgpu dependency. Defines protocol version, input, toolbar command descriptions and frame metadata. OS surface rights travel out of band; receiver-local port values are not serialized.

Current prototype uses viewport/sequence validation. The architecture document's complete identity generations, capture/IME and joint switch contracts remain design requirements, not implemented fields. Rust source compatibility does not imply IPC compatibility.
