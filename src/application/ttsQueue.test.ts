import { describe, expect, it } from "vitest";
import type { DocumentSection } from "./reader";
import {
  buildTtsQueue,
  highlightAtPlaybackPosition,
  maxTtsChunkCharacters,
} from "./ttsQueue";

const sections: DocumentSection[] = [
  {
    id: "one",
    title: "First section",
    blocks: [
      { kind: "paragraph", text: "One short sentence. Another sentence!" },
    ],
  },
  {
    id: "two",
    title: "Second section",
    blocks: [{ kind: "paragraph", text: "Final words here." }],
  },
];

describe("TTS queue", () => {
  it("bounds chunks and keeps exact section and block offsets", () => {
    const longWord = "я".repeat(maxTtsChunkCharacters + 10);
    const queue = buildTtsQueue(
      [
        {
          id: "long",
          title: "Title",
          blocks: [{ kind: "paragraph", text: `${longWord}. End.` }],
        },
      ],
      0,
      "book",
    );
    expect(
      queue.every((chunk) => chunk.text.length <= maxTtsChunkCharacters),
    ).toBe(true);
    expect(queue.some((chunk) => chunk.blockIndex === 0)).toBe(true);
    for (const chunk of queue.filter((value) => value.blockIndex === 0)) {
      expect(
        `${longWord}. End.`.slice(chunk.startOffset, chunk.endOffset),
      ).toBe(chunk.text);
    }
  });

  it("continues across sections only for whole-book scope", () => {
    const sectionQueue = buildTtsQueue(sections, 0, "section");
    const bookQueue = buildTtsQueue(sections, 0, "book");
    expect(sectionQueue.every((chunk) => chunk.sectionIndex === 0)).toBe(true);
    expect(bookQueue.some((chunk) => chunk.sectionIndex === 1)).toBe(true);
  });

  it("maps native playback progress to a word range", () => {
    const chunk = buildTtsQueue(sections, 1, "section").find(
      (value) => value.blockIndex === 0,
    )!;
    expect(highlightAtPlaybackPosition(chunk, 0, 9)).toMatchObject({
      sectionIndex: 1,
      blockIndex: 0,
      startOffset: 0,
      endOffset: 5,
    });
    expect(highlightAtPlaybackPosition(chunk, 8.9, 9)).toMatchObject({
      startOffset: 12,
      endOffset: 16,
    });
    expect(
      highlightAtPlaybackPosition(chunk, 0.25, 9, [
        {
          startOffset: 6,
          endOffset: 7,
          startSeconds: 0.2,
          endSeconds: 0.3,
        },
      ]),
    ).toMatchObject({ startOffset: 6, endOffset: 11 });
  });
});
