# Changelog

## Unreleased

## 1.3.0 - 2026-08-08

- Promoted the accepted 1.3 RC2 codebase to the stable 1.3.0 release.
- Added the complete audiobook library and player, local and optional BYOK
  read-aloud providers, narration export, audiobook statistics, achievements,
  metadata editing, and Explorer audio associations.
- Included the final responsive player, light-theme contrast, narration
  continuation, security, recovery, accessibility, and release-audit fixes.
- Added a dedicated unpacked Windows x64 Steam Depot package while keeping
  Steam achievements local until a protected App ID build is configured.
- Added a bilingual privacy policy, AI-use disclosure, asset-provenance record,
  and a package-specific distribution-claims matrix for public listings.
- Made metadata-provider requests identify the installed stable package version
  instead of retaining an RC-specific User-Agent.

## 1.3.0-rc.2 - 2026-08-08

- Updated `pdfjs-dist` to 6.2.108, the first release outside the reviewed
  arbitrary-JavaScript-execution advisory range, and added a release-gate
  minimum-version check.
- Replaced the retired FantLab metadata adapter with the current public
  Inventaire search API. Open Library and Inventaire now respect the selected
  Russian or English language; Inventaire descriptions and strictly validated
  fixed-host cover paths can be applied to the local record.
- Made TTS cache writes collision-safe and durable when identical fragments are
  prepared concurrently. Export playlists now use unique sidecars and restore
  the previous playlist if replacement fails instead of deleting it first.
- Prevented overlapping native TTS polling and ignored stale cloud preparation
  failures after stop or provider changes.
- Hardened SQLite recovery so the database, WAL, and shared-memory sidecars are
  quarantined and restored as one set, including rollback after a partial move.
- Removed avoidable production panics from Windows audio event registration and
  expanded regression coverage for playback polling, cache concurrency, export
  replacement, database sidecars, and Inventaire response validation.

## 1.3.0-rc.1 - 2026-08-08

- Reworked the audiobook player into a bounded responsive composition: the
  cover and metadata remain in their own column, playback controls no longer
  overflow into the queue, and parts, chapters, and bookmarks use consistent
  cards that collapse to one column on narrow windows.
- Fixed light-theme button contrast across dialogs and text-to-speech controls.
  Primary and secondary variants now retain their own surfaces inside shared
  action rows, and disabled actions remain readable without hover.
- Fixed local Windows narration stopping after a short section title. Native
  `MediaEnded` now takes precedence over Windows MediaPlayer's trailing paused
  session state, so the TTS queue advances into the first text block and keeps
  reading subsequent fragments.
- Started the audiobook foundation with an isolated native Windows MediaPlayer
  service and diagnostic Tauri commands for probing, loading, playback,
  pause, seek, speed, volume, state inspection, and stop.
- Added a 20 GiB per-file boundary, a safe local-audio allowlist, explicit DRM
  rejection, and separate CUE/M3U/M3U8 classification ahead of audiobook
  library import.
- Added the separate SQLite audiobook/part model, bounded single and
  multi-part import, natural part ordering, content-aware rescans, source
  availability tracking, and dedicated watched audio folders.
- Automatic grouping treats a selected folder as one audiobook while keeping
  single files in the root of a watched collection separate; nested folders
  remain stable groups even when only one part is temporarily available.
- Added a dedicated Audiobooks destination with lazy database loading, local
  search, library totals, availability-aware cards, ordered part details,
  explicit file/folder import, watched-folder rescans, and localized status
  reporting. The destination remains separate from text-book navigation.
- Connected the audiobook-player destination to the isolated native Windows
  MediaPlayer worker with play/pause, 15-second seeking, previous/next parts,
  automatic part advance, an ordered queue, 0.5x–3.0x speed, and persistent
  volume/speed preferences.
- Added bounded durable audiobook progress. The player restores the last part
  and position, records observed media durations, saves every five seconds and
  at lifecycle boundaries, and derives whole-book progress atomically without
  modifying source audio.
- Added bounded local CUE, M3U, and M3U8 import. Playlist order is preserved,
  CUE track titles and frame-accurate positions become chapters, remote URLs
  and paths escaping the descriptor folder are rejected, and descriptors are
  capped at 2 MiB and 10,000 parsed entries.
- Added audiobook sleep timers for 15, 30, 45, or 60 minutes and the end of the
  current part, plus local per-position bookmarks with optional notes.
