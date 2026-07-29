# ApriReader 0.9.0-beta.5 release decision

Decision date: 2026-07-29  
Candidate commit: `78ce0b93379ec1d4c55442a3d2afaf4334bf612a`  
Installer SHA-256:
`8A0F212E9618D59C2DBB8605DAE8C637DB63CE29E6184339EBFE032F2DC574C1`

## Current decision

**HOLD for public promotion. Continue external release validation.**

The standard current-user installation was verified as ApriReader
`0.9.0-beta.5`. The product owner installed and manually tested this candidate
and reported no critical defects.

This is a successful product-owner smoke pass. It does not claim completion of
unreported Windows 10, Narrator, high-scaling, forced-colors, code-signing, or
protected Steamworks matrix cells.

## Confirmed

- The installed executable reports FileVersion and ProductVersion
  `0.9.0-beta.5`.
- The candidate was built from the recorded commit and its installer hash
  matches the candidate record.
- The full repository gate passed with 34 frontend tests and 48 Rust tests.
- Initial keyboard focus is covered for reflow and fixed-layout readers.
- The product owner reported no critical defects after installing and testing
  beta.5.
- The public candidate contains no protected Steamworks file.

## Required before GO

- Complete and record the Windows 10 x64 participant matrix.
- Complete Windows Narrator, 150%, 200%, and 250% scaling, and forced-colors
  cells on the exact release candidate.
- Complete the protected Steamworks online/offline/Overlay matrix.
- Sign and timestamp the public installer, or retain it as an explicitly
  unsigned closed-beta artifact.
- Review complete transitive copyright text before public distribution.
- Record the final product-owner `GO` decision against the exact promoted
  installer hash.

No critical application defect is currently reported. Stage 9 remains open
only because the external release gates above do not yet have evidence.
