import { invoke } from "@tauri-apps/api/core";

export interface CloudTtsStatus {
  configured: boolean;
}

export interface CloudTtsVoice {
  id: string;
  name: string;
  language: string;
  category: string;
}

export interface CloudTtsTiming {
  startOffset: number;
  endOffset: number;
  startSeconds: number;
  endSeconds: number;
}

export interface PreparedCloudTtsAudio {
  path: string;
  voiceId: string;
  characterCount: number;
  timings: CloudTtsTiming[];
}

export interface CloudTtsSettings {
  stability: number;
  similarityBoost: number;
  style: number;
  speakerBoost: boolean;
}

export function getCloudTtsStatus(): Promise<CloudTtsStatus> {
  return invoke<CloudTtsStatus>("cloud_tts_status");
}

export function saveCloudTtsKey(apiKey: string): Promise<CloudTtsStatus> {
  return invoke<CloudTtsStatus>("cloud_tts_save_key", { apiKey });
}

export function deleteCloudTtsKey(): Promise<CloudTtsStatus> {
  return invoke<CloudTtsStatus>("cloud_tts_delete_key");
}

export function listCloudTtsVoices(): Promise<CloudTtsVoice[]> {
  return invoke<CloudTtsVoice[]>("cloud_tts_list_voices");
}

export function prepareCloudTtsSection(
  text: string,
  voiceId: string,
  settings: CloudTtsSettings,
): Promise<PreparedCloudTtsAudio> {
  return invoke<PreparedCloudTtsAudio>("cloud_tts_prepare_section", {
    text,
    voiceId,
    settings,
  });
}