- Added configurable close behavior while audio is active: ask each time,
  continue in the Windows system tray, or exit completely. The tray can reopen
  ApriReader, toggle playback, or exit.
- Added Windows audio-output selection to the player. The selected enabled
  device is remembered locally, with an explicit return to the system default.
- Added local audiobook metadata editing for title, author, narrator, series,
  genres, language, year, and description, validated local cover replacement,
  and explicit Russian/English online metadata search using the reviewed
  provider pipeline.
- Added bounded active-listening sessions, audiobook totals on Statistics, and
  seven separate audiobook achievements. Audio goals remain local and do not
  enter the text-reading Steam synchronization queue.
- Added installer-owned Explorer associations for all reviewed native and
  system-codec audiobook extensions plus local CUE/M3U/M3U8 descriptors. Shell
  activation uses the bounded single-instance queue, imports or deduplicates in
  Rust, and immediately opens the matching audiobook player.
- Generalized the existing book launch queue into a 32-entry mixed book/audio
  queue while retaining extension allowlists, case-insensitive deduplication,
  source-file immutability, and rejection of DRM and executable paths.
- Added explicit local read-aloud for the current section of a reflow book.
  The reader lists installed Windows voices, remembers the selected voice and
  0.5x–2.0x speech rate, and provides play, pause/resume, and stop without a
  cloud request or bundled speech model.
- Kept speech generation behind a dedicated Windows worker with a 20,000
  character request limit, 64 MiB WAV limit, and 64-file app-local cache. The
  Windows Runtime voice API is preferred, with classic desktop SAPI as a
  compatibility fallback when Runtime voice discovery is unavailable.
- Added continuous local narration from the current section through the rest
  of a reflow book. Text is split on sentence and safe word boundaries into
  fragments no larger than 1,200 UTF-16 units; the next fragment is prepared
  in the background while the current WAV plays.
- Added automatic reader navigation at section boundaries, remembered
  current-section/whole-book scope, fragment progress, active-word focus, and
  automatic scrolling. Word focus follows the real native playback position
  within each short fragment and coexists with annotations and bionic text.
- Added an optional ElevenLabs BYOK voice provider. Local Windows voices remain
  the default; selecting ElevenLabs requires a user-supplied key and explicit
  first-send consent that explains external processing and possible quota cost.
- ElevenLabs keys are written only to Windows Credential Manager and never
  returned to the WebView after saving. Requests use the fixed official API
  host, 1,200-unit queue fragments, bounded JSON/audio responses, validated
  MP3 data, and a 64-file provider cache.
- ElevenLabs character alignment now drives exact provider-timed word focus.
  Local Windows voices retain A8's position-based fallback, and changing or
  deleting provider settings invalidates the active narration generation.
- Added up to 20 local voice presets containing provider, voice, and speech
  rate. Presets can be created, applied, renamed, updated, and deleted without
  storing provider credentials.
- Added an optional 100-entry local pronunciation dictionary for words and
  phrases. Case-insensitive whole-word replacements affect only synthesized
  text, never the source book, and can be disabled without deleting rules.
- Pronunciation expansion is capped at 2,000 UTF-16 units per queue fragment.
  A source-offset map returns ElevenLabs character timing to the original book
  text so active-word focus remains accurate after substitutions.
- Added optional Google Cloud Text-to-Speech BYOK with a separate protected
  Credential Manager key, provider-specific first-send consent, and automatic
  language filtering from the current book.
- Google voice discovery labels Standard, WaveNet, Neural2, Studio, and Chirp
  HD families. Synthesis uses the fixed official REST host, a native-only
  `x-goog-api-key` header, bounded plain-text input, validated MP3 output, and
  a separate 64-file cache. Keys never enter URLs, WebView storage, or presets.
- Google narration reuses the A8 whole-book queue, A10 pronunciation rules and
  presets, and native playback-rate control. Because synchronous Google REST
  responses contain no character timing, active-word focus uses the existing
  position-based fallback rather than claiming exact provider synchronization.
- Added Azure AI Speech BYOK with a user-selected resource region from the
  official 33-region allowlist, a separate Credential Manager key and consent,
  and language-filtered neural voice discovery.
- Azure requests use only the validated `*.tts.speech.microsoft.com` regional
  host, native `Ocp-Apim-Subscription-Key`, fully escaped bounded SSML, validated
  MP3 output, and a separate 64-file cache. The selected non-secret region is
  stored locally and included in Azure voice presets.
