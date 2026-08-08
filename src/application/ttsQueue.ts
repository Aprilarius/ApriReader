import type { DocumentSection } from "./reader";
import type { CloudTtsTiming } from "./cloudTts";

export const maxTtsChunkCharacters = 1_200;
export const maxTtsSessionChunks = 50_000;

export interface TtsWordRange {
  startOffset: number;
  endOffset: number;
}

export interface TtsQueueChunk {
  id: string;
  sectionIndex: number;
  blockIndex: number;
  startOffset: number;
  endOffset: number;
  text: string;
  words: TtsWordRange[];
}

export interface TtsHighlightRange {
  sectionIndex: number;
  blockIndex: number;
  startOffset: number;
  endOffset: number;
}

export type TtsReadingScope = "section" | "book";

const sentencePattern = /[^.!?…\n]+(?:[.!?…]+|$)|\n+/gu;
const wordPattern = /[\p{L}\p{N}][\p{L}\p{M}\p{N}'’-]*/gu;

function boundedRanges(text: string) {
  const ranges: Array<{ start: number; end: number }> = [];
  for (const match of text.matchAll(sentencePattern)) {
    const rawStart = match.index;
    const rawEnd = rawStart + match[0].length;
    let start = rawStart;
    let end = rawEnd;
    while (start < end && /\s/u.test(text[start]!)) start += 1;
    while (end > start && /\s/u.test(text[end - 1]!)) end -= 1;
    if (end <= start) continue;
    while (end - start > maxTtsChunkCharacters) {
      const target = start + maxTtsChunkCharacters;
      const candidate = text.slice(start, target + 1);
      const whitespace = Math.max(
        candidate.lastIndexOf(" "),
        candidate.lastIndexOf("\n"),
        candidate.lastIndexOf("\t"),
      );
      const split =
        whitespace > maxTtsChunkCharacters / 2 ? start + whitespace : target;
      ranges.push({ start, end: split });
      start = split;
      while (start < end && /\s/u.test(text[start]!)) start += 1;
    }
    if (end > start) ranges.push({ start, end });
  }
  return ranges;
}

function chunkWords(text: string, baseOffset: number): TtsWordRange[] {
  return Array.from(text.matchAll(wordPattern), (match) => ({
    startOffset: baseOffset + match.index,
    endOffset: baseOffset + match.index + match[0].length,
  }));
}

export function buildTtsQueue(
  sections: DocumentSection[],
  startSection: number,
  scope: TtsReadingScope,
): TtsQueueChunk[] {
  const chunks: TtsQueueChunk[] = [];
  const first = Math.max(0, Math.min(startSection, sections.length - 1));
  const last = scope === "section" ? first + 1 : sections.length;
  for (let sectionIndex = first; sectionIndex < last; sectionIndex += 1) {
    const section = sections[sectionIndex];
    if (!section) continue;
    const sources = [
      { blockIndex: -1, text: section.title },
      ...section.blocks.map((block, blockIndex) => ({
        blockIndex,
        text: block.text,
      })),
    ];
    for (const source of sources) {
      for (const range of boundedRanges(source.text)) {
        const text = source.text.slice(range.start, range.end);
        const words = chunkWords(text, range.start);
        if (words.length === 0) continue;
        chunks.push({
          id: `${sectionIndex}:${source.blockIndex}:${range.start}:${range.end}`,
          sectionIndex,
          blockIndex: source.blockIndex,
          startOffset: range.start,
          endOffset: range.end,
          text,
          words,
        });
        if (chunks.length > maxTtsSessionChunks) {
          throw new Error("TTS_QUEUE_LIMIT");
        }
      }
    }
  }
  return chunks;
}

export function highlightAtPlaybackPosition(
  chunk: TtsQueueChunk,
  positionSeconds: number,
  durationSeconds: number,
  timings: CloudTtsTiming[] = [],
): TtsHighlightRange | null {
  if (chunk.words.length === 0) return null;
  if (timings.length > 0) {
    const timing =
      timings.find(
        (value) =>
          value.startSeconds <= positionSeconds &&
          value.endSeconds > positionSeconds,
      ) ??
      [...timings]
        .reverse()
        .find((value) => value.startSeconds <= positionSeconds) ??
      timings[0];
    if (timing) {
      const offset = chunk.startOffset + timing.startOffset;
      const word =
        chunk.words.find(
          (value) => value.startOffset <= offset && value.endOffset > offset,
        ) ??
        chunk.words.find((value) => value.startOffset >= offset) ??
        chunk.words.at(-1);
      if (word) {
        return {
          sectionIndex: chunk.sectionIndex,
          blockIndex: chunk.blockIndex,
          startOffset: word.startOffset,
          endOffset: word.endOffset,
        };
      }
    }
  }
  const progress =
    durationSeconds > 0
      ? Math.max(0, Math.min(0.999_999, positionSeconds / durationSeconds))
      : 0;
  const word = chunk.words[Math.floor(progress * chunk.words.length)]!;
  return {
    sectionIndex: chunk.sectionIndex,
    blockIndex: chunk.blockIndex,
    startOffset: word.startOffset,
    endOffset: word.endOffset,
  };
}
