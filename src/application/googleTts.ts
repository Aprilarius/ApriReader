import { invoke } from "@tauri-apps/api/core";

export interface GoogleTtsStatus {
  configured: boolean;
}

export interface GoogleTtsVoice {
  id: string;
  name: string;
  language: string;
  category: string;
  gender: "female" | "male" | "unknown";
}

export interface PreparedGoogleTtsAudio {
  path: string;
  voiceId: string;
  characterCount: number;
}

export interface GoogleTtsSettings {
  pitch: number;
}

export function getGoogleTtsStatus(): Promise<GoogleTtsStatus> {
  return invoke<GoogleTtsStatus>("google_tts_status");
}

export function saveGoogleTtsKey(apiKey: string): Promise<GoogleTtsStatus> {
  return invoke<GoogleTtsStatus>("google_tts_save_key", { apiKey });
}

export function deleteGoogleTtsKey(): Promise<GoogleTtsStatus> {
  return invoke<GoogleTtsStatus>("google_tts_delete_key");
}

export function listGoogleTtsVoices(
  languageCode?: string,
): Promise<GoogleTtsVoice[]> {
  return invoke<GoogleTtsVoice[]>("google_tts_list_voices", {
    languageCode: languageCode?.trim() || null,
  });
}

export function prepareGoogleTtsSection(
  text: string,
  voiceId: string,
  languageCode: string,
  settings: GoogleTtsSettings,
): Promise<PreparedGoogleTtsAudio> {
  return invoke<PreparedGoogleTtsAudio>("google_tts_prepare_section", {
    text,
    voiceId,
    languageCode,
    settings,
  });
}
