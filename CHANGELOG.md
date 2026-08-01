# Changelog

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
