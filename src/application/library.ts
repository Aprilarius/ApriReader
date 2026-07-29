import { convertFileSrc, invoke, isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

export type Book = {
  id: number;
  sourcePath: string;
  title: string;
  author: string;
  format: string;
  fileSize: number;
  coverPath: string | null;
  addedAt: string;
  isAvailable: boolean;
  progress: number;
  subtitle: string;
  isbn: string;
  publisher: string;
  publishedYear: string;
  language: string;
  series: string;
  genres: string;
  description: string;
  metadataSource: string;
  metadataProviderId: string | null;
  metadataUpdatedAt: string | null;
  coverSource: string;
  lastOpenedAt: number | null;
  isFavorite: boolean;
};

export type WatchedFolder = {
  id: number;
  path: string;
  lastScannedAt: string | null;
};

export type ImportSummary = {
  imported: number;
  duplicates: number;
  failed: number;
  errors: string[];
};

const bookExtensions = [
  "epub",
  "fb2",
  "txt",
  "html",
  "htm",
  "md",
  "markdown",
  "pdf",
  "cbz",
  "cbr",
  "docx",
];

export const coverUrl = (path: string) => convertFileSrc(path);

export async function listBooks(): Promise<Book[]> {
  if (!isTauri()) return [];
  return invoke<Book[]>("list_books");
}

export async function chooseAndImportBooks(): Promise<ImportSummary | null> {
  const selected = await open({
    multiple: true,
    directory: false,
    filters: [{ name: "Books", extensions: bookExtensions }],
  });
  if (!selected) return null;
  const paths = Array.isArray(selected) ? selected : [selected];
  return invoke<ImportSummary>("import_books", { paths });
}

export async function chooseAndWatchFolder(): Promise<ImportSummary | null> {
  const selected = await open({ directory: true, multiple: false });
  if (!selected || Array.isArray(selected)) return null;
  return invoke<ImportSummary>("add_watched_folder", { path: selected });
}

export async function listWatchedFolders(): Promise<WatchedFolder[]> {
  if (!isTauri()) return [];
  return invoke<WatchedFolder[]>("list_watched_folders");
}

export async function scanWatchedFolders(): Promise<ImportSummary> {
  return invoke<ImportSummary>("scan_watched_folders");
}

export async function setBookFavorite(
  bookId: number,
  favorite: boolean,
): Promise<Book> {
  return invoke<Book>("set_book_favorite", { bookId, favorite });
}

export async function removeBooks(bookIds: number[]): Promise<number> {
  return invoke<number>("remove_books", { bookIds });
}
