# Product specification

ApriReader is a premium, privacy-first desktop reader and personal digital
library for Windows 10/11 x64. It should feel like a calm private library, not a
file manager.

On first launch, ApriReader may ask how to address the user. This lightweight
local profile contains only an optional display name: it has no account,
password, Windows identity lookup, cloud synchronization, or network request.
The user can continue with a name or skip the step, then change or remove the
name in Settings. When present, it appears only in the time-aware library
greeting. The value stays in app-local storage.

The first public release targets DRM-free EPUB, PDF, FB2, TXT, HTML, Markdown,
CBZ, CBR, and DOCX. Source books remain in their original locations. Data is
local by default; network access always follows an explicit user action.

Stage 1 provides a real local library. A user can add individual files, register
folders, rescan those folders, search the resulting library, and filter it by
format. ApriReader fingerprints content to avoid duplicates, refreshes a record
when a file at the same path changes, shows when a source becomes unavailable,
extracts embedded EPUB/FB2 metadata and covers, and keeps rolling local database
backups. Import never modifies or relocates a source book.

Audiobook stage A1 stores audio titles separately from text books. A user can
import one audio file, select several parts together, import an audiobook
directory, or register a watched audio collection. Selected files sharing a
directory and files inside an explicitly imported folder are grouped into one
ordered audiobook. Direct files in the root of a watched collection remain
separate, while nested directories represent multi-part books. Natural numeric
ordering places part 2 before part 10.

Each audio file is limited to 20 GiB, one audiobook to 100 GiB and 1,000 parts,
and one watched collection scan to 100,000 supported files with a maximum
recursion depth of 12. Symbolic links, DRM formats, descriptors, and unknown
extensions never enter the audio database. Descriptor files are accepted only
through the bounded local CUE/M3U/M3U8 parser described below. SHA-256
fingerprints and canonical
paths support repeat scans and changed files. Missing parts remain visible as
unavailable so later playback stages can explain an incomplete book without
discarding local progress or touching source files.

Audiobook stage A2 exposes that model through a separate Audiobooks
destination. Audio data is loaded only when the destination is opened. The
screen supports local title/author search, file import, complete-folder import,
watched-folder registration and repeat scanning. Cards report part count,
total size, progress, and incomplete-source state; selecting a card shows its
naturally ordered parts and disables the player destination when source media
is incomplete.

Audiobook stage A3 connects the player to the native Windows media boundary.
It provides play/pause, 15-second backward and forward seeking, previous/next
part navigation, automatic advance, a selectable ordered queue, volume, and
0.5x–3.0x playback speed. Speed and volume are device-local preferences. The
last part and second are restored, while observed part durations and aggregate
book progress are saved locally every five seconds and at pause, seek, part
change, and close boundaries. Source audio remains read-only.

Audiobook stage A4 accepts local CUE, M3U, and M3U8 descriptors up to 2 MiB.
Remote URLs, absolute entries, and relative paths that escape the descriptor's
directory are rejected. Playlist order is preserved; CUE `INDEX 01` positions
and track titles become ordered chapters. The player adds 15/30/45/60-minute
and end-of-part sleep timers plus durable local bookmarks with optional notes.
While audio is active, closing the window either asks, continues in the Windows
system tray, or exits according to a device-local preference. The ask dialog
can remember either decision, and Settings can change it later.

Audiobook stage A5 lets the listener choose an enabled Windows audio-output
device or return to the system default. This preference stays on the device.
The audiobook details drawer supports local title, author, narrator, series,
genre, language, year, description, and cover editing plus explicit
Russian/English online metadata search. Cover files are validated and copied
to app-local storage; source audio is never rewritten.

