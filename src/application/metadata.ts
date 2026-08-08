import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { Book } from "./library";

export type MetadataLanguage = "ru" | "en";

export type BookMetadataInput = {
  title: string;
  author: string;
  subtitle: string;
  isbn: string;
  publisher: string;
  publishedYear: string;
  language: string;
  series: string;
  genres: string;
  description: string;
};

export type MetadataCandidate = {
  provider: "Open Library" | "Inventaire";
  providerId: string;
  title: string;
  author: string;
  isbn: string;
  publisher: string;
  publishedYear: string;
  language: string;
  series: string;
  genres: string;
  coverId: number | null;
  coverPath: string;
  description: string;
};

export const metadataFromBook = (book: Book): BookMetadataInput => ({
  title: book.title,
  author: book.author,
  subtitle: book.subtitle,
  isbn: book.isbn,
  publisher: book.publisher,
  publishedYear: book.publishedYear,
  language: book.language,
  series: book.series,
  genres: book.genres,
  description: book.description,
});

export const updateBookMetadata = (
  bookId: number,
  metadata: BookMetadataInput,
) => invoke<Book>("update_book_metadata", { bookId, metadata });

export const searchMetadata = (
  bookId: number,
  query: string,
  language: MetadataLanguage,
) =>
  invoke<MetadataCandidate[]>("search_metadata", { bookId, query, language });

export const applyMetadataCandidate = (
  bookId: number,
  candidate: MetadataCandidate,
) => invoke<Book>("apply_metadata_candidate", { bookId, candidate });

export const removeExternalCover = (bookId: number) =>
  invoke<Book>("remove_external_cover", { bookId });

export async function chooseAndSetLocalCover(
  bookId: number,
): Promise<Book | null> {
  const selected = await open({
    multiple: false,
    directory: false,
    filters: [
      {
        name: "Images",
        extensions: ["jpg", "jpeg", "png", "webp"],
      },
    ],
  });
  if (!selected || Array.isArray(selected)) return null;
  return invoke<Book>("set_local_cover", { bookId, path: selected });
}
