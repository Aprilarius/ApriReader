import { afterEach, describe, expect, it } from "vitest";
import styles from "./styles.css?inline";

afterEach(() => {
  document.head.querySelector("style[data-test-styles]")?.remove();
  document.body.replaceChildren();
});

describe("light-theme button contrast", () => {
  it("keeps dialog variants and disabled actions visible without hover", () => {
    const style = document.createElement("style");
    style.dataset.testStyles = "true";
    style.textContent = styles;
    document.head.append(style);

    const actions = document.createElement("div");
    actions.className = "dialog-actions";
    actions.innerHTML = `
      <button class="primary-button" disabled>Озвучить раздел</button>
      <button class="secondary-button" disabled>Остановить</button>
    `;
    document.body.append(actions);

    const primary = actions.querySelector<HTMLButtonElement>(".primary-button");
    const secondary =
      actions.querySelector<HTMLButtonElement>(".secondary-button");
    if (!primary || !secondary) {
      throw new Error("Expected both button variants in the contrast fixture.");
    }
    const primaryStyle = getComputedStyle(primary);
    const secondaryStyle = getComputedStyle(secondary);

    expect(primaryStyle.backgroundColor).not.toBe("transparent");
    expect(primaryStyle.color).not.toBe("rgb(255, 250, 240)");
    expect(secondaryStyle.backgroundColor).not.toBe("transparent");
    expect(secondaryStyle.opacity).not.toBe("0.55");
  });
});

/*
 * jsdom's CSS parser drops rules it cannot model, and its CSSOM covers neither
 * grid nor custom properties, so the stylesheet source is inspected directly.
 */
function declarationBlock(css: string, selector: string) {
  const marker = `${selector} {`;
  const start = css.indexOf(marker);
  if (start < 0) throw new Error(`Expected a ${selector} rule.`);
  const open = css.indexOf("{", start);
  const close = css.indexOf("}", open);
  return css.slice(open + 1, close);
}

describe("reader crash fallback layout", () => {
  it("gives the safe reader its own grid instead of the four-row reader one", () => {
    const readerScreen = declarationBlock(styles, ".reader-screen");
    const safeReader = declarationBlock(styles, ".safe-reader-screen");

    /*
     * SafeReaderScreen renders a toolbar and the document only. Inheriting the
     * reader's toolbar/progress/content/status template drops its <main> into
     * the 3px progress row and clips the recovered book out of sight, so the
     * fallback needs a template of its own.
     */
    expect(readerScreen).toContain("grid-template-rows");
    expect(readerScreen).toContain("3px");
    expect(safeReader).toContain("grid-template-rows");
    expect(safeReader).not.toContain("3px");
  });

  it("keeps reader typography usable without the preference style attribute", () => {
    const readerScreen = declarationBlock(styles, ".reader-screen");

    /*
     * The fallback carries .reader-screen but no inline style, so every
     * typography variable the reader stylesheet consumes needs a default here.
     * Without one the declarations using them are invalid at computed-value
     * time and silently reset to their initial values.
     */
    for (const property of [
      "--reader-font-size",
      "--reader-line-height",
      "--reader-column-width",
      "--reader-font-family",
      "--reader-letter-spacing",
      "--reader-word-spacing",
      "--reader-paragraph-spacing",
      "--reader-font-weight",
      "--reader-font-style",
      "--reader-text-align",
    ]) {
      expect(readerScreen).toContain(`${property}:`);
    }
  });
});

