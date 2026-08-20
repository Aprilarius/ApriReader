import { useCallback, useEffect, useState } from "react";
import { isLocale, translate, type Locale, type TranslationKey } from "./i18n";
import { readLocalValue, writeLocalValue } from "./localStorage";

const storageKey = "aprireader.locale";
const selectionKey = "aprireader.languageSelected";

function initialLocale(): Locale {
  const stored = readLocalValue(storageKey);
  if (isLocale(stored)) return stored;
  return "ru";
}

export function useLocale() {
  const [locale, setLocale] = useState<Locale>(initialLocale);
  const [languageSelected, setLanguageSelected] = useState(
    () => readLocalValue(selectionKey) === "true",
  );

  useEffect(() => {
    document.documentElement.lang = locale;
  }, [locale]);

  const selectLocale = useCallback((next: Locale) => {
    writeLocalValue(storageKey, next);
    setLocale(next);
  }, []);
  const confirmLocale = useCallback(
    (next: Locale) => {
      selectLocale(next);
      writeLocalValue(selectionKey, "true");
      setLanguageSelected(true);
    },
    [selectLocale],
  );
  const t = useCallback(
    (key: TranslationKey) => translate(locale, key),
    [locale],
  );
  return { locale, t, selectLocale, confirmLocale, languageSelected };
}
