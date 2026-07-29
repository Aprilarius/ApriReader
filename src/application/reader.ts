import { invoke, isTauri } from "@tauri-apps/api/core";

export type BlockKind =
  | "heading"
  | "paragraph"
  | "quote"
  | "listItem"
  | "code"
  | "divider";

export type DocumentBlock = {
  kind: BlockKind;
  text: string;
};

export type DocumentSection = {
  id: string;
  title: string;
  blocks: DocumentBlock[];
};

export type DocumentModel = {
  bookId: number;
  title: string;
  author: string;
  format: string;
  sections: DocumentSection[];
  progress: number;
  lastSection: number;
  sectionProgress: number;
};

export async function loadDocument(bookId: number): Promise<DocumentModel> {
  if (!isTauri()) {
    throw new Error("The reader is available in the desktop application.");
  }
  return invoke<DocumentModel>("load_document", { bookId });
}

export async function saveReadingPosition(
  bookId: number,
  section: number,
  sectionProgress: number,
  progress: number,
): Promise<void> {
  if (!isTauri()) return;
  await invoke("save_reading_position", {
    bookId,
    section,
    sectionProgress,
    progress,
  });
}