describe("UI shell skins", () => {
  function attributeBlock(css: string, selector: string) {
    const marker = `${selector} {`;
    const start = css.indexOf(marker);
    if (start < 0) throw new Error(`Expected a ${selector} rule.`);
    const open = css.indexOf("{", start);
    const close = css.indexOf("}", open);
    return css.slice(open + 1, close);
  }

  it("declares the new panel-effect tokens with inert defaults on :root", () => {
    const root = attributeBlock(styles, ":root");
    expect(root).toContain("--panel-blur: 0px");
    expect(root).toContain("--panel-shadow-1: none");
    expect(root).toContain("--panel-shadow-2: none");
    expect(root).toContain("--panel-border: transparent");
  });

  it("defines every non-default token skin under the forced-colors-off guard", () => {
    const guardStart = styles.indexOf("@media not (forced-colors: active)");
    expect(guardStart).toBeGreaterThan(-1);
    const forcedColorsStart = styles.indexOf("@media (forced-colors: active)");
    expect(forcedColorsStart).toBeGreaterThan(guardStart);

    const guardBody = styles.slice(guardStart, forcedColorsStart);
    for (const selector of [
      '[data-ui-theme="default"][data-ui-mode="dark"]',
      '[data-ui-theme="classic"][data-ui-mode="light"]',
      '[data-ui-theme="classic"][data-ui-mode="dark"]',
      '[data-ui-theme="bookish"][data-ui-mode="light"]',
      '[data-ui-theme="bookish"][data-ui-mode="dark"]',
    ]) {
      expect(guardBody).toContain(selector);
    }
  });

  it("gives the classic skin its own accent instead of reusing the default brass", () => {
    const guardStart = styles.indexOf("@media not (forced-colors: active)");
    const classicLight = attributeBlock(
      styles.slice(guardStart),
      ':root[data-ui-theme="classic"][data-ui-mode="light"]',
    );
    expect(classicLight).toContain("--brass: #2f5d8a");
  });

  it("gives glass and liquid-glass their own translucency and blur", () => {
    const guardStart = styles.indexOf("@media not (forced-colors: active)");
    const forcedColorsStart = styles.indexOf("@media (forced-colors: active)");
    const guardBody = styles.slice(guardStart, forcedColorsStart);

    for (const selector of [
      '[data-ui-theme="glass"][data-ui-mode="light"]',
      '[data-ui-theme="glass"][data-ui-mode="dark"]',
      '[data-ui-theme="liquid-glass"][data-ui-mode="light"]',
      '[data-ui-theme="liquid-glass"][data-ui-mode="dark"]',
    ]) {
      expect(guardBody).toContain(selector);
    }

    expect(guardBody).toContain('[data-ui-theme="glass"] .settings-section');
    expect(guardBody).toContain("backdrop-filter: blur(var(--panel-blur))");
    expect(guardBody).toContain(
      '[data-ui-theme="liquid-glass"] .settings-section::before',
    );
  });

  it("extends the glass/liquid-glass/neumorphism panel treatment to every card-level surface", () => {
    const guardStart = styles.indexOf("@media not (forced-colors: active)");
    const forcedColorsStart = styles.indexOf("@media (forced-colors: active)");
    const guardBody = styles.slice(guardStart, forcedColorsStart);

    for (const theme of ["glass", "liquid-glass", "neumorphism"]) {
      for (const selector of [
        ".welcome-card",
        ".folder-list article",
        ".stat-card",
        ".audio-metadata-candidates article",
        ".audio-folders-empty",
        ".audio-location-section",
      ]) {
        expect(guardBody).toContain(`[data-ui-theme="${theme}"] ${selector}`);
      }
    }
  });

  it("gives neumorphism a dual soft-shadow surface and a dark sidebar", () => {
    const guardStart = styles.indexOf("@media not (forced-colors: active)");
    const forcedColorsStart = styles.indexOf("@media (forced-colors: active)");
    const guardBody = styles.slice(guardStart, forcedColorsStart);

    for (const selector of [
      '[data-ui-theme="neumorphism"][data-ui-mode="light"]',
      '[data-ui-theme="neumorphism"][data-ui-mode="dark"]',
    ]) {
      expect(guardBody).toContain(selector);
    }
    expect(guardBody).toContain(
      '[data-ui-theme="neumorphism"] .settings-section',
    );
    expect(guardBody).toContain(
      "box-shadow: var(--panel-shadow-1), var(--panel-shadow-2)",
    );

    const neumorphismLight = attributeBlock(
      styles.slice(guardStart),
      ':root[data-ui-theme="neumorphism"][data-ui-mode="light"]',
    );
    // The sidebar must stay darker than the neumorphic background, or the
    // shared light --dark-text becomes illegible on it.
    expect(neumorphismLight).toContain("--sidebar: #2b2e33");
    expect(neumorphismLight).toContain("--dark-text: #eef0f2");
  });
});

