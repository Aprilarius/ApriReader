import { convertFileSrc, invoke, isTauri } from "@tauri-apps/api/core";

export type ComicPage = {
  index: number;
  name: string;
  path: string;
  mime: string;
};

export type SpecialDocument = {
  bookId: number;
  title: string;
  author: string;
  format: string;
  kind: "pdf" | "comic";
  sourcePath: string | null;
  pages: ComicPage[];
  progress: number;
  lastPage: number;
};

export async function loadSpecialDocument(
  bookId: number,
): Promise<SpecialDocument> {
  if (!isTauri()) {
    throw new Error("The fixed-layout reader is available in the desktop app.");
  }
  return invoke<SpecialDocument>("load_special_document", { bookId });
}

export const localAssetUrl = (path: string) =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window
    ? convertFileSrc(path)
    : path;
