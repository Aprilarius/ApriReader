# Architecture

ApriReader uses Tauri 2, React with strict TypeScript, Rust stable, and SQLite.

## Layers

- `ui`: React screens, reusable components, localization, and design tokens.
- `application`: user workflows and typed Tauri command wrappers.
- `domain`: books, reading positions, annotations, statistics, achievements.
- `readers`: one explicit adapter per format family.
- `infrastructure`: SQLite, files, cache, backups, and opt-in network clients.
- `integrations`: metadata, Steam, crash reports, TTS, and future sync.

Stage 1 adds a Rust-owned library database and importer. React receives only
serialized book records through explicit commands. The schema stores source
paths, SHA-256 fingerprints, metadata, cached cover paths, availability, and
watched folders. The unique fingerprint prevents content duplicates; source
path upsert handles an edited file without creating a second card.

Import treats every book as untrusted input. Archive paths must be relative and
free of parent traversal, symbolic links are skipped during folder scans, XML
and cover extraction have size limits, and no embedded script or external
resource is executed. Only validated embedded image bytes are written to the
app-local cover cache. Source files are opened read-only and never moved.

SQLite backups use `VACUUM INTO` after a WAL checkpoint and retain the ten most
recent app-generated copies.

Stage 2 implements the first reader adapters in `src-tauri/src/reader.rs`.
TXT, Markdown, HTML, EPUB, and FB2 are converted to a serialized
`DocumentModel` containing sections and typed plain-text blocks. EPUB spine
order becomes the section order. FB2 honors the XML encoding declaration.
React never receives book-authored HTML, URLs, styles, or scripts and renders
all block text through normal escaped JSX nodes.

The reader enforces per-entry and total text limits and rejects archive
traversal. SQLite stores overall progress, last section, and section progress.
Typography preferences stay local in the WebView profile. Future annotations
use `ReadingLocator` and `AnnotationAnchor` without changing the normalized
document boundary.

Stage 3 implements `ReadingLocator` as a stable section identifier, normalized
block index, and UTF-16 text range. The saved selected text is the
`AnnotationAnchor` used for human verification if a source book changes.
Bookmarks use the same locator with an empty range. SQLite owns annotation CRUD
and an FTS5 index built only from normalized plain-text blocks when a book is
opened. Search returns section and block locators; React navigates without
receiving source markup. Markdown export writes a new user-selected file and
never edits the source book.

Stage 4 adds a separate fixed-layout boundary in
`src-tauri/src/special_reader.rs`. PDF source bytes are copied to a
fingerprint-keyed app-local reader cache only after the PDF signature is
validated, then rendered by PDF.js in its worker;
React renders only the page canvas and does not attach a scripting or
annotation layer from the document. CBZ uses the bounded ZIP reader already in
the project. CBR uses the pure-Rust `rars` adapter for RAR3/RAR5 families.
Archive entries must have safe relative paths, valid image extensions, bounded
per-page and aggregate sizes, and recognized image signatures before they are
exposed through the scoped asset protocol.

DOCX remains inside the normalized reflow boundary. Rust reads only
`word/document.xml`, converts paragraphs, headings, quotes, and list items to
plain typed blocks, and never sends book-authored HTML, relationships, macros,
links, or external resources to the WebView. SQLite reuses the existing
reading-position fields for a fixed-format page index and overall progress.

Stage 5 adds an explicit `MetadataProvider` boundary in
`src-tauri/src/metadata.rs`. Only the Rust backend can contact the fixed HTTPS
Open Library Search and Covers endpoints. Requests use a named User-Agent,
return bounded JSON/image bodies, are limited to one request per second, and
cache normalized search results in SQLite for 30 days. No arbitrary provider
URL crosses the command boundary.

Migration 5 stores editable bibliographic fields, provider provenance, the
original embedded-cover path, active cover source, and the metadata cache.
Manual or provider-applied fields survive a later source rescan. External
covers receive app-generated names under the existing scoped cover cache;
signature validation happens before writing. Removing one restores the
embedded cover path and deletes only a verified app-managed external file.