Statistics separately report active audiobook listening today and in total,
started titles, and completed titles. Only native playing intervals count,
with bounded event gaps so sleep, suspension, or a stalled renderer cannot
invent listening time. Seven audiobook-only achievements cover first playback,
completion, ten completed titles, and progressive listening-time milestones.

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
restored progress. A PDF must have a valid PDF file signature before it enters
the app-local reader cache. CBZ and CBR use an image-sequence reader with natural page
ordering, single/two-page layouts, and left-to-right or right-to-left reading.
DOCX joins the safe reflow reader as semantic text rather than promising a
pixel-identical Word layout. Fixed-format cache files remain app-local and
source books are never modified.

Stage 5 adds local manual editing for title, author, subtitle, ISBN, publisher,
publication year, language, series, genres, and description. A user explicitly
chooses Russian or English before sending an ISBN or title/author query. English
search uses Open Library; Russian search combines Russian-edition results from
Open Library and the public FantLab bibliographic API, labels every provider,
and removes duplicate editions. Search never runs during startup, import,
folder scanning, or reading. Responses are cached locally for 30 days and
requests are limited to one user-triggered search per second.

Applying a candidate records its provider, identifier, and update time. A
selected Open Library cover is downloaded only while applying that candidate,
validated as an image, and stored in the app-local cache. The user can remove
it and return to the embedded cover or fallback without changing the source
book.

Only while the manual editor is open, the displayed cover is an explicit file
selection control. A selected JPG, PNG, or WebP image is limited to 10 MiB,
validated by signature, and copied into the app-local cover cache. Restoring the
embedded cover deletes only the app-managed copy. Neither action changes the
source book or runs implicitly.

Genre metadata is a local, comma-separated multi-value field. Import collects
bounded EPUB subjects and FB2 genres; an explicitly selected Open Library
candidate may provide a bounded subject list. The user can edit and normalize
the values locally, and the source book remains unchanged.

Selecting up to 2,000 characters in a reflow book exposes a Translate action
with Google Translate and Yandex Translate choices. ApriReader detects whether
the selection is predominantly English or Russian, constructs only a fixed
HTTPS translator URL for the corresponding EN-RU direction, and opens it in
the user's default browser. It does not fetch or render remote translator
content inside the application.

Before the first handoff, ApriReader clearly states that the selected text will
be sent to the chosen external service and requires explicit confirmation.
That consent is stored locally and can never cause background transmission;
each later handoff still requires choosing a provider. Whole-book translation,
language-package import, model download, and arbitrary translator URLs are not
supported. Source books remain unchanged.

The Windows installer registers ApriReader as a viewer for every supported
book extension: EPUB, FB2, TXT, HTML/HTM, Markdown/MD, PDF, CBZ, CBR, and DOCX.
Opening one of those files from Explorer imports it into the local library when
needed and immediately opens the matching safe reader. If ApriReader is already
running, the existing window is focused and receives the path instead of
leaving a second instance open. Duplicate content opens the existing local
record. The normal importer validates the path, size, signature, archive
boundaries, and normalized content; the source book remains in place and is
never modified.

Audiobook stage A6 registers a separate ApriReader Audiobook viewer type for
AAC, FLAC, M4A/M4B, MP3, WAV, WMA, 3G2/3GP, AMR, AIF/AIFF, ALAC, APE, CAF,
MKA, MPC, OGA/OGG, OPUS, WV, and local CUE/M3U/M3U8 descriptors. Formats in
the system-codec tier remain dependent on a decoder installed in Windows; the
association does not claim bundled codec support. AAX/AAXC/M4P and unknown
extensions are not registered.

Opening an associated audio file imports or reconnects it through the same
bounded database transaction as manual audio import, reuses duplicate content,
and immediately opens the audiobook player. If ApriReader is already running,
the existing window receives the path and is focused. Descriptor traversal and
remote URLs remain rejected, and source audio is never modified.

Audiobook stage A7 adds an explicit Read Aloud panel to reflow readers. It
reads only the current section, lists voices already installed in Windows,
remembers the selected voice and 0.5x–2.0x rate, and exposes play,
pause/resume, and stop. Text and generated audio remain local. A section over
20,000 Unicode characters is rejected visibly instead of being truncated, and
generated WAV data is limited to 64 MiB with a 64-section app-local cache.

