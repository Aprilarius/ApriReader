# Manual tests — Stage 9 candidate

## External selected-text translation

1. Open a reflow book, select a short English phrase, choose Translate, then
   Google Translate. Confirm the first use shows the privacy disclosure and no
   browser opens before Continue.
2. Continue and confirm the default browser opens Google Translate with the
   exact selection and EN-RU direction. Return to ApriReader, select Russian
   text, choose Yandex Translate, and confirm RU-EN direction without a second
   disclosure.
3. Cancel the first-use disclosure and confirm nothing opens and consent is not
   remembered. Restart ApriReader and confirm the disclosure still appears.
4. Select more than 2,000 characters and confirm ApriReader shows the limit
   message without opening a browser. Confirm Settings has no dictionary or
   language-package panel.
5. Disconnect the network and confirm ApriReader itself remains responsive;
   only the external browser page may fail to load.

## Optional local profile

1. Start with no `aprireader.localProfile` value. Confirm a focused welcome
   screen asks how to address the user, explains local-only storage, offers
   Continue and Skip, and keeps the language switch reachable.
2. Confirm Continue is disabled for an empty or whitespace-only name. Enter a
   name with leading, trailing, and repeated whitespace; continue and confirm
   the library greeting uses the normalized name with the correct time of day.
3. Reset the profile and choose Skip. Restart and confirm the welcome screen
   does not repeat and the library uses a generic time-aware greeting.
4. In Settings, change the display name and confirm the greeting updates.
   Remove the name, restart, and confirm the generic greeting returns.
5. Confirm the profile creates no network request, Windows identity prompt,
   password, account selector, or change to books and reading data.

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

## Windows file associations

1. Install the NSIS candidate for the current user. In Windows Settings or
   Explorer's Open with menu, confirm ApriReader is offered for EPUB, FB2, TXT,
   HTML/HTM, Markdown/MD, PDF, CBZ, CBR, and DOCX.
2. With ApriReader closed, double-click one valid disposable fixture in every
   supported format. Confirm ApriReader starts, adds the book to its local
   library only once, and opens the correct reflow or fixed-layout reader.
3. Keep ApriReader open, then double-click a second fixture in Explorer.
   Confirm the existing window is focused, no second ApriReader window remains,
   and the selected book opens.
4. Open the same book again and confirm the existing library record and saved
   progress are reused. Confirm the source file hash and timestamp are
   unchanged.
5. Try a renamed executable, a missing path, an oversized fixture, and malformed
   book containers under disposable extensions. Confirm ApriReader reports an
   error, performs no external request, executes no content, and remains usable.
6. Uninstall ApriReader and confirm its Explorer handler entries are removed
   without deleting source books or app-local user data.

The Rust integration suite complements step 7 with disposable valid and
malformed fixtures for all nine public formats. It opens them through the real
reader adapters, rejects unsafe or structurally invalid input, checks that HTML
does not expose an external script URL, and verifies every source fixture
remains byte-for-byte unchanged. This automated coverage does not replace the
manual oversized-input and external-request checks.

## Reader usability regression

1. Start the release EXE and confirm no Command Prompt or console window opens.
   Confirm the library greeting matches the local time, the sidebar contains no
   roadmap stage or build-status copy, and the library search disappears on
   every non-Library destination. Search for a missing title and confirm the UI
   reports no matches rather than claiming that the whole library is empty.
2. Open a reflow book and confirm focus starts on the toolbar back control.
   Press Tab and confirm focus follows the toolbar without scrolling to the
   chapter footer. Use the top toolbar arrows to move between chapters.
3. In text settings, try every system-font profile and the bundled Literata,
   Lora, Merriweather, Source Serif 4, Charis SIL, and IBM Plex Serif families.
   For each bundled family, switch between normal and italic and confirm the
   weight selector exposes only its real Thin through Black range. Change size,
   line height, width, letter spacing, word spacing, paragraph spacing, and
   alignment. Confirm the preview changes with family, style, and weight.
   Restart and confirm the choices persist and Cyrillic text keeps the selected
   face.
4. Import a disposable TTF/OTF/WOFF/WOFF2 fixture. Confirm it appears by its
   filename, applies to the book, and the original file remains unchanged.
   Confirm a renamed non-font and a file larger than 24 MB are rejected.
5. Enable bionic highlighting. Confirm word beginnings become bold while
   selection, highlighting, notes, quotes, translation, and search still use
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
10. Open a large/slow book and immediately open another book. Confirm only the
    newest request reaches the reader. Scroll, close within 350 ms, reopen, and
    confirm the last position was flushed. Switch chapters immediately after a
    scroll and confirm the old chapter timer cannot overwrite the new chapter.
11. Make an imported source unavailable, then open an identical relocated copy
    through Explorer. Confirm the existing card reconnects to the new path and
    opens without a duplicate. Replace a disposable cached PDF or comic with a
    newer valid source at the same path and confirm its app-local cache rebuilds.
12. Deny clipboard access, select text, and choose Copy quote. Confirm the quote
    remains in Annotations and the status says copying was unavailable instead
    of claiming clipboard success.
