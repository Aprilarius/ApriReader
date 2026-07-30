# ApriReader 1.0.0-rc.1 test checklist

## Automated release identity

- Run only `pnpm rc:build`.
- Confirm the build rejects a modified Git tree before compilation.
- Confirm `candidate-record.json` reports:
  - version `1.0.0-rc.1`;
  - channel `release-candidate`;
  - source tree state `clean`;
  - changed-file count `0`;
  - the exact HEAD commit;
  - matching installer and source-manifest SHA-256 values.
- Confirm the evidence archive contains the SBOM, source manifest, security
  review, closed-beta matrix, Steam checklist, and manual tests.

## Product-owner smoke

1. Install over the most recent beta and confirm the existing local library,
   progress, favorites, annotations, imported fonts, statistics, and settings
   remain intact.
2. Start ApriReader normally and confirm no console window appears.
3. Open one reflow book, one PDF, and one comic.
4. Confirm wheel paging, continuous and spread layouts, page counters,
   typography, bionic highlighting, and context-menu suppression.
5. Confirm single and batch library removal leave source books on disk.
6. Confirm Russian and English UI, keyboard focus, Narrator announcements,
   high scaling, and forced-colors behavior.
7. Restart the application and confirm reading position and settings persist.

## Promotion boundary

This RC1 remains unsigned until a release certificate and timestamp service are
available. Public 1.0 promotion additionally requires the remaining Windows
participant matrix, protected Steamworks evidence, transitive copyright review,
and a dated product-owner GO decision tied to the exact installer and source
manifest hashes.
