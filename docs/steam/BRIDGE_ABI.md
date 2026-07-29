# Protected Steam bridge ABI

The public ApriReader repository never contains the Steamworks SDK, App ID,
publisher credentials, `steam_api64.dll`, or the closed bridge binary.

The protected Steam build places `aprireader_steam_bridge.dll` beside
`aprireader.exe`. ApriReader loads only that exact absolute path with restricted
DLL search flags. The bridge must export this C ABI:

```c
int aprireader_steam_init(void);
int aprireader_steam_unlock(const char *canonical_achievement_id);
int aprireader_steam_store(void);
int aprireader_steam_overlay_enabled(void);
void aprireader_steam_shutdown(void);
```

- `init` initializes Steamworks, requests current user stats, waits for the
  result callback, and returns `1` only when achievements are ready.
- `unlock` calls `ISteamUserStats::SetAchievement` for the exact UTF-8 ID and
  returns `1` on acceptance.
- `store` calls `ISteamUserStats::StoreStats`, pumps callbacks with a bounded
  timeout, and returns `1` only after a successful `UserStatsStored_t`.
- `overlay_enabled` returns `1` when the Steam Overlay is active.
- `shutdown` releases callbacks and calls `SteamAPI_Shutdown`.

The bridge validates every incoming ID against the Steam App Admin definitions.
ApriReader independently filters the queue against its canonical registry.
No App ID crosses this ABI or enters SQLite.

Build the open shell with:

```powershell
pnpm tauri:steam
```

Then copy the bridge and the Valve runtime into the protected output directory.
Launch the application through Steam; do not ship `steam_appid.txt`.
