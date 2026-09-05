import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { useUiTheme, uiModeKey, uiThemeKey } from "./useUiTheme";

describe("useUiTheme", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.removeAttribute("data-ui-theme");
    document.documentElement.removeAttribute("data-ui-mode");
  });

  it("defaults to the default skin and light mode", () => {
    const { result } = renderHook(() => useUiTheme());
    expect(result.current.uiTheme).toBe("default");
    expect(result.current.uiMode).toBe("light");
    expect(document.documentElement.getAttribute("data-ui-theme")).toBe(
      "default",
    );
    expect(document.documentElement.getAttribute("data-ui-mode")).toBe("light");
  });

  it("falls back to the default skin for an invalid stored value", () => {
    localStorage.setItem(uiThemeKey, "not-a-real-skin");
    localStorage.setItem(uiModeKey, "not-a-real-mode");
    const { result } = renderHook(() => useUiTheme());
    expect(result.current.uiTheme).toBe("default");
    expect(result.current.uiMode).toBe("light");
  });

  it("persists a skin change and applies it to the document element", () => {
    const { result } = renderHook(() => useUiTheme());
    act(() => result.current.setUiTheme("glass"));
    expect(result.current.uiTheme).toBe("glass");
    expect(localStorage.getItem(uiThemeKey)).toBe("glass");
    expect(document.documentElement.getAttribute("data-ui-theme")).toBe(
      "glass",
    );
  });

  it("persists a mode change and applies it to the document element", () => {
    const { result } = renderHook(() => useUiTheme());
    act(() => result.current.setUiMode("dark"));
    expect(result.current.uiMode).toBe("dark");
    expect(localStorage.getItem(uiModeKey)).toBe("dark");
    expect(document.documentElement.getAttribute("data-ui-mode")).toBe("dark");
  });

  it("restores a previously stored skin and mode on mount", () => {
    localStorage.setItem(uiThemeKey, "neumorphism");
    localStorage.setItem(uiModeKey, "dark");
    const { result } = renderHook(() => useUiTheme());
    expect(result.current.uiTheme).toBe("neumorphism");
    expect(result.current.uiMode).toBe("dark");
    expect(document.documentElement.getAttribute("data-ui-theme")).toBe(
      "neumorphism",
    );
    expect(document.documentElement.getAttribute("data-ui-mode")).toBe("dark");
  });
});
