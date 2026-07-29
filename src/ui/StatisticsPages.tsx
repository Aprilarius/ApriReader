import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  type FormEvent,
} from "react";
import {
  clearReadingStatistics,
  getAchievements,
  getStatistics,
  setDailyGoal,
  type AchievementProgress,
  type StatisticsSnapshot,
} from "../application/statistics";
import { Icon } from "./icons";
import type { TranslationKey } from "./i18n";

type Translator = (key: TranslationKey) => string;

export function StatisticsPage({
  t,
  onChanged,
}: {
  t: Translator;
  onChanged: (snapshot: StatisticsSnapshot) => void;
}) {
  const [statistics, setStatistics] = useState<StatisticsSnapshot | null>(null);
  const [goal, setGoal] = useState(20);
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);

  const refresh = useCallback(
    () =>
      getStatistics()
        .then((value) => {
          setStatistics(value);
          setGoal(value.dailyGoalMinutes);
          onChanged(value);
        })
        .catch((reason) => setMessage(String(reason))),
    [onChanged],
  );

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const maxDay = useMemo(
    () =>
      Math.max(
        1,
        ...(statistics?.calendar.map((day) => day.activeSeconds) ?? []),
      ),
    [statistics],
  );

  const saveGoal = async (event: FormEvent) => {
    event.preventDefault();
    setBusy(true);
    try {
      await setDailyGoal(goal);
      setMessage(t("goalSaved"));
      await refresh();
    } catch (reason) {
      setMessage(String(reason));
    } finally {
      setBusy(false);
    }
  };

  const clear = async () => {
    if (!window.confirm(t("clearStatisticsConfirm"))) return;
    setBusy(true);
    try {
      await clearReadingStatistics();
      setMessage(t("statisticsCleared"));
      await refresh();
    } catch (reason) {
      setMessage(String(reason));
    } finally {
      setBusy(false);
    }
  };

  if (!statistics) {
    return <p className="notice">{message || t("loading")}</p>;
  }
  const goalSeconds = statistics.dailyGoalMinutes * 60;
  const goalPercent = Math.min(
    100,
    Math.round((statistics.todayActiveSeconds / goalSeconds) * 100),
  );

  return (
    <div className="statistics-page">
      {message && (
        <p className="notice" role="status">
          {message}
        </p>
      )}
      <section className="stat-grid" aria-label={t("statistics")}>
        <StatValue
          icon="reading"
          value={formatDuration(statistics.totalActiveSeconds, t)}
          label={t("activeReadingTime")}
        />
        <StatValue
          icon="library"
          value={String(statistics.booksOpened)}
          label={t("booksOpened")}
        />
        <StatValue
          icon="achievement"
          value={String(statistics.booksCompleted)}
          label={t("completed")}
        />
        <StatValue
          icon="statistics"
          value={String(statistics.currentStreak)}
          label={t("streak")}
        />
      </section>

      <div className="statistics-layout">
        <section className="statistics-panel calendar-panel">
          <div className="section-heading">
            <h2>{t("readingCalendar")}</h2>
            <span>{t("lastTwelveWeeks")}</span>
          </div>
          <div className="reading-calendar" aria-label={t("readingCalendar")}>
            {statistics.calendar.map((day) => {
              const level =
                day.activeSeconds === 0
                  ? 0
                  : Math.max(1, Math.ceil((day.activeSeconds / maxDay) * 4));
              return (
                <span
                  className={`calendar-day level-${level}`}
                  key={day.date}
                  title={`${day.date} · ${formatDuration(day.activeSeconds, t)}`}
                  aria-label={`${day.date}: ${formatDuration(day.activeSeconds, t)}`}
                />
              );
            })}
          </div>
          <div className="calendar-legend">
            <span>{t("less")}</span>
            {[0, 1, 2, 3, 4].map((level) => (
              <i className={`calendar-day level-${level}`} key={level} />
            ))}
            <span>{t("more")}</span>
          </div>
        </section>

        <section className="statistics-panel goal-panel">
          <div className="section-heading">
            <h2>{t("dailyGoal")}</h2>
            <span>{goalPercent}%</span>
          </div>
          <strong>
            {formatDuration(statistics.todayActiveSeconds, t)} /{" "}
            {formatDuration(goalSeconds, t)}
          </strong>
          <div className="goal-track" aria-hidden="true">
            <span style={{ width: `${goalPercent}%` }} />
          </div>
          <form
            className="goal-form"
            onSubmit={(event) => void saveGoal(event)}
          >
            <label>
              {t("goalMinutes")}
              <input
                type="number"
                min={5}
                max={240}
                value={goal}
                onChange={(event) => setGoal(Number(event.target.value))}
              />
            </label>
            <button className="secondary-button" type="submit" disabled={busy}>
              {t("saveGoal")}
            </button>
          </form>
        </section>
      </div>

      <section className="statistics-panel detail-statistics">
        <div>
          <span>{t("wordsRead")}</span>
          <strong>{statistics.wordsRead.toLocaleString()}</strong>
        </div>
        <div>
          <span>{t("pagesRead")}</span>
          <strong>{statistics.pagesRead.toLocaleString()}</strong>
        </div>
        <div>
          <span>{t("longestStreak")}</span>
          <strong>{statistics.longestStreak}</strong>
        </div>
        <button
          className="danger-button"
          type="button"
          disabled={busy}
          onClick={() => void clear()}
        >
          {t("clearStatistics")}
        </button>
      </section>
      <p className="privacy-note">{t("statisticsPrivacy")}</p>
    </div>
  );
}

