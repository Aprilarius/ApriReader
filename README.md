# ApriReader

ApriReader is a privacy-first desktop reader and personal digital library for
Windows 10/11 x64.

It reads DRM-free EPUB, PDF, FB2, TXT, HTML, Markdown, CBZ, CBR, and DOCX files
and plays local audiobooks through the Windows media stack.
Books remain in their original locations, reading data stays local, and network
access occurs only for explicit metadata, translation, or selected BYOK speech
actions.

## Download

Download the current Windows installer from
[GitHub Releases](https://github.com/Aprilarius/ApriReader/releases/latest).

The Windows installer is not yet code-signed. Windows may therefore identify it
as coming from an unknown publisher. Download it only from the official
Aprilarius/ApriReader release page and verify the published SHA-256 value.

## Highlights

- Local library, folders, duplicate detection, favorites, authors, and series.
- Continuous reading and two-page book spreads with persistent page counters.
- PDF canvas reader and safe comic reader with single/two-page and LTR/RTL
  modes.
- Fine typography with six bundled OFL serif families, system profiles,
  imported local fonts, and optional bionic highlighting.
- Full-text search, bookmarks, highlights, notes, quotes, and Markdown export.
- Selected-text EN-RU and RU-EN translation through an explicit Google or
  Yandex browser handoff with first-use privacy consent.
- Local statistics, reading goals, activity calendar, and 42 achievements.
- Keyboard navigation, Windows Narrator support, high scaling, and forced
  colors.
- App-local backups and guarded recovery from a damaged library database.
- Explorer file associations for every supported book and audiobook format,
  with safe import-and-open in the existing ApriReader window.
- Local read-aloud using voices already installed in Windows, with continuous
  whole-book mode, active-word focus, automatic section transitions, voice,
  rate, pause/resume, and stop controls.
- Optional ElevenLabs BYOK narration with provider timing, explicit text-send
  consent, and API-key storage in Windows Credential Manager.
- Optional Google Cloud TTS BYOK narration with Russian and multilingual cloud
  voices, separate consent, and protected API-key storage.
- Optional Azure AI Speech BYOK with region-bound neural voices, protected key
  storage, and safely escaped synthesis requests.
- Local voice presets and a pronunciation dictionary that changes only the
  synthesized copy while preserving focus against the original book text.
- Provider-specific voice expressiveness, inspectable/clearable speech caches,
  and explicit export to a portable M3U8 playlist with numbered local parts.

## Privacy and safety

- No telemetry or advertising.
- No bundled books, catalogs, generated covers, speech model, or language
  model. Local read-aloud uses Windows; optional ElevenLabs, Google Cloud, or
  Azure requests occur only after the user supplies a key, selects that
  provider, and accepts its separate disclosure.
- Book content is treated as untrusted and cannot execute embedded scripts.
- Source books are never modified or deleted by library removal.
- Open Library and Inventaire are contacted only after an explicit metadata
  action; provider hosts and Inventaire image paths are fixed and validated.
- The GitHub build contains no Steamworks SDK, App ID, bridge, or credentials.

## Development

Prerequisites:

- Node.js 22+
- pnpm 11.9.0
- stable Rust
- [Tauri Windows prerequisites](https://v2.tauri.app/start/prerequisites/)

```powershell
pnpm install --frozen-lockfile
pnpm tauri dev
```

Run the complete local gate:

```powershell
pnpm check
```

Build a clean public GitHub artifact:

```powershell
pnpm github:build
```

Build a clean signed public artifact after configuring the external Windows
certificate and timestamp service described in
[`docs/release/WINDOWS_SIGNING.md`](docs/release/WINDOWS_SIGNING.md):

```powershell
pnpm github:signed-build
```

Generated installers and evidence archives are written to
`release/candidates/` and are excluded from source control.

## Documentation

- [Product specification](docs/product/PRODUCT_SPEC.md)
- [Architecture](docs/architecture/ARCHITECTURE.md)
- [Manual tests](docs/testing/MANUAL_TESTS.md)
- [Dependency policy](docs/legal/DEPENDENCY_POLICY.md)
- [Windows release signing](docs/release/WINDOWS_SIGNING.md)
- [Changelog](CHANGELOG.md)

## License

ApriReader is licensed under Apache-2.0. Third-party attribution and exact
release inventories are available in `THIRD_PARTY_NOTICES.md`,
`release/THIRD_PARTY_LICENSES.md`, and
`release/aprireader-sbom.cdx.json`. Bundled font licenses are packaged under
`licenses/fonts`.
