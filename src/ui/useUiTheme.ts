import { useCallback, useEffect, useState } from "react";
import { readLocalValue, writeLocalValue } from "./localStorage";

export type UiTheme =
  | "default"
  | "classic"
  | "bookish"
  | "glass"
  | "liquid-glass"
  | "neumorphism";
export type UiMode = "light" | "dark";

export const uiThemeKey = "aprireader.ui-theme";
export const uiModeKey = "aprireader.ui-mode";

export const uiThemes: readonly UiTheme[] = [
  "default",
  "classic",
  "bookish",
  "glass",
  "liquid-glass",
  "neumorphism",
];

function isUiTheme(value: string | null): value is UiTheme {
  return uiThemes.includes(value as UiTheme);
}

function isUiMode(value: string | null): value is UiMode {
  return value === "light" || value === "dark";
}

function initialUiTheme(): UiTheme {
  const stored = readLocalValue(uiThemeKey);
  return isUiTheme(stored) ? stored : "default";
}

function initialUiMode(): UiMode {
  const stored = readLocalValue(uiModeKey);
  return isUiMode(stored) ? stored : "light";
}

export function useUiTheme() {
  const [uiTheme, setUiThemeState] = useState<UiTheme>(initialUiTheme);
  const [uiMode, setUiModeState] = useState<UiMode>(initialUiMode);

  useEffect(() => {
    document.documentElement.setAttribute("data-ui-theme", uiTheme);
  }, [uiTheme]);

  useEffect(() => {
    document.documentElement.setAttribute("data-ui-mode", uiMode);
  }, [uiMode]);

  const setUiTheme = useCallback((next: UiTheme) => {
    writeLocalValue(uiThemeKey, next);
    setUiThemeState(next);
  }, []);

  const setUiMode = useCallback((next: UiMode) => {
    writeLocalValue(uiModeKey, next);
    setUiModeState(next);
  }, []);

  return { uiTheme, uiMode, setUiTheme, setUiMode };
}
