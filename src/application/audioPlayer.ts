import { invoke } from "@tauri-apps/api/core";
import type { AudiobookRecord } from "./audiobooks";

export interface AudioPlaybackSnapshot {
  phase:
    | "idle"
    | "opening"
    | "ready"
    | "buffering"
    | "playing"
    | "paused"
    | "ended"
    | "failed";
  path: string | null;
  positionSeconds: number;
  durationSeconds: number;
  playbackRate: number;
  volume: number;
  canSeek: boolean;
  canPause: boolean;
  lastError: string | null;
}

export interface AudiobookBookmarkRecord {
  id: number;
  audiobookId: number;
  partIndex: number;
  positionSeconds: number;
  note: string;
  createdAt: string;
}

export interface AudiobookChapterRecord {
  id: number;
  audiobookId: number;
  partIndex: number;
  title: string;
  startSeconds: number;
  ordinal: number;
}

export interface AudioOutputDevice {
  id: string;
  name: string;
  isDefault: boolean;
  isEnabled: boolean;
}

export function loadAudioFile(path: string): Promise<AudioPlaybackSnapshot> {
  return invoke<AudioPlaybackSnapshot>("audio_load_file", { path });
}

export function playAudio(): Promise<AudioPlaybackSnapshot> {
  return invoke<AudioPlaybackSnapshot>("audio_play");
}

export function pauseAudio(): Promise<AudioPlaybackSnapshot> {
  return invoke<AudioPlaybackSnapshot>("audio_pause");
}

export function seekAudio(seconds: number): Promise<AudioPlaybackSnapshot> {
  return invoke<AudioPlaybackSnapshot>("audio_seek", { seconds });
}

export function setAudioRate(rate: number): Promise<AudioPlaybackSnapshot> {
  return invoke<AudioPlaybackSnapshot>("audio_set_rate", { rate });
}

export function setAudioVolume(volume: number): Promise<AudioPlaybackSnapshot> {
  return invoke<AudioPlaybackSnapshot>("audio_set_volume", { volume });
}

export function getAudioSnapshot(): Promise<AudioPlaybackSnapshot> {
  return invoke<AudioPlaybackSnapshot>("audio_snapshot");
}

export function stopAudio(): Promise<AudioPlaybackSnapshot> {
  return invoke<AudioPlaybackSnapshot>("audio_stop");
}

export function listAudioOutputDevices(): Promise<AudioOutputDevice[]> {
  return invoke<AudioOutputDevice[]>("audio_list_output_devices");
}

export function setAudioOutputDevice(
  deviceId: string,
): Promise<AudioPlaybackSnapshot> {
  return invoke<AudioPlaybackSnapshot>("audio_set_output_device", { deviceId });
}

export function saveAudiobookPosition(
  audiobookId: number,
  partIndex: number,
  positionSeconds: number,
  durationSeconds: number,
): Promise<AudiobookRecord> {
  return invoke<AudiobookRecord>("save_audiobook_position", {
    audiobookId,
    partIndex,
    positionSeconds,
    durationSeconds,
  });
}

export function listAudiobookBookmarks(
  audiobookId: number,
): Promise<AudiobookBookmarkRecord[]> {
  return invoke<AudiobookBookmarkRecord[]>("list_audiobook_bookmarks", {
    audiobookId,
  });
}

export function createAudiobookBookmark(
  audiobookId: number,
  partIndex: number,
  positionSeconds: number,
  note: string,
): Promise<AudiobookBookmarkRecord> {
  return invoke<AudiobookBookmarkRecord>("create_audiobook_bookmark", {
    audiobookId,
    partIndex,
    positionSeconds,
    note,
  });
}

export function deleteAudiobookBookmark(bookmarkId: number): Promise<void> {
  return invoke<void>("delete_audiobook_bookmark", { bookmarkId });
}

export function listAudiobookChapters(
  audiobookId: number,
): Promise<AudiobookChapterRecord[]> {
  return invoke<AudiobookChapterRecord[]>("list_audiobook_chapters", {
    audiobookId,
  });
}
