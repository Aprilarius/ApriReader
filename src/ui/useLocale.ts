import { useCallback, useState } from "react";
import { translations, type Locale, type TranslationKey } from "./i18n";

const storageKey = "aprireader.locale";

function initialLocale(): Locale {
  const stored = localStorage.getItem(storageKey);
  return stored === "en" || stored === "ru" ? stored : "ru";
}

export function useLocale() {
  const [locale, setLocale] = useState<Locale>(initialLocale);
  const toggleLocale = useCallback(() => {
    setLocale((current) => {
      const next = current === "ru" ? "en" : "ru";
      localStorage.setItem(storageKey, next);
      document.documentElement.lang = next;
      return next;
    });
  }, []);
  const t = useCallback(
    (key: TranslationKey) => translations[locale][key],
    [locale],
  );
  return { locale, t, toggleLocale };
}
