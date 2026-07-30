# ApriReader beta.8 forced-colors smoke

## Scope

This gate covers the beta.8 Windows forced-colors implementation across the
application shell, library, settings, statistics, achievements, reflow reader,
PDF reader, and comic reader.

## Automated evidence

- Reader choice controls expose their selected state through `aria-pressed`.
- Reflow layout, alignment, and theme behavior remains covered by the reader
  component suite.
- Comic single-page and two-page choices remain covered by the fixed-reader
  component suite.
- The production CSS build accepts the complete `forced-colors: active` layer.
- The normal format, lint, typecheck, test, production-build, Rust, dependency,
  and SBOM gates must pass before packaging.

## Visual contract

- Shell and control surfaces use Windows system colors.
- Active and selected states use both a system highlight and a visible border.
- Locked, unlocked, progress, calendar, warning, and annotation states do not
  depend on the normal application palette alone.
- Disabled controls remain readable.
- Book covers, PDF pages, and comic artwork retain their source colors.

## Installed-candidate check

The product owner should test the packaged beta.8 candidate under both Windows
High Contrast Black and High Contrast White:

1. Open every main destination and Settings.
2. Confirm selected books, filters, favorites, progress, calendar levels, and
   locked or unlocked achievements remain distinguishable.
3. Open a reflow book and exercise layout, alignment, theme, search,
   annotations, and page navigation.
4. Open a PDF and a comic; confirm controls follow system colors while page
   content remains unchanged.
5. Confirm keyboard focus and disabled controls remain visible.

This operating-system theme matrix is external evidence. It is not marked as
passed by source tests or a production build alone.
