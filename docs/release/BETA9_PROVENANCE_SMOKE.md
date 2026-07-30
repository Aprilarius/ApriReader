# ApriReader beta.9 provenance smoke

## Automated expectations

1. `pnpm beta:build` completes the full repository gate before packaging.
2. `candidate-record.json` contains `sourceCommit`, `sourceTreeState`,
   `sourceChangedFileCount`, `sourceManifest`, and `sourceManifestSha256`.
3. `SOURCE_SHA256SUMS.txt` lists every Git-tracked or non-ignored untracked
   source file in deterministic path order.
4. The SHA-256 of `SOURCE_SHA256SUMS.txt` matches
   `sourceManifestSha256`.
5. The build stops if the source snapshot differs after compilation.
6. `scripts/build_beta_candidate.ps1 -RequireCleanTree` rejects a modified
   working tree before running the expensive build.
7. The evidence archive contains the security, closed-beta, accessibility,
   Steam, and manual-test records used for the candidate decision.

## Promotion boundary

A `sourceTreeState` of `modified` is truthful evidence for an internal
closed-beta candidate, not approval for public promotion. The signed release
candidate must be created from a reviewed clean commit with
`-RequireCleanTree`, then receive a product-owner decision tied to both the
installer SHA-256 and source-manifest SHA-256.
