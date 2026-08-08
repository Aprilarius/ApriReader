import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  clearReadingStatistics,
  getAudiobookAchievements,
  getAudiobookStatistics,
  getAchievements,
  getStatistics,
  setDailyGoal,
  type StatisticsSnapshot,
} from "../application/statistics";
import { translations, type TranslationKey } from "./i18n";
import { AchievementsPage, StatisticsPage } from "./StatisticsPages";

vi.mock("../application/statistics", () => ({
  clearReadingStatistics: vi.fn(),
  getAudiobookAchievements: vi.fn(),
  getAudiobookStatistics: vi.fn(),
  getAchievements: vi.fn(),
  getStatistics: vi.fn(),
  setDailyGoal: vi.fn(),
}));

const t = (key: TranslationKey) => translations.en[key];
const snapshot: StatisticsSnapshot = {
  totalActiveSeconds: 3_720,
  todayActiveSeconds: 900,
  booksOpened: 2,
  booksCompleted: 1,
  wordsRead: 8_500,
  pagesRead: 42,
  currentStreak: 2,
  longestStreak: 4,
  dailyGoalMinutes: 20,
  calendar: [{ date: "2026-07-28", activeSeconds: 900 }],
};

describe("statistics and achievements", () => {
  beforeEach(() => {
    vi.mocked(getStatistics).mockResolvedValue(snapshot);
    vi.mocked(getAchievements).mockResolvedValue([]);
    vi.mocked(getAudiobookAchievements).mockResolvedValue([]);
    vi.mocked(getAudiobookStatistics).mockResolvedValue({
      totalActiveSeconds: 1_800,
      todayActiveSeconds: 300,
      audiobooksStarted: 2,
      audiobooksCompleted: 1,
    });
    vi.mocked(setDailyGoal).mockResolvedValue();
    vi.mocked(clearReadingStatistics).mockResolvedValue();
  });

  it("renders stored aggregates and changes the daily goal explicitly", async () => {
    render(<StatisticsPage t={t} onChanged={vi.fn()} />);
    expect(await screen.findByText("1 h 2 min")).toBeInTheDocument();
    expect(screen.getByText(/8.500/u)).toBeInTheDocument();
    expect(screen.getByText("Audiobook statistics")).toBeInTheDocument();
    expect(screen.getByText("Audiobooks started")).toBeInTheDocument();
    const goal = screen.getByRole("spinbutton", { name: "Minutes per day" });
    fireEvent.change(goal, { target: { value: "30" } });
    fireEvent.click(screen.getByRole("button", { name: "Save goal" }));
    await waitFor(() => expect(setDailyGoal).toHaveBeenCalledWith(30));
  });

  it("requires confirmation before deleting local statistics", async () => {
    const confirmation = vi.spyOn(window, "confirm").mockReturnValue(true);
    render(<StatisticsPage t={t} onChanged={vi.fn()} />);
    await screen.findByText("Reading calendar");
    fireEvent.click(screen.getByRole("button", { name: "Delete statistics" }));
    await waitFor(() => expect(clearReadingStatistics).toHaveBeenCalledOnce());
    confirmation.mockRestore();
  });

  it("shows canonical achievement progress", async () => {
    vi.mocked(getAchievements).mockResolvedValue([
      {
        id: "first_book_opened",
        category: "library",
        current: 1,
        target: 1,
        unlockedAt: 123,
      },
      {
        id: "twenty_five_books_finished",
        category: "completion",
        current: 7,
        target: 25,
        unlockedAt: null,
      },
      {
        id: "reading_10_hours",
        category: "time",
        current: 3_600,
        target: 36_000,
        unlockedAt: null,
      },
    ]);
    vi.mocked(getAudiobookAchievements).mockResolvedValue([
      {
        id: "audio_30_minutes",
        category: "audio_time",
        current: 1_800,
        target: 1_800,
        unlockedAt: 456,
      },
    ]);
    render(<AchievementsPage t={t} />);
    expect(await screen.findByText("First book")).toBeInTheDocument();
    expect(screen.getAllByText("Unlocked")).toHaveLength(2);
    expect(screen.getByText("25 finished books")).toBeInTheDocument();
    expect(screen.getByText("7 / 25")).toBeInTheDocument();
    expect(screen.getByText("1 h 0 min / 10 h 0 min")).toBeInTheDocument();
    expect(screen.getByText("Unlocked 2 of 4")).toBeInTheDocument();
    expect(screen.getByText("Audiobook achievements")).toBeInTheDocument();
    expect(screen.getByText("By ear")).toBeInTheDocument();
  });
});
