import { invoke, isTauri } from "@tauri-apps/api/core";

export const maxTtsCharacters = 20_000;

export interface TtsVoice {
  id: string;
  name: string;
  language: string;
  gender: "female" | "male" | "unknown";
  isDefault: boolean;
}

export interface PreparedTtsAudio {
  path: string;
  voiceId: string;
  characterCount: number;
}

export function listTtsVoices(): Promise<TtsVoice[]> {
  if (!isTauri()) return Promise.resolve([]);
  return invoke<TtsVoice[]>("tts_list_voices");
}

export function prepareTtsSection(
  text: string,
  voiceId: string,
  rate: number,
): Promise<PreparedTtsAudio> {
  return invoke<PreparedTtsAudio>("tts_prepare_section", {
    text,
    voiceId,
    rate,
  });
}
