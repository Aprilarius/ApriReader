import { describe, expect, it } from "vitest";
import {
  buildExternalTranslationUrl,
  detectTranslationDirection,
  externalTranslationMaxCharacters,
  normalizeTranslationText,
} from "./externalTranslation";

describe("external translation", () => {
  it("opens English text in Google with an English to Russian direction", () => {
    const url = new URL(buildExternalTranslationUrl("google", "A quiet book"));
    expect(url.origin).toBe("https://translate.google.com");
    expect(url.searchParams.get("sl")).toBe("en");
    expect(url.searchParams.get("tl")).toBe("ru");
    expect(url.searchParams.get("text")).toBe("A quiet book");
  });

  it("opens Russian text in Yandex with a Russian to English direction", () => {
    const url = new URL(buildExternalTranslationUrl("yandex", "Тихая книга"));
    expect(url.origin).toBe("https://translate.yandex.com");
    expect(url.searchParams.get("source_lang")).toBe("ru");
    expect(url.searchParams.get("target_lang")).toBe("en");
    expect(url.searchParams.get("text")).toBe("Тихая книга");
  });

  it("uses the script that occurs most often for mixed selections", () => {
    expect(detectTranslationDirection("hello мир").sourceLanguage).toBe("en");
    expect(detectTranslationDirection("hello мирыыы").sourceLanguage).toBe(
      "ru",
    );
  });

  it("rejects blank and oversized selections", () => {
    expect(() => normalizeTranslationText("   ")).toThrow();
    expect(() =>
      normalizeTranslationText(
        "a".repeat(externalTranslationMaxCharacters + 1),
      ),
    ).toThrow();
  });
});
