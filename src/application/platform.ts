import { invoke, isTauri } from "@tauri-apps/api/core";

export type PlatformCapabilities = {
  platform: string;
  desktop: boolean;
  filesystemPaths: boolean;
  watchedFolders: boolean;
  launchFileArguments: boolean;
  systemTray: boolean;
  steamIntegration: boolean;
  nativeAudiobookPlayback: boolean;
  backgroundAudio: boolean;
  audioOutputSelection: boolean;
  localTextToSpeech: boolean;
  protectedCloudCredentials: boolean;
};

export const conservativePlatformCapabilities: PlatformCapabilities = {
  platform: "unknown",
  desktop: false,
  filesystemPaths: false,
  watchedFolders: false,
  launchFileArguments: false,
  systemTray: false,
  steamIntegration: false,
  nativeAudiobookPlayback: false,
  backgroundAudio: false,
  audioOutputSelection: false,
  localTextToSpeech: false,
  protectedCloudCredentials: false,
};

export async function getPlatformCapabilities(): Promise<PlatformCapabilities> {
  if (!isTauri()) return conservativePlatformCapabilities;
  return invoke<PlatformCapabilities>("get_platform_capabilities");
}
