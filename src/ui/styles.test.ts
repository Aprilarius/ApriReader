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
