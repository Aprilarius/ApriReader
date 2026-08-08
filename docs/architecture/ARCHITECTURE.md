# Architecture

ApriReader uses Tauri 2, React with strict TypeScript, Rust stable, and SQLite.

## Layers

- `ui`: React screens, reusable components, localization, and design tokens.
- `application`: user workflows and typed Tauri command wrappers.
- `domain`: books, reading positions, annotations, statistics, achievements.
- `readers`: one explicit adapter per format family.
- `infrastructure`: SQLite, files, cache, backups, and opt-in network clients.
- `integrations`: metadata, Steam, crash reports, TTS, and future sync.

## Audiobook foundation

Audiobook stage A0 adds `src-tauri/src/audio_prototype.rs` as an isolated
native boundary. A dedicated MTA worker thread owns
`Windows.Media.Playback.MediaPlayer`; React and the Tauri command dispatcher
never own WinRT media objects. Typed commands provide probe, load, play, pause,
seek, playback-rate, volume, snapshot, and stop operations. This serialization
also gives later stages one place to coordinate persistence, sleep timers,
chapters, output devices, and system media controls.

Local input is canonicalized and checked before it reaches Windows Media. One
part is limited to 20 GiB. AAX, AAXC, and M4P are rejected as DRM formats;
CUE, M3U, and M3U8 are classified as descriptors and are never sent directly
to the decoder. Baseline Windows formats and formats that may
depend on an installed system codec are reported separately. Media-open and
media-failure events feed a serializable state snapshot without exposing COM
or WinRT handles to the frontend.

Audiobook stage A1 adds migration 11 with separate `audiobooks`,
`audiobook_parts`, and `watched_audio_folders` tables. A book owns up to 1,000
ordered parts and no more than 100 GiB; each part independently keeps its
canonical source path, SHA-256 fingerprint, format, size, availability, and
future duration slot. Playback progress fields belong to the audiobook rather
than to an individual file.

Manual multi-selection groups files that share a directory. Importing a
directory groups its direct audio files and treats nested directories as
independent books. A watched root keeps its direct single-file books separate,
while every nested directory has a stable folder identity even if only one
part is currently available. Discovery is recursive to 12 levels, skips
symbolic links, stops at 100,000 audio files, naturally orders numeric part
names, and never edits source media. Rescans update changed paths, reconnect
unavailable content, and retain missing parts as unavailable rather than
silently deleting listening history.

Audiobook stage A2 keeps the React integration behind the typed
`application/audiobooks.ts` boundary. `App` loads audiobook rows and watched
folders only after the dedicated route is entered, then requests parts for the
currently selected card with a monotonic request token so a slower prior
selection cannot replace a newer one. File import, directory import, watched
directory registration, and rescans all return the same bounded summary
contract. The player destination remains a separate screen.

Audiobook stage A3 promotes the native prototype into the active playback
boundary. A load command now waits for `MediaOpened` or a bounded failure and
`MediaEnded` becomes an explicit serializable state used for automatic queue
advance. React polls immutable snapshots without owning WinRT objects; all
transport mutations remain serialized on the dedicated MTA worker.

Audiobook stage A4 adds migration 12 with cascade-owned bookmark and chapter
tables. Descriptor parsing is local and bounded to 2 MiB and 10,000 entries;
resolved audio must stay within the descriptor directory, and the existing
1,000-part and 100 GiB book limits still apply. M3U order bypasses natural
sorting. CUE chapters map their source paths back to imported part ordinals and
store frame-accurate 75 fps offsets.

The native close coordinator only intercepts the main-window close request
while playback is opening, buffering, or playing. It reads the synchronized
device-local ask/tray/exit preference; ask emits a narrow frontend event, tray
hides the window without stopping the serialized media worker, and exit pauses
before application shutdown. A Tauri tray menu exposes reopen, play/pause, and
exit without adding a second media owner.

Audiobook stage A5 keeps output enumeration and switching on that same MTA
worker. Windows `DeviceInformation` results are bounded to 128 audio-render
devices, disabled endpoints stay visible but cannot be selected, and
`MediaPlayer.SetAudioDevice` remains the only playback-owner mutation. The
device identifier is a device-local preference and an empty value means the
current Windows system default.

