# Security review — Stage 9

Review date: 2026-07-29. Scope: public GitHub text-release candidate and the
protected Steam adapter boundary.

## Confirmed boundaries

- Book and package content is parsed by bounded Rust adapters. It is never
  inserted as authored HTML, executed as script, or allowed to initiate a
  request.
- Source books remain read-only and in place. Generated covers, reader assets,
  backups, recovery quarantine, metadata cache, and language packages stay
  under app-local data.
- Network access exists only behind the explicit Open Library search/apply
  workflow. The public build has no telemetry, Steamworks SDK, bridge, App ID,
  catalog, TTS, model, or background update client.
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
