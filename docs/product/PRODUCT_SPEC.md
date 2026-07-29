# Product specification

ApriReader is a premium, privacy-first desktop reader and personal digital
library for Windows 10/11 x64. It should feel like a calm private library, not a
file manager.

The first public release targets DRM-free EPUB, PDF, FB2, TXT, HTML, Markdown,
CBZ, CBR, and DOCX. Source books remain in their original locations. Data is
local by default; network access always follows an explicit user action.

Stage 1 provides a real local library. A user can add individual files, register
folders, rescan those folders, search the resulting library, and filter it by
format. ApriReader fingerprints content to avoid duplicates, refreshes a record
when a file at the same path changes, shows when a source becomes unavailable,
extracts embedded EPUB/FB2 metadata and covers, and keeps rolling local database
backups. Import never modifies or relocates a source book.

Stage 2 adds a distraction-free reflow reader for TXT, HTML, Markdown, EPUB,
and FB2. Books open from the details panel or by double-clicking a library card.
The reader provides a table of contents, section navigation, paper/sepia/night
themes, adjustable type size, line spacing and text width, and automatically
restores the last saved section and scroll position.

Stage 3 adds local full-text search, stable reading locators, chapter bookmarks,
highlights, notes, saved quotes, and Markdown export. Text selection is limited
to one normalized block so anchors remain explicit and deterministic. Search
and annotation data stay in the local SQLite library; exports occur only after
the user chooses a destination.

Stage 4 completes the first-release format set. PDF uses a dedicated
fixed-layout canvas viewer with page navigation, zoom, keyboard controls, and
restored progress. CBZ and CBR use an image-sequence reader with natural page
ordering, single/two-page layouts, and left-to-right or right-to-left reading.
DOCX joins the safe reflow reader as semantic text rather than promising a
pixel-identical Word layout. Fixed-format cache files remain app-local and
source books are never modified.

Stage 5 adds local manual editing for title, author, subtitle, ISBN, publisher,
publication year, language, series, genres, and description. A user may explicitly
send an ISBN or title/author query to Open Library, compare several candidates,
and apply exactly one chosen result. Search never runs during startup, import,
folder scanning, or reading. Responses are cached locally for 30 days and
requests are limited to one per second.

Applying a candidate records its provider, identifier, and update time. A
selected Open Library cover is downloaded only while applying that candidate,
validated as an image, and stored in the app-local cache. The user can remove
it and return to the embedded cover or fallback without changing the source
book.

Genre metadata is a local, comma-separated multi-value field. Import collects
bounded EPUB subjects and FB2 genres; an explicitly selected Open Library
candidate may provide a bounded subject list. The user can edit and normalize
the values locally, and the source book remains unchanged.

Stage 6 adds user-imported offline language packages. Selecting text in a
reflow book exposes dictionary and translation actions without sending the
selection anywhere. Dictionary lookup uses verified local JSON packages.
Translation uses a selected, verified ONNX package through a text-in/text-out
provider contract.

ApriReader does not bundle, recommend, discover, or download dictionaries or
models. The permissively licensed ONNX CPU runtime is part of the application;
packages cannot add native code. Import is always explicit. Every package must declare its
source, SPDX license, attribution, engine compatibility, exact file sizes, and
SHA-256 hashes. Unsupported licenses, unsafe archive paths, oversized content,
and modified installed files are rejected.

Stage 7 records active reading sessions locally. Time is counted only while a
reader is open, the app is visible and focused, and the user interacted
recently. Long gaps, duplicate heartbeats, and an open idle window do not
inflate time, words, or pages. The user can inspect totals, streaks, an
84-day calendar, and a configurable daily goal, then delete all statistics
without deleting books, progress, or annotations.

A single canonical registry defines local achievements for opening and
finishing books, active time, reading volume, streaks, annotations, authors,
series, and the daily goal. Unlocks are idempotent and local. Steam mapping and
offline synchronization remain Stage 8 work.

