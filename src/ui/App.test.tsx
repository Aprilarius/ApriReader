import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { emptyStatistics } from "../application/statistics";
import type { Book } from "../application/library";
import { App } from "./App";

const invokeMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (path: string) => path,
  invoke: invokeMock,
  isTauri: () => true,
}));

let mockBooks: Book[] = [];

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
    mockBooks = [];
    invokeMock.mockImplementation((command: string, invokeArgs?: unknown) => {
      if (command === "list_books") return Promise.resolve(mockBooks);
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
