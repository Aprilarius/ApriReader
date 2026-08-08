# Roadmap

- [x] Stage 0 — Tauri/React/Rust foundation, design system, empty AppShell,
      SQLite migration framework, localization, tests, CI, license, and docs.
- [x] Stage 1 — library database, import, duplicates, watched folders, virtual
      collections, embedded EPUB/FB2 metadata and covers, and local backups.
- [x] Stage 2 — reflow reader for TXT, HTML, Markdown, EPUB, and FB2.
- [x] Stage 3 — annotations, locators, bookmarks, notes, quotes, and FTS5.
- [x] Stage 4 — PDF, CBZ/CBR, DOCX and format-specific UX.
- [x] Stage 5 — opt-in Russian/English metadata providers, manual metadata
      editing, and validated app-local cover replacement/restoration.
- [x] Stage 6 — explicit selected-text handoff to Google or Yandex Translate
      in the default browser, limited to EN-RU and RU-EN with first-use consent.
- [x] Stage 7 — active reading statistics and canonical achievements.
      Achievement depth now includes 42 progressive goals, completed-book
      discovery rules, and local multi-value genres.
- [x] Stage 8 — Steam adapter and offline achievement synchronization.
- [x] Audiobooks A0 — isolated Windows MediaPlayer prototype, safe format and
      size validation, native probe/load, play/pause, seek, 0.5x-3.0x speed,
      volume, state snapshots, and real PCM WAV smoke coverage.
- [x] Audiobooks A1 — audio library schema, bounded multi-part import,
      automatic grouping, watched folders, and source availability handling.
- [x] Audiobooks A2 — dedicated lazy-loaded audiobook library, file/folder
      import actions, watched audio collections, search, availability-aware
      cards, part details, and the prepared player destination.
- [x] Audiobooks A3 — native play/pause, 15-second seek, part navigation and
      automatic advance, 0.5x–3.0x speed, volume, ordered queue, resume, and
      bounded durable position/duration progress.
- [x] Audiobooks A4 — bounded local CUE/M3U/M3U8 import, CUE chapters, sleep
      timer, durable bookmarks, remembered close behavior, and Windows tray
      playback controls.
- [x] Audiobooks A5 — output device selection, metadata, listening statistics,
      and dedicated audiobook achievements.
- [x] Audiobooks A6 — Explorer associations, bounded import-or-dedupe,
      existing-instance forwarding, immediate player opening, and release
      hardening.
- [x] Audiobooks A7 — local Windows text-to-speech for the current reflow
      section, installed-voice and rate selection, bounded WAV caching, and
      play/pause/stop through the existing audio owner.
- [x] Audiobooks A8 — bounded whole-book speech queue from the current section,
      background preparation of the next fragment, automatic section
      transitions, position-synchronized word focus, and auto-scroll.
- [x] Audiobooks A9 — opt-in ElevenLabs BYOK, Windows Credential Manager key
      storage, bounded fixed-host requests, explicit text-transfer consent,
      cloud voice selection, and provider character timing.
- [x] Audiobooks A10 — local pronunciation dictionary with source-offset
      preservation, enable/disable control, and user-defined voice presets.
- [x] Audiobooks A11 — Google Cloud Text-to-Speech BYOK with a fixed native
      API boundary, Credential Manager storage, language-aware voice discovery,
      separate consent, and bounded MP3 synthesis.
- [x] Audiobooks A12 — Azure AI Speech BYOK with a reviewed 33-region allowlist,
      protected key, safe SSML, language-aware voices and separate consent.
- [x] Audiobooks A13 — provider-specific expressive controls, per-provider cache
      inspection/clearing, and bounded incremental M3U8 audiobook export.
- [ ] Audiobooks A14+ — further providers only after a separate API, privacy,
      licensing, quota, and release-boundary review.
- [ ] Stage 9 — release hardening, accessibility, security and closed beta.
      Engineering hardening and candidate gates are implemented; completion
      awaits the remaining Windows 10/11 closed-beta and protected Steamworks
      evidence. The product owner reported a successful beta.4 manual smoke
      pass on 2026-07-29; the scoped HOLD decision is recorded in
      `docs/release/BETA4_RELEASE_DECISION.md`. An automation-assisted Windows
      11 accessibility smoke found a reader-entry focus jump; the beta.5 source
      fix and remaining matrix cells are recorded in
      `docs/release/WINDOWS11_BETA4_ACCESSIBILITY_SMOKE.md`. The installed
      beta.5 candidate then passed product-owner smoke testing with no critical
      defects reported; its scoped HOLD decision is recorded in
      `docs/release/BETA5_RELEASE_DECISION.md`. The beta.6 source adds a
      persistent Windows Narrator support toggle, language tagging, and
      optional chapter/page announcements while preserving essential control
      semantics. The beta.7 source hardens short and narrow layouts, preserves
      all compact navigation names and the language switch, and reflows reader
      controls for high-scaling viewports. The real Windows 10/11 scaling
      matrix remains an external candidate test. The beta.8 source applies
      Windows system colors across the shell and every reader, reinforces
      active and informational states without color alone, and preserves
      source document imagery. High Contrast Black and White remain external
      installed-candidate checks. The product owner reported beta.8 successful
      on 2026-07-30. Beta.9 hardens candidate provenance with an exact source
      manifest, a truthful clean or modified tree state, and a guard against
      source changes during packaging. Version 1.0.0-rc.1 consolidates the
      tested accessibility work and requires a clean Git tree for every
      release-candidate build. The product owner installed RC1 and reported no
      bugs on 2026-07-30; the exact scoped PASS and remaining public-release
      gates are recorded in `docs/release/RC1_RELEASE_DECISION.md`. The product
      owner selected an unsigned public GitHub 1.0 release as the next delivery
      profile; its clean-build, evidence, warning, and verification rules are
      recorded in `docs/release/GITHUB_1_0_RELEASE_POLICY.md`. Code signing and
      the protected Steamworks profile remain separate follow-up gates. The
      exact clean 1.0 artifact passed its full build gate and received GO for
      publication in `docs/release/GITHUB_1_0_RELEASE_DECISION.md`. The public
      GitHub release was published and its anonymous installer download was
      verified on 2026-07-30.

## Windows shell integration

- [x] Book file associations — the NSIS installer registers all supported
      extensions as ApriReader viewer types; Explorer activation imports and
      opens the selected local book in the existing single application
      instance through the normal untrusted-book boundary.
- [x] Audiobook file associations — the NSIS installer registers every safe
      supported audio and local CUE/M3U/M3U8 descriptor extension; Explorer
      activation imports or reuses the audiobook and opens its player in the
      existing instance without accepting DRM or executable paths.

## Library navigation completion

- [x] Optional local profile — a skippable first-launch name prompt,
      personalized time-aware greeting, and local Settings edit/removal without
      accounts, passwords, OS identity access, or network integration.

- [x] Library removal — confirmed single-book and batch removal while source
      files remain untouched.

- [x] Reading Now — unfinished opened books ordered by the last successful
      reading time, with direct resume actions and truthful unavailable-source
      states.
- [x] Favorites — persistent local favorite markers and a dedicated view.
- [x] Authors — local author grouping and book drill-down.
- [x] Series — local series grouping and ordered book drill-down.
