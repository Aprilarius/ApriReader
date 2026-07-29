# ApriReader

ApriReader is a privacy-first desktop reader and personal library for Windows.
This repository currently contains the Stage 9 closed-beta candidate: a Tauri
2 shell, React with strict TypeScript, Rust infrastructure, SQLite migrations,
local file and folder import, duplicate detection, embedded EPUB/FB2 metadata
and covers, a safe reflow reader for TXT/HTML/Markdown/EPUB/FB2, persisted
reading position, rolling backups, localization, tests, and the approved design
system. The reader also provides local FTS5 search, stable locators, bookmarks,
highlights, notes, saved quotes, and user-directed Markdown export.

PDF now has a dedicated worker-backed canvas viewer, CBZ/CBR have a bounded
image-sequence reader with natural ordering and LTR/RTL spreads, and DOCX is
normalized into the safe reflow reader.
Stage 5 adds local metadata editing and an explicit, cached Open Library lookup
with candidate comparison, provenance, rate limiting, and removable
app-local external covers. Multi-value genres are extracted from bounded
EPUB/FB2 metadata, may be applied from a selected Open Library candidate, and
remain locally editable without changing source books.

## Development

Prerequisites: Node.js 22+, pnpm 10+, Rust stable, and the
[Tauri Windows prerequisites](https://v2.tauri.app/start/prerequisites/).

```powershell
pnpm install
pnpm tauri dev
```

Run all local checks with `pnpm check`.

Build the reviewed Windows closed-beta installer and its local evidence package
with `pnpm beta:build`. Generated candidates are written under
`release/candidates/` and are intentionally excluded from source control.

The app makes network requests only after an explicit Open Library metadata
search or candidate apply action. It contains no telemetry, sample books,
generated covers, Steamworks, TTS, or unrestricted catalog service.

Stage 6 adds verified user-imported offline dictionaries and text-in/text-out
ONNX translation packages. No dictionary, model, or online package catalog is
bundled; the permissively licensed ONNX CPU runtime is included by the
application. See `docs/language/PACKAGE_FORMAT.md`.

Stage 7 adds privacy-preserving active reading sessions, repeatable local
statistics, an 84-day calendar, daily goals, and a canonical local achievement
registry with 42 progressively harder goals. Author, genre, and series
discovery counts completed books only. Idle or unfocused windows do not
accumulate time. Steam integration remains explicitly deferred to Stage 8.

Stage 8 adds a persistent offline achievement queue and a separate protected
Steam bridge profile. The normal build remains fully local and contains no
Steamworks SDK, App ID, or proprietary binary. Protected build and verification
details live under `docs/steam`.
