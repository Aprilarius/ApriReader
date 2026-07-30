# ApriReader 1.0.0-rc.1 release decision

Decision date: 2026-07-30

Candidate commit: `4846e77d2de66ab02f46fee8afc8b452da66f25f`

Installer SHA-256:
`B638FFED3A171D13FF7359A2E548B919454ED1A144EB51E004127EFD7B3E51C5`

Source manifest SHA-256:
`45D600E5F62CA93B9DA493099CC41094C00CF5093C58FD65211891281382B5C3`

## Current decision

**PASS for the product-owner installed smoke. HOLD for public 1.0 promotion
until the remaining external release gates have evidence.**

The product owner installed and tested the exact RC1 candidate and reported no
bugs on 2026-07-30. No replacement build is required for a known application
defect.

## Confirmed

- The candidate record reports version `1.0.0-rc.1` and channel
  `release-candidate`.
- RC1 was built from the clean commit above with zero changed source files.
- The installer and 163-file source manifest independently match the recorded
  SHA-256 values.
- The repository gate passed 40 frontend tests, 48 Rust tests, formatting,
  lint, typecheck, production build, Clippy, security and license audit, and
  SBOM verification.
- The product owner reported that the installed candidate has no observed
  bugs.
- The installer is an unsigned public-profile artifact and contains no
  protected Steamworks file.

## Required before public GO

- Sign and timestamp the promoted Windows installer, or explicitly define a
  separate unsigned distribution policy.
- Complete and archive the remaining Windows 10/11 participant matrix cells,
  including RU/EN, Narrator, scaling, forced colors, recovery, malformed
  fixtures, and the 1,000-book library.
- Complete the protected Steamworks online, offline, retry, achievement, and
  Overlay matrix in the authorized environment.
- Complete the transitive copyright and notice review for binary distribution.
- Record an explicit dated public `GO` decision tied to the final installer and
  source-manifest hashes.

Stage 9 remains open only for these external release gates. No unresolved
application defect is currently recorded.
