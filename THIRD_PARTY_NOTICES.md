# Third-party notices

ApriReader uses open-source dependencies distributed through Cargo and pnpm.
Their exact versions and transitive dependency graphs are locked in
`Cargo.lock` and `pnpm-lock.yaml`.

Stage 1 directly uses Tauri and its dialog plugin (MIT OR Apache-2.0), React
(MIT), rusqlite/SQLite (MIT/Public Domain), serde (MIT OR Apache-2.0), thiserror
(MIT OR Apache-2.0), base64 (MIT OR Apache-2.0), quick-xml (MIT), sha2
(MIT OR Apache-2.0), and zip (MIT).

Stage 2 enables quick-xml's optional `encoding_rs` dependency (MIT OR
Apache-2.0) for XML encoding declarations.

Stage 3 adds no third-party packages and uses the FTS5 module in the already
bundled SQLite library.

Stage 4 adds Mozilla PDF.js through `pdfjs-dist` (Apache-2.0) and the pure-Rust
`rars` archive library (MIT OR Apache-2.0). DOCX and CBZ reuse the existing
quick-xml and zip dependencies.

Stage 5 adds ureq (MIT OR Apache-2.0) with its Rustls HTTPS stack and
serde_json (MIT OR Apache-2.0) for explicit Open Library requests and cached
responses.

Stage 6 adds ort (MIT OR Apache-2.0) and the Microsoft ONNX Runtime (MIT) CPU
runtime. ApriReader does not bundle models or dictionary data.

Stage 9 adds no dependency. The release inventory generated from both lockfiles
is `release/aprireader-sbom.cdx.json`; `pnpm release:audit` verifies that it is
current and that the reviewed direct dependency set has not changed. Complete
transitive copyright text remains a mandatory public-binary release gate in
`docs/release/SECURITY_REVIEW.md`.