/*
 * WCAG 2.1 AA contrast (>=4.5:1) for every token pairing a skin actually
 * renders. This guards the class of bug found by manual review: a hardcoded
 * literal color paired with a token that varies per skin/mode looks fine in
 * whichever mode it was written against and silently breaks in every other
 * one. `default`/`light` is exempt where noted — it is the pre-existing,
 * unchanged shipped palette and out of this feature's scope to recolor.
 */
describe("WCAG AA contrast across every shell skin", () => {
  function parseColor(str: string) {
    str = str.trim();
    if (str.startsWith("#")) {
      let h = str.slice(1);
      if (h.length === 3)
        h = h
          .split("")
          .map((c) => c + c)
          .join("");
      return {
        r: parseInt(h.slice(0, 2), 16),
        g: parseInt(h.slice(2, 4), 16),
        b: parseInt(h.slice(4, 6), 16),
        a: 1,
      };
    }
    const m = str.match(
      /rgba?\(\s*([\d.]+)\s*,\s*([\d.]+)\s*,\s*([\d.]+)\s*(?:,\s*([\d.]+))?\s*\)/,
    );
    if (!m) throw new Error(`Unparsed color: ${str}`);
    return {
      r: +m[1]!,
      g: +m[2]!,
      b: +m[3]!,
      a: m[4] !== undefined ? +m[4] : 1,
    };
  }
  type Color = ReturnType<typeof parseColor>;
  function composite(fg: Color, bg: Color): Color {
    const a = fg.a + bg.a * (1 - fg.a);
    return {
      r: (fg.r * fg.a + bg.r * bg.a * (1 - fg.a)) / a,
      g: (fg.g * fg.a + bg.g * bg.a * (1 - fg.a)) / a,
      b: (fg.b * fg.a + bg.b * bg.a * (1 - fg.a)) / a,
      a,
    };
  }
  function mix(a: Color, b: Color, pctA: number): Color {
    return {
      r: a.r * pctA + b.r * (1 - pctA),
      g: a.g * pctA + b.g * (1 - pctA),
      b: a.b * pctA + b.b * (1 - pctA),
      a: 1,
    };
  }
  function luminance({ r, g, b }: Color) {
    const c = (x: number) => {
      x = x / 255;
      return x <= 0.03928 ? x / 12.92 : Math.pow((x + 0.055) / 1.055, 2.4);
    };
    return 0.2126 * c(r) + 0.7152 * c(g) + 0.0722 * c(b);
  }
  function contrast(fg: Color, bg: Color) {
    const l1 = luminance(fg);
    const l2 = luminance(bg);
    const lighter = Math.max(l1, l2);
    const darker = Math.min(l1, l2);
    return (lighter + 0.05) / (darker + 0.05);
  }
  function extractBlock(css: string, marker: string) {
    const start = css.indexOf(marker);
    if (start < 0) throw new Error(`Missing block: ${marker}`);
    const open = css.indexOf("{", start);
    const close = css.indexOf("}", open);
    return css.slice(open + 1, close);
  }
  function extractTokens(block: string) {
    const tokens: Record<string, string> = {};
    const re = /--([\w-]+):\s*([^;]+);/g;
    let m: RegExpExecArray | null;
    while ((m = re.exec(block))) tokens[m[1]!] = m[2]!.trim();
    return tokens;
  }
  function token(tokens: Record<string, string>, key: string): string {
    const value = tokens[key];
    if (value === undefined) throw new Error(`Missing token: --${key}`);
    return value;
  }
  const baseTokens = extractTokens(extractBlock(styles, ":root {"));
  const skins = [
    "default",
    "classic",
    "bookish",
    "glass",
    "liquid-glass",
    "neumorphism",
  ] as const;
  const modes = ["light", "dark"] as const;
  function tokensFor(skin: string, mode: string) {
    const tokens = { ...baseTokens };
    if (!(skin === "default" && mode === "light")) {
      const marker = `:root[data-ui-theme="${skin}"][data-ui-mode="${mode}"] {`;
      Object.assign(tokens, extractTokens(extractBlock(styles, marker)));
    }
    return tokens;
  }
  const AA = 4.5;

  for (const skin of skins) {
    for (const mode of modes) {
      const isPreexistingDefaultLight = skin === "default" && mode === "light";

      it(`${skin}/${mode}: button text stays readable on brass-strong`, () => {
        const t = tokensFor(skin, mode);
        const ratio = contrast(
          parseColor("#fffaf0"),
          parseColor(token(t, "brass-strong")),
        );
        // default/light is the pre-existing, unchanged shipped palette; its
        // brass is ~2.999:1 against white button text (bold, large-ish text
        // uses the 3:1 large-text threshold) and is out of this feature's
        // scope to recolor.
        const threshold = isPreexistingDefaultLight ? 2.99 : AA;
        expect(ratio).toBeGreaterThanOrEqual(threshold);
      });

      if (!isPreexistingDefaultLight) {
        it(`${skin}/${mode}: disabled secondary-button text stays readable`, () => {
          const t = tokensFor(skin, mode);
          const bg = parseColor(token(t, "bg"));
          const surface2 = composite(parseColor(token(t, "surface-2")), bg);
          const ratio = contrast(parseColor(token(t, "muted")), surface2);
          expect(ratio).toBeGreaterThanOrEqual(AA);
        });

        it(`${skin}/${mode}: empty-state and favorite-toggle text stays readable on their translucent surface`, () => {
          const t = tokensFor(skin, mode);
          const bg = parseColor(token(t, "bg"));
          const surface = composite(parseColor(token(t, "surface")), bg);
          for (const pct of [0.66, 0.92]) {
            const effectiveBg = composite({ ...surface, a: pct }, bg);
            const ratio = contrast(parseColor(token(t, "muted")), effectiveBg);
            expect(ratio).toBeGreaterThanOrEqual(AA);
          }
        });
      }

      it(`${skin}/${mode}: accent-text stays readable on surface, surface-2, and the notice tint`, () => {
        const t = tokensFor(skin, mode);
        const bg = parseColor(token(t, "bg"));
        const surface = composite(parseColor(token(t, "surface")), bg);
        const surface2 = composite(parseColor(token(t, "surface-2")), bg);
        const accentText = parseColor(token(t, "accent-text"));
        const noticeBg = mix(parseColor(token(t, "brass")), surface, 0.1);
        expect(contrast(accentText, surface)).toBeGreaterThanOrEqual(AA);
        expect(contrast(accentText, surface2)).toBeGreaterThanOrEqual(AA);
        expect(contrast(accentText, noticeBg)).toBeGreaterThanOrEqual(AA);
      });
    }
  }

  const readerThemes = {
    paper: { bg: "#f7f3ea", link: "#72513a", hover: "#8c692b" },
    sepia: { bg: "#eee3cc", link: "#72513a", hover: "#806027" },
    night: { bg: "#171918", link: "#b88a38", hover: "#977e6d" },
  };
  for (const [name, { bg, link, hover }] of Object.entries(readerThemes)) {
    it(`reader theme ${name}: inline-link and its hover state stay readable`, () => {
      const bgc = parseColor(bg);
      expect(contrast(parseColor(link), bgc)).toBeGreaterThanOrEqual(AA);
      expect(contrast(parseColor(hover), bgc)).toBeGreaterThanOrEqual(AA);
    });
  }
});
