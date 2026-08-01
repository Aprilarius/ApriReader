import { openUrl } from "@tauri-apps/plugin-opener";

export type ExternalTranslationProvider = "google" | "yandex";
export type TranslationDirection = {
  sourceLanguage: "en" | "ru";
  targetLanguage: "en" | "ru";
};

export const externalTranslationConsentKey =
  "aprireader.external-translation-consent.v1";
export const externalTranslationMaxCharacters = 2_000;

export function detectTranslationDirection(text: string): TranslationDirection {
  const cyrillic = text.match(/[\p{Script=Cyrillic}]/gu)?.length ?? 0;
  const latin = text.match(/[\p{Script=Latin}]/gu)?.length ?? 0;
  return cyrillic > latin
    ? { sourceLanguage: "ru", targetLanguage: "en" }
    : { sourceLanguage: "en", targetLanguage: "ru" };
}

export function normalizeTranslationText(text: string): string {
  const normalized = text.trim();
  if (!normalized) throw new Error("translation text is empty");
  if (Array.from(normalized).length > externalTranslationMaxCharacters) {
    throw new Error("translation text is too long");
  }
  return normalized;
}

export function buildExternalTranslationUrl(
  provider: ExternalTranslationProvider,
  text: string,
): string {
  const normalized = normalizeTranslationText(text);
  const direction = detectTranslationDirection(normalized);
  const url = new URL(
    provider === "google"
      ? "https://translate.google.com/"
      : "https://translate.yandex.com/",
  );

  if (provider === "google") {
    url.searchParams.set("sl", direction.sourceLanguage);
    url.searchParams.set("tl", direction.targetLanguage);
    url.searchParams.set("text", normalized);
    url.searchParams.set("op", "translate");
  } else {
    url.searchParams.set("source_lang", direction.sourceLanguage);
    url.searchParams.set("target_lang", direction.targetLanguage);
    url.searchParams.set("text", normalized);
  }
  return url.toString();
}

export async function openExternalTranslation(
  provider: ExternalTranslationProvider,
  text: string,
): Promise<void> {
  await openUrl(buildExternalTranslationUrl(provider, text));
}
