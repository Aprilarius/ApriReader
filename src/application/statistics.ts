import { invoke, isTauri } from "@tauri-apps/api/core";

export type StatisticsDay = {
  date: string;
  activeSeconds: number;
};

export type StatisticsSnapshot = {
  totalActiveSeconds: number;
  todayActiveSeconds: number;
  booksOpened: number;
  booksCompleted: number;
  wordsRead: number;
  pagesRead: number;
  currentStreak: number;
  longestStreak: number;
  dailyGoalMinutes: number;
  calendar: StatisticsDay[];
};

export type AchievementProgress = {
  id: string;
  category: string;
  current: number;
  target: number;
  unlockedAt: number | null;
};

export async function startReadingSession(
  bookId: number,
  progress: number,
  words: number,
  pages: number,
): Promise<string | null> {
  if (!isTauri()) return null;
  return invoke<string>("start_reading_session", {
    bookId,
    progress,
    words,
    pages,
  });
}

export async function recordReadingActivity(
  token: string,
  active: boolean,
  progress: number,
  words: number,
  pages: number,
): Promise<void> {
  if (!isTauri()) return;
  await invoke("record_reading_activity", {
    token,
    active,
    progress,
    words,
    pages,
  });
}

export async function endReadingSession(token: string): Promise<void> {
  if (!isTauri()) return;
  await invoke("end_reading_session", { token });
}

export async function getStatistics(): Promise<StatisticsSnapshot> {
  if (!isTauri()) return emptyStatistics();
  return invoke<StatisticsSnapshot>("get_statistics");
}

export async function getAchievements(): Promise<AchievementProgress[]> {
  if (!isTauri()) return [];
  return invoke<AchievementProgress[]>("get_achievements");
}

export async function setDailyGoal(minutes: number): Promise<void> {
  if (!isTauri()) return;
  await invoke("set_daily_goal", { minutes });
}

export async function clearReadingStatistics(): Promise<void> {
  if (!isTauri()) return;
  await invoke("clear_reading_statistics");
}

export function emptyStatistics(): StatisticsSnapshot {
  return {
    totalActiveSeconds: 0,
    todayActiveSeconds: 0,
    booksOpened: 0,
    booksCompleted: 0,
    wordsRead: 0,
    pagesRead: 0,
    currentStreak: 0,
    longestStreak: 0,
    dailyGoalMinutes: 20,
    calendar: [],
  };
}
