import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { translations, type TranslationKey } from "./i18n";
import { LanguagePackagesPage } from "./App";
import {
  chooseAndImportLanguagePackage,
  listLanguagePackages,
  removeLanguagePackage,
} from "../application/languageTools";

vi.mock("../application/languageTools", () => ({
  chooseAndImportLanguagePackage: vi.fn(),
  listLanguagePackages: vi.fn(),
  removeLanguagePackage: vi.fn(),
}));

const t = (key: TranslationKey) => translations.en[key];
const installed = {
  id: "synthetic-en",
  version: "1.0.0",
  name: "Synthetic English",
  kind: "dictionary" as const,
  sourceLanguage: "en",
  targetLanguage: null,
  licenseSpdx: "MIT",
  attribution: "Synthetic test data",
  engine: "aprireader-dictionary-v1",
  verified: true,
};

describe("LanguagePackagesPage", () => {
  beforeEach(() => {
    vi.mocked(listLanguagePackages).mockResolvedValue([]);
    vi.mocked(chooseAndImportLanguagePackage).mockReset();
    vi.mocked(removeLanguagePackage).mockReset();
  });

  it("shows a truthful empty state without downloading anything", async () => {
    render(<LanguagePackagesPage t={t} />);
    expect(
      await screen.findByText("No language packages installed"),
    ).toBeInTheDocument();
    expect(chooseAndImportLanguagePackage).not.toHaveBeenCalled();
  });

  it("imports only after the explicit package action", async () => {
    vi.mocked(chooseAndImportLanguagePackage).mockResolvedValue(installed);
    vi.mocked(listLanguagePackages)
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([installed]);
    render(<LanguagePackagesPage t={t} />);
    await screen.findByText("No language packages installed");
    fireEvent.click(screen.getByRole("button", { name: "Import package" }));
    await waitFor(() =>
      expect(screen.getByText("Synthetic English")).toBeInTheDocument(),
    );
    expect(screen.getByText("Verified")).toBeInTheDocument();
  });
});
