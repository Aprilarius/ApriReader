# ApriReader 1.1 GitHub release decision

Decision date: 2026-08-01

Decision: **PUBLISHED**

Public release:
`https://github.com/Aprilarius/ApriReader/releases/tag/v1.1.0`

## Approved artifact

- Version: `1.1.0`
- Channel: `github-release`
- Source commit: `5c5480b34a56e090daddae22b36ae24199981cf6`
- Source tree: `clean`
- Changed source files at build time: `0`
- Installer: `ApriReader-1.1.0-windows-x64-setup.exe`
- Installer size: `15,164,324` bytes
- Installer SHA-256:
  `35D2CF935DA8500C3F185BE92D83FCDC5F36132AE62D95613D1290539E01F017`
- Source manifest SHA-256:
  `0349CCF72C8A7AF9AB4507C3697132488BEC7A9F31B0504018A72C36CEDAAD70`
- Authenticode state: `NotSigned`

## Evidence

- `pnpm github:build` passed formatting, lint, TypeScript, all 52 UI tests,
  the production build, Rust formatting, Clippy with warnings denied, all 53
  Rust tests, the release security audit, SBOM validation, and third-party
  license validation.
- `pnpm audit --prod` reported no known production dependency vulnerabilities.
- The current RustSec advisory database reported no vulnerable Rust
  dependencies; the allowed warnings are either absent from the Windows target
  tree or unmaintained transitive packages without a vulnerability advisory.
- The exact candidate was installed over ApriReader 1.0 on Windows. The library
  and reading progress were preserved, the application launched successfully,
  all supported file-handler registrations were present, and direct opening of
  an EPUB forwarded it to the existing application instance.
- The evidence archive contains the source manifest, candidate record,
  installer checksum, SBOM, third-party licenses, and release test documents.

## Accepted residual conditions

- The installer is unsigned and may show an unknown-publisher or SmartScreen
  warning. The public release states this clearly and publishes the checksum.
- Explorer can list ApriReader as a handler for all supported formats, but the
  installer intentionally does not seize the user's existing default apps.
- The Tauri bundler reported that updater bundle-type metadata was unavailable.
  ApriReader 1.1 has no automatic updater, so the warning does not affect the
  approved installer scope.

## Publication verification

- Published at `2026-08-01T12:48:38Z` as the latest non-prerelease release.
- The public release API reports all six approved assets; GitHub also provides
  its standard source-code archives.
- An anonymous download of the public installer returned `15,164,324` bytes.
- The anonymously downloaded installer SHA-256 is
  `35D2CF935DA8500C3F185BE92D83FCDC5F36132AE62D95613D1290539E01F017`,
  matching the approved local candidate.
