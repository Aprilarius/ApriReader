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

const { openUrl } = vi.hoisted(() => ({
  openUrl: vi.fn<(url: string) => Promise<void>>().mockResolvedValue(undefined),
}));

vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl }));

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
  beforeEach(() => {
    localStorage.clear();
    openUrl.mockReset();
  });

  it("asks for consent once before opening a selected phrase in a translator", async () => {
    render(
      <ReaderScreen
        document={document}
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );
    const paragraph = within(screen.getByRole("main")).getByText(
      "Only safe text is rendered.",
    );
    const range = window.document.createRange();
    range.setStart(paragraph.firstChild!, 0);
    range.setEnd(paragraph.firstChild!, 9);
    const selected = window.getSelection()!;
    selected.removeAllRanges();
    selected.addRange(range);
    fireEvent.mouseUp(paragraph);

    fireEvent.click(screen.getByRole("button", { name: "Translate" }));
    fireEvent.click(screen.getByRole("menuitem", { name: "Google Translate" }));
    expect(
      screen.getByRole("group", { name: "Open an external translator?" }),
    ).toHaveTextContent("The selected text will be sent to Google Translate");
    expect(openUrl).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "Continue" }));
    await waitFor(() => expect(openUrl).toHaveBeenCalledOnce());
    const opened = new URL(openUrl.mock.calls[0]![0]);
    expect(opened.origin).toBe("https://translate.google.com");
    expect(opened.searchParams.get("text")).toBe("Only safe");
    expect(opened.searchParams.get("sl")).toBe("en");
    expect(opened.searchParams.get("tl")).toBe("ru");
    expect(
      localStorage.getItem("aprireader.external-translation-consent.v1"),
    ).toBe("accepted");
  });

  it("keeps a quote saved when clipboard access is unavailable", async () => {
    render(
      <ReaderScreen
        document={document}
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );
    const paragraph = within(screen.getByRole("main")).getByText(
      "Only safe text is rendered.",
    );
    const range = window.document.createRange();
    range.setStart(paragraph.firstChild!, 0);
    range.setEnd(paragraph.firstChild!, 9);
    const selected = window.getSelection()!;
    selected.removeAllRanges();
    selected.addRange(range);
    fireEvent.mouseUp(paragraph);

    fireEvent.click(screen.getByRole("button", { name: "Copy quote" }));

    expect(
      await screen.findByText(
        "Quote saved, but clipboard access was unavailable",
      ),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Annotations" }));
    expect(await screen.findAllByText("Only safe")).toHaveLength(2);
  });

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

  it("tags book language and respects optional reader announcements", () => {
    const { container } = render(
      <ReaderScreen
        document={document}
        t={t}
        language="en-US"
        screenReaderSupport={false}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );
    expect(container.querySelector(".reader-screen")).toHaveAttribute(
      "lang",
      "en-US",
    );
    expect(container.querySelector(".reader-page-status")).toHaveAttribute(
      "aria-live",
      "off",
    );
    expect(screen.queryByText("Chapter: Opening")).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Back to library" }),
    ).toBeInTheDocument();
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
      target: { value: "literata" },
    });
    fireEvent.change(screen.getByLabelText("Font style"), {
      target: { value: "italic" },
    });
    fireEvent.change(screen.getByLabelText("Font weight"), {
      target: { value: "800" },
    });
    fireEvent.click(
      screen.getByRole("checkbox", { name: /Bionic highlighting/ }),
    );
    const spread = screen.getByRole("button", { name: /Book spread/ });
    expect(spread).toHaveAttribute("aria-pressed", "false");
    fireEvent.click(spread);
    expect(spread).toHaveAttribute("aria-pressed", "true");
    expect(localStorage.getItem("aprireader.reader.preferences")).toContain(
      '"fontChoice":"literata"',
    );
    expect(localStorage.getItem("aprireader.reader.preferences")).toContain(
      '"fontStyle":"italic"',
    );
    expect(localStorage.getItem("aprireader.reader.preferences")).toContain(
      '"fontWeight":800',
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

  it("offers bundled families and only real weights for the selected font", () => {
    render(
      <ReaderScreen
        document={document}
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Text settings" }));
    const family = screen.getByLabelText("Reading font");
    for (const name of [
      "Literata",
      "Lora",
      "Merriweather",
      "Source Serif 4",
      "Charis SIL",
      "IBM Plex Serif",
    ]) {
      expect(within(family).getByRole("option", { name })).toBeInTheDocument();
    }

    fireEvent.change(family, { target: { value: "lora" } });
    const weight = screen.getByLabelText("Font weight");
    expect(within(weight).getAllByRole("option")).toHaveLength(4);
    expect(
      within(weight).queryByRole("option", { name: "Black" }),
    ).not.toBeInTheDocument();
    expect(within(weight).getByRole("option", { name: "Bold" })).toHaveValue(
      "700",
    );
    expect(
      screen.getByText(/The quick brown fox jumps over the lazy dog/),
    ).toHaveClass("reader-font-preview");
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