A7 does not silently continue into the next section, highlight the current
word, download voices, or call a cloud speech provider. Whole-book chunking,
word synchronization, and optional BYOK providers require later, separately
reviewed stages.

Audiobook stage A8 adds a remembered choice between the current section and
the whole book from the current section. Narration splits text into local
fragments no larger than 1,200 UTF-16 units, prepares the next fragment during
playback, and moves to the next section automatically. A session is limited to
50,000 fragments; larger books remain available section by section instead of
creating an unbounded queue.

The reader marks and scrolls to the active word using the native WAV playback
position within the current short fragment. This position-based focus is
preserved alongside annotations and bionic rendering without changing locator
offsets. It is not claimed as phoneme-accurate engine metadata. Manual section
navigation, panel close, or changed voice/rate/scope stops the owned session.

Audiobook stage A9 adds ElevenLabs as an optional BYOK provider. Windows voices
remain the default and require no network. The user must explicitly select
ElevenLabs, provide an API key, and accept a first-send disclosure stating that
selected book fragments leave the device, are processed under ElevenLabs terms
and privacy policy, and may consume paid quota. No request occurs during app
startup, import, scanning, ordinary reading, or local narration.

The key is stored in Windows Credential Manager and is never displayed after
saving. The user can delete it from the Read Aloud panel, which also clears
consent and returns to local voices. The provider voice list is fetched only
after ElevenLabs is selected with a stored key. Each speech request contains
one bounded A8 fragment; returned MP3 and exact character timing are validated
and cached locally. Provider failure never falls back by sending text to a
different service.

Audiobook stage A10 allows up to 20 named voice presets containing provider,
voice ID, and rate. Presets never contain API keys. A separate, locally stored
pronunciation dictionary contains at most 100 unique source/replacement pairs,
matches complete words or phrases without case sensitivity, and can be turned
off without deleting it. Saving or changing a rule stops the current owned
narration so cached speech cannot outlive the selected settings.

Dictionary replacement changes only the temporary synthesis request. The
displayed document, locators, annotations, search index, and source file remain
unchanged. Expanded fragments above 2,000 UTF-16 units fail visibly. Cloud
character timing is translated back to source offsets before word focus.

Audiobook stage A11 adds Google Cloud Text-to-Speech as a second optional BYOK
service. The user must enable that API and billing in their own Google Cloud
project, provide a key, select Google explicitly, and accept a provider-specific
disclosure before book text leaves the device. Google consent and credentials
are independent from ElevenLabs and are cleared only by deleting the Google
key. No Google request occurs during startup, import, scanning, or local speech.

The key is stored only as `ApriReader/GoogleCloudTtsApiKey` in Windows
Credential Manager and is sent in the native `x-goog-api-key` header. It never
enters a URL, preset, log, status message, or WebView storage. Requests use
plain text of at most 2,000 Unicode characters and 4,800 UTF-8 bytes against a
fixed host. Returned base64 MP3 is size/signature validated before an atomic
app-local cache write. Service availability, billing, price, and quotas remain
the responsibility of the user's Google Cloud project.

Audiobook stage A12 adds Azure AI Speech BYOK. The user selects the exact region
of their Speech resource, saves its key, and accepts an Azure-specific disclosure
before text is sent. Only 33 reviewed public regions are accepted; arbitrary,
government, China, custom-subdomain, and user-supplied hosts are unsupported.
The key is stored only as `ApriReader/AzureSpeechApiKey`; the region is a local
non-secret preference. Synthesis accepts at most 2,000 characters, XML-escapes
all temporary text into at most 16 KiB SSML, and validates up to 32 MiB MP3.

