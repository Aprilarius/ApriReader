import { invoke } from "@tauri-apps/api/core";

export type StartupHealth = {
  previousExitUnclean: boolean;
  recoveredFromBackup: boolean;
  quarantinedDatabase: string | null;
};

export async function getStartupHealth(): Promise<StartupHealth> {
  return invoke<StartupHealth>("get_startup_health");
}
