import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { readLocalValue, writeLocalValue } from "../ui/localStorage";

export type AudioCloseBehavior = "ask" | "tray" | "exit";
export type AudioCloseDecision = Exclude<AudioCloseBehavior, "ask">;

const audioCloseBehaviorKey = "aprireader.audio.closeBehavior";

export function readAudioCloseBehavior(): AudioCloseBehavior {
  const value = readLocalValue(audioCloseBehaviorKey);
  return value === "tray" || value === "exit" ? value : "ask";
}

export async function setAudioCloseBehavior(
  behavior: AudioCloseBehavior,
): Promise<void> {
  writeLocalValue(audioCloseBehaviorKey, behavior);
  await invoke("set_audio_close_behavior", { behavior });
}

export function syncAudioCloseBehavior(
  behavior: AudioCloseBehavior,
): Promise<void> {
  return invoke("set_audio_close_behavior", { behavior });
}

export function resolveAudioClose(
  decision: AudioCloseDecision,
  remember: boolean,
): Promise<void> {
  if (remember) writeLocalValue(audioCloseBehaviorKey, decision);
  return invoke("resolve_audio_close", { decision, remember });
}

export function listenForAudioCloseRequest(
  handler: () => void,
): Promise<UnlistenFn> {
  return listen("audio-close-requested", handler);
}
