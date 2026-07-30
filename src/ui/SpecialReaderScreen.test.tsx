import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { SpecialDocument } from "../application/fixedReader";
import { translations, type TranslationKey } from "./i18n";
import { SpecialReaderScreen } from "./SpecialReaderScreen";

const t = (key: TranslationKey) => translations.en[key];

const comic: SpecialDocument = {
  bookId: 7,
  title: "A synthetic comic",
  author: "",
  format: "CBZ",
  kind: "comic",
  sourcePath: null,
  progress: 0,
  lastPage: 0,
  pages: [
    { index: 0, name: "1.png", path: "C:\\cache\\1.png", mime: "image/png" },
    { index: 1, name: "2.png", path: "C:\\cache\\2.png", mime: "image/png" },
    { index: 2, name: "3.png", path: "C:\\cache\\3.png", mime: "image/png" },
  ],
};

describe("SpecialReaderScreen", () => {
  it("places initial keyboard focus on the fixed reader toolbar", () => {
    render(
      <SpecialReaderScreen
        document={comic}
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );
    expect(
      screen.getByRole("button", { name: "Back to library" }),
    ).toHaveFocus();
  });

  it("tags book language and disables optional page announcements", () => {
    const { container } = render(
      <SpecialReaderScreen
        document={comic}
        t={t}
        language="en-US"
        screenReaderSupport={false}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );
    expect(container.querySelector(".fixed-reader")).toHaveAttribute(
      "lang",
      "en-US",
    );
    expect(container.querySelector('[role="status"]')).toHaveAttribute(
      "aria-live",
      "off",
    );
    expect(
      screen.getByRole("button", { name: "Back to library" }),
    ).toBeInTheDocument();
  });

  it("navigates comic pages and offers a two-page spread", () => {
    render(
      <SpecialReaderScreen
        document={comic}
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );
    expect(screen.getByAltText("Comic page 1")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Single page" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    fireEvent.click(screen.getByRole("button", { name: /Next/ }));
    expect(screen.getByAltText("Comic page 2")).toBeInTheDocument();
    const doublePage = screen.getByRole("button", {
      name: "Two-page spread",
    });
    expect(doublePage).toHaveAttribute("aria-pressed", "false");
    fireEvent.click(doublePage);
    expect(doublePage).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByAltText("Comic page 3")).toBeInTheDocument();
  });

  it("switches comic reading direction without leaving the viewer", () => {
    render(
      <SpecialReaderScreen
        document={comic}
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Left to right" }));
    expect(
      screen.getByRole("button", { name: "Right to left" }),
    ).toBeInTheDocument();
  });
});