export function AchievementsPage({ t }: { t: Translator }) {
  const [achievements, setAchievements] = useState<AchievementProgress[]>([]);
  const [message, setMessage] = useState("");

  useEffect(() => {
    void getAchievements()
      .then(setAchievements)
      .catch((reason) => {
        setMessage(String(reason));
      });
  }, []);

  return (
    <div className="achievements-page">
      <p className="privacy-note">{t("achievementsHint")}</p>
      {achievements.length > 0 && (
        <p className="achievement-summary" role="status">
          {t("achievementsProgress")
            .replace(
              "{unlocked}",
              String(
                achievements.filter(
                  (achievement) => achievement.unlockedAt !== null,
                ).length,
              ),
            )
            .replace("{total}", String(achievements.length))}
        </p>
      )}
      {message && <p className="error-message">{message}</p>}
      <div className="achievement-grid">
        {achievements.map((achievement) => {
          const unlocked = achievement.unlockedAt !== null;
          const percent = Math.min(
            100,
            Math.round((achievement.current / achievement.target) * 100),
          );
          return (
            <article
              className={`achievement-card ${unlocked ? "unlocked" : ""}`}
              key={achievement.id}
            >
              <span className="achievement-emblem">
                <Icon name="achievement" />
              </span>
              <div>
                <small>{achievementCategory(achievement.category, t)}</small>
                <h2>{achievementTitle(achievement.id, t)}</h2>
                <p>{achievementDescription(achievement.id, t)}</p>
                <div className="achievement-progress">
                  <span style={{ width: `${percent}%` }} />
                </div>
                <output>
                  {unlocked
                    ? t("achievementUnlocked")
                    : formatAchievementProgress(achievement, t)}
                </output>
              </div>
            </article>
          );
        })}
      </div>
    </div>
  );
}

function StatValue({
  icon,
  value,
  label,
}: {
  icon: "reading" | "library" | "achievement" | "statistics";
  value: string;
  label: string;
}) {
  return (
    <article className="stat-card">
      <span className="stat-icon">
        <Icon name={icon} />
      </span>
      <div>
        <strong>{value}</strong>
        <small>{label}</small>
      </div>
    </article>
  );
}

function formatDuration(seconds: number, t: Translator) {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  if (hours > 0)
    return `${hours} ${t("hoursShort")} ${minutes} ${t("minutesShort")}`;
  return `${minutes} ${t("minutesShort")}`;
}

function achievementTitle(id: string, t: Translator) {
  const keys: Record<string, TranslationKey> = {
    first_book_opened: "achievementFirstBook",
    first_book_finished: "achievementFirstFinish",
    five_books_finished: "achievementFiveFinish",
    reading_30_minutes: "achievementThirtyMinutes",
    ten_thousand_words: "achievementTenThousandWords",
    hundred_pages: "achievementHundredPages",
    three_day_streak: "achievementThreeDayStreak",
    five_annotations: "achievementFiveAnnotations",
    three_authors: "achievementThreeAuthors",
    three_series: "achievementThreeSeries",
    daily_goal_met: "achievementDailyGoal",
  };
  const key = keys[id];
  if (key) return t(key);
  const tier = achievementTier(id);
  return tier
    ? t(tier.title).replace("{count}", tier.count.toLocaleString())
    : t("achievements");
}

function achievementDescription(id: string, t: Translator) {
  const keys: Record<string, TranslationKey> = {
    first_book_opened: "achievementFirstBookHint",
    first_book_finished: "achievementFirstFinishHint",
    five_books_finished: "achievementFiveFinishHint",
    reading_30_minutes: "achievementThirtyMinutesHint",
    ten_thousand_words: "achievementTenThousandWordsHint",
    hundred_pages: "achievementHundredPagesHint",
    three_day_streak: "achievementThreeDayStreakHint",
    five_annotations: "achievementFiveAnnotationsHint",
    three_authors: "achievementThreeAuthorsHint",
    three_series: "achievementThreeSeriesHint",
    daily_goal_met: "achievementDailyGoalHint",
  };
  const key = keys[id];
  if (key) return t(key);
  const tier = achievementTier(id);
  return tier
    ? t(tier.hint).replace("{count}", tier.count.toLocaleString())
    : t("achievementsHint");
}

