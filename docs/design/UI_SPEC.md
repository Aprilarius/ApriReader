# ApriReader UI specification

The approved visual language is warm, calm, elegant, restrained, and
desktop-first. Centralized tokens live in `src/ui/styles.css`.

- Background: ivory `#F7F3EA`; surface: cream `#FCFAF5`.
- Navigation: charcoal `#1B1D1C`; raised charcoal `#252725`.
- Text: graphite `#292A28`; secondary `#77746D`.
- Accent: brass `#B88A38` and light brass `#D4AD62`.
- Headings use a bookish system serif; controls use Segoe UI Variable.
- Desktop uses a 232 px navigation rail, fluid library, and 372 px details
  panel. Narrow layouts collapse the details panel and then the navigation rail.
- Interactive targets are at least 44 px and keyboard focus is always visible.
- Empty states must be truthful. Do not invent books, progress, covers, or
  statistics.
- The shell never exposes roadmap stages, build profiles, candidate status, or
  other development-process labels. The sidebar footer contains only the
  language action.
- The library greeting follows the local time of day. Library search is shown
  only on the Library destination, and an empty filtered result is distinct
  from an empty library.
- The first launch uses one calm, focused welcome card asking how to address
  the user, with Continue and Skip actions. It states that the optional name
  stays on the device, keeps the language switch reachable, and never resembles
  account registration. Settings expose the same single field plus an explicit
  Remove name action. A saved name extends the time-aware greeting; a skipped
  or removed name leaves the greeting natural and generic.
- Covers come from user files; missing covers use a programmatic 2:3 fallback.
  Do not download or generate demonstration cover art.

Stage 1 retains the approved shell and adds a compact import toolbar, format
collection chips, a responsive cover grid, availability badges, a selected-book
details panel, and a watched-folder list. Import and rescan feedback appears
inline and remains keyboard accessible. No reading progress is invented before
the reader exists.

Stage 2 uses a quieter full-window reading surface. A compact top bar exposes
back, table-of-contents, and typography controls; the text column remains
centered and independently scrollable. Paper, sepia, and night themes reuse the
same restrained palette. Settings appear in a temporary side panel, while the
book content has no decorative frame. On narrow screens, toolbar labels collapse
to icons and the text column keeps safe 18 px side margins.

Stage 3 extends the same toolbar with search, bookmark, and annotations actions.
Search and annotation collections use temporary side panels rather than
replacing the reading surface. Selecting text reveals a compact bottom action
bar for highlight, note, and quote actions. Highlights use translucent brass;
notes add a restrained underline; saved quotes use a dashed underline. All
annotation controls preserve 44 px targets and the paper/sepia/night palettes.

Stage 4 gives fixed-layout formats a dark, distraction-free stage while
preserving the approved charcoal and brass language. PDF has page, zoom, and
keyboard controls around a centered canvas. Comics have single-page and
two-page modes plus an explicit LTR/RTL direction control; page images never
receive decorative cropping. DOCX deliberately reuses the reflow typography
and annotation UX because its normalized semantic structure behaves like a
text book, not a Word page preview. Fixed-layout controls collapse cleanly on
narrow screens and keep visible keyboard focus.

Stage 5 keeps metadata work inside the existing book-details drawer. The
default state remains a quiet book summary. Separate secondary actions open a
compact manual editor or provider search view. Search includes an explicit
Russian/English switch, explains which fixed providers receive the query,
presents source-labelled candidate cards, and requires an explicit Apply
action. Only in manual edit mode, the cover becomes a keyboard-accessible image
picker and exposes embedded-cover restoration with a plain local-cache policy.
Forms, candidate lists, status, and error messages remain keyboard accessible
and scroll within the drawer.

The manual editor includes a comma-separated Genres field and candidate cards
show provider genres when present. The details view displays the normalized
local values without turning them into an online catalog.

The text-selection action bar includes Translate. Activating it reveals two
plain choices, Google Translate and Yandex Translate, without leaving the
selection surface. The first provider choice expands a concise privacy notice
in place with Continue and Cancel; only Continue sends the selected text by
opening the fixed translator URL in the default browser. Selections above
2,000 characters receive a clear limit message. Settings contain no language
package manager or development-facing translator diagnostics.

Stage 7 replaces the Statistics and Achievements placeholders with real local
data. Statistics use the existing four-card rhythm, a restrained 12-week brass
activity calendar, a visible daily-goal meter, compact totals, and an explicit
delete action. Achievements use two-column warm surface cards with a muted
locked state and a brass unlocked state. Empty or zero values remain truthful;
the UI never fabricates reading activity.

The expanded achievement view keeps goals ordered from easier to harder within
each metric family, shows an unlocked/total summary, and preserves visible
progress for all locked long-term goals. Time thresholds use readable hours
and minutes rather than raw seconds.

