# Roadmap

- [x] Stage 0 — Tauri/React/Rust foundation, design system, empty AppShell,
      SQLite migration framework, localization, tests, CI, license, and docs.
- [x] Stage 1 — library database, import, duplicates, watched folders, virtual
      collections, embedded EPUB/FB2 metadata and covers, and local backups.
- [x] Stage 2 — reflow reader for TXT, HTML, Markdown, EPUB, and FB2.
- [x] Stage 3 — annotations, locators, bookmarks, notes, quotes, and FTS5.
- [x] Stage 4 — PDF, CBZ/CBR, DOCX and format-specific UX.
- [x] Stage 5 — opt-in metadata providers and manual metadata editing.
- [x] Stage 6 — licensed offline dictionaries and translation packages.
- [x] Stage 7 — active reading statistics and canonical achievements.
      Achievement depth now includes 42 progressive goals, completed-book
      discovery rules, and local multi-value genres.
- [x] Stage 8 — Steam adapter and offline achievement synchronization.
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
      release-candidate build.

## Library navigation completion

- [x] Library removal — confirmed single-book and batch removal while source
      files remain untouched.

- [x] Reading Now — unfinished opened books ordered by the last successful
      reading time, with direct resume actions and truthful unavailable-source
      states.
- [x] Favorites — persistent local favorite markers and a dedicated view.
- [x] Authors — local author grouping and book drill-down.
- [x] Series — local series grouping and ordered book drill-down.
