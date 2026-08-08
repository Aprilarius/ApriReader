import { describe, expect, it } from "vitest";
import {
  applyPronunciationDictionary,
  emptyTtsPreferences,
  parseTtsPreferences,
  remapCloudTtsTimings,
  type TtsPronunciationRule,
} from "./ttsPreferences";

const rule = (
  source: string,
  replacement: string,
  id = "rule1",
): TtsPronunciationRule => ({ id, source, replacement });

describe("TTS preferences", () => {
  it("fails closed for malformed or oversized persisted data", () => {
    expect(parseTtsPreferences("not-json")).toEqual(emptyTtsPreferences());
    expect(parseTtsPreferences("x".repeat(65_537))).toEqual(
      emptyTtsPreferences(),
    );
  });

  it("filters invalid, duplicate, and excessive persisted entries", () => {
    const parsed = parseTtsPreferences(
      JSON.stringify({
        dictionaryEnabled: false,
        presets: [
          {
            id: "preset1",
            name: " Calm ",
            provider: "local",
            voiceId: "voice",
            rate: 1,
          },
          {
            id: "bad",
            name: "Bad",
            provider: "unknown",
            voiceId: "voice",
            rate: 1,
          },
        ],
        pronunciations: [
          rule("SQL", "sequel", "first"),
          rule("sql", "duplicate", "second"),
          rule("bad\nsource", "ignored", "bad"),
        ],
      }),
    );
    expect(parsed.dictionaryEnabled).toBe(false);
    expect(parsed.presets).toHaveLength(1);
    expect(parsed.presets[0]?.name).toBe("Calm");
    expect(parsed.pronunciations).toEqual([rule("SQL", "sequel", "first")]);
  });

  it("replaces the longest non-overlapping whole phrase case-insensitively", () => {
    const result = applyPronunciationDictionary("SQL and sqlish. New York!", [
      rule("SQL", "sequel"),
      rule("New", "old", "short"),
      rule("New York", "New-York", "long"),
    ]);
    expect(result.text).toBe("sequel and sqlish. New-York!");
    expect(result.sourceOffsets).toHaveLength(result.text.length + 1);
    expect(result.sourceOffsets.at(-1)).toBe(
      "SQL and sqlish. New York!".length,
    );
  });

  it("maps provider timing from spoken text back to the source word", () => {
    const pronounced = applyPronunciationDictionary("Read SQL now", [
      rule("SQL", "sequel"),
    ]);
    const start = pronounced.text.indexOf("sequel");
    const remapped = remapCloudTtsTimings(
      [
        {
          startOffset: start + 2,
          endOffset: start + 3,
          startSeconds: 0.2,
          endSeconds: 0.3,
        },
      ],
      pronounced.sourceOffsets,
    );
    expect(remapped[0]?.startOffset).toBeGreaterThanOrEqual(5);
    expect(remapped[0]?.startOffset).toBeLessThan(8);
  });

  it("rejects pathological dictionary expansion", () => {
    expect(() =>
      applyPronunciationDictionary("a ".repeat(1_000), [
        rule("a", "very-long-pronunciation"),
      ]),
    ).toThrow("TTS_PRONUNCIATION_EXPANSION_LIMIT");
  });
});
