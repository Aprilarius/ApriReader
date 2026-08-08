import { invoke } from "@tauri-apps/api/core";

export interface AzureTtsStatus {
  configured: boolean;
}

export interface AzureTtsRegion {
  id: string;
  name: string;
}

export interface AzureTtsVoice {
  id: string;
  name: string;
  language: string;
  category: string;
  gender: "female" | "male" | "unknown";
}

export interface PreparedAzureTtsAudio {
  path: string;
  voiceId: string;
  characterCount: number;
}

export interface AzureTtsSettings {
  pitchPercent: number;
}

export const getAzureTtsStatus = () =>
  invoke<AzureTtsStatus>("azure_tts_status");
export const listAzureTtsRegions = () =>
  invoke<AzureTtsRegion[]>("azure_tts_regions");
export const saveAzureTtsKey = (apiKey: string) =>
  invoke<AzureTtsStatus>("azure_tts_save_key", { apiKey });
export const deleteAzureTtsKey = () =>
  invoke<AzureTtsStatus>("azure_tts_delete_key");
export const listAzureTtsVoices = (region: string, language?: string) =>
  invoke<AzureTtsVoice[]>("azure_tts_list_voices", {
    region,
    language: language?.trim() || null,
  });
export const prepareAzureTtsSection = (
  text: string,
  voiceId: string,
  language: string,
  region: string,
  settings: AzureTtsSettings,
) =>
  invoke<PreparedAzureTtsAudio>("azure_tts_prepare_section", {
    text,
    voiceId,
    language,
    region,
    settings,
  });
