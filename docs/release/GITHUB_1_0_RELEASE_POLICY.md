# ApriReader 1.0 GitHub release policy

Decision date: 2026-07-30

## Scope

The first public release is the normal GitHub profile. It contains local
achievements but no Steamworks SDK, bridge, App ID, credentials, or protected
binary. Steam distribution remains a separate future gate.

## Unsigned installer decision

The product owner explicitly chose to publish the GitHub version before
obtaining a code-signing certificate. Version 1.0 therefore ships as an
unsigned NSIS installer and must be described as such on the release page.

Users should download only from the official `Aprilarius/ApriReader` release
and compare its SHA-256 value with the published checksum. Code signing remains
a planned trust improvement and is not represented as complete.

## Public-release requirements

- Build from a clean tagged commit through `pnpm github:build`.
- Record channel `github-release`, source state `clean`, and zero changed files.
- Publish installer SHA-256 and source-manifest SHA-256.
- Attach the installer, checksum, SBOM, full third-party license report, and
  evidence archive.
- Verify the public release page and downloadable installer after upload.
- Keep protected Steam files outside the repository and public artifacts.
