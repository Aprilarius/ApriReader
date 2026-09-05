import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { SpecialDocument } from "../application/fixedReader";
import { translations, type TranslationKey } from "./i18n";
import { SpecialReaderScreen } from "./SpecialReaderScreen";

vi.mock("pdfjs-dist/legacy/build/pdf.mjs", () => ({
  GlobalWorkerOptions: {},
  getDocument: () => ({
    promise: Promise.resolve({
      numPages: 1,
      getPage: () => Promise.reject(new Error("no canvas in tests")),
    }),
    destroy: () => Promise.resolve(),
  }),
}));
vi.mock("pdfjs-dist/legacy/build/pdf.worker.min.mjs?url", () => ({
  default: "pdf.worker.mjs",
}));

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

const pdf: SpecialDocument = {
  bookId: 9,
  title: "A synthetic PDF",
  author: "",
  format: "PDF",
  kind: "pdf",
  sourcePath: "C:\\cache\\readers\\deadbeef.pdf",
  progress: 0,
  lastPage: 0,
  pages: [],
};

describe("SpecialReaderScreen", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("loads the PDF through the scoped asset protocol, never a raw filesystem read", async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      arrayBuffer: () => Promise.resolve(new ArrayBuffer(4)),
    });
    vi.stubGlobal("fetch", fetchMock);
    vi.stubGlobal(
      "ResizeObserver",
      class {
        observe() {}
        unobserve() {}
        disconnect() {}
      },
    );

    render(
      <SpecialReaderScreen
        document={pdf}
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );

    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(1));
    // Outside a real Tauri webview, localAssetUrl() passes the path through
    // unchanged, so the fetched target is exactly the cached PDF's own path
    // rather than any plugin-fs read or a different, unscoped location.
    expect(fetchMock).toHaveBeenCalledWith(pdf.sourcePath);
  });

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

  it("uses a horizontal touch swipe to change comic pages", () => {
    const { container } = render(
      <SpecialReaderScreen
        document={comic}
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );
    const stage = container.querySelector(".comic-stage");
    expect(stage).not.toBeNull();
    fireEvent.touchStart(stage!, {
      touches: [{ identifier: 1, clientX: 260, clientY: 120 }],
    });
    fireEvent.touchEnd(stage!, {
      changedTouches: [{ identifier: 1, clientX: 120, clientY: 130 }],
    });
    expect(screen.getByAltText("Comic page 2")).toBeInTheDocument();
  });

  it("zooms a comic with a two-finger pinch", () => {
    const { container } = render(
      <SpecialReaderScreen
        document={comic}
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );
    const stage = container.querySelector(".comic-stage");
    expect(stage).not.toBeNull();
    fireEvent.touchStart(stage!, {
      touches: [
        { identifier: 1, clientX: 100, clientY: 100 },
        { identifier: 2, clientX: 200, clientY: 100 },
      ],
    });
    fireEvent.touchMove(stage!, {
      touches: [
        { identifier: 1, clientX: 50, clientY: 100 },
        { identifier: 2, clientX: 250, clientY: 100 },
      ],
    });
    expect(screen.getByText("200%")).toBeInTheDocument();
    expect(stage).toHaveClass("zoomed");
  });
});