Audiobook stage A13 adds bounded provider-specific expressiveness. ElevenLabs
accepts stability, similarity, style, and speaker-boost settings in its native
request; Google accepts pitch from -20 to +20 semitones; Azure accepts an
escaped SSML pitch from -50% to +50%. These settings remain local, become part
of cache identity, and are included in voice presets without credentials.

The Read Aloud panel reports cache file counts and bytes for each provider and
can clear one provider or all recognized TTS files. Clearing matches only exact
ApriReader-generated names and never recursively deletes the cache root.

An explicit export action synthesizes the selected section or remaining book
into at most 5,000 numbered WAV/MP3 parts, an M3U8 playlist, and at most 6 GiB
of aggregate media. Each prepared cache part is copied immediately into a new
sibling media directory before normal cache pruning. A partial export is
isolated and removed on cancellation. Cloud export repeats the provider/quota
warning; playback-only rate changes are not misrepresented as embedded audio.

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
environment. Synchronization stays automatic and does not expose build,
provider, queue, or Overlay diagnostics in the user-facing Settings page.

Stage 9 hardens the text-release candidate without expanding its feature set.
ApriReader checks SQLite integrity at startup, identifies an interrupted prior
session, and restores only from the newest valid app-generated backup when the
active database cannot be opened. The damaged database and its sidecars are
preserved in an app-local recovery quarantine; source books are never changed.

The shell remains fully reachable with keyboard and screen-reader navigation
under Windows scaling and forced-colors mode. Large search results render in
bounded batches while truthful totals remain visible. Every candidate build
must pass the repository security gate, reproduce its lockfile-derived SBOM,
record the exact source-tree state and a SHA-256 source manifest, and complete
the documented closed-beta checks before it can be called a public GitHub
release. The protected Steam checklist gates only the separate Steam profile.
Packaging must stop if source files change while the candidate is being built.

Opening any reader places focus on its visible top toolbar. The first Tab
therefore follows the reader controls instead of moving a stale focus target
to chapter-footer or page-footer controls.

Reader polish adds system-font profiles and bundled OFL-licensed Literata,
Lora, Merriweather, Source Serif 4, Charis SIL, and IBM Plex Serif families.
Family, normal/italic style, and the real weights available for that family are
selected independently; variable fonts apply optical sizing automatically.
Explicit import of a local TTF, OTF, WOFF, or WOFF2 file remains available.
Imported fonts are copied into app-local storage after extension, signature,
and size checks; the source file remains unchanged. Typography also includes
size, line height, column width, letter spacing, word spacing, paragraph
spacing, and alignment. Optional bionic highlighting bolds the beginning of
words without changing their text or annotation locators.

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

Settings provide an enabled-by-default Screen Reader Support option. It controls
only ApriReader's additional chapter, page, and reading-state announcements;
essential accessible names remain present when it is disabled. When trustworthy
book language metadata is available, every reader exposes a normalized language
tag so Windows Narrator can select the appropriate installed voice. ApriReader
bundles no speech engine or voice model; its explicit local Read Aloud action
uses speech services and voices installed in Windows.

High Windows scaling must not remove any destination or reading action.
Short desktop layouts keep the navigation rail independently scrollable. At
compact widths, destinations move to a horizontally scrollable bottom rail
while the language switch remains fixed and reachable. Icon-only navigation
keeps explicit accessible names. Reflow reader actions may use a second
horizontal toolbar row, while fixed-format controls and page position remain
visible without reducing interactive targets below 44 CSS pixels.

When Windows forced-colors mode is active, ApriReader uses the system canvas,
text, button, highlight, link, and disabled colors throughout the shell and
all readers. Selected, active, locked, unlocked, progress, calendar, warning,
and annotation states must remain distinguishable through borders, line
styles, or patterns rather than color alone. Source book covers, rendered PDF
pages, and comic artwork retain their document colors. Disabled controls
remain legible, and every visual choice continues to expose its semantic state.
