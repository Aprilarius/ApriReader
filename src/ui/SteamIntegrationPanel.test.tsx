import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  getSteamIntegrationStatus,
  syncSteamAchievements,
  type SteamIntegrationStatus,
} from "../application/steam";
import { translations, type TranslationKey } from "./i18n";
import { SteamIntegrationPanel } from "./App";

vi.mock("../application/steam", () => ({
  getSteamIntegrationStatus: vi.fn(),
  syncSteamAchievements: vi.fn(),
  syncSteamIfAvailable: vi.fn(),
}));

const t = (key: TranslationKey) => translations.en[key];
const publicStatus: SteamIntegrationStatus = {
  buildProfile: "github",
  bridgeInstalled: false,
  providerAvailable: false,
  overlayEnabled: null,
  pendingUnlocks: 2,
  syncedUnlocks: 0,
  lastAttemptAt: null,
  lastError: null,
};

describe("SteamIntegrationPanel", () => {
  beforeEach(() => {
    vi.mocked(getSteamIntegrationStatus).mockResolvedValue(publicStatus);
    vi.mocked(syncSteamAchievements).mockResolvedValue({
      attempted: 2,
      synchronized: 2,
      pending: 0,
      overlayEnabled: true,
    });
  });

  it("truthfully explains that the public build has no Steamworks SDK", async () => {
    render(<SteamIntegrationPanel t={t} />);
    expect(await screen.findByText("Steam unavailable")).toBeInTheDocument();
    expect(
      screen.getByText(/public GitHub build contains no Steamworks SDK/u),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Synchronize" })).toBeDisabled();
  });

  it("synchronizes pending canonical unlocks only after an available provider", async () => {
    vi.mocked(getSteamIntegrationStatus)
      .mockResolvedValueOnce({
        ...publicStatus,
        buildProfile: "steam",
        bridgeInstalled: true,
        providerAvailable: true,
      })
      .mockResolvedValueOnce({
        ...publicStatus,
        buildProfile: "steam",
        bridgeInstalled: true,
        providerAvailable: true,
        pendingUnlocks: 0,
        syncedUnlocks: 2,
        overlayEnabled: true,
      });
    render(<SteamIntegrationPanel t={t} />);
    fireEvent.click(await screen.findByRole("button", { name: "Synchronize" }));
    await waitFor(() => expect(syncSteamAchievements).toHaveBeenCalledOnce());
    expect(
      await screen.findByText("Achievements synchronized with Steam."),
    ).toBeInTheDocument();
  });
});
