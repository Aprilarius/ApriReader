# Steam integration test checklist

These checks require the protected Steamworks environment and cannot run in
public CI.

1. Confirm the public `pnpm check` and normal production build contain no
   Steamworks files, App ID, or bridge.
2. Build with `pnpm tauri:steam`, install the protected bridge beside the EXE,
   and launch through the Steam client.
3. Confirm Settings reports the Steam profile, available provider, and Overlay
   state.
4. Unlock one local achievement while online. Confirm the queue becomes empty,
   `SetAchievement` uses the canonical ID, `StoreStats` succeeds, and the
   Overlay notification appears once.
5. Unlock two achievements in Steam Offline Mode. Restart while still offline
   and confirm both remain pending with bounded failure information.
6. Return online and launch through Steam. Confirm both pending IDs synchronize
   once and remain locally unlocked.
7. Kill the app during a failed store, restart, and confirm the pending queue
   survives without duplicate Steam unlocks.
8. Remove or rename the bridge and confirm ApriReader falls back to local
   achievements without failing startup.
9. Confirm all 42 canonical IDs and their RU/EN names and descriptions in Steam
   App Admin match `ACHIEVEMENTS.json`. Specifically verify the new completion,
   author, genre, series, time, volume, streak, and annotation ladders.
10. Confirm the Overlay, online, offline, and retry results in the protected
    release log before promoting the build.
