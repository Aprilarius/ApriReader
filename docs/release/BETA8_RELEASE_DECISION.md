# ApriReader 0.9.0-beta.8 release decision

Decision date: 2026-07-30

Recorded HEAD: `3db9c306475d013fab5d8004ed2361a8e6a62049`

Installer SHA-256:
`D01CA6BC3BEE8CB7A3FD2AAA9D2CB392A43FFEF223F38FD5EF9B3C587D29D119`

## Current decision

**PASS for the scoped product-owner forced-colors smoke. HOLD for public
promotion.**

The product owner installed and tested beta.8 and reported that it works
correctly. This closes the scoped beta.8 forced-colors check without claiming
completion of unreported Windows, protected Steamworks, signing, or participant
matrix cells.

## Confirmed

- The beta.8 installer hash matches its candidate record.
- The complete repository gate passed with 40 frontend tests and 48 Rust tests.
- Windows forced-colors handling covers the shell and every reader.
- The product owner reported a successful installed-candidate test on
  2026-07-30 with no defect identified.
- The public candidate contains no protected Steamworks file.

## Provenance limitation

The beta.8 candidate record contains the repository HEAD but predates the
source-tree-state and source-manifest evidence introduced for beta.9. The
working tree contained the staged beta changes, so the recorded HEAD alone
must not be interpreted as a complete reproducible-source identifier.

## Required before GO

- Complete and record the remaining Windows 10/11 participant matrix cells.
- Complete the protected Steamworks online/offline/Overlay matrix.
- Sign and timestamp the promoted installer.
- Review complete transitive copyright text.
- Build the final release candidate from a clean source tree and record the
  product-owner `GO` decision against both installer and source-manifest hashes.
