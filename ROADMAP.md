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
      `docs/release/BETA4_RELEASE_DECISION.md`.

## Library navigation completion

- [x] Library removal — confirmed single-book and batch removal while source
      files remain untouched.

- [x] Reading Now — unfinished opened books ordered by the last successful
      reading time, with direct resume actions and truthful unavailable-source
      states.
- [x] Favorites — persistent local favorite markers and a dedicated view.
- [x] Authors — local author grouping and book drill-down.
- [x] Series — local series grouping and ordered book drill-down.
