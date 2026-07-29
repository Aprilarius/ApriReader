# Closed beta checklist

Do not mark Stage 9 complete until this checklist has real participant and
build evidence.

## Candidate

- Run `pnpm beta:build` and retain its generated candidate record and evidence
  archive.
- Record version, commit, build date, Windows version, and SHA-256.
- Confirm `pnpm check`, normal production build, and protected Steam build pass.
- Attach the current `release/aprireader-sbom.cdx.json`.
- Confirm no protected Steam file is present in the public candidate.

## Participants

- Use volunteers who explicitly accept a pre-release local application.
- Ask participants not to share copyrighted books or personal library paths.
- Do not collect telemetry. Diagnostics are created and sent only after the
  participant explicitly chooses to do so.

## Required matrix

- Windows 10 x64 and Windows 11 x64.
- Russian and English.
- Keyboard-only, Windows Narrator, 100%, 150%, 200%, and 250% scaling, and
  Windows forced-colors/high-contrast mode.
- Empty, 100-book, 1,000-book, unavailable-source, and watched-folder libraries.
- One valid and one intentionally malformed synthetic fixture for each public
  format.
- Forced termination during reading followed by restart and progress check.
- A copied test database corruption followed by backup recovery; never corrupt
  a participant's only library.

## Exit criteria

- No open data-loss, code-execution, implicit-network, inaccessible-navigation,
  or startup blocker.
- No unresolved severity-high issue.
- All lower-severity issues have an owner and release decision.
- Product owner signs a dated go/no-go record linked to the exact candidate
  hash.
