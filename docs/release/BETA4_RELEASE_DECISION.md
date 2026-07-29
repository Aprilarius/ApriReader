# ApriReader 0.9.0-beta.4 release decision

Decision date: 2026-07-29  
Candidate commit: `75c29e468b9e0feac0a2829348f4feebbbd6b3d1`  
Installer SHA-256:
`2DF3AA1A9B5EF50F6BBCF75749AB1D614C78DB46F07EE5BA86B78DACAD978ABA`

## Current decision

**HOLD for public promotion. Continue closed-beta validation.**

The product owner reported that the installed application works correctly in
their manual test. This records a successful product-owner smoke pass for the
exact beta.4 candidate, not completion of every required matrix cell.

## Confirmed

- The public Windows 11 x64 candidate builds from the recorded commit.
- The installer metadata reports ApriReader `0.9.0-beta.4`.
- The installer hash matches the candidate record.
- The repository gate passed formatting, lint, typecheck, frontend tests,
  production build, Rust formatting, Clippy, Rust tests, security audit, and
  SBOM verification.
- The disposable valid/malformed matrix passed for all nine public formats,
  with source fixtures unchanged.
- Product-owner manual smoke testing was reported successful on 2026-07-29.
- No protected Steamworks file is included in the public candidate.

## Required before GO

- Complete and record the Windows 10 x64 participant matrix.
- Complete the remaining Windows 11 accessibility and scaling cells if they
  were not part of the reported smoke test.
- Complete the protected Steamworks build and online/offline/Overlay matrix in
  the authorized release environment.
- Add code signing and timestamping, or explicitly accept unsigned
  closed-beta-only distribution.
- Review complete transitive copyright text before public binary distribution.
- Record defects and a final product-owner `GO` decision against this exact
  installer hash, or build and identify a replacement candidate.

Stage 9 remains open until these external gates have evidence. No missing
application feature is currently identified.