- Added provider-specific expressive controls: ElevenLabs stability, similarity,
  style and speaker boost; Google pitch; and bounded Azure SSML pitch. The
  values remain local, participate in cache identity, and are saved in presets.
- Added per-provider TTS cache statistics and explicit clearing without recursive
  cache-root deletion or access to unrelated app files.
- Added explicit narration export to a user-selected M3U8 playlist plus a unique
  sibling media directory. Up to 5,000 validated app-cache parts and 6 GiB are
  copied incrementally, so normal 64-file cache pruning cannot break long exports.
  Cancellation removes only the registered partial export; cloud export warns
  about provider quota and does not claim to embed player-only speed changes.

## 1.2.0 - 2026-08-02

- Added an explicit Russian/English metadata switch. Russian searches combine
  Russian-edition Open Library results with bounded public FantLab results,
  label their source, remove duplicates, and cache by language.
- Made the cover clickable only in manual edit mode, with validated local JPG,
  PNG, and WebP import up to 10 MiB plus embedded-cover restoration. Source
  books and selected source images remain unchanged.
- Raised bounded fixed-layout limits for large local collections: PDF files up
  to 2 GiB, CBZ/CBR archives up to 4 GiB, and comic image payloads up to 6 GiB
  while retaining per-page, page-count, and archive-path protections.
- Aligned the existing `@tauri-apps/api` package with the Tauri 2.11 runtime so
  production NSIS packaging accepts the reviewed dependency set.
- Updated the build-only Tauri CLI to 2.11.4 so NSIS bundle-type metadata is
  patched without the previous `__TAURI_BUNDLE_TYPE` warning.
- Added a fail-closed Authenticode release profile that keeps credentials out
  of the repository, signs both the application and installer through Tauri,
  requires timestamping, and records verified public signature metadata.

## 1.1.0 - 2026-08-01

- Hardened file handling with bounded post-open reads for books and fonts,
  bounded metadata responses and folder scans, validated fixed-reader cache
  identities, atomic PDF caching, and stale comic/PDF cache refresh.
- Fixed reader lifecycle races so a slower earlier open cannot replace the
  newest book, pending positions are flushed on close, chapter changes cannot
  be overwritten by an old scroll timer, and reading sessions stay isolated
  per book.
- Reconnect an unavailable duplicate to its relocated source, tolerate blocked
  WebView storage without breaking controls, and report quote clipboard failure
  without losing the saved annotation.
- Replaced the unavailable custom language-package workflow with a selected
  text Translate menu for Google and Yandex, automatic EN-RU/RU-EN direction,
  a 2,000-character limit, fixed URL permissions, and first-use consent.
- Removed the language-package Settings panel and bundled ONNX runtime.
- Added a skippable first-launch local profile with an optional display name,
  personalized time-aware greeting, and edit/removal controls in Settings. It
  uses no account, password, OS identity lookup, dependency, or network access.
- Added bundled Literata, Lora, Merriweather, Source Serif 4, Charis SIL, and
  IBM Plex Serif reading families with separate normal/italic and real-weight
  selectors while preserving system profiles and local font import.
- Removed roadmap/build-status copy from the user interface, made greetings
  time-aware, made filtered-library empty states truthful, localized book-detail
  closing controls, and limited library search to its relevant destination.
- Recalculate reader pagination after font style changes and after the selected
  bundled face has actually loaded; added a live multilingual font preview.
- Register all supported book extensions in the Windows NSIS installer and
  safely import/open Explorer-launched books in the existing ApriReader
  instance.

## 1.0.0

- Released the complete privacy-first Windows library and reader for EPUB, PDF,
  FB2, TXT, HTML, Markdown, CBZ, CBR, and DOCX.
- Included local metadata editing, opt-in Open Library lookup, annotations,
  offline language packages, statistics, 42 achievements, reading layouts,
  detailed typography, imported fonts, bionic highlighting, and recovery.
- Completed keyboard, Narrator, high-scaling, forced-colors, safe format
  parsing, source-preserving removal, and release-provenance hardening.
- Added a clean-tree `github-release` build profile, deterministic transitive
  license report, SBOM, source manifest, and public release documentation.
- Published the GitHub profile without Steamworks, telemetry, bundled books,
  models, TTS, or background network access. The initial Windows installer is
  intentionally unsigned and may show an unknown-publisher warning.

## 1.0.0-rc.1 release candidate