The registry contains 42 progressively harder goals, from opening and
finishing a first book through 250 completed books, 500 active hours, a
365-day streak, 100 authors, 20 genres, and 50 series. Author, genre, and series
progress counts only books that reached completion after being opened in the
reader. Genre values are split and compared case-insensitively; opening several
books without finishing them cannot advance discovery achievements.

Stage 8 adds an offline-first Steam synchronization queue. Every local unlock
keeps its canonical identifier and remains pending until a protected Steam
provider confirms storage. Failed or offline attempts never remove queue data,
and successful retries do not issue the same local queue entry twice.

The public GitHub build contains no Steamworks SDK, bridge, App ID, or
credentials and continues to provide all achievements locally. A separate
Steam build profile loads a protected bridge supplied only by the release
environment. Settings reports the active profile and queue state without
claiming that Steam is connected when it is not.

Stage 9 hardens the text-release candidate without expanding its feature set.
ApriReader checks SQLite integrity at startup, identifies an interrupted prior
session, and restores only from the newest valid app-generated backup when the
active database cannot be opened. The damaged database and its sidecars are
preserved in an app-local recovery quarantine; source books are never changed.

The shell remains fully reachable with keyboard and screen-reader navigation
under Windows scaling and forced-colors mode. Large search results render in
bounded batches while truthful totals remain visible. Every candidate build
must pass the repository security gate, reproduce its lockfile-derived SBOM,
and complete the documented closed-beta and protected Steam checklists before
it can be called a public release.

Reader polish adds system-font profiles plus explicit import of a local TTF,
OTF, WOFF, or WOFF2 file. Imported fonts are copied into app-local storage
after extension, signature, and size checks; the source file remains unchanged.
Typography includes size, line height, column width, weight, letter spacing,
word spacing, paragraph spacing, and alignment. Optional bionic highlighting
bolds the beginning of words without changing their text or annotation
locators.

The reading toolbar always exposes previous/next chapter controls. Optional
page-wheel navigation moves one viewport per gesture and crosses to the
adjacent chapter at an edge. Browser-authored context menus are suppressed;
keyboard copy and ApriReader's selection actions remain available.

Reflow books offer two persistent reading layouts. Continuous text remains the
default and scrolls vertically in one centered column. Book spread mode flows
the same normalized blocks across two fixed-height pages and turns one complete
spread per wheel gesture. It falls back to one page when the reader window is
too narrow and crosses chapter boundaries without requiring the chapter
footer.

A persistent footer reports the current reflow page against the measured book
total. Continuous mode uses “Page N of M”; a wide spread uses “Pages N–N+1 of
M”. Page totals are derived from the actual normalized layout and are
recalculated after viewport or typography changes.

The Reading Now destination contains only books that were opened successfully
and have not reached completion. It orders them by the last local reading time,
shows their saved progress and source availability, and provides a direct
Continue Reading action. Merely importing or rescanning a source must not place
it in Reading Now.

Favorites are explicit local user data. A book can be marked or unmarked from
its library card or details panel, and the dedicated Favorites destination
updates immediately. The marker survives restarts and rescans without changing
the source book.

The Authors destination groups the current local library by trimmed,
case-insensitive author metadata. It shows truthful book counts, keeps books
without author metadata in an explicit Unknown Author group, and opens a local
drill-down containing the normal library cards. Editing metadata immediately
updates the grouping without changing the source book.

The Series destination groups the current local library by trimmed,
case-insensitive series metadata and collapses repeated whitespace. It shows
truthful book counts, keeps books without series metadata in an explicit No
Series group, and opens a local drill-down containing the normal library cards.
Books in a series use locale-aware natural title order, so numeric title
suffixes remain intuitive. Editing metadata immediately rebuilds the groups
without changing the source book.

Library records can be removed individually from book details or in a batch
from an explicit selection mode. Removal always requires confirmation and
deletes only ApriReader-owned metadata, progress, annotations, search data, and
cache files. The source book remains unchanged in its original location.
