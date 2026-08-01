import { useCallback, useState } from "react";
import { readLocalValue, writeLocalValue } from "./localStorage";

export const screenReaderSupportKey = "aprireader.screenReaderSupport";

function readScreenReaderSupport() {
  return readLocalValue(screenReaderSupportKey) !== "false";
}

export function useScreenReaderSupport() {
  const [screenReaderSupport, setStoredValue] = useState(
    readScreenReaderSupport,
  );

  const setScreenReaderSupport = useCallback((enabled: boolean) => {
    setStoredValue(enabled);
    writeLocalValue(screenReaderSupportKey, String(enabled));
  }, []);

  return { screenReaderSupport, setScreenReaderSupport };
}

export function normalizeBookLanguage(value: string) {
  const normalized = value.trim();
  if (!normalized) return undefined;

  const knownLanguages: Record<string, string> = {
    en: "en-US",
    eng: "en-US",
    english: "en-US",
    ru: "ru-RU",
    rus: "ru-RU",
    russian: "ru-RU",
    русский: "ru-RU",
  };
  const known = knownLanguages[normalized.toLocaleLowerCase()];
  if (known) return known;

  if (!/^[a-z]{2,3}(?:-[a-z0-9]{2,8})*$/i.test(normalized)) return undefined;
  try {
    return Intl.getCanonicalLocales(normalized)[0];
  } catch {
    return undefined;
  }
}
