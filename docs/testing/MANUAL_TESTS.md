# Manual tests — Stage 9 candidate

## Reading Now

1. Start with a book that has never been opened. Confirm it is absent from
   Reading Now.
2. Open the book successfully, return to the shell, and confirm it appears with
   the saved progress and a Continue Reading action.
3. Open a second book and confirm it sorts before the first. Rescan a watched
   folder and confirm the order does not change.
4. Temporarily make one source unavailable. Confirm its card remains visible,
   reports the unavailable state, and cannot be resumed.
5. Finish a book and confirm it leaves Reading Now without being removed from
   the library.

## Favorites

1. Mark a book from its library card. Confirm the heart exposes its pressed
   state and the book appears immediately in Favorites.
2. Select a different book and mark it from the details panel. Restart the app
   and confirm both markers persist.
3. Remove a book from Favorites and confirm its card leaves that view without
   deleting the library record, progress, annotations, or source file.
4. Make a favorite source unavailable and confirm its card remains visible with
   the normal unavailable-source state.

## Authors

1. Open Authors with books from several known authors and books without author
   metadata. Confirm every book appears in exactly one group and the localized
   counts are truthful.
2. Select an author and confirm only that author's books appear, with working
   progress, favorite, selection, and open actions. Use All Authors to return.
3. Edit a book's author metadata and confirm the group counts and drill-down
   update without a restart or source-file change.
4. Confirm author names that differ only by case share one group and Unknown
   Author remains a plain local-metadata fallback.

## Series

1. Open Series with books from several known series and books without series
   metadata. Confirm every book appears in exactly one group and the localized
   counts are truthful.
2. Select a series and confirm only that series' books appear, with working
   progress, favorite, selection, and open actions. Use All Series to return.
3. Confirm titles with numeric suffixes use natural order, such as `Book 2`
   before `Book 10`, without inventing volume numbers.
4. Edit a book's series metadata and confirm the groups update without a
   restart or source-file change. Confirm names that differ only by case or
   repeated whitespace share one group and No Series remains explicit.

## Genres and long-term achievements

1. Import disposable EPUB and FB2 fixtures containing multiple genre or subject
   values. Confirm the metadata editor shows normalized comma-separated genres,
   duplicates are removed, and the source files remain byte-identical.
2. Edit genres with commas, semicolons, repeated whitespace, mixed case, and
   duplicates. Save and restart; confirm the normalized local values persist.
3. Perform an explicit Open Library search and inspect a candidate with
   subjects. Confirm genres appear before Apply and only the chosen candidate
   changes the local book record.
4. Open but do not finish books from several authors, genres, and series.
   Confirm discovery achievement progress does not increase. Finish the books
   and confirm each distinct normalized value is counted once.
5. Confirm Achievements reports the unlocked count out of 42 and that each
   family increases from easy to long-term thresholds. Verify the 250-book,
   500-hour, 365-day, 100-author, 20-genre, and 50-series goals remain locked
   with truthful progress.
6. Restart and clear statistics in a disposable profile. Confirm unlocks and
   queued Steam entries follow the existing local reset policy while books,
   genres, reading positions, and annotations remain.

## Release candidate matrix

1. Start the production `aprireader.exe` in Russian and English. Use only Tab,
   Shift+Tab, Enter, Space, Escape, and arrow keys to reach every navigation
   destination, import action, filter, book, reader control, Settings action,
   Statistics action, and dialog. Confirm the skip link moves focus to content.
2. Repeat the shell and reader flow with Windows Narrator. Confirm route titles,
   book name/author/format/progress, alerts, recovery messages, and reading
   progress are announced without unlabeled controls.
3. Test Windows scaling at 100%, 150%, 200%, and 250%, then Windows
   forced-colors/high-contrast mode. Every destination must remain reachable;
   compact navigation may scroll but must not hide Settings or Statistics.
4. Load a synthetic 1,000-book library. Search and change format filters.
   Confirm only 120 cards initially render, the total remains truthful, “Show
   more” adds the next batch, and keyboard focus stays responsive.
5. Force-close the app while a synthetic book is open, restart, and confirm the
   interrupted-session notice appears, library integrity is checked, and saved
   reading progress remains usable.
6. On a disposable copy of app-local data, create a valid backup, replace
   `library.db` with invalid bytes, and restart. Confirm the library is restored,
   a recovery notice appears, and the damaged database exists under the local
   `recovery` directory. Confirm the source book bytes are unchanged.
7. Import valid and malformed synthetic fixtures for EPUB, PDF, FB2, TXT, HTML,
   Markdown, CBZ, CBR, and DOCX. Confirm malformed/oversized/unsafe input gives
   an error and never runs script or loads an external URL.
8. Run `pnpm check`. Confirm the security audit and SBOM-current check are the
   final gates. Review `docs/release/SECURITY_REVIEW.md`.
9. Execute `docs/release/CLOSED_BETA_CHECKLIST.md`. For the protected build,
   also execute `docs/release/STEAM_RC_CHECKLIST.md` and
   `docs/steam/TEST_CHECKLIST.md`.

The Rust integration suite complements step 7 with disposable valid and
malformed fixtures for all nine public formats. It opens them through the real
reader adapters, rejects unsafe or structurally invalid input, checks that HTML
does not expose an external script URL, and verifies every source fixture
remains byte-for-byte unchanged. This automated coverage does not replace the
manual oversized-input and external-request checks.

## Reader usability regression

1. Start the release EXE and confirm no Command Prompt or console window opens.
2. Open a reflow book and use the top toolbar arrows to move between chapters
   without scrolling to the footer.
3. In text settings, try every system-font profile and change size, line
   height, width, weight, letter spacing, word spacing, paragraph spacing, and
   alignment. Restart and confirm the choices persist.
4. Import a disposable TTF/OTF/WOFF/WOFF2 fixture. Confirm it appears by its
   filename, applies to the book, and the original file remains unchanged.
   Confirm a renamed non-font and a file larger than 24 MB are rejected.
5. Enable bionic highlighting. Confirm word beginnings become bold while
   selection, highlighting, notes, quotes, dictionary, and search still use
   the exact original text.
6. With page-wheel navigation enabled, roll once to move about one viewport.
   At the end or beginning of a chapter, roll again and confirm the adjacent
   chapter opens at its corresponding edge. Disable the option and confirm
   normal smooth scrolling returns.
7. Right-click the library and reader. Confirm the browser context menu does
   not appear; Ctrl+C and ApriReader selection actions still work.
8. Switch from the default Continuous text layout to Book spread. At a wide
   window confirm two equal pages are visible, text flows left-to-right, one
   wheel gesture turns one complete spread, and chapter edges open the
   adjacent chapter. Resize below 980 px and confirm one page remains visible.
   Switch back and confirm the same approximate section progress is restored.
   Restart and confirm the selected layout persists.
9. Confirm the bottom counter reads `Page N of M` in continuous mode and
   `Pages N–N+1 of M` in a wide spread. Turn pages, resize the window, and
   change font size and spacing. Confirm the current range advances and the
   measured total updates without covering reader controls.
