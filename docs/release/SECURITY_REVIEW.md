# Security review — Stage 9

Review date: 2026-07-29; A13 TTS assets boundary updated 2026-08-08. Scope: public
GitHub text/audio candidate, optional ElevenLabs/Google Cloud/Azure BYOK, and
the protected Steam adapter boundary.

## Confirmed boundaries

- Book and package content is parsed by bounded Rust adapters. It is never
  inserted as authored HTML, executed as script, or allowed to initiate a
  request.
- Source books remain read-only and in place. Generated covers, reader assets,
  backups, recovery quarantine, metadata cache, and language packages stay
  under app-local data.
- Network access exists only behind explicit metadata, translation handoff, or
  optional ElevenLabs/Google Cloud/Azure BYOK actions. Cloud speech requires
  provider selection,
  a user-supplied key, and first-send disclosure; no text is sent during
  startup, import, scanning, reading, or local Windows narration. The public
  build has no telemetry, Steamworks SDK, bridge, App ID, catalog, speech
  model, provider key, or background update client.
- The ElevenLabs key is stored as a Generic Credential in Windows Credential
  Manager and never crosses the Tauri serialization boundary. Rust uses only
  the fixed official API host, bounded voice/JSON/audio responses, validated
  voice identifiers and MP3 signatures, exact-alignment checks, and a 64-file
  app-local cache. Errors never contain the credential header.
- A10 voice presets contain no key or token. The local pronunciation store is
  capped at 20 presets, 100 rules, and 64 KiB of persisted JSON. Replacements
  operate on synthesis copies only, enforce whole-word boundaries and a 2,000
  UTF-16 post-expansion limit, and cannot alter or write source books.
- The Google Cloud key uses a separate Generic Credential and native header.
  The fixed v1 voices/synthesis host, validated language and voice identifiers,
  4,800-byte input ceiling, bounded JSON/base64/MP3 data, signature checks, and
  separate 64-file cache are enforced by Rust. The key cannot enter request
  URLs, WebView storage, voice presets, or the ElevenLabs credential slot.
- Azure Speech accepts only 33 reviewed region identifiers and constructs a
  fixed Microsoft Speech suffix. Its independent credential never enters the
  WebView; SSML metacharacters are escaped and text/SSML/audio/cache are bounded.
- A13 expressive values are finite and range-checked in Rust and enter provider
  cache digests. Export accepts only canonical direct children with exact TTS
  names, limits sessions/parts/bytes, copies incrementally, and publishes an
  M3U8 plus a unique media directory. Cache clearing never recursively removes
  the cache root; cancellation removes only its registered partial directory.
- Tauri grants only core defaults plus explicit open/save dialogs. The asset
  protocol is limited to app-local covers, imported fonts, and reader cache.
- CSP rejects dynamic script evaluation and unrestricted sources. Inline style
  remains allowed because reader typography uses React style properties; no
  book-authored style crosses into the WebView.
- Steam bridge loading is compile-time gated and uses the documented protected
  ABI. Protected artifacts are denied by the release audit.
- User fonts require an explicit file choice, a supported extension, a matching
  font signature, and a 24 MB size limit. Only a SHA-256-named app-local copy is
  exposed to the WebView; fonts are never downloaded or loaded from a book.

## Failure and recovery

SQLite migrations are transactional. Startup runs `PRAGMA quick_check`.
If the active database is invalid, only an independently checked app-generated
backup can replace it. The damaged database and sidecars move to an app-local
quarantine first, making the recovery reversible. If no valid backup exists,
startup fails instead of creating an empty library over user data.

## Automated gate

Run `pnpm release:audit`. It fails when reviewed direct dependencies,
capabilities, asset scopes, CSP constraints, protected-file rules, or the
lockfile-derived SBOM are stale. Candidate packaging also records the source
tree state and a hashed source manifest, and aborts if the source snapshot
changes during the build.

## Residual release gates

- Complete keyboard, Narrator, 100–250% Windows scaling, forced-colors, and
  malformed-format manual passes on Windows 10 and Windows 11.
- Keep the generated transitive license report current and review every
  metadata-only declaration before a future dependency upgrade.
- Run the protected Steam online/offline/Overlay matrix in Steamworks.
- Record closed-beta consent, candidate hash, defects, and go/no-go decision.