function achievementTier(id: string) {
  const tiers: Record<
    string,
    { title: TranslationKey; hint: TranslationKey; count: number }
  > = {
    ten_books_finished: tier(
      "achievementBooksTier",
      "achievementBooksTierHint",
      10,
    ),
    twenty_five_books_finished: tier(
      "achievementBooksTier",
      "achievementBooksTierHint",
      25,
    ),
    fifty_books_finished: tier(
      "achievementBooksTier",
      "achievementBooksTierHint",
      50,
    ),
    hundred_books_finished: tier(
      "achievementBooksTier",
      "achievementBooksTierHint",
      100,
    ),
    two_hundred_fifty_books_finished: tier(
      "achievementBooksTier",
      "achievementBooksTierHint",
      250,
    ),
    reading_10_hours: tier(
      "achievementHoursTier",
      "achievementHoursTierHint",
      10,
    ),
    reading_50_hours: tier(
      "achievementHoursTier",
      "achievementHoursTierHint",
      50,
    ),
    reading_100_hours: tier(
      "achievementHoursTier",
      "achievementHoursTierHint",
      100,
    ),
    reading_500_hours: tier(
      "achievementHoursTier",
      "achievementHoursTierHint",
      500,
    ),
    hundred_thousand_words: tier(
      "achievementWordsTier",
      "achievementWordsTierHint",
      100_000,
    ),
    million_words: tier(
      "achievementWordsTier",
      "achievementWordsTierHint",
      1_000_000,
    ),
    ten_million_words: tier(
      "achievementWordsTier",
      "achievementWordsTierHint",
      10_000_000,
    ),
    thousand_pages: tier(
      "achievementPagesTier",
      "achievementPagesTierHint",
      1_000,
    ),
    ten_thousand_pages: tier(
      "achievementPagesTier",
      "achievementPagesTierHint",
      10_000,
    ),
    seven_day_streak: tier(
      "achievementStreakTier",
      "achievementStreakTierHint",
      7,
    ),
    thirty_day_streak: tier(
      "achievementStreakTier",
      "achievementStreakTierHint",
      30,
    ),
    hundred_day_streak: tier(
      "achievementStreakTier",
      "achievementStreakTierHint",
      100,
    ),
    year_streak: tier(
      "achievementStreakTier",
      "achievementStreakTierHint",
      365,
    ),
    twenty_five_annotations: tier(
      "achievementAnnotationsTier",
      "achievementAnnotationsTierHint",
      25,
    ),
    hundred_annotations: tier(
      "achievementAnnotationsTier",
      "achievementAnnotationsTierHint",
      100,
    ),
    ten_authors: tier(
      "achievementAuthorsTier",
      "achievementAuthorsTierHint",
      10,
    ),
    twenty_five_authors: tier(
      "achievementAuthorsTier",
      "achievementAuthorsTierHint",
      25,
    ),
    fifty_authors: tier(
      "achievementAuthorsTier",
      "achievementAuthorsTierHint",
      50,
    ),
    hundred_authors: tier(
      "achievementAuthorsTier",
      "achievementAuthorsTierHint",
      100,
    ),
    three_genres: tier("achievementGenresTier", "achievementGenresTierHint", 3),
    five_genres: tier("achievementGenresTier", "achievementGenresTierHint", 5),
    ten_genres: tier("achievementGenresTier", "achievementGenresTierHint", 10),
    twenty_genres: tier(
      "achievementGenresTier",
      "achievementGenresTierHint",
      20,
    ),
    ten_series: tier("achievementSeriesTier", "achievementSeriesTierHint", 10),
    twenty_five_series: tier(
      "achievementSeriesTier",
      "achievementSeriesTierHint",
      25,
    ),
    fifty_series: tier(
      "achievementSeriesTier",
      "achievementSeriesTierHint",
      50,
    ),
  };
  return tiers[id];
}

function tier(title: TranslationKey, hint: TranslationKey, count: number) {
  return { title, hint, count };
}

function formatAchievementProgress(
  achievement: AchievementProgress,
  t: Translator,
) {
  if (achievement.category === "time") {
    return `${formatDuration(achievement.current, t)} / ${formatDuration(achievement.target, t)}`;
  }
  return `${achievement.current.toLocaleString()} / ${achievement.target.toLocaleString()}`;
}

function achievementCategory(category: string, t: Translator) {
  const keys: Record<string, TranslationKey> = {
    library: "achievementCategoryLibrary",
    completion: "achievementCategoryCompletion",
    time: "achievementCategoryTime",
    volume: "achievementCategoryVolume",
    streak: "achievementCategoryStreak",
    notes: "achievementCategoryNotes",
    discovery: "achievementCategoryDiscovery",
    goal: "achievementCategoryGoal",
  };
  return t(keys[category] ?? "achievements");
}
