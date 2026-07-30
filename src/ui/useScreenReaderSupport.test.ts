import { describe, expect, it } from "vitest";
import { normalizeBookLanguage } from "./useScreenReaderSupport";

describe("normalizeBookLanguage", () => {
  it("maps common English and Russian metadata to useful language tags", () => {
    expect(normalizeBookLanguage("eng")).toBe("en-US");
    expect(normalizeBookLanguage("Русский")).toBe("ru-RU");
    expect(normalizeBookLanguage("de-DE")).toBe("de-DE");
  });

  it("omits missing or invalid language metadata", () => {
    expect(normalizeBookLanguage("")).toBeUndefined();
    expect(normalizeBookLanguage("English, Russian")).toBeUndefined();
  });
});
