import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { DocumentModel } from "../application/reader";
import { translations, type TranslationKey } from "./i18n";
import { ReaderScreen } from "./ReaderScreen";

const document: DocumentModel = {
  bookId: 1,
  title: "A quiet fixture",
  author: "Test Author",
  format: "EPUB",
  progress: 0.25,
  lastSection: 0,
  sectionProgress: 0.5,
  sections: [
    {
      id: "opening",
      title: "Opening",
      blocks: [
        { kind: "paragraph", text: "Only safe text is rendered." },
        { kind: "quote", text: "A quoted thought." },
      ],
    },
    {
      id: "ending",
      title: "Ending",
      blocks: [{ kind: "paragraph", text: "The end." }],
    },
  ],
};

const t = (key: TranslationKey) => translations.en[key];

describe("ReaderScreen", () => {
  beforeEach(() => localStorage.clear());

  it("places initial keyboard focus on the reader toolbar", () => {
    render(
      <ReaderScreen
        document={document}
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );
    expect(
      screen.getByRole("button", { name: "Back to library" }),
    ).toHaveFocus();
  });

  it("renders normalized book text and navigates through the table of contents", () => {
    render(
      <ReaderScreen
        document={document}
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );
    expect(
      within(screen.getByRole("main")).getByText("Only safe text is rendered."),
    ).toBeInTheDocument();
    expect(screen.getByText("Page 1 of 2")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Table of contents" }));
    fireEvent.click(screen.getByRole("button", { name: /02 Ending/ }));
    expect(screen.getByText("The end.")).toBeInTheDocument();
  });

  it("persists typography choices locally", () => {
    render(
      <ReaderScreen
        document={document}
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Text settings" }));
    const size = screen.getByRole("slider", { name: /Font size/ });
    fireEvent.change(size, { target: { value: "24" } });
    expect(localStorage.getItem("aprireader.reader.preferences")).toContain(
      '"fontSize":24',
    );
    fireEvent.change(screen.getByLabelText("Reading font"), {
      target: { value: "clear" },
    });
    fireEvent.click(
      screen.getByRole("checkbox", { name: /Bionic highlighting/ }),
    );
    fireEvent.click(screen.getByRole("button", { name: /Book spread/ }));
    expect(localStorage.getItem("aprireader.reader.preferences")).toContain(
      '"fontChoice":"clear"',
    );
    expect(localStorage.getItem("aprireader.reader.preferences")).toContain(
      '"bionicReading":true',
    );
    expect(localStorage.getItem("aprireader.reader.preferences")).toContain(
      '"layout":"spread"',
    );
    expect(screen.getByRole("main")).toHaveClass("layout-spread");
    expect(
      globalThis.document.querySelector(".reader-document-spread"),
    ).toBeInTheDocument();
    expect(screen.getByText("Pages 1–2 of 4")).toBeInTheDocument();
  });

  it("renders optional focus highlighting without changing the text", () => {
    localStorage.setItem(
      "aprireader.reader.preferences",
      JSON.stringify({ bionicReading: true }),
    );
    render(
      <ReaderScreen
        document={document}
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );
    expect(
      globalThis.document.querySelector('[data-reader-block="0"]'),
    ).toHaveTextContent("Only safe text is rendered.");
    expect(
      globalThis.document.querySelector(".bionic-word strong"),
    ).toHaveTextContent("On");
  });

  it("turns a wheel gesture at the chapter edge into the next chapter", () => {
    render(
      <ReaderScreen
        document={document}
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );
    const reader = screen.getByRole("main");
    Object.defineProperties(reader, {
      clientHeight: { configurable: true, value: 600 },
      scrollHeight: { configurable: true, value: 1200 },
      scrollTop: { configurable: true, value: 600, writable: true },
    });
    fireEvent.wheel(reader, { deltaY: 120 });
    expect(screen.getByText("The end.")).toBeInTheDocument();
  });

  it("turns a wheel gesture into one horizontal spread", () => {
    localStorage.setItem(
      "aprireader.reader.preferences",
      JSON.stringify({ layout: "spread" }),
    );
    render(
      <ReaderScreen
        document={document}
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );
    const reader = screen.getByRole("main");
    const scrollTo = vi.fn();
    Object.defineProperties(reader, {
      clientWidth: { configurable: true, value: 800 },
      scrollWidth: { configurable: true, value: 2400 },
      scrollLeft: { configurable: true, value: 0, writable: true },
      scrollTo: { configurable: true, value: scrollTo },
    });
    fireEvent.wheel(reader, { deltaY: 120 });
    expect(scrollTo).toHaveBeenCalledWith({
      left: 800,
      behavior: "smooth",
    });
  });

  it("creates a bookmark and exposes it in the annotations panel", async () => {
    render(
      <ReaderScreen
        document={document}
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Bookmark" }));
    fireEvent.click(screen.getByRole("button", { name: "Annotations" }));
    await waitFor(() =>
      expect(screen.getByText("opening · 1")).toBeInTheDocument(),
    );
    expect(
      screen.getByRole("button", { name: "Export as Markdown" }),
    ).toBeEnabled();
  });

  it("opens the full-text search panel without leaving the reader", () => {
    render(
      <ReaderScreen
        document={document}
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Search this book" }));
    expect(
      screen.getByRole("searchbox", { name: "Search this book" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Find" })).toBeInTheDocument();
  });
});
