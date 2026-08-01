# ApriReader 1.1 GitHub release policy

Decision date: 2026-08-01

## Scope

Version 1.1.0 is the normal public GitHub profile. It adds the local profile,
bundled reader fonts, selected-text browser translation, Windows book-file
associations, reader lifecycle fixes, and bounded file/cache hardening. It
contains no Steamworks SDK, bridge, App ID, credentials, or protected binary.

## Unsigned installer decision

The product owner requested publication before a code-signing certificate is
available. The release page must identify the NSIS installer as unsigned and
publish its SHA-256 checksum. Users should download it only from the official
`Aprilarius/ApriReader` release.

## Public-release requirements

- Build from a clean commit through `pnpm github:build`.
- Record channel `github-release`, source state `clean`, and zero changed files.
- Publish the installer and source-manifest SHA-256 values.
- Attach the installer, checksum, SBOM, complete license report, and evidence
  archive.
- Verify the published release page and anonymously downloadable installer.
- Keep protected Steam files outside the repository and public artifacts.
