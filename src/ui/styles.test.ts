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
