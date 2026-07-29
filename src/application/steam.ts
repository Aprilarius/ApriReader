import { invoke, isTauri } from "@tauri-apps/api/core";

export type SteamIntegrationStatus = {
  buildProfile: "github" | "steam";
  bridgeInstalled: boolean;
  providerAvailable: boolean;
  overlayEnabled: boolean | null;
  pendingUnlocks: number;
  syncedUnlocks: number;
  lastAttemptAt: number | null;
  lastError: string | null;
};

export type SteamSyncResult = {
  attempted: number;
  synchronized: number;
  pending: number;
  overlayEnabled: boolean | null;
};

export async function getSteamIntegrationStatus(): Promise<SteamIntegrationStatus> {
  if (!isTauri()) return emptySteamStatus();
  return invoke<SteamIntegrationStatus>("get_steam_integration_status");
}

export async function syncSteamAchievements(): Promise<SteamSyncResult> {
  if (!isTauri()) {
    return {
      attempted: 0,
      synchronized: 0,
      pending: 0,
      overlayEnabled: null,
    };
  }
  return invoke<SteamSyncResult>("sync_steam_achievements");
}

export async function syncSteamIfAvailable(): Promise<void> {
  const status = await getSteamIntegrationStatus();
  if (status.providerAvailable && status.pendingUnlocks > 0) {
    await syncSteamAchievements();
  }
}

export function emptySteamStatus(): SteamIntegrationStatus {
  return {
    buildProfile: "github",
    bridgeInstalled: false,
    providerAvailable: false,
    overlayEnabled: null,
    pendingUnlocks: 0,
    syncedUnlocks: 0,
    lastAttemptAt: null,
    lastError: null,
  };
}
