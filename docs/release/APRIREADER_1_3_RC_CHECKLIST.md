# ApriReader 1.3.0 RC1 checklist

Date: 2026-08-08

## Candidate scope

- Audiobook stages A0-A13: bounded import, watched folders, player, progress,
  CUE/M3U/M3U8, chapters, bookmarks, sleep timer, tray behavior, output-device
  selection, metadata, listening statistics, achievements, and Explorer
  associations.
- Local Windows read-aloud plus optional ElevenLabs, Google Cloud TTS, and
  Azure AI Speech BYOK providers.
- Whole-book narration queue, active-word focus, pronunciation rules, voice
  presets, provider expressiveness, cache management, and bounded M3U8 export.
- Final responsive audiobook-player layout, light-theme action contrast fix,
  and Windows narration continuation fix.

## Required automated gates

- [x] Prettier formatting check.
- [x] ESLint with zero warnings.
- [x] Frontend test suite: 81 tests passed.
- [x] TypeScript and Vite production build.
- [x] Rust formatting check.
- [x] Clippy with warnings denied.
- [x] Rust test suite: 88 tests passed.
- [x] Release security and dependency audit.
- [x] Current CycloneDX SBOM and third-party license bundle.
- [ ] Clean-tree RC build with source manifest and unchanged-source guard.

## Candidate verification

- [ ] EXE reports version `1.3.0-rc.1`.
- [ ] Application opens a working ApriReader window.
- [ ] Installer SHA-256 and size are recorded in candidate evidence.
- [ ] Candidate record points to the exact clean Git commit.
- [ ] Installer and application signature status are recorded truthfully.

## Product-owner evidence already received

- [x] Audiobook player layout correction accepted.
- [x] Light-theme action contrast correction accepted.
- [x] Local Windows TTS continues beyond the short section title.
- [x] Product owner reported the implemented A0-A13 build working without known
      bugs before consolidation into RC1.

## External gates before final 1.3.0

- Install/upgrade RC1 over the stable 1.2.0 build on a disposable Windows test
  profile and confirm the library, progress, credentials, and associations are
  retained.
- Complete the applicable Windows 10/11 manual matrix in
  `docs/testing/MANUAL_TESTS.md`, including scaling, forced colors, system-codec
  formats, tray behavior, and malformed input.
- Record the product-owner RC decision. Promote to `1.3.0` only after the RC is
  accepted; do not reuse the RC installer as the final release artifact.
- A future signed build still requires an external Authenticode certificate and
  trusted timestamp.
