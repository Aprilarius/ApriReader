import { invoke, isTauri } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import type { TtsProvider } from "./ttsPreferences";

export interface TtsCacheProviderSummary {
  provider: TtsProvider;
  files: number;
  bytes: number;
}

export interface TtsCacheSummary {
  totalFiles: number;
  totalBytes: number;
  providers: TtsCacheProviderSummary[];
}

export interface TtsExportStarted {
  sessionId: string;
  expectedParts: number;
}

export interface TtsExportResult {
  playlistPath: string;
  mediaDirectory: string;
  parts: number;
  bytes: number;
}

export interface TtsExportPart {
  sourcePath: string;
  title: string;
}

export const maxTtsExportParts = 5_000;

export function getTtsCacheSummary(): Promise<TtsCacheSummary> {
  if (!isTauri())
    return Promise.resolve({ totalFiles: 0, totalBytes: 0, providers: [] });
  return invoke<TtsCacheSummary>("tts_cache_summary");
}

export function clearTtsCache(
  provider?: TtsProvider,
): Promise<TtsCacheSummary> {
  return invoke<TtsCacheSummary>("tts_clear_cache", {
    provider: provider ?? null,
  });
}

export async function chooseTtsExportPath(
  title: string,
): Promise<string | null> {
  if (!isTauri()) return null;
  return save({
    defaultPath: `${safeFileName(title)}-narration.m3u8`,
    filters: [{ name: "M3U8 playlist", extensions: ["m3u8"] }],
  });
}

export function beginTtsExport(
  playlistPath: string,
  expectedParts: number,
): Promise<TtsExportStarted> {
  return invoke<TtsExportStarted>("tts_begin_export", {
    playlistPath,
    expectedParts,
  });
}

export function appendTtsExportPart(
  sessionId: string,
  part: TtsExportPart,
): Promise<number> {
  return invoke<number>("tts_append_export_part", { sessionId, part });
}

export function finishTtsExport(sessionId: string): Promise<TtsExportResult> {
  return invoke<TtsExportResult>("tts_finish_export", { sessionId });
}

export function cancelTtsExport(sessionId: string): Promise<void> {
  return invoke<void>("tts_cancel_export", { sessionId });
}

function safeFileName(value: string) {
  return (
    value
      .replace(/[<>:"/\\|?*]/g, "")
      .split("")
      .filter((character) => character.charCodeAt(0) > 31)
      .join("")
      .trim()
      .slice(0, 80) || "book"
  );
}
