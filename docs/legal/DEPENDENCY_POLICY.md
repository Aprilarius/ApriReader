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
  Open Library and FantLab HTTPS metadata adapters.
- `serde_json` — MIT OR Apache-2.0, for bounded provider responses and the
  local metadata cache.

No Google Books integration, API key, HTML scraper, background catalog client,
or unrestricted network endpoint is included. FantLab access is limited to its
public bibliographic API at `https://api.fantlab.ru/search-editions`; it adds no
library dependency, credential, executable, or background request.

No Steamworks SDK, telemetry, catalog, bundled TTS engine or speech model,
generated cover, font, or media binary is included.

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

Audiobook stage A6 adds no dependency. It extends the reviewed installer-owned
association set and reuses the same single-instance plugin, native queue,
bounded audio importer, Windows decoder boundary, and SQLite transaction.
ApriReader still bundles no codec pack and does not register DRM formats.

Audiobook stage A0 makes the already locked `windows` 0.61.3 crate a direct
Windows-only dependency under its MIT OR Apache-2.0 license. Only Foundation,
Media Core, Media Playback, Storage/Streams, and WinRT initialization bindings
are enabled. Audio decoding and system media controls remain operating-system
services; ApriReader bundles no FFmpeg binary, codec pack, DRM component,
speech model, or network media service.

Audiobook stage A1 adds no dependency. SHA-256 fingerprinting, SQLite
migrations, filesystem traversal, backups, and serialization reuse the
existing reviewed Rust stack. Import performs no network request and bundles
no metadata parser, codec, media executable, or DRM component.

Audiobook stage A4 enables Tauri's existing `tray-icon` feature. Its locked
transitive tray implementation is used only for local window and playback
lifecycle actions. CUE/M3U/M3U8 parsing is implemented in-tree and performs no
network access; it rejects remote references and directory escapes.

Audiobook stage A5 adds no package. It only enables the
`Devices_Enumeration` namespace on the already locked Windows bindings so the
existing MediaPlayer worker can list and select local audio-render endpoints.
Metadata, cover validation, SQLite statistics, and achievements reuse the
reviewed in-tree and locked dependency set; no codec, telemetry, speech, or
network-audio service is added.

Audiobook stage A7 adds no package. It enables Speech Synthesis, desktop SAPI,
COM, audio-format, and locale-name bindings on the already locked Windows
crate. Synthesis uses voices and services installed in Windows; ApriReader
bundles no voice, speech model, codec, executable, cloud client, or credential.
Generated RIFF WAV files are bounded and kept only in the app-local cache.

Audiobook stage A8 adds no package or native capability. Queue construction,
sentence/word boundaries, generation invalidation, active-word rendering, and
prefetch coordination are implemented in the existing TypeScript/React layer.
All synthesis still crosses the bounded A7 Windows command; no provider,
network permission, model, voice, or background service is introduced.

Audiobook stage A9 adds no package. It enables the Windows Credentials bindings
on the already locked `windows` crate and reuses the reviewed Rustls-only
`ureq`, `serde_json`, `base64`, and SHA-256 stack. The only new service endpoint
is the fixed official `https://api.elevenlabs.io` host. No ElevenLabs SDK,
binary, model, voice asset, analytics client, or provider credential ships with
ApriReader. Service availability, quota, pricing, and content processing remain
external to the application and require the user's own account and key.

Audiobook stage A10 adds no dependency, service endpoint, permission, binary,
or content asset. Voice presets and pronunciation rules use the existing
bounded WebView preference store and contain no credentials. Replacement and
source-offset mapping are implemented in the existing TypeScript layer.

Audiobook stage A11 adds no package, SDK, executable, model, voice asset, or
permission. It reuses the reviewed Rustls-only `ureq`, JSON, base64, SHA-256,
Windows Credential Manager, and native MediaPlayer stack. The only additional
service host is fixed to `https://texttospeech.googleapis.com`; user-supplied
keys, Google Cloud billing, quotas, generated audio rights, and provider terms
remain external to ApriReader.

Audiobook stage A12 adds no package, SDK, model, binary, asset, or permission.
It reuses the existing Rustls HTTP, Windows Credential Manager, SHA-256 and
MediaPlayer stack. Network access is limited to an allowlisted regional
`tts.speech.microsoft.com` host. Azure account, resource, key, region, quota,
billing, data processing and generated-audio rights remain user/provider scope.

Audiobook stage A13 adds no dependency, endpoint, executable, codec, model,
voice asset, or permission. Expressive JSON/SSML fields reuse the reviewed
provider clients. Cache accounting, exact-name deletion, incremental file copy,
and M3U8 generation use the Rust standard library and the existing explicit
save dialog. Exported provider audio rights remain governed by the user's
provider account and applicable terms.

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
remain local unless the user explicitly performs the Open Library/FantLab
metadata search.

Screen reader support adds no dependency or voice package. It uses standard
HTML accessibility semantics exposed by the existing Windows WebView to a
screen reader selected and configured by the user; the separate explicit A7
Read Aloud action uses the reviewed Windows speech bindings above.

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

The Windows release pipeline uses `@tauri-apps/cli` 2.11.4 under its existing
Apache-2.0 OR MIT license. This build-only update aligns the NSIS bundler with
the current Rust Tauri runtime and restores reliable bundle-type metadata
patching; it adds no application capability or packaged runtime dependency.
Authenticode credentials remain external to the repository. A signed build
uses a code-signing certificate already installed in the Windows certificate
store plus the certificate provider's timestamp service.

The existing `@tauri-apps/api` frontend package is aligned to 2.11.1 under the
same Apache-2.0 OR MIT license so its major/minor release matches the reviewed
Tauri 2.11 Rust runtime and CLI. This version alignment adds no capability,
network endpoint, or new package family.