Migration 13 extends audiobook rows with narrator, series, genres,
description, language, publication year, provider provenance, and cover
provenance. Manual updates and validated local JPG/PNG/WebP cover copies never
modify source audio. Explicit Russian/English searches reuse the bounded
metadata cache, rate limit, candidate contract, and reviewed provider clients
already used for text books.

The same migration adds separate listening sessions. Activity is credited only
while native state is playing, accepts at most a 45-second event gap, and caps
one event at 30 active seconds. Audiobook totals and seven `audio_*`
achievements are derived locally; their unlocks deliberately bypass the text
reading Steam synchronization queue.

Audiobook stage A7 adds `src-tauri/src/tts.rs` as a second isolated native
worker. It prefers `Windows.Media.SpeechSynthesis` and falls back to desktop
SAPI voice tokens when Runtime voice discovery is unavailable. Requests are
serialized, limited to 20,000 Unicode characters, and returned only as valid
RIFF WAV data no larger than 64 MiB. At most 64 generated sections remain in
the app-local TTS cache; source books are never written.

React submits only the normalized text of the current reflow section through
`application/tts.ts`. The selected installed voice and 0.5x–2.0x synthesis
rate are device-local preferences. Generated WAV playback reuses the single
native audio owner, so play, pause/resume, stop, reader changes, and audiobook
playback cannot create competing media owners. Whole-book chunking, word
highlighting, and network/BYOK voices are outside A7.

Audiobook stage A8 builds a frontend-only queue of immutable references into
the already bounded `DocumentModel`. Sentence and word boundaries produce
fragments of at most 1,200 UTF-16 units; a session is capped at 50,000
fragments. Only the current and next fragment are kept in the frontend
prepared-audio map, while the native 20,000-character, 64 MiB, and 64-file
cache limits remain authoritative.

The current fragment owns its section, block, and UTF-16 offsets. Native audio
position and duration select the active word range, which React composes with
annotation and bionic spans without changing source text or stable locator
offsets. The next fragment is synthesized while the current one plays.
`MediaEnded` advances the queue and preserves the TTS side panel while an
internal transition saves the new section position. Manual reader navigation,
panel close, voice/rate/scope changes, or a playback failure invalidate the
session generation and stop its audio. Exact engine-provided phoneme timing
and network/BYOK voices remain later gates.

Audiobook stage A9 adds `cloud_tts.rs` as a fixed-host ElevenLabs BYOK adapter.
The WebView can query only configured/not-configured state; the API key is
validated and stored as a Generic Credential named
`ApriReader/ElevenLabsApiKey` in Windows Credential Manager. It is read only by
Rust immediately before an explicit voice-list or speech request and is never
serialized back to React, persisted in local storage, or included in errors.

Voice discovery reads at most 4 MiB and 100 entries from the official v2 API.
Speech accepts no more than 2,000 Unicode characters, while the A8 queue sends
at most 1,200 UTF-16 units per request. The fixed official HTTPS endpoint
returns a response capped at 48 MiB; decoded MP3 is capped at 32 MiB and
validated by signature before an atomic app-local cache write. Provider audio
and the 64-file provider cache never modify the source book.

The original alignment must concatenate exactly to the submitted fragment and
must provide equal, finite, monotonic character/time arrays. Rust converts the
provider character sequence into UTF-16 offsets; React maps the active native
playback time to the containing source word. Missing or inconsistent timing
fails closed instead of silently claiming exact synchronization. Local Windows
voices continue to use the A8 position-based fallback.

Audiobook stage A10 keeps voice presets and pronunciation rules in a bounded,
versioned local preference document. Presets contain only provider, voice ID,
rate, and a user label; provider credentials remain exclusively in Windows
Credential Manager. The dictionary accepts at most 100 unique case-insensitive
whole words or phrases and can be disabled without discarding entries.

Rules are applied to a temporary queue fragment immediately before synthesis,
never to the document model or source book. Each generated UTF-16 boundary is
mapped back to its source boundary. ElevenLabs timings are remapped through
that table before active-word lookup; local voices retain proportional focus.
Post-replacement text is capped at 2,000 UTF-16 units to prevent pathological
expansion and to remain inside the reviewed cloud request boundary.