Audiobook stage A2 adds a distinct Audiobooks destination immediately after
the text library. It keeps the same warm shell but uses compact horizontal
audio cards with a square-ended cover treatment, part and size facts,
listening-progress tracks, and a visible incomplete-source badge. Selection
opens an audiobook-specific details drawer with an ordered part list. File,
folder, watched-folder, and rescan actions stay explicit. The prepared player
destination identifies the selected title without exposing unrelated text-book
controls.

Audiobook stage A3 turns that destination into a focused three-column player:
cover and current part, primary transport controls, and the ordered queue.
The central hierarchy gives the play/pause action the strongest emphasis;
15-second seek, part navigation, timeline, speed, and volume remain plainly
labelled and keyboard accessible. At narrower widths the queue moves below the
player, then all columns stack without hiding control names. Status and decoder
errors are announced, and High Contrast keeps selected queue and play states
visible without depending on color alone.

Audiobook stage A4 extends the same player with a labelled sleep-timer select,
remaining-time status, CUE chapter list, and local bookmark composer/list. A
close-time modal appears only during active playback when the preference is
Ask; it offers continue-in-tray, exit, cancel, and an explicit remember-choice
checkbox. Settings always exposes ask/tray/exit so a remembered decision is
reversible. Tray labels use the ApriReader product name consistently.

Audiobook stage A5 adds a labelled output-device select beside speed, sleep,
and volume controls. Disabled endpoints cannot be chosen and the system-default
choice stays explicit. The details drawer gains the same quiet details/edit/
provider-search modes as text books, extended with narrator. Statistics place
audio totals in a distinct panel, while Achievements split reading and
audiobook goals under clear headings without mixing their progress semantics.

Audiobook stage A6 adds no shell-only configuration screen. Opening an
associated audio file focuses the existing ApriReader window and goes directly
to the populated audiobook player; it never pauses at an empty library or
details placeholder. Decoder or validation failures use the existing visible
and announced error surface.

Audiobook stage A7 adds a labelled Read Aloud action to the reflow-reader
toolbar. It opens the existing right-side panel pattern with an installed-voice
selector, a 0.5x–2.0x rate selector, current-section character count, and
primary play/pause plus secondary stop controls. Local-processing copy remains
visible. Over-limit sections, missing voices, and Windows synthesis failures
use the announced warning surface; closing the panel or changing sections
stops speech owned by that panel.

Audiobook stage A8 adds a two-choice Current section / Whole book from this
section scope group and a visible fragment counter. Whole-book narration keeps
the side panel open while the reader advances sections. The active word uses a
brass underline and translucent fill, switches to the Windows Highlight color
in forced-colors mode, and scrolls into view only when it leaves the readable
viewport. The focus treatment can nest inside annotation marks and bionic
spans without replacing either semantic state.

Audiobook stage A9 places a Voice provider group above the existing voice
selector. Local Windows voices stay selected by default. ElevenLabs exposes a
password-style key field only while unconfigured, a quiet Windows-protected
status with explicit Delete key action after saving, and the provider voice
list. The key is never echoed or represented as a fake masked stored value.

The first attempt to narrate with ElevenLabs opens an in-panel disclosure with
Cancel and Agree and start. It names the external provider, text transfer,
provider policy, and possible quota cost. Voice discovery alone never counts
as text-send consent. Provider errors use the announced warning surface; exact
timing reuses the A8 active-word visual and forced-colors treatment.

Audiobook stage A10 adds two collapsed groups to the read-aloud panel. Voice
presets expose a named local preset selector plus explicit Apply, Save/update,
rename, and Delete actions. The pronunciation group shows its rule count and
an enable checkbox, two bounded text inputs, and keyboard-accessible edit and
delete actions. Both groups use the existing warning/status surface and retain
visible focus and borders in forced-colors mode.

Audiobook stage A11 adds Google Cloud TTS as a third provider radio choice.
Its password field and protected-key status mirror the reviewed ElevenLabs
pattern but use independent storage and consent. Copy states that Google Cloud
billing and API enablement are required and recommends an API-restricted key.
The voice selector shows locale and family; discovery is filtered to the book
language. Google has its own first-send disclosure and cannot inherit consent
from ElevenLabs.

Audiobook stage A12 adds Azure AI Speech as a fourth provider. A required region
selector precedes the protected key field; help text states that key and region
must match. Voices are filtered by the book language and show locale/type.
Azure uses independent consent and key deletion. Voice presets also retain the
non-secret region so applying a preset restores a coherent Azure configuration.

Audiobook stage A13 adds a collapsed Voice expressiveness group after cloud
voice selection. ElevenLabs exposes stability, similarity, style, and speaker
boost; Google and Azure expose provider-valid pitch controls. Every control has
a visible numeric value, resets active narration, and is retained by presets.

A collapsed Speech cache group reports total and per-provider file/byte counts.
It offers explicit current-provider and all-provider clearing actions. Export
appears beside playback actions and opens a separate confirmation: local export
explains the M3U8/media-folder result, while cloud export warns about quota and
player-only speed. During sequential export the panel announces current/total
parts and offers cancellation after the in-flight provider request.

