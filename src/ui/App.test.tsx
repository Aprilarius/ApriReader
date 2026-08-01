import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { emptyStatistics } from "../application/statistics";
import type { Book } from "../application/library";
import { App } from "./App";
import { greetingKeyForHour } from "./greeting";
import { localProfileKey } from "./useLocalProfile";

const invokeMock = vi.hoisted(() => vi.fn());
const listenMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (path: string) => path,
  invoke: invokeMock,
  isTauri: () => true,
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: listenMock,
}));

let mockBooks: Book[] = [];
let launchPaths: string[] = [];

function bookFixture(overrides: Partial<Book>): Book {
  return {
    id: 1,
    sourcePath: "C:\\Books\\Fixture.txt",
    title: "Fixture",
    author: "Fixture Author",
    format: "TXT",
    fileSize: 128,
    coverPath: null,
    addedAt: "2026-07-29",
    isAvailable: true,
    progress: 0,
    subtitle: "",
    isbn: "",
    publisher: "",
    publishedYear: "",
    language: "",
    series: "",
    genres: "",
    description: "",
    metadataSource: "embedded",
    metadataProviderId: null,
    metadataUpdatedAt: null,
    coverSource: "embedded",
    lastOpenedAt: null,
    isFavorite: false,
    ...overrides,
  };
}

