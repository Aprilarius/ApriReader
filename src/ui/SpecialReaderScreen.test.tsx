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
    fireEvent.click(screen.getByRole("button", { name: /Next/ }));
    expect(screen.getByAltText("Comic page 2")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Two-page spread" }));
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
