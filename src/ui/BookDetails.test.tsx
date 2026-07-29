import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { Book } from "../application/library";
import {
  applyMetadataCandidate,
  searchMetadata,
  updateBookMetadata,
} from "../application/metadata";
import { BookDetails } from "./App";
import { translations, type TranslationKey } from "./i18n";

vi.mock("../application/metadata", async (importOriginal) => {
  const actual =
    await importOriginal<typeof import("../application/metadata")>();
  return {
    ...actual,
    updateBookMetadata: vi.fn(),
    searchMetadata: vi.fn(),
    applyMetadataCandidate: vi.fn(),
    removeExternalCover: vi.fn(),
  };
});

const t = (key: TranslationKey) => translations.en[key];
const book: Book = {
  id: 4,
  sourcePath: "C:\\Books\\quiet.txt",
  title: "Quiet Book",
  author: "Local Author",
  format: "TXT",
  fileSize: 100,
  coverPath: null,
  addedAt: "2026-07-28",
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
};

describe("BookDetails", () => {
  it("edits metadata locally", async () => {
    const updated = {
      ...book,
      title: "Corrected title",
      metadataSource: "manual",
    };
    vi.mocked(updateBookMetadata).mockResolvedValue(updated);
    const onUpdated = vi.fn();
    render(
      <BookDetails
        book={book}
        t={t}
        busy={false}
        onRead={vi.fn()}
        onFavorite={vi.fn()}
        onRemove={vi.fn()}
        onUpdated={onUpdated}
        onClose={vi.fn()}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Edit metadata" }));
    fireEvent.change(screen.getByLabelText("Title"), {
      target: { value: "Corrected title" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() =>
      expect(updateBookMetadata).toHaveBeenCalledWith(
        4,
        expect.objectContaining({ title: "Corrected title" }),
      ),
    );
    expect(onUpdated).toHaveBeenCalledWith(updated);
  });

  it("does not search until the user submits and then compares candidates", async () => {
    vi.mocked(searchMetadata).mockResolvedValue([
      {
        provider: "Open Library",
        providerId: "/works/OL1W",
        title: "Quiet Book",
        author: "Catalog Author",
        isbn: "9780000000001",
        publisher: "Catalog Press",
        publishedYear: "2020",
        language: "eng",
        genres: "Science fiction, Adventure",
        coverId: 42,
      },
    ]);
    render(
      <BookDetails
        book={book}
        t={t}
        busy={false}
        onRead={vi.fn()}
        onFavorite={vi.fn()}
        onRemove={vi.fn()}
        onUpdated={vi.fn()}
        onClose={vi.fn()}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Find metadata" }));
    expect(searchMetadata).not.toHaveBeenCalled();
    fireEvent.click(
      screen.getByRole("button", { name: "Search Open Library" }),
    );
    expect(await screen.findByText("Catalog Author")).toBeInTheDocument();
    expect(screen.getByText("Cover available")).toBeInTheDocument();
    expect(applyMetadataCandidate).not.toHaveBeenCalled();
  });

  it("toggles the local favorite marker", async () => {
    const updated = { ...book, isFavorite: true };
    const onFavorite = vi.fn().mockResolvedValue(updated);
    const onUpdated = vi.fn();
    render(
      <BookDetails
        book={book}
        t={t}
        busy={false}
        onRead={vi.fn()}
        onFavorite={onFavorite}
        onRemove={vi.fn()}
        onUpdated={onUpdated}
        onClose={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Add to favorites" }));
    await waitFor(() => expect(onFavorite).toHaveBeenCalledWith(book));
    expect(onUpdated).toHaveBeenCalledWith(updated);
  });

  it("removes one book only after the explicit confirmation callback", async () => {
    const onRemove = vi.fn().mockResolvedValue(true);
    render(
      <BookDetails
        book={book}
        t={t}
        busy={false}
        onRead={vi.fn()}
        onFavorite={vi.fn()}
        onRemove={onRemove}
        onUpdated={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    fireEvent.click(
      screen.getByRole("button", { name: "Remove from library" }),
    );
    await waitFor(() => expect(onRemove).toHaveBeenCalledWith(book));
    expect(
      screen.getByText(/The source book file remains on disk/),
    ).toBeInTheDocument();
  });
});
