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

External translation adds `@tauri-apps/plugin-opener` and
`tauri-plugin-opener` — MIT OR Apache-2.0. Their sole reviewed use is opening
fixed HTTPS URLs for Google Translate and Yandex Translate after an explicit
selection action and first-use disclosure. The capability scope permits no
other URL host. ApriReader bundles no translation model, language package, or
external-service code.

Stage 8 adds no dependency. The public build does not link or redistribute the
Steamworks SDK. `steam-build` is an empty compile-time feature that enables an
adapter for a separately supplied protected bridge ABI. The following files
must never be committed: `aprireader_steam_bridge.dll`, `steam_api64.dll`,
`steam_appid.txt`, App IDs, publisher keys, or Steamworks headers/libraries.

Windows file-association support adds `tauri-plugin-single-instance` —
MIT OR Apache-2.0. It forwards Windows shell activation arguments to the
existing process and adds no network, file parsing, registry-writing, or
telemetry capability. Installer-owned associations use Tauri's existing
bundler.

Stage 9 otherwise adds no runtime or build dependency. The reviewed direct
dependency set is enforced by `scripts/release_audit.py`, and the exact
Cargo/pnpm lockfile inventory is stored as
`release/aprireader-sbom.cdx.json`. A changed manifest or lockfile requires
regenerating the SBOM and repeating the license review before a candidate may
be distributed.

The optional local profile and first-launch welcome screen add no dependency.
They use React and app-local WebView storage already present in ApriReader and
perform no operating-system identity lookup or network request.

The NSIS candidate is produced by Tauri's existing bundler and adds no package
dependency. Its evidence folder includes the Apache-2.0 license, notices, and
current SBOM. Code signing remains an external release credential and is never
stored in the public repository.

Reader polish adds no executable dependency. ApriReader bundles Literata, Lora,
Merriweather, Source Serif 4, Charis SIL, and IBM Plex Serif under the SIL Open
Font License 1.1. Only the reviewed variable or static faces required by the
family/style/weight selectors are packaged, and every complete OFL text ships
under `licenses/fonts`. A font chosen by the user remains private local content,
is never redistributed by ApriReader, and is copied only after the documented
format, signature, and size checks.

Genre metadata and the expanded achievement registry add no dependency.
Parsing uses the existing bounded XML and JSON adapters, and all genre values
remain local unless the user explicitly performs the existing Open Library
search.

Screen reader support adds no dependency, voice package, speech model, or TTS
engine. It uses standard HTML accessibility semantics exposed by the existing
Windows WebView to a screen reader selected and configured by the user.

High-scaling support adds no dependency. It uses the existing CSS layout,
overflow, media-query, and semantic HTML capabilities of the Windows WebView.

Windows forced-colors support adds no dependency, palette package, or external
service. It uses system color keywords, CSS media queries, and existing HTML
accessibility semantics provided by the Windows WebView.

Candidate source provenance adds no application dependency. Git is a
release-environment tool only; it enumerates tracked and non-ignored source
files, while SHA-256 hashing uses PowerShell and .NET already present in the
Windows build environment.

The RC build profile adds no dependency. It is a stricter invocation of the
reviewed candidate builder and requires a clean Git tree before compilation.

The GitHub 1.0 profile adds no dependency and contains no Steamworks file.
Its initial NSIS installer is intentionally unsigned under the dated product
owner policy in `docs/release/GITHUB_1_0_RELEASE_POLICY.md`. The generated
`release/THIRD_PARTY_LICENSES.md` inventories the Windows production graph and
collects every packaged license or notice text plus explicit metadata-only
declarations; the release audit rejects a stale report.