Audiobook stage A11 adds `google_tts.rs` as a second isolated native BYOK
adapter. Its only hosts are the Google Cloud v1 voices and synchronous
synthesis endpoints. A distinct Generic Credential target prevents provider
key confusion; the key is sent only through `x-goog-api-key`, never a URL or
Tauri payload after saving. Voice and speech JSON, decoded MP3, voice count,
input characters, UTF-8 input bytes, and provider cache are independently
bounded and validated.

Voice discovery optionally sends the current book's validated BCP-47 language
as a filter and returns bounded identifiers, locale, gender, and inferred voice
family. Synthesis uses plain text and MP3 without SSML or arbitrary effects.
Google's synchronous REST response has no timing array, so the existing A8
position fallback drives focus. Playback rate remains local, preserving
compatibility with voice families that do not accept synthesis-rate controls.

Audiobook stage A12 adds `azure_tts.rs`. The frontend can select only one of 33
reviewed public Azure Speech regions; Rust validates that identifier before
constructing the fixed `https://<region>.tts.speech.microsoft.com` host. The
resource key has its own Generic Credential and is sent only through
`Ocp-Apim-Subscription-Key`. Book text is XML-escaped into bounded SSML, and
returned MP3 bytes are bounded and signature-checked before an atomic cache
write. Azure REST has no timing array, so A8 position-based focus is retained.

Audiobook stage A13 carries provider-specific expressive values through typed
frontend adapters into Rust validation. Values participate in provider cache
digests, so a changed pitch or ElevenLabs voice setting cannot reuse stale
audio. The WebView stores only non-secret preferences and presets.

`tts_assets.rs` owns cache enumeration, deletion, and export sessions. It
recognizes only exact 64-hex ApriReader TTS filenames and direct children of the
app-local TTS directory. Export is a bounded state machine: a selected M3U8
path starts one of at most two sessions, each validated cache part is copied
immediately into a unique partial media directory, and completion atomically
publishes the directory and playlist. Cancellation can delete only the partial
directory registered in the native session map. Limits are 5,000 parts,
64 MiB per part, and 6 GiB total.

Progress writes cross a separate database command. SQLite validates finite
part indices, positions, and observed durations, updates the current part
duration, and atomically derives whole-book progress. When every duration is
known, progress is duration-weighted; otherwise it uses a bounded part-based
estimate. The UI persists at five-second movement intervals and lifecycle
boundaries, with monotonic load tokens preventing an older part request from
replacing a newer selection.

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
Size limits are enforced again while streaming from the opened handle, so a
file that changes after its initial metadata check cannot force an unbounded
allocation. Watched-folder discovery also has a global file-count ceiling.

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

The optional local profile is deliberately frontend-only. `useLocalProfile`
stores one bounded, normalized display name and an onboarding-complete flag in
the same app-local WebView storage used for locale and reader preferences.
React renders the value as text, never authored markup. No Windows identity
API, database migration, native command, dependency, network request, or
multi-user account model is involved.

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
Cache identities are accepted only when derived from a valid hexadecimal book
fingerprint. PDF copies are streamed through a bounded atomic temporary file;
PDF and comic derivatives are rebuilt when the source is newer, and an
interrupted comic extraction is never treated as complete.

DOCX remains inside the normalized reflow boundary. Rust reads only
`word/document.xml`, converts paragraphs, headings, quotes, and list items to
plain typed blocks, and never sends book-authored HTML, relationships, macros,
links, or external resources to the WebView. SQLite reuses the existing
reading-position fields for a fixed-format page index and overall progress.

Stage 5 adds an explicit `MetadataProvider` boundary in
`src-tauri/src/metadata.rs`. Only the Rust backend can contact the fixed HTTPS
Open Library Search/Covers endpoints and FantLab edition-search endpoint.
English search uses Open Library with an English-edition constraint; Russian
search uses a Russian-edition constraint and merges bounded FantLab results.
Requests use a named User-Agent, return bounded JSON/image bodies, are limited
to one user-triggered search per second, and cache normalized results by query
and language in SQLite for 30 days. One provider may fail without discarding
valid results from the other. No arbitrary provider URL crosses the command
boundary.

