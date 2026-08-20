import { describe, expect, it } from "vitest";
import { isLocale, supportedLocales, translate } from "./i18n";

describe("internationalization", () => {
  it("publishes all supported interface languages", () => {
    expect(supportedLocales.map(({ id }) => id)).toEqual([
      "ru",
      "en",
      "az",
      "it",
      "de",
      "tr",
    ]);
    for (const locale of supportedLocales)
      expect(isLocale(locale.id)).toBe(true);
    expect(isLocale("fr")).toBe(false);
  });

  it.each([
    ["az", "Kitabxana"],
    ["it", "Libreria"],
    ["de", "Bibliothek"],
    ["tr", "Kitaplık"],
  ] as const)("translates core navigation for %s", (locale, library) => {
    expect(translate(locale, "library")).toBe(library);
    expect(translate(locale, "readerBack")).not.toBe("Back to library");
  });

  it("uses English for untranslated specialist strings", () => {
    expect(translate("de", "ttsCloudConsentTitle")).toBe(
      translate("en", "ttsCloudConsentTitle"),
    );
  });
});
