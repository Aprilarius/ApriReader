import { invoke, isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { MetadataCandidate, MetadataLanguage } from "./metadata";

const audioExtensions = [
  "aac",
  "flac",
  "m4a",
  "m4b",
  "mp3",
  "wav",
  "wma",
  "3g2",
  "3gp",
  "amr",
  "aif",
  "aiff",
  "alac",
  "ape",
  "caf",
  "mka",
  "mpc",
  "oga",
  "ogg",
  "opus",
  "wv",
  "cue",
  "m3u",
  "m3u8",
];

export interface AudiobookRecord {
  id: number;
  title: string;
  author: string;
  coverPath: string | null;
  addedAt: string;
  isAvailable: boolean;
  totalSize: number;
  partCount: number;
  totalDurationSeconds: number;
  progress: number;
  lastPartIndex: number;
  lastPositionSeconds: number;
  narrator: string;
  series: string;
  genres: string;
  description: string;
  language: string;
  publishedYear: string;
  metadataSource: string;
  metadataProviderId: string | null;
  metadataUpdatedAt: string | null;
  coverSource: string;
}

export interface AudiobookMetadataInput {
  title: string;
  author: string;
  narrator: string;
  series: string;
  genres: string;
  description: string;
  language: string;
  publishedYear: string;
}

export interface AudiobookPartRecord {
  id: number;
  audiobookId: number;
  sourcePath: string;
  title: string;
  format: string;
  fileSize: number;
  durationSeconds: number | null;
  ordinal: number;
  isAvailable: boolean;
}

export interface WatchedAudioFolder {
  id: number;
  path: string;
  lastScannedAt: string | null;
}

export interface AudioImportSummary {
  importedBooks: number;
  importedParts: number;
  duplicateParts: number;
  failed: number;
  errors: string[];
}

export function listAudiobooks(): Promise<AudiobookRecord[]> {
  if (!isTauri()) return Promise.resolve([]);
  return invoke<AudiobookRecord[]>("list_audiobooks");
}

export function listAudiobookParts(
  audiobookId: number,
): Promise<AudiobookPartRecord[]> {
  return invoke<AudiobookPartRecord[]>("list_audiobook_parts", {
    audiobookId,
  });
}

export function importAudiobooks(paths: string[]): Promise<AudioImportSummary> {
  return invoke<AudioImportSummary>("import_audiobooks", { paths });
}

export async function chooseAndImportAudiobooks(): Promise<AudioImportSummary | null> {
  const selected = await open({
    multiple: true,
    directory: false,
    filters: [{ name: "Audiobooks", extensions: audioExtensions }],
  });
  if (!selected) return null;
  return importAudiobooks(Array.isArray(selected) ? selected : [selected]);
}

export async function chooseAndImportAudiobookFolder(): Promise<AudioImportSummary | null> {
  const selected = await open({ directory: true, multiple: false });
  if (!selected || Array.isArray(selected)) return null;
  return importAudiobooks([selected]);
}

export function addWatchedAudioFolder(
  path: string,
): Promise<AudioImportSummary> {
  return invoke<AudioImportSummary>("add_watched_audio_folder", { path });
}

export function listWatchedAudioFolders(): Promise<WatchedAudioFolder[]> {
  if (!isTauri()) return Promise.resolve([]);
  return invoke<WatchedAudioFolder[]>("list_watched_audio_folders");
}

export async function chooseAndWatchAudioFolder(): Promise<AudioImportSummary | null> {
  const selected = await open({ directory: true, multiple: false });
  if (!selected || Array.isArray(selected)) return null;
  return addWatchedAudioFolder(selected);
}

export function scanWatchedAudioFolders(): Promise<AudioImportSummary> {
  return invoke<AudioImportSummary>("scan_watched_audio_folders");
}

export function audiobookMetadataFromRecord(
  book: AudiobookRecord,
): AudiobookMetadataInput {
  return {
    title: book.title,
    author: book.author,
    narrator: book.narrator,
    series: book.series,
    genres: book.genres,
    description: book.description,
    language: book.language,
    publishedYear: book.publishedYear,
  };
}

export function updateAudiobookMetadata(
  audiobookId: number,
  metadata: AudiobookMetadataInput,
): Promise<AudiobookRecord> {
  return invoke<AudiobookRecord>("update_audiobook_metadata", {
    audiobookId,
    metadata,
  });
}

export function searchAudiobookMetadata(
  audiobookId: number,
  query: string,
  language: MetadataLanguage,
): Promise<MetadataCandidate[]> {
  return invoke<MetadataCandidate[]>("search_audiobook_metadata", {
    audiobookId,
    query,
    language,
  });
}

export function applyAudiobookMetadataCandidate(
  audiobookId: number,
  candidate: MetadataCandidate,
): Promise<AudiobookRecord> {
  return invoke<AudiobookRecord>("apply_audiobook_metadata_candidate", {
    audiobookId,
    candidate,
  });
}

export async function chooseAndSetAudiobookCover(
  audiobookId: number,
): Promise<AudiobookRecord | null> {
  const path = await open({
    multiple: false,
    directory: false,
    filters: [{ name: "Images", extensions: ["jpg", "jpeg", "png", "webp"] }],
  });
  if (!path || Array.isArray(path)) return null;
  return invoke<AudiobookRecord>("set_audiobook_local_cover", {
    audiobookId,
    path,
  });
}
