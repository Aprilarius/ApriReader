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
compact manual editor or an Open Library search view. Search explains before
submission that it will send a query, presents source-labelled candidate cards,
and requires an explicit Apply action. External-cover removal includes a plain
local-cache policy. Forms, candidate lists, status, and error messages remain
keyboard accessible and scroll within the drawer.

The manual editor includes a comma-separated Genres field and candidate cards
show provider genres when present. The details view displays the normalized
local values without turning them into an online catalog.

Stage 6 extends the existing text-selection action bar with Dictionary and
Translate. Results remain inside the temporary selection surface and never
replace the reading page. A verified-package manager lives in Settings and
uses the established warm surface cards, plain empty states, visible license
and attribution, and an explicit removal confirmation. It must never resemble
an online model catalog or imply that a package is bundled.

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

Stage 8 adds a restrained Steam integration card to Settings. It always names
the active GitHub or Steam build profile, pending and synchronized unlock
counts, provider availability, and Overlay state. The public build explains
that local achievements remain available without Steamworks. Synchronization
is automatic only when the protected provider is available and is also exposed
as a clear manual retry action.

Steam capsule artwork reuses the approved ivory, charcoal, brass, and walnut
palette. Base capsules contain only original artwork and the ApriReader name;
the library hero contains no text.

Stage 9 preserves the same visual language while adding a keyboard skip link,
route focus management, explicit progress semantics, recovery status messages,
and Windows forced-colors support. Compact navigation must scroll instead of
hiding destinations at high Windows scaling. Library cards render in batches
of 120 with a plain “show more” action and an announced visible/total count.

Reader typography remains in the existing right-side panel. Font profiles,
local import, fine spacing and weight controls, alignment, bionic highlighting,
page-wheel behavior, themes, and their short explanations form one scrollable
settings surface. Previous/next chapter arrows remain visible in the top
toolbar so chapter navigation never depends on reaching the document footer.

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
