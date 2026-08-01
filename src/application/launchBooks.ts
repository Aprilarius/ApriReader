import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { Book } from "./library";

export async function takeLaunchBookPaths(): Promise<string[]> {
  if (!isTauri()) return [];
  return invoke<string[]>("take_launch_book_paths");
}

export async function openBookPath(path: string): Promise<Book> {
  return invoke<Book>("open_book_path", { path });
}

export async function listenForLaunchBooks(
  handler: () => void,
): Promise<UnlistenFn> {
  if (!isTauri()) return () => undefined;
  return listen("open-book-paths", handler);
}
