import { invoke, isTauri } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";

export type AnnotationKind = "bookmark" | "highlight" | "note" | "quote";

export type ReadingLocator = {
  sectionId: string;
  blockIndex: number;
  startOffset: number;
  endOffset: number;
};

export type AnnotationRecord = {
  id: number;
  bookId: number;
  kind: AnnotationKind;
  locator: ReadingLocator;
  selectedText: string;
  note: string;
  createdAt: string;
  updatedAt: string;
};

export type SearchResult = {
  sectionId: string;
  sectionTitle: string;
  blockIndex: number;
  excerpt: string;
};

export type CreateAnnotation = {
  bookId: number;
  kind: AnnotationKind;
  sectionId: string;
  blockIndex: number;
  startOffset: number;
  endOffset: number;
  selectedText: string;
  note: string;
};

export async function searchBook(
  bookId: number,
  query: string,
): Promise<SearchResult[]> {
  if (!isTauri()) return [];
  return invoke<SearchResult[]>("search_book", { bookId, query });
}

export async function listAnnotations(
  bookId: number,
): Promise<AnnotationRecord[]> {
  if (!isTauri()) return [];
  return invoke<AnnotationRecord[]>("list_annotations", { bookId });
}

export async function createAnnotation(
  input: CreateAnnotation,
): Promise<AnnotationRecord> {
  if (!isTauri()) return temporaryAnnotation(input);
  return invoke<AnnotationRecord>("create_annotation", input);
}

export async function updateAnnotationNote(
  annotationId: number,
  note: string,
): Promise<AnnotationRecord> {
  return invoke<AnnotationRecord>("update_annotation_note", {
    annotationId,
    note,
  });
}

export async function deleteAnnotation(annotationId: number): Promise<void> {
  if (!isTauri()) return;
  await invoke("delete_annotation", { annotationId });
}

export async function chooseAndExportAnnotations(
  bookId: number,
  title: string,
): Promise<boolean> {
  if (!isTauri()) return false;
  const path = await save({
    defaultPath: `${safeFileName(title)}-annotations.md`,
    filters: [{ name: "Markdown", extensions: ["md"] }],
  });
  if (!path) return false;
  await invoke("export_annotations", { bookId, path });
  return true;
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

function temporaryAnnotation(input: CreateAnnotation): AnnotationRecord {
  const now = new Date().toISOString();
  return {
    id: Date.now(),
    bookId: input.bookId,
    kind: input.kind,
    locator: {
      sectionId: input.sectionId,
      blockIndex: input.blockIndex,
      startOffset: input.startOffset,
      endOffset: input.endOffset,
    },
    selectedText: input.selectedText,
    note: input.note,
    createdAt: now,
    updatedAt: now,
  };
}