Migration 10 adds the bounded `books.genres` text field. EPUB `dc:subject`, FB2
`genre`, Open Library `subject`, and manual values pass through a 12-value,
64-character-per-value normalization boundary. Repeated values are removed
case-insensitively and stored as a canonical comma-separated local string.

Stage 6 adds `LanguagePackageManager` in
`src-tauri/src/language_tools.rs`. A language package is a ZIP container with a
bounded `manifest.json` and a closed set of payload names. Import checks
archive paths, entry count, declared and actual sizes, SHA-256, engine
compatibility, and an allowlist of permissive SPDX licenses before an atomic
move into app-local storage. Installed files are reverified before use.

`DictionaryProvider` reads only normalized JSON entries from verified packages.
`TranslationProvider` is an explicit boundary; its ONNX implementation uses the
application-bundled ONNX Runtime with a hash-verified, text-in/text-out
`model.onnx`. Packages cannot supply native libraries or custom helpers. No
package URL crosses into the provider, no model is loaded at startup, and
selected book text never leaves the process. Package format details are
documented in `docs/language/PACKAGE_FORMAT.md`.

Stage 7 adds append-only `reading_activity_events` and bounded
`reading_sessions` in SQLite. React reports visibility, focus, recent
interaction, reading progress, normalized word position, or fixed page
position every 15 seconds. Rust owns the clock and credits at most 30 seconds
only when the previous heartbeat is no more than 45 seconds old. Stale,
duplicate, hidden, blurred, or idle heartbeats add no time or volume.

Statistics are rebuilt from local events, including an 84-day calendar,
current and longest streaks, words, pages, opened/completed books, and a local
daily goal. Deleting statistics removes only sessions, events, and unlocks;
books, reading positions, and annotations remain.

`statistics.rs` is also the canonical achievement registry and local provider.
Conditions are evaluated idempotently and persisted by stable achievement ID.
Stage 8 may add a Steam adapter against these IDs, but Stage 7 contains no
Steamworks dependency or network synchronization.

The canonical registry now contains 42 stable identifiers. Completion,
author, genre, and series ladders use completed books only. Multi-value genres
are split in Rust and deduplicated case-insensitively; arbitrary SQL column
names or book-authored queries never cross this boundary. Existing unlock rows
remain valid while new achievements enter the offline Steam queue normally.

Stage 8 adds migration 7 and `achievement_sync_queue`. A newly unlocked local
achievement is enqueued exactly once. Entries remain pending across restarts
and failed stores; they are marked synchronized only after the provider
confirms the whole batch. Queue IDs are filtered against the same canonical
registry used by local evaluation.

`steam.rs` owns the `SteamAchievementProvider` boundary. The public build uses
the GitHub profile and never loads Steam code. The `steam-build` feature can
load only `aprireader_steam_bridge.dll` beside the executable with restricted
Windows DLL search flags. The protected bridge owns Steamworks initialization,
callbacks, `SetAchievement`, `StoreStats`, Overlay state, and App ID context.
Its ABI is documented in `docs/steam/BRIDGE_ABI.md`; SDK files and secrets stay
outside this repository.

Stage 9 adds a startup health boundary to `database.rs`. Every open runs
`PRAGMA quick_check` after migrations and writes a local clean-exit marker.
When the active database fails to open, ApriReader selects the newest backup
that independently passes `quick_check`, moves the damaged database and WAL
sidecars into the app-local `recovery` directory, restores the valid copy, and
reports the event to the UI. A normal process exit updates the marker. No
recovery path opens or writes a source book.

Release checks are dependency-free Python scripts. `release_audit.py` pins the
reviewed direct dependency set, protected-file denylist, CSP, capabilities, and
asset-protocol scope. `generate_sbom.py` creates a deterministic CycloneDX 1.5
component inventory from both lockfiles. CI rejects stale inventories or a
security-boundary change that has not been reviewed.

The Stage 9 public beta uses a current-user NSIS bundle with RU/EN installer
languages. `scripts/build_beta_candidate.ps1` runs the complete gate before
packaging and records the installer hash, build environment, SBOM, notices, and
release checklists under the ignored `release/candidates` directory. Bundler
tools stay under `src-tauri/target/.tauri` so candidate builds do not require
write access to a shared system cache.

