import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AudiobookRecord } from "./audiobooks";
import type { Book } from "./library";

export type OpenedLaunchFile =
  | { kind: "book"; item: Book }
  | { kind: "audiobook"; item: AudiobookRecord };

export async function takeLaunchPaths(): Promise<string[]> {
  if (!isTauri()) return [];
  return invoke<string[]>("take_launch_paths");
}

export async function openLaunchPath(path: string): Promise<OpenedLaunchFile> {
  return invoke<OpenedLaunchFile>("open_launch_path", { path });
}

export async function listenForLaunchFiles(
  handler: () => void,
): Promise<UnlistenFn> {
  if (!isTauri()) return () => undefined;
  return listen("open-file-paths", handler);
}