- Consolidated the tested beta.6 Narrator, beta.7 scaling, beta.8 forced-colors,
  and beta.9 provenance work into the first 1.0 release candidate.
- Added a dedicated `rc:build` gate that requires a clean Git tree and records
  the `release-candidate` channel in its evidence.
- Prevented release-candidate packaging through the generic builder unless the
  clean-tree requirement is explicitly active.
- Retained the complete source manifest, security review, SBOM, manual matrix,
  and Steam boundary evidence beside the installer.

## 0.9.0-beta.9 provenance candidate

- Added a complete SHA-256 manifest for every tracked or untracked source file
  included in a closed-beta build.
- Candidate records now identify whether the source tree was clean or modified,
  report its changed-file count, and bind the source manifest by hash.
- Abort packaging if any source file changes while checks and the Windows
  installer are being built.
- Added an optional clean-tree requirement for the eventual signed release
  candidate and bundled the complete release-test documentation with evidence.

## 0.9.0-beta.8 forced-colors candidate

- Added complete Windows forced-colors styling for the application shell,
  settings, library, statistics, achievements, and every reader.
- Kept selected, active, locked, unlocked, progress, calendar, warning, and
  annotation states distinguishable through borders and patterns instead of
  relying on color alone.
- Preserved the original appearance of book covers, PDF pages, and comic pages
  while surrounding controls follow Windows system colors.
- Exposed pressed state for layout, alignment, theme, and comic spread controls
  so assistive technology can report the current choice.

## 0.9.0-beta.7 high-scaling candidate

- Kept the complete navigation reachable in short high-scaling windows by
  making the vertical rail scroll independently.
- Preserved the language switch in compact bottom navigation and added stable
  accessible names to every icon-only destination.
- Reduced and reflowed Settings, language-package, statistics, and achievement
  surfaces at narrow logical widths.
- Added a two-row, horizontally scrollable reader toolbar below 520 CSS pixels
  so every chapter, search, annotation, and typography action remains
  reachable.
- Hardened PDF and comic toolbars, page controls, page status, long headings,
  and reader panels against narrow or short viewports.

## 0.9.0-beta.6 Narrator support candidate

- Added an enabled-by-default screen reader support setting with persistent
  local on/off state.
- Reflow readers now expose book language metadata and optionally announce
  chapter and page changes through Windows Narrator.
- PDF and comic readers expose the same language metadata and optional page
  announcements.
- Essential names for buttons and controls remain available even when the
  optional announcements are disabled.

## 0.9.0-beta.5 keyboard-focus candidate

- Reader entry now places keyboard focus on the visible top toolbar instead of
  leaving focus on a control from the previous screen.
- Added the missing accessible name and the same initial focus behavior to PDF
  and comic reader back controls.
- Added regression coverage for initial focus in reflow and fixed-layout
  readers.

## 0.9.0-beta.4 format hardening candidate

- Added a disposable automated matrix for valid and malformed EPUB, PDF, FB2,
  TXT, HTML, Markdown, CBZ, CBR, and DOCX fixtures.
- Confirmed that every matrix fixture remains byte-for-byte unchanged after
  opening or safe rejection.
- Added PDF signature validation before a fixed-layout document enters the
  app-local reader cache.

## 0.9.0-beta.3 library removal candidate

- Added confirmed single-book removal from the details panel.
- Added an explicit batch-selection mode with a selected count, filtered
  Select all action, cancellation, and one transactional removal command.
- Source book files remain untouched. ApriReader removes only its local record,
  progress, annotations, full-text index, and app-managed caches.

## 0.9.0-beta.2 localization candidate

- Replaced the unnatural Russian watched-folder action
  `Наблюдать за папкой` with `Сканировать папку`.
- Prepared the first source-controlled closed-beta snapshot so candidate
  evidence can reference an exact Git commit.

## 0.9.0-beta.1 closed-beta packaging

- Enabled the reviewed current-user NSIS installer with an English/Russian
  language selector and Apache-2.0 license metadata.
- Added a reproducible `pnpm beta:build` gate that runs the complete repository
  checks, builds the installer, records its SHA-256 and Windows build context,
  and packages the SBOM and release checklists as local beta evidence.
- Kept protected Steamworks files out of the public candidate. Code signing,
  Windows 10/11 participant evidence, the protected Steam matrix, and the
  product-owner go/no-go decision remain external release gates.

## Long-term achievements and genres