Reader font import is owned by `fonts.rs`. It accepts only a user-selected
regular font file up to 24 MB with a matching TrueType, OpenType, WOFF, or
WOFF2 signature, hashes its bytes, and writes an app-managed immutable copy
under the scoped local `fonts` directory. The WebView receives only the
generated local path and family identifier. No font parser dependency,
download, catalog, or source-book mutation is introduced.

Typography preferences, focus highlighting, and page-wheel behavior remain
local WebView preferences. Focus highlighting preserves the exact concatenated
text, so existing UTF-16 annotation locators and selection verification remain
stable.

The reflow layout preference is also WebView-local. Continuous mode measures
progress on the vertical scroll axis. Spread mode uses bounded CSS
multi-columns over the same escaped React block tree and measures progress on
the horizontal axis; it never reparses text or changes block indices. A
responsive single-column spread uses the same page-turn and locator logic.

Reflow page totals are viewport-relative rather than source metadata. An
offscreen, aria-hidden measurer lays out one normalized section at a time with
the active typography, records its page count, and then releases that section
before measuring the next. The visible counter combines those counts with the
saved section progress; no book-authored markup or external resource enters the
measurement tree.

Migration 8 adds a nullable `books.last_opened_at` Unix timestamp for the
Reading Now view. It is written only after a reader adapter has opened a book
successfully or when a valid reading position is saved. Folder scans continue
to update source availability without changing reading recency. Existing
unfinished progress is backfilled once during migration.

Migration 9 adds the constrained `books.is_favorite` flag. The Rust database
boundary owns favorite updates and returns the complete updated book record so
every active view stays consistent. Toggling the marker creates an app-local
database backup and never opens or writes the source book.

Author grouping is a derived React view over the complete `BookRecord` list and
adds no schema or dependency. Keys use locale-aware lowercase matching after
trimming metadata, while the first stored display spelling remains visible.
Books inside a group sort by title; editing metadata rebuilds the groups from
the updated records.

Series grouping is also a derived React view and adds no schema or dependency.
Keys use locale-aware lowercase matching after trimming and collapsing repeated
whitespace, while the first stored display spelling remains visible. Books
inside a group use an `Intl.Collator` with numeric comparison for deterministic
natural title order. The empty series key sorts last and receives only the
localized No Series label.

Library removal crosses one typed `remove_books` command. Rust deduplicates and
bounds the requested IDs, deletes the selected records in one transaction, and
relies on SQLite foreign-key cascades for annotations and reading sessions.
FTS5 rows are removed explicitly. App-managed covers and fixed-reader caches
are cleaned only after their paths are proven to be inside the matching local
cache root. Source paths are never opened for writing or deletion.

Screen reader support is a WebView-local preference and adds no native command
or dependency. Reader roots inherit a validated BCP 47 language tag derived
from local book metadata. Optional chapter and page updates use polite,
atomic live regions; disabling the preference switches those regions off while
preserving structural roles, labels, and keyboard behavior.

High-scaling behavior remains CSS- and semantic-HTML-only. Shell navigation
uses bounded independent overflow regions so short windows never clip the
route list behind the fixed language action. Compact route buttons own explicit
accessible names independent of hidden visual labels. Reader toolbar height is
a responsive CSS variable shared with side-panel positioning, allowing the
action row to reflow without covering book content or changing reading
locators.

Forced-colors support is also CSS- and semantic-HTML-only. A single
`forced-colors: active` layer remaps application surfaces and controls to
Windows system colors, then reinforces state with borders and line styles.
Choice controls expose `aria-pressed` independently of their visual treatment.
Only document imagery and fixed-reader canvases opt out of forced recoloring;
book bytes and rendered page content are never transformed or rewritten.

Closed-beta provenance is produced from Git's tracked and non-ignored
untracked source set. The candidate builder hashes each source file into
`SOURCE_SHA256SUMS.txt`, records the manifest hash, HEAD commit, clean or
modified tree state, and changed-file count, then repeats the snapshot after
the production installer build. A changed snapshot aborts packaging. The
eventual signed release can invoke the same builder with `-RequireCleanTree`.
The `rc:build` entry point additionally selects the `release-candidate`
channel and refuses to run unless that clean-tree guard is active.