describe("App", () => {
  beforeEach(() => {
    localStorage.clear();
    localStorage.setItem(
      localProfileKey,
      JSON.stringify({ onboardingComplete: true, displayName: "" }),
    );
    mockBooks = [];
    launchPaths = [];
    listenMock.mockResolvedValue(vi.fn());
    invokeMock.mockImplementation((command: string, invokeArgs?: unknown) => {
      if (command === "list_books") return Promise.resolve(mockBooks);
      if (command === "take_launch_book_paths") {
        const paths = launchPaths;
        launchPaths = [];
        return Promise.resolve(paths);
      }
      if (command === "list_watched_folders") return Promise.resolve([]);
      if (command === "get_statistics") return Promise.resolve(emptyStatistics);
      if (command === "get_startup_health")
        return Promise.resolve({
          previousExitUnclean: false,
          recoveredFromBackup: false,
          quarantinedDatabase: null,
        });
      if (command === "set_book_favorite") {
        const args = invokeArgs as
          | { bookId: number; favorite: boolean }
          | undefined;
        const current = mockBooks.find((book) => book.id === args?.bookId);
        if (!current) return Promise.reject(new Error("Book not found"));
        const updated = { ...current, isFavorite: Boolean(args?.favorite) };
        mockBooks = mockBooks.map((book) =>
          book.id === updated.id ? updated : book,
        );
        return Promise.resolve(updated);
      }
      if (command === "remove_books") {
        const args = invokeArgs as { bookIds: number[] } | undefined;
        const ids = new Set(args?.bookIds ?? []);
        const previousLength = mockBooks.length;
        mockBooks = mockBooks.filter((book) => !ids.has(book.id));
        return Promise.resolve(previousLength - mockBooks.length);
      }
      return Promise.reject(new Error(`Unavailable in unit test: ${command}`));
    });
  });

  it("shows a truthful empty library", async () => {
    render(<App />);
    expect(
      await screen.findByRole("heading", {
        name: "Ваша библиотека пока пуста",
      }),
    ).toBeInTheDocument();
  });

  it("uses a greeting that matches the local time of day", () => {
    expect(greetingKeyForHour(6)).toBe("greetingMorning");
    expect(greetingKeyForHour(12)).toBe("greetingAfternoon");
    expect(greetingKeyForHour(18)).toBe("greetingEvening");
  });

  it("creates an optional local profile on first launch", async () => {
    localStorage.setItem("aprireader.locale", "en");
    localStorage.removeItem(localProfileKey);
    render(<App />);

    expect(
      screen.getByRole("heading", { name: "What should we call you?" }),
    ).toBeInTheDocument();
    const continueButton = screen.getByRole("button", { name: "Continue" });
    expect(continueButton).toBeDisabled();

    fireEvent.change(screen.getByLabelText("Your name"), {
      target: { value: "  Bahadur   Ali  " },
    });
    fireEvent.click(continueButton);

    expect(
      await screen.findByRole("heading", {
        name: /Good (morning|afternoon|evening), Bahadur Ali!/,
      }),
    ).toBeInTheDocument();
    expect(JSON.parse(localStorage.getItem(localProfileKey) ?? "")).toEqual({
      onboardingComplete: true,
      displayName: "Bahadur Ali",
    });
  });

  it("allows first-launch onboarding to be skipped", async () => {
    localStorage.setItem("aprireader.locale", "en");
    localStorage.removeItem(localProfileKey);
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Skip" }));
    expect(await screen.findByRole("searchbox")).toBeInTheDocument();
    expect(JSON.parse(localStorage.getItem(localProfileKey) ?? "")).toEqual({
      onboardingComplete: true,
      displayName: "",
    });
  });

  it("changes and removes the local greeting name in Settings", () => {
    localStorage.setItem("aprireader.locale", "en");
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));

    fireEvent.change(screen.getByLabelText("Your name"), {
      target: { value: "Alex" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(screen.getByRole("status")).toHaveTextContent(
      "Local profile saved.",
    );

    fireEvent.click(screen.getByRole("button", { name: "Library" }));
    expect(
      screen.getByRole("heading", {
        name: /Good (morning|afternoon|evening), Alex!/,
      }),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getByRole("button", { name: "Remove name" }));
    fireEvent.click(screen.getByRole("button", { name: "Library" }));
    expect(
      screen.getByRole("heading", {
        name: /Good (morning|afternoon|evening)!/,
      }),
    ).toBeInTheDocument();
  });

  it("keeps development labels out of the shell and hides library search elsewhere", async () => {
    localStorage.setItem("aprireader.locale", "en");
    render(<App />);
    expect(
      screen.queryByText(/release quality|stage 9/i),
    ).not.toBeInTheDocument();
    expect(await screen.findByRole("searchbox")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    expect(screen.queryByRole("searchbox")).not.toBeInTheDocument();
  });

  it("shows a truthful empty state when a library search has no matches", async () => {
    localStorage.setItem("aprireader.locale", "en");
    mockBooks = [bookFixture({ title: "A visible book" })];
    render(<App />);
    fireEvent.change(await screen.findByRole("searchbox"), {
      target: { value: "missing title" },
    });
    expect(
      await screen.findByRole("heading", { name: "No books found" }),
    ).toBeInTheDocument();
    expect(screen.queryByText("Your library is empty")).not.toBeInTheDocument();
  });

  it("imports and opens a book passed by a Windows file association", async () => {
    localStorage.setItem("aprireader.locale", "en");
    const launchedBook = bookFixture({
      id: 91,
      sourcePath: "C:\\Books\\Opened from Explorer.txt",
      title: "Opened from Explorer",
    });
    launchPaths = [launchedBook.sourcePath];
    invokeMock.mockImplementation((command: string) => {
      if (command === "list_books") return Promise.resolve(mockBooks);
      if (command === "list_watched_folders") return Promise.resolve([]);
      if (command === "get_statistics") return Promise.resolve(emptyStatistics);
      if (command === "get_startup_health")
        return Promise.resolve({
          previousExitUnclean: false,
          recoveredFromBackup: false,
          quarantinedDatabase: null,
        });
      if (command === "take_launch_book_paths") {
        const paths = launchPaths;
        launchPaths = [];
        return Promise.resolve(paths);
      }
      if (command === "open_book_path") {
        mockBooks = [launchedBook];
        return Promise.resolve(launchedBook);
      }
      if (command === "load_document") {
        return Promise.resolve({
          bookId: launchedBook.id,
          title: launchedBook.title,
          author: launchedBook.author,
          format: launchedBook.format,
          progress: 0,
          lastSection: 0,
          sectionProgress: 0,
          sections: [
            {
              id: "main",
              title: "Opened from Explorer",
              blocks: [{ kind: "paragraph", text: "Associated file content" }],
            },
          ],
        });
      }
      return Promise.reject(new Error(`Unavailable in unit test: ${command}`));
    });

    render(<App />);

    expect(
      (await screen.findAllByText("Associated file content")).length,
    ).toBeGreaterThan(0);
    expect(invokeMock).toHaveBeenCalledWith("open_book_path", {
      path: launchedBook.sourcePath,
    });
    expect(invokeMock).toHaveBeenCalledWith("load_document", {
      bookId: launchedBook.id,
    });
  });

  it("keeps the newest reader request when two books are opened quickly", async () => {
    localStorage.setItem("aprireader.locale", "en");
    const first = bookFixture({ id: 1, title: "First book" });
    const second = bookFixture({ id: 2, title: "Second book" });
    mockBooks = [first, second];
    let resolveFirst!: (document: unknown) => void;
    const firstLoad = new Promise((resolve) => {
      resolveFirst = resolve;
    });
    const fallback = invokeMock.getMockImplementation() as (
      command: string,
      invokeArgs?: unknown,
    ) => Promise<unknown>;
    invokeMock.mockImplementation((command: string, invokeArgs?: unknown) => {
      if (command !== "load_document") return fallback(command, invokeArgs);
      const bookId = (invokeArgs as { bookId: number }).bookId;
      if (bookId === first.id) return firstLoad;
      return Promise.resolve({
        bookId: second.id,
        title: second.title,
        author: second.author,
        format: second.format,
        progress: 0,
        lastSection: 0,
        sectionProgress: 0,
        sections: [
          {
            id: "second",
            title: second.title,
            blocks: [{ kind: "paragraph", text: "Second content" }],
          },
        ],
      });
    });

    render(<App />);
    fireEvent.doubleClick(
      await screen.findByRole("button", { name: /First book —/ }),
    );
    fireEvent.doubleClick(
      screen.getByRole("button", { name: /Second book —/ }),
    );
    expect(
      (await screen.findAllByText("Second content")).length,
    ).toBeGreaterThan(0);

    await act(async () => {
      resolveFirst({
        bookId: first.id,
        title: first.title,
        author: first.author,
        format: first.format,
        progress: 0,
        lastSection: 0,
        sectionProgress: 0,
        sections: [
          {
            id: "first",
            title: first.title,
            blocks: [{ kind: "paragraph", text: "First content" }],
          },
        ],
      });
      await firstLoad;
    });
    expect(screen.queryByText("First content")).not.toBeInTheDocument();
    expect(screen.getAllByText("Second content").length).toBeGreaterThan(0);
  });

  it("shows watched folders as a real collection source", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Коллекции" }));
    expect(
      screen.getByRole("heading", { name: "Коллекции", level: 1 }),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Наблюдаемые папки ещё не добавлены."),
    ).toBeInTheDocument();
  });

  it("shows a truthful empty reading-now section", async () => {
    localStorage.setItem("aprireader.locale", "en");
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Reading Now" }));
    expect(
      await screen.findByRole("heading", {
        name: "No books in progress yet",
      }),
    ).toBeInTheDocument();
  });

  it("orders unfinished opened books by recency", async () => {
    localStorage.setItem("aprireader.locale", "en");
    mockBooks = [
      bookFixture({
        id: 1,
        title: "Older book",
        progress: 0.25,
        lastOpenedAt: 100,
      }),
      bookFixture({
        id: 2,
        title: "Finished book",
        progress: 1,
        lastOpenedAt: 300,
      }),
      bookFixture({
        id: 3,
        title: "Never opened",
        progress: 0,
        lastOpenedAt: null,
      }),
      bookFixture({
        id: 4,
        title: "Newest book",
        progress: 0.6,
        lastOpenedAt: 200,
      }),
      bookFixture({
        id: 5,
        title: "Missing book",
        isAvailable: false,
        progress: 0.4,
        lastOpenedAt: 150,
      }),
    ];
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Reading Now" }));
    const continueButtons = await screen.findAllByRole("button", {
      name: /Continue reading —/,
    });
    expect(
      continueButtons.map((button) => button.getAttribute("aria-label")),
    ).toEqual([
      "Continue reading — Newest book",
      "Continue reading — Missing book",
      "Continue reading — Older book",
    ]);
    expect(
      screen.getByRole("button", {
        name: "Continue reading — Missing book",
      }),
    ).toBeDisabled();
    expect(screen.queryByText("Finished book")).not.toBeInTheDocument();
    expect(screen.queryByText("Never opened")).not.toBeInTheDocument();
  });

  it("adds and removes a book through the favorites view", async () => {
    localStorage.setItem("aprireader.locale", "en");
    mockBooks = [bookFixture({ id: 7, title: "Saved locally" })];
    render(<App />);
    await screen.findByRole("button", { name: /Saved locally —/ });

    fireEvent.click(screen.getByRole("button", { name: "Add to favorites" }));
    expect(
      await screen.findByRole("button", { name: "Remove from favorites" }),
    ).toHaveAttribute("aria-pressed", "true");

    fireEvent.click(screen.getByRole("button", { name: "Favorites" }));
    expect(
      await screen.findByRole("button", { name: /Saved locally —/ }),
    ).toBeInTheDocument();
    fireEvent.click(
      screen.getByRole("button", { name: "Remove from favorites" }),
    );
    expect(
      await screen.findByRole("heading", { name: "No favorite books yet" }),
    ).toBeInTheDocument();
  });

  it("removes one book from details while preserving the source-file policy", async () => {
    localStorage.setItem("aprireader.locale", "en");
    mockBooks = [bookFixture({ id: 31, title: "Remove one" })];
    const confirm = vi.spyOn(window, "confirm").mockReturnValue(true);
    render(<App />);

    fireEvent.click(await screen.findByRole("button", { name: /Remove one/ }));
    fireEvent.click(
      screen.getByRole("button", { name: "Remove from library" }),
    );

    expect(
      await screen.findByRole("heading", { name: "Your library is empty" }),
    ).toBeInTheDocument();
    expect(confirm).toHaveBeenCalledWith(
      "Remove “Remove one” from the library? The source file will remain on disk.",
    );
    expect(invokeMock).toHaveBeenCalledWith("remove_books", {
      bookIds: [31],
    });
    confirm.mockRestore();
  });

  it("selects and removes several books as one batch", async () => {
    localStorage.setItem("aprireader.locale", "en");
    mockBooks = [
      bookFixture({ id: 41, title: "Keep me" }),
      bookFixture({ id: 42, title: "Batch one" }),
      bookFixture({ id: 43, title: "Batch two" }),
    ];
    const confirm = vi.spyOn(window, "confirm").mockReturnValue(true);
    render(<App />);

    fireEvent.click(
      await screen.findByRole("button", { name: "Select books" }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Select “Batch one”" }));
    fireEvent.click(screen.getByRole("button", { name: "Select “Batch two”" }));
    expect(screen.getByText("Selected: 2")).toBeInTheDocument();
    fireEvent.click(
      screen.getByRole("button", { name: "Remove from library" }),
    );

    expect(
      await screen.findByRole("button", { name: /Keep me/ }),
    ).toBeInTheDocument();
    expect(screen.queryByText("Batch one")).not.toBeInTheDocument();
    expect(screen.queryByText("Batch two")).not.toBeInTheDocument();
    expect(confirm).toHaveBeenCalledWith(
      "Remove the selected books (2) from the library? The source files will remain on disk.",
    );
    expect(invokeMock).toHaveBeenCalledWith("remove_books", {
      bookIds: [42, 43],
    });
    confirm.mockRestore();
  });

  it("groups local metadata by author and opens an author drill-down", async () => {
    localStorage.setItem("aprireader.locale", "en");
    mockBooks = [
      bookFixture({
        id: 11,
        title: "Kindred",
        author: "Octavia Butler",
      }),
      bookFixture({
        id: 12,
        title: "Parable of the Sower",
        author: "octavia butler",
      }),
      bookFixture({
        id: 13,
        title: "A Wizard of Earthsea",
        author: "Ursula Le Guin",
      }),
      bookFixture({
        id: 14,
        title: "Anonymous fixture",
        author: "",
      }),
    ];
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Authors" }));

    const butler = await screen.findByRole("button", {
      name: "Octavia Butler, 2 books",
    });
    expect(
      screen.getByRole("button", { name: "Ursula Le Guin, 1 book" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Unknown author, 1 book" }),
    ).toBeInTheDocument();

    fireEvent.click(butler);
    expect(
      screen.getByRole("heading", { name: "Octavia Butler", level: 2 }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /Kindred —/ }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /Parable of the Sower —/ }),
    ).toBeInTheDocument();
    expect(screen.queryByText("A Wizard of Earthsea")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "All authors" }));
    expect(
      screen.getByRole("button", { name: "Octavia Butler, 2 books" }),
    ).toBeInTheDocument();
  });

  it("groups local series metadata and orders a series drill-down by title", async () => {
    localStorage.setItem("aprireader.locale", "en");
    mockBooks = [
      bookFixture({
        id: 21,
        title: "Volume 10",
        author: "A. Writer",
        series: "The Archive",
      }),
      bookFixture({
        id: 22,
        title: "Volume 2",
        author: "A. Writer",
        series: "  the   archive ",
      }),
      bookFixture({
        id: 23,
        title: "Another Story",
        series: "Earth Cycle",
      }),
      bookFixture({
        id: 24,
        title: "Standalone",
        series: "",
      }),
    ];
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Series" }));

    const archive = await screen.findByRole("button", {
      name: "The Archive, 2 books",
    });
    expect(
      screen.getByRole("button", { name: "Earth Cycle, 1 book" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "No series, 1 book" }),
    ).toBeInTheDocument();

    fireEvent.click(archive);
    expect(
      screen.getByRole("heading", { name: "The Archive", level: 2 }),
    ).toBeInTheDocument();
    const seriesBooks = screen.getAllByRole("button", {
      name: /Volume \d+ —/,
    });
    expect(
      seriesBooks.map((button) => button.getAttribute("aria-label")),
    ).toEqual([
      expect.stringMatching(/^Volume 2 —/),
      expect.stringMatching(/^Volume 10 —/),
    ]);
    expect(screen.queryByText("Another Story")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "All series" }));
    expect(
      screen.getByRole("button", { name: "The Archive, 2 books" }),
    ).toBeInTheDocument();
  });

  it("persists the selected locale", async () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Switch to English" }));
    expect(
      await screen.findByRole("heading", { name: "Your library is empty" }),
    ).toBeInTheDocument();
    expect(localStorage.getItem("aprireader.locale")).toBe("en");
  });

  it("persists the screen reader support preference", async () => {
    localStorage.setItem("aprireader.locale", "en");
    const { unmount } = render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    const toggle = await screen.findByRole("checkbox", {
      name: /Announce reading changes/,
    });
    expect(toggle).toBeChecked();
    expect(
      screen.queryByRole("heading", { name: "Steam achievements" }),
    ).not.toBeInTheDocument();

    fireEvent.click(toggle);
    expect(toggle).not.toBeChecked();
    expect(localStorage.getItem("aprireader.screenReaderSupport")).toBe(
      "false",
    );

    unmount();
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    expect(
      await screen.findByRole("checkbox", {
        name: /Announce reading changes/,
      }),
    ).not.toBeChecked();
  });

  it("keeps keyboard navigation available and exposes a skip link", () => {
    render(<App />);
    expect(
      screen.getByRole("link", { name: "Перейти к содержимому" }),
    ).toHaveAttribute("href", "#main-content");
    expect(
      screen.getByRole("button", { name: "Настройки" }),
    ).toBeInTheDocument();
    const contextMenu = new MouseEvent("contextmenu", {
      bubbles: true,
      cancelable: true,
    });
    document.dispatchEvent(contextMenu);
    expect(contextMenu.defaultPrevented).toBe(true);
  });

  it("keeps compact navigation names independent from visible labels", () => {
    localStorage.setItem("aprireader.locale", "en");
    render(<App />);
    const navigation = screen.getByRole("navigation", { name: "Library" });
    for (const label of [
      "Library",
      "Reading Now",
      "Collections",
      "Authors",
      "Series",
      "Favorites",
      "Achievements",
      "Statistics",
      "Settings",
    ]) {
      expect(
        within(navigation).getByRole("button", { name: label }),
      ).toHaveAttribute("aria-label", label);
    }
    expect(
      screen.getByRole("button", { name: "Переключить на русский" }),
    ).toBeInTheDocument();
  });

  it("renders a large library in bounded batches", async () => {
    mockBooks = Array.from({ length: 250 }, (_, index) =>
      bookFixture({
        id: index + 1,
        sourcePath: `C:\\Books\\Book ${index + 1}.txt`,
        title: `Book ${String(index + 1).padStart(3, "0")}`,
      }),
    );
    render(<App />);
    await screen.findByRole("button", { name: /Book 001/ });
    expect(screen.getAllByRole("button", { name: /Book \d{3}/ })).toHaveLength(
      120,
    );
    fireEvent.click(screen.getByRole("button", { name: "Показать ещё" }));
    expect(screen.getAllByRole("button", { name: /Book \d{3}/ })).toHaveLength(
      240,
    );
  });
});
