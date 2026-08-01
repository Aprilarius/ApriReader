# ApriReader 1.1.0

ApriReader 1.1 makes the Windows reader more personal, flexible, and resilient
while keeping books and reading data local by default.

## Highlights

- Optional local display name with a time-aware library greeting.
- Six bundled OFL serif families with independent style and real-weight
  selection: Literata, Lora, Merriweather, Source Serif 4, Charis SIL, and IBM
  Plex Serif.
- Explicit EN-RU and RU-EN selected-text handoff to Google or Yandex Translate
  in the default browser, with first-use privacy consent.
- Explorer associations for every supported format, including safe import,
  duplicate reuse, immediate opening, and forwarding to an existing instance.
- Reader reliability fixes for rapid book switches, position flushing, chapter
  navigation, reading-session attribution, and relocated duplicate sources.
- Stronger bounds around books, fonts, metadata responses, watched folders,
  PDF/comic caches, and interrupted cache generation.
- Cleaner Settings and application shell without development or provider
  diagnostics.

## Privacy and installation

The public build contains no telemetry, Steamworks files, translation model,
language package, account service, or background catalog. Translation occurs
only after the user selects text and chooses an external provider.

The Windows installer is currently unsigned and may show an unknown-publisher
warning. Download it only from the official release and verify the published
SHA-256 checksum.
