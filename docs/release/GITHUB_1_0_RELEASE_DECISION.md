# ApriReader 1.0 GitHub release decision

Decision date: 2026-07-30

Decision: **PUBLISHED**

Public release:
`https://github.com/Aprilarius/ApriReader/releases/tag/v1.0.0`

## Approved artifact

- Version: `1.0.0`
- Channel: `github-release`
- Source commit: `3a0ff18db8dee43c3251df616dfbda4488c01ba2`
- Source tree: `clean`
- Changed source files at build time: `0`
- Installer: `ApriReader-1.0.0-windows-x64-setup.exe`
- Installer SHA-256:
  `8CBBA870FA2B53CA4FD01234BBD98B07CB28DDBA85EC080BB4BB6DFAFC0035F0`
- Source manifest SHA-256:
  `2736326CDC5D25E067699F948107B1F617ADC885B0014464710AA12B41DA1C0C`
- Authenticode state: `NotSigned`

## Evidence

- `pnpm github:build` passed the formatting, lint, TypeScript, UI test,
  production build, Rust formatting, Clippy, Rust test, release security, SBOM,
  and third-party-license gates.
- All 40 UI tests and 48 Rust tests passed.
- The product owner installed and tested the functionally equivalent RC1 build
  on Windows 10 and reported no bugs on 2026-07-30.
- The 1.0 changes after RC1 are limited to version and public-release metadata,
  documentation, provenance, and license-report generation.
- The evidence archive contains the exact source manifest, candidate record,
  installer checksum, SBOM, third-party licenses, and release test documents.

## Accepted residual conditions

- The installer is intentionally published before code signing and may show an
  unknown-publisher or SmartScreen warning. The release page must state this
  clearly and publish the checksum.
- The GitHub build contains no protected Steamworks files. Steam publication
  remains a separate future release profile and verification gate.
- The Tauri bundler reported that updater bundle-type metadata was unavailable.
  ApriReader 1.0 does not include or advertise an automatic updater, so this
  warning does not affect the approved installer scope.

## Publication verification

- Published at `2026-07-30T14:15:49Z` as the latest non-prerelease release.
- The public release exposes all six approved assets plus GitHub's two source
  archives.
- Every uploaded asset digest matched its local SHA-256 value before
  publication.
- An anonymous download of the public installer returned HTTP `200` and
  `9,508,323` bytes, matching the approved local artifact size.