- Expanded the canonical local and Steam achievement registry from 11 to 42
  progressively harder goals, including 250 completed books, 500 active hours,
  a 365-day streak, 100 authors, 20 genres, and 50 series.
- Author, genre, and series discovery now advances only from completed books.
- Added bounded multi-value genre metadata to migration 10, manual editing,
  EPUB/FB2 import, selected Open Library candidates, search, and book details.

## Library navigation completion

- Added local Reading Now, Favorites, Authors, and Series destinations.
- Series metadata is grouped case-insensitively with truthful counts, a plain
  No Series fallback, and natural title ordering inside each drill-down.

## Reader usability polish

- Built the release executable as a Windows GUI application so no console
  window opens beside ApriReader.
- Added four reading-font profiles, safe local custom-font import, font weight,
  letter/word/paragraph spacing, alignment, and wider size/line/column ranges.
- Added optional bionic word-prefix highlighting and page-by-page mouse-wheel
  navigation across chapter boundaries.
- Added persistent previous/next chapter controls to the top toolbar and
  suppressed the browser context menu.
- Added persistent Continuous text and responsive Book spread layouts for
  reflow books. Spread mode flows normalized text across two pages, falls back
  to one page in narrow windows, and turns a complete spread per wheel gesture.
- Added a live bottom page counter with a single-page continuous label and a
  two-page spread range, recalculated from the active viewport and typography.

## Stage 9 engineering hardening

- Added SQLite startup integrity checks, clean-exit tracking, reversible
  quarantine, and recovery from the newest independently valid backup.
- Added skip navigation, route focus, progress semantics, forced-colors support,
  scaling-safe navigation, and bounded 120-card library rendering.
- Added a repository security gate, deterministic CycloneDX lockfile inventory,
  and release/closed-beta checklists.
- Stage 9 remains open pending Windows 10/11 beta, complete transitive notices,
  and protected Steamworks release-candidate evidence.

## Stage 8

- Added an idempotent offline queue for canonical achievement unlocks.
- Added a protected Steam bridge adapter with a separate build feature and no
  public Steamworks dependency or App ID.
- Added truthful Steam profile, queue, Overlay, and manual retry status in
  Settings plus automatic retry when the protected provider is available.
- Added canonical Steam App Admin data, protected integration documentation,
  online/offline/Overlay test instructions, and branded store capsule assets.

## Stage 7

- Added foreground-, focus-, and interaction-aware reading sessions with
  backend-owned time and idle-gap protection.
- Added repeatable local aggregates, an 84-day calendar, streaks, reading
  volume, daily goals, and user-controlled statistics deletion.
- Added one canonical achievement registry with idempotent local unlocks.
- Replaced Statistics and Achievements placeholders and connected real
  dashboard values.

## Stage 6

- Added safe, hash-verified offline dictionary and translation packages.
- Added local dictionary lookup and ONNX translation actions for selected text.
- Added a Settings package manager with license, attribution, integrity state,
  explicit import, and confirmed removal.
- Added package validation tests and a documented versioned package contract.

## 0.1.0 — Unreleased

- Added the Stage 0 Tauri 2, React, strict TypeScript, and Rust foundation.
- Added the approved design tokens and responsive empty AppShell.
- Added RU/EN localization with persisted preference.
- Added the Stage 1 local SQLite library, file/folder import, SHA-256 duplicate
  detection, availability refresh, and rolling backups.
- Added safe embedded EPUB/FB2 metadata and cover extraction.
- Added library search, format collections, watched-folder management, and
  truthful book details.
- Added the Stage 2 safe reflow reader for TXT, HTML, Markdown, EPUB, and FB2.
- Added EPUB spine navigation, FB2 legacy encoding support, saved reading
  position, adjustable typography, and paper/sepia/night themes.
- Added Stage 3 stable locators, bookmarks, highlights, editable notes, saved
  quotes, local SQLite FTS5 search, and user-directed Markdown export.
- Added the Stage 4 PDF canvas viewer with zoom, page navigation, keyboard
  controls, and restored progress.
- Added safe CBZ/CBR extraction, natural page ordering, single/two-page comic
  layouts, LTR/RTL reading direction, and semantic DOCX reflow.
- Added Stage 5 manual metadata editing and opt-in Open Library search with
  candidate comparison, local caching, provenance, and rate limiting.
- Added validated app-local external covers with explicit removal and embedded
  cover restoration.
- Added automated checks, CI, licensing, dependency policy, and project docs.