Migration 5 stores editable bibliographic fields, provider provenance, the
original embedded-cover path, active cover source, and the metadata cache.
Manual or provider-applied fields survive a later source rescan. External
covers receive app-generated names under the existing scoped cover cache;
signature validation happens before writing. Removing one restores the
embedded cover path and deletes only a verified app-managed external file.
The same managed-file boundary accepts a user-selected JPG, PNG, or WebP cover
only from manual edit mode. Rust rechecks the suffix, 10 MiB limit, and image
signature, copies the bytes locally, and never writes to the source image or
book.

Migration 10 adds the bounded `books.genres` text field. EPUB `dc:subject`, FB2
`genre`, Open Library `subject`, and manual values pass through a 12-value,
64-character-per-value normalization boundary. Repeated values are removed
case-insensitively and stored as a canonical comma-separated local string.

External translation is a narrow, explicit browser-handoff boundary. The
frontend accepts at most 2,000 selected Unicode characters, counts Cyrillic
and Latin letters to choose RU-EN or EN-RU, and uses `URL.searchParams` to
encode the text. It can construct only `https://translate.google.com/` or
`https://translate.yandex.com/` URLs. Tauri opener permissions repeat that
exact host allowlist, so neither book content nor application state can supply
an arbitrary destination.

The first handoff requires a local consent flag set only by the user's
Continue action. No translation happens at startup or in the background, and
the WebView never loads translator HTML. The operating system opens the URL in
the default browser. ApriReader bundles no model, dictionary, language pack,
or ONNX runtime and keeps no copy of the external result.

Stage 7 adds append-only `reading_activity_events` and bounded
`reading_sessions` in SQLite. React reports visibility, focus, recent
interaction, reading progress, normalized word position, or fixed page
position every 15 seconds. Rust owns the clock and credits at most 30 seconds
only when the previous heartbeat is no more than 45 seconds old. Stale,
duplicate, hidden, blurred, or idle heartbeats add no time or volume.
The frontend keeps session metrics per book and closes each asynchronous
session idempotently, preventing rapid book switches from attributing progress
to the wrong title. Reader-open requests use a monotonic generation, while
pending position writes are cancelled or flushed at navigation and close
boundaries.

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

Bundled reading fonts live under `src/assets/fonts` and are emitted as hashed
Vite assets. Variable normal/italic pairs cover Literata, Lora, Merriweather,
and Source Serif 4; reviewed static faces cover Charis SIL and IBM Plex Serif.
The reader registry owns each family's CSS stack and available weights, clamps
a persisted weight to the closest supported value when the family changes,
and persists normal/italic style separately. Optical sizing remains automatic,
and the Merriweather width axis stays at its normal default.

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

Windows file associations are installer-owned declarations under
`bundle.fileAssociations`; the application never writes association registry
keys at runtime. The single-instance plugin is registered before every other
plugin. Initial command-line book or audiobook paths and paths forwarded by a
later Windows shell activation enter one bounded, case-insensitively
deduplicated native queue capped at 32 entries. Only recognized book, safe
audio, and local CUE/M3U/M3U8 extensions enter that queue.

React drains the queue through one typed tagged result. Rust routes books
through the same bounded `inspect_book` and database import boundary used by
manual import. Audio routes through descriptor discovery, file/aggregate size
limits, content fingerprinting, and the normal audiobook group transaction.
Duplicate content returns its existing record; a valid new record is opened in
the matching reader or audiobook player. DRM extensions, remote playlist
entries, executable paths, and ambiguous launch groups are rejected. No shell
argument becomes authored HTML, a URL request, or an executable path.

Closed-beta provenance is produced from Git's tracked and non-ignored
untracked source set. The candidate builder hashes each source file into
`SOURCE_SHA256SUMS.txt`, records the manifest hash, HEAD commit, clean or
modified tree state, and changed-file count, then repeats the snapshot after
the production installer build. A changed snapshot aborts packaging. The
eventual signed release can invoke the same builder with `-RequireCleanTree`.
The `rc:build` entry point additionally selects the `release-candidate`
channel and refuses to run unless that clean-tree guard is active.
The stricter `github:build` entry point records the `github-release` channel,
requires the same clean source state, and packages only the public profile.
Steam evidence remains independent because no protected file may enter this
source tree or GitHub artifact.
