# Stage 4 format corpus

The automated corpus is generated in memory by the Rust tests so the repository
does not ship third-party books or opaque binary fixtures:

- `reader::tests::docx_becomes_safe_semantic_sections` builds a DOCX package
  containing headings, paragraphs, lists, quotes, and Unicode text.
- `special_reader::tests::cbz_pages_use_natural_order_and_safe_cache_names`
  builds a CBZ with `page1`, `page2`, and `page10`.
- `special_reader::tests::cbr_reads_rar3_family_without_external_tools` builds
  a stored RAR 2.9/3-family CBR.
- `special_reader::tests::cbr_reads_rar5_family_without_external_tools` builds
  a stored RAR 5-family CBR.
- Archive-traversal and non-image tests cover hostile entry names and rejected
  content.

Real multi-page PDF and large CBR samples are exercised by the focused Windows
manual test because they cannot be redistributed safely in this repository.
See `docs/testing/MANUAL_TESTS.md`.