Stage 8 keeps Steam synchronization out of the user-facing Settings page.
Synchronization is automatic only when the protected provider is available;
build profile, queue, provider, and Overlay diagnostics remain implementation
details rather than user controls. The public build continues to provide local
achievements without Steamworks.

Steam capsule artwork reuses the approved ivory, charcoal, brass, and walnut
palette. Base capsules contain only original artwork and the ApriReader name;
the library hero contains no text.

Stage 9 preserves the same visual language while adding a keyboard skip link,
route focus management, explicit progress semantics, recovery status messages,
and Windows forced-colors support. Compact navigation must scroll instead of
hiding destinations at high Windows scaling. Library cards render in batches
of 120 with a plain “show more” action and an announced visible/total count.

Reader typography remains in the existing right-side panel. The font-family
selector keeps the system profiles and adds Literata, Lora, Merriweather,
Source Serif 4, Charis SIL, and IBM Plex Serif. Separate selectors expose
normal/italic style and only the named weights genuinely available for the
selected family. A local Cyrillic/Latin preview reflects the active family,
style, and weight. Local import, fine spacing controls, alignment, bionic
highlighting, page-wheel behavior, themes, and their short explanations form
one scrollable settings surface. Previous/next chapter arrows remain visible
in the top toolbar so chapter navigation never depends on reaching the
document footer. On reader entry, keyboard focus starts on the labelled back
control in that toolbar without scrolling the document.

The same panel begins with a two-choice reading-layout control. Continuous text
keeps the approved centered vertical column. Book spread uses two equal
fixed-height pages with a restrained central rule and no decorative page
frames. A complete spread advances per wheel gesture. Below 980 px the spread
becomes one page while keeping paginated navigation.

A compact, non-interactive page counter stays centered at the bottom of the
reading surface. It uses the muted text and paper colors, never covers controls,
and changes between singular continuous-page and plural spread-range wording.

Reading Now reuses the approved library cards and progress treatment rather
than introducing a second visual system. A quiet summary card explains the
view, and each available book has a visible Continue Reading action. Missing
sources remain visible but cannot be opened. Empty state copy must explain that
opening a library book is what adds it to this view.

Favorite controls use the existing heart icon in a 44 px circular target on
book cards and a labelled action in book details. The selected state uses the
brass accent and remains exposed through `aria-pressed`. The Favorites
destination reuses the same truthful cards, availability state, and responsive
grid as the library.

Authors uses a responsive grid of warm surface cards with restrained circular
initials, author names, and localized book counts. Selecting an author replaces
the group grid with a compact back header and the existing book-card grid.
Unknown author metadata is labelled plainly rather than receiving an invented
name or portrait.

Series reuses the same responsive card rhythm with a restrained stacked-book
mark instead of author initials. Selecting a series replaces the group grid
with a compact back header and the existing book-card grid in natural title
order. Books without series metadata remain visible in a plain No Series group;
the interface never invents volume numbers or collection artwork.

The library toolbar exposes a calm selection mode for batch removal. Selected
cards use the existing brass state, the toolbar reports the exact count, and
Select all applies to the current filtered result. Book details exposes the
same removal as a restrained danger action. Both flows explicitly confirm that
the source files stay on disk.

Settings begins with a restrained Accessibility card. Its Screen Reader Support
checkbox is enabled by default and clearly explains that voice and speed belong
to Windows. Disabling it silences ApriReader's live chapter and page
announcements without removing button names, keyboard focus, labels, or other
essential semantics. The current state is repeated in the existing compact
status badge style.

High-scaling layouts preserve reachability rather than compressing every
surface. A short vertical rail scrolls between destinations while its language
action remains outside that scroll area. Below 700 CSS pixels the rail becomes
a bottom strip with horizontally scrollable destinations and a fixed language
button. Hidden visual labels never supply the only accessible name.

Below 520 CSS pixels, the reflow reader keeps book identity on the first toolbar
row and exposes every reading action in a second horizontally scrollable row.
The page counter truncates only as a last visual fallback while retaining its
full accessible text. Settings cards reduce padding and reflow actions;
statistics, achievements, PDF, and comic controls remain
inside their own scrollable surfaces.

Windows High Contrast Black and High Contrast White map the interface to
`Canvas`, `CanvasText`, `ButtonFace`, `ButtonText`, `GrayText`, `Highlight`,
and `HighlightText`. Active controls use a visible highlight border as well as
the system highlight fill. Selected library cards and unlocked achievements
use heavier or double outlines; locked achievements and reading-calendar
levels use dashed or patterned outlines. Notes and quotes remain distinct
through solid and dashed highlight borders. Book covers and rendered PDF or
comic content keep their source colors, while their surrounding toolbars,
stages, controls, and page counters use system colors.
