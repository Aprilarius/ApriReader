# Steam release-candidate checklist

1. Run `pnpm check`.
2. Build the public candidate and record its SHA-256.
3. Build the protected profile with `pnpm tauri:steam` only in the authorized
   release environment.
4. Repeat `docs/steam/TEST_CHECKLIST.md` with the release App ID and protected
   bridge.
5. Confirm all 42 canonical achievements exist in Steam App Admin before
   testing synchronization; a missing remote ID blocks promotion.
6. Confirm the public artifact contains no bridge, Steamworks DLL, App ID,
   publisher credential, header, or SDK library.
7. Confirm the protected artifact starts offline, preserves queued unlocks,
   retries after reconnection, and reports Overlay truthfully.
8. Verify RU/EN, keyboard, Narrator, high scaling, forced colors, recovery,
   malformed fixtures, and a 1,000-book library against the exact candidate.
9. Archive the SBOM, notices, test record, candidate hash, and product-owner
   go/no-go decision together.

A successful local protected-profile build is not a Steam release candidate
until the Steamworks and closed-beta evidence above exists.
