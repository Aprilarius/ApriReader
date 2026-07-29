# Dependency policy

Dependencies require a known SPDX license compatible with Apache-2.0 and
commercial distribution. MIT, BSD-2-Clause, BSD-3-Clause, ISC, Apache-2.0, and
Zlib are normally acceptable after source verification.

GPL, AGPL, unknown, custom source-available, and non-commercial licenses are
not accepted. LGPL and MPL require written review. Every release updates
`THIRD_PARTY_NOTICES.md` and produces an SBOM.

Stage 1 adds only:

- `@tauri-apps/plugin-dialog` — MIT OR Apache-2.0.
- `base64` — MIT OR Apache-2.0.
- `quick-xml` — MIT.
- `sha2` — MIT OR Apache-2.0.
- `zip` with only its `deflate` feature — MIT.

Stage 2 enables quick-xml's `encoding` feature. Its added `encoding_rs`
dependency is MIT OR Apache-2.0 and provides declared legacy XML encoding
support for FB2. No rendering engine or book-authored JavaScript dependency was
added.

Stage 3 adds no dependencies. Full-text search uses SQLite FTS5 already bundled
through `rusqlite`.

Stage 4 adds:

- `pdfjs-dist` — Apache-2.0, used only by the isolated PDF canvas viewer.
- `rars` — MIT OR Apache-2.0, a pure-Rust reader for CBR archives.

DOCX parsing uses the existing permissively licensed `zip` and `quick-xml`
dependencies. CBZ uses the existing `zip` adapter. No external archive
executable, office runtime, or book-authored script engine is included.

Stage 5 adds:

- `ureq` — MIT OR Apache-2.0, with Rustls and JSON features for the fixed
  Open Library HTTPS adapter.
- `serde_json` — MIT OR Apache-2.0, for bounded provider responses and the
  local metadata cache.

No Google Books integration, API key, HTML scraper, background catalog client,
or unrestricted network endpoint is included.

No Steamworks SDK, telemetry, catalog, TTS engine, model, generated cover, font,
or media binary is included.

Stage 6 adds:

- `ort` — MIT OR Apache-2.0, compiled with `std` and the ONNX Runtime 1.24 API
  contract. Its verified CPU runtime is bundled by the dependency build; user
  packages cannot supply native libraries.

Imported language packages accept only MIT, Apache-2.0, BSD-2-Clause,
BSD-3-Clause, ISC, or Zlib SPDX identifiers. The user-provided package must
include source URL and attribution. ApriReader does not redistribute imported
payloads.

Stage 8 adds no dependency. The public build does not link or redistribute the
Steamworks SDK. `steam-build` is an empty compile-time feature that enables an
adapter for a separately supplied protected bridge ABI. The following files
must never be committed: `aprireader_steam_bridge.dll`, `steam_api64.dll`,
`steam_appid.txt`, App IDs, publisher keys, or Steamworks headers/libraries.

Stage 9 adds no runtime or build dependency. The reviewed direct dependency set
is enforced by `scripts/release_audit.py`, and the exact Cargo/pnpm lockfile
inventory is stored as `release/aprireader-sbom.cdx.json`. A changed manifest
or lockfile requires regenerating the SBOM and repeating the license review
before a candidate may be distributed.

The NSIS candidate is produced by Tauri's existing bundler and adds no package
dependency. Its evidence folder includes the Apache-2.0 license, notices, and
current SBOM. Code signing remains an external release credential and is never
stored in the public repository.

Reader polish adds no dependency and bundles no additional font. A font chosen
by the user is private local content, is never redistributed by ApriReader, and
is copied only after the documented format, signature, and size checks.

Genre metadata and the expanded achievement registry add no dependency.
Parsing uses the existing bounded XML and JSON adapters, and all genre values
remain local unless the user explicitly performs the existing Open Library
search.
