use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::{
    collections::HashSet,
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error;

const MAX_ACTIVE_GAP_SECONDS: i64 = 45;
const MAX_CREDITED_SECONDS: i64 = 30;
const MAX_WORDS_PER_EVENT: i64 = 5_000;
const MAX_PAGES_PER_EVENT: i64 = 20;
const CALENDAR_DAYS: i64 = 84;

#[derive(Debug, Error)]
pub enum StatisticsError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("the selected book does not exist")]
    MissingBook,
    #[error("the reading session is invalid or has already ended")]
    InvalidSession,
    #[error("the reading activity values are invalid")]
    InvalidActivity,
    #[error("the daily reading goal is invalid")]
    InvalidDailyGoal,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatisticsDay {
    pub date: String,
    pub active_seconds: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatisticsSnapshot {
    pub total_active_seconds: i64,
    pub today_active_seconds: i64,
    pub books_opened: i64,
    pub books_completed: i64,
    pub words_read: i64,
    pub pages_read: i64,
    pub current_streak: i64,
    pub longest_streak: i64,
    pub daily_goal_minutes: i64,
    pub calendar: Vec<StatisticsDay>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AchievementProgress {
    pub id: &'static str,
    pub category: &'static str,
    pub current: i64,
    pub target: i64,
    pub unlocked_at: Option<i64>,
}

#[derive(Clone, Copy)]
enum AchievementMetric {
    BooksOpened,
    BooksCompleted,
    ActiveSeconds,
    WordsRead,
    PagesRead,
    CurrentStreak,
    Annotations,
    Authors,
    Genres,
    Series,
    TodaySeconds,
}

#[derive(Clone, Copy)]
pub(crate) struct AchievementDefinition {
    pub(crate) id: &'static str,
    pub(crate) category: &'static str,
    pub(crate) target: i64,
    metric: AchievementMetric,
}

const fn achievement(
    id: &'static str,
    category: &'static str,
    target: i64,
    metric: AchievementMetric,
) -> AchievementDefinition {
    AchievementDefinition {
        id,
        category,
        target,
        metric,
    }
}

pub(crate) const ACHIEVEMENTS: &[AchievementDefinition] = &[
    achievement(
        "first_book_opened",
        "library",
        1,
        AchievementMetric::BooksOpened,
    ),
    achievement(
        "first_book_finished",
        "completion",
        1,
        AchievementMetric::BooksCompleted,
    ),
    achievement(
        "five_books_finished",
        "completion",
        5,
        AchievementMetric::BooksCompleted,
    ),
    achievement(
        "ten_books_finished",
        "completion",
        10,
        AchievementMetric::BooksCompleted,
    ),
    achievement(
        "twenty_five_books_finished",
        "completion",
        25,
        AchievementMetric::BooksCompleted,
    ),
    achievement(
        "fifty_books_finished",
        "completion",
        50,
        AchievementMetric::BooksCompleted,
    ),
    achievement(
        "hundred_books_finished",
        "completion",
        100,
        AchievementMetric::BooksCompleted,
    ),
    achievement(
        "two_hundred_fifty_books_finished",
        "completion",
        250,
        AchievementMetric::BooksCompleted,
    ),
    achievement(
        "reading_30_minutes",
        "time",
        30 * 60,
        AchievementMetric::ActiveSeconds,
    ),
    achievement(
        "reading_10_hours",
        "time",
        10 * 60 * 60,
        AchievementMetric::ActiveSeconds,
    ),
    achievement(
        "reading_50_hours",
        "time",
        50 * 60 * 60,
        AchievementMetric::ActiveSeconds,
    ),
    achievement(
        "reading_100_hours",
        "time",
        100 * 60 * 60,
        AchievementMetric::ActiveSeconds,
    ),
    achievement(
        "reading_500_hours",
        "time",
        500 * 60 * 60,
        AchievementMetric::ActiveSeconds,
    ),
    achievement(
        "ten_thousand_words",
        "volume",
        10_000,
        AchievementMetric::WordsRead,
    ),
    achievement(
        "hundred_thousand_words",
        "volume",
        100_000,
        AchievementMetric::WordsRead,
    ),
    achievement(
        "million_words",
        "volume",
        1_000_000,
        AchievementMetric::WordsRead,
    ),
    achievement(
        "ten_million_words",
        "volume",
        10_000_000,
        AchievementMetric::WordsRead,
    ),
    achievement("hundred_pages", "volume", 100, AchievementMetric::PagesRead),
    achievement(
        "thousand_pages",
        "volume",
        1_000,
        AchievementMetric::PagesRead,
    ),
    achievement(
        "ten_thousand_pages",
        "volume",
        10_000,
        AchievementMetric::PagesRead,
    ),
    achievement(
        "three_day_streak",
        "streak",
        3,
        AchievementMetric::CurrentStreak,
    ),
    achievement(
        "seven_day_streak",
        "streak",
        7,
        AchievementMetric::CurrentStreak,
    ),
    achievement(
        "thirty_day_streak",
        "streak",
        30,
        AchievementMetric::CurrentStreak,
    ),
    achievement(
        "hundred_day_streak",
        "streak",
        100,
        AchievementMetric::CurrentStreak,
    ),
    achievement(
        "year_streak",
        "streak",
        365,
        AchievementMetric::CurrentStreak,
    ),
    achievement(
        "five_annotations",
        "notes",
        5,
        AchievementMetric::Annotations,
    ),
    achievement(
        "twenty_five_annotations",
        "notes",
        25,
        AchievementMetric::Annotations,
    ),
    achievement(
        "hundred_annotations",
        "notes",
        100,
        AchievementMetric::Annotations,
    ),
    achievement("three_authors", "discovery", 3, AchievementMetric::Authors),
    achievement("ten_authors", "discovery", 10, AchievementMetric::Authors),
    achievement(
        "twenty_five_authors",
        "discovery",
        25,
        AchievementMetric::Authors,
    ),
    achievement("fifty_authors", "discovery", 50, AchievementMetric::Authors),
    achievement(
        "hundred_authors",
        "discovery",
        100,
        AchievementMetric::Authors,
    ),
    achievement("three_genres", "discovery", 3, AchievementMetric::Genres),
    achievement("five_genres", "discovery", 5, AchievementMetric::Genres),
    achievement("ten_genres", "discovery", 10, AchievementMetric::Genres),
    achievement("twenty_genres", "discovery", 20, AchievementMetric::Genres),
    achievement("three_series", "discovery", 3, AchievementMetric::Series),
    achievement("ten_series", "discovery", 10, AchievementMetric::Series),
    achievement(
        "twenty_five_series",
        "discovery",
        25,
        AchievementMetric::Series,
    ),
    achievement("fifty_series", "discovery", 50, AchievementMetric::Series),
    achievement("daily_goal_met", "goal", 1, AchievementMetric::TodaySeconds),
];

struct AchievementMetrics {
    books_opened: i64,
    books_completed: i64,
    active_seconds: i64,
    today_seconds: i64,
    words_read: i64,
    pages_read: i64,
    current_streak: i64,
    annotations: i64,
    authors: i64,
    genres: i64,
    series: i64,
    daily_goal_seconds: i64,
}

impl AchievementMetrics {
    fn value(&self, metric: AchievementMetric) -> i64 {
        match metric {
            AchievementMetric::BooksOpened => self.books_opened,
            AchievementMetric::BooksCompleted => self.books_completed,
            AchievementMetric::ActiveSeconds => self.active_seconds,
            AchievementMetric::WordsRead => self.words_read,
            AchievementMetric::PagesRead => self.pages_read,
            AchievementMetric::CurrentStreak => self.current_streak,
            AchievementMetric::Annotations => self.annotations,
            AchievementMetric::Authors => self.authors,
            AchievementMetric::Genres => self.genres,
            AchievementMetric::Series => self.series,
            AchievementMetric::TodaySeconds => {
                i64::from(self.today_seconds >= self.daily_goal_seconds)
            }
        }
    }
}

pub fn start_session(
    connection: &Connection,
    book_id: i64,
    progress: f64,
    words: i64,
    pages: i64,
) -> Result<String, StatisticsError> {
    validate_activity(progress, words, pages)?;
    let exists = connection
        .query_row("SELECT 1 FROM books WHERE id = ?1", [book_id], |row| {
            row.get::<_, i64>(0)
        })
        .optional()?
        .is_some();
    if !exists {
        return Err(StatisticsError::MissingBook);
    }
    let now = unix_seconds();
    let token = format!("rs-{book_id:x}-{now:x}-{:x}", unix_nanos());
    connection.execute(
        "INSERT INTO reading_sessions(
            token, book_id, started_at, last_seen_at, last_progress, last_words, last_pages
         ) VALUES (?1, ?2, ?3, ?3, ?4, ?5, ?6)",
        params![token, book_id, now, progress, words, pages],
    )?;
    Ok(token)
}

pub fn record_activity(
    connection: &Connection,
    token: &str,
    active: bool,
    progress: f64,
    words: i64,
    pages: i64,
) -> Result<(), StatisticsError> {
    record_activity_at(
        connection,
        token,
        active,
        progress,
        words,
        pages,
        unix_seconds(),
    )
}

fn record_activity_at(
    connection: &Connection,
    token: &str,
    active: bool,
    progress: f64,
    words: i64,
    pages: i64,
    now: i64,
) -> Result<(), StatisticsError> {
    validate_activity(progress, words, pages)?;
    let session = connection
        .query_row(
            "SELECT id, last_seen_at, last_words, last_pages
             FROM reading_sessions
             WHERE token = ?1 AND ended_at IS NULL",
            [token],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )
        .optional()?
        .ok_or(StatisticsError::InvalidSession)?;
    let elapsed = now.saturating_sub(session.1);
    let eligible = active && elapsed > 0 && elapsed <= MAX_ACTIVE_GAP_SECONDS;
    let active_seconds = if eligible {
        elapsed.min(MAX_CREDITED_SECONDS)
    } else {
        0
    };
    let words_read = if eligible {
        words
            .saturating_sub(session.2)
            .clamp(0, MAX_WORDS_PER_EVENT)
    } else {
        0
    };
    let pages_read = if eligible {
        pages
            .saturating_sub(session.3)
            .clamp(0, MAX_PAGES_PER_EVENT)
    } else {
        0
    };
    if active_seconds > 0 || words_read > 0 || pages_read > 0 {
        connection.execute(
            "INSERT INTO reading_activity_events(
                session_id, occurred_at, active_seconds, words_read, pages_read, progress
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(session_id, occurred_at) DO NOTHING",
            params![
                session.0,
                now,
                active_seconds,
                words_read,
                pages_read,
                progress
            ],
        )?;
    }
    connection.execute(
        "UPDATE reading_sessions
         SET last_seen_at = ?1, last_progress = ?2, last_words = ?3, last_pages = ?4
         WHERE id = ?5",
        params![now, progress, words, pages, session.0],
    )?;
    Ok(())
}

pub fn end_session(connection: &Connection, token: &str) -> Result<(), StatisticsError> {
    let changed = connection.execute(
        "UPDATE reading_sessions
         SET ended_at = ?1
         WHERE token = ?2 AND ended_at IS NULL",
        params![unix_seconds(), token],
    )?;
    if changed == 0 {
        return Err(StatisticsError::InvalidSession);
    }
    Ok(())
}

pub fn snapshot(connection: &Connection) -> Result<StatisticsSnapshot, StatisticsError> {
    let totals = aggregate_totals(connection)?;
    let calendar = calendar(connection)?;
    let activity_days = activity_days(connection)?;
    let (current_streak, longest_streak) = streaks(&activity_days);
    Ok(StatisticsSnapshot {
        total_active_seconds: totals.0,
        today_active_seconds: totals.1,
        books_opened: totals.2,
        books_completed: totals.3,
        words_read: totals.4,
        pages_read: totals.5,
        current_streak,
        longest_streak,
        daily_goal_minutes: daily_goal(connection)?,
        calendar,
    })
}

pub fn achievements(connection: &Connection) -> Result<Vec<AchievementProgress>, StatisticsError> {
    let snapshot = snapshot(connection)?;
    let annotations = connection.query_row("SELECT COUNT(*) FROM annotations", [], |row| {
        row.get::<_, i64>(0)
    })?;
    let authors = distinct_completed(connection, "author")?;
    let genres = distinct_completed_genres(connection)?;
    let series = distinct_completed(connection, "series")?;
    let metrics = AchievementMetrics {
        books_opened: snapshot.books_opened,
        books_completed: snapshot.books_completed,
        active_seconds: snapshot.total_active_seconds,
        today_seconds: snapshot.today_active_seconds,
        words_read: snapshot.words_read,
        pages_read: snapshot.pages_read,
        current_streak: snapshot.current_streak,
        annotations,
        authors,
        genres,
        series,
        daily_goal_seconds: snapshot.daily_goal_minutes * 60,
    };
    let now = unix_seconds();
    for definition in ACHIEVEMENTS {
        if metrics.value(definition.metric) >= definition.target {
            let inserted = connection.execute(
                "INSERT INTO achievement_unlocks(achievement_id, unlocked_at)
                 VALUES (?1, ?2)
                 ON CONFLICT(achievement_id) DO NOTHING",
                params![definition.id, now],
            )?;
            if inserted > 0 {
                connection.execute(
                    "INSERT INTO achievement_sync_queue(achievement_id, unlocked_at)
                     VALUES (?1, ?2)
                     ON CONFLICT(achievement_id) DO NOTHING",
                    params![definition.id, now],
                )?;
            }
        }
    }
    connection.execute(
        "INSERT OR IGNORE INTO achievement_sync_queue(achievement_id, unlocked_at)
         SELECT achievement_id, unlocked_at FROM achievement_unlocks",
        [],
    )?;
    ACHIEVEMENTS
        .iter()
        .map(|definition| {
            let unlocked_at = connection
                .query_row(
                    "SELECT unlocked_at FROM achievement_unlocks WHERE achievement_id = ?1",
                    [definition.id],
                    |row| row.get::<_, i64>(0),
                )
                .optional()?;
            Ok(AchievementProgress {
                id: definition.id,
                category: definition.category,
                current: metrics.value(definition.metric),
                target: definition.target,
                unlocked_at,
            })
        })
        .collect()
}

pub fn set_daily_goal(connection: &Connection, minutes: i64) -> Result<(), StatisticsError> {
    if !(5..=240).contains(&minutes) {
        return Err(StatisticsError::InvalidDailyGoal);
    }
    connection.execute(
        "INSERT INTO app_metadata(key, value) VALUES('daily_goal_minutes', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [minutes.to_string()],
    )?;
    Ok(())
}

pub fn clear(connection: &Connection) -> Result<(), StatisticsError> {
    connection.execute("DELETE FROM achievement_sync_queue", [])?;
    connection.execute("DELETE FROM achievement_unlocks", [])?;
    connection.execute("DELETE FROM reading_sessions", [])?;
    Ok(())
}

fn validate_activity(progress: f64, words: i64, pages: i64) -> Result<(), StatisticsError> {
    if !progress.is_finite() || !(0.0..=1.0).contains(&progress) || words < 0 || pages < 0 {
        return Err(StatisticsError::InvalidActivity);
    }
    Ok(())
}

fn aggregate_totals(
    connection: &Connection,
) -> Result<(i64, i64, i64, i64, i64, i64), rusqlite::Error> {
    connection.query_row(
        "SELECT
            COALESCE((SELECT SUM(active_seconds) FROM reading_activity_events), 0),
            COALESCE((SELECT SUM(active_seconds) FROM reading_activity_events
                WHERE date(occurred_at, 'unixepoch', 'localtime') = date('now', 'localtime')), 0),
            (SELECT COUNT(DISTINCT book_id) FROM reading_sessions),
            (SELECT COUNT(DISTINCT reading_sessions.book_id)
                FROM reading_sessions
                JOIN books ON books.id = reading_sessions.book_id
                WHERE books.progress >= 0.995),
            COALESCE((SELECT SUM(words_read) FROM reading_activity_events), 0),
            COALESCE((SELECT SUM(pages_read) FROM reading_activity_events), 0)",
        [],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        },
    )
}

fn calendar(connection: &Connection) -> Result<Vec<StatisticsDay>, rusqlite::Error> {
    let mut statement = connection.prepare(
        "WITH RECURSIVE days(day) AS (
            SELECT date('now', 'localtime', ?1)
            UNION ALL
            SELECT date(day, '+1 day') FROM days
            WHERE day < date('now', 'localtime')
         )
         SELECT days.day, COALESCE(SUM(reading_activity_events.active_seconds), 0)
         FROM days
         LEFT JOIN reading_activity_events
           ON date(reading_activity_events.occurred_at, 'unixepoch', 'localtime') = days.day
         GROUP BY days.day
         ORDER BY days.day",
    )?;
    let offset = format!("-{} days", CALENDAR_DAYS - 1);
    let rows = statement.query_map([offset], |row| {
        Ok(StatisticsDay {
            date: row.get(0)?,
            active_seconds: row.get(1)?,
        })
    })?;
    rows.collect()
}

fn activity_days(connection: &Connection) -> Result<Vec<StatisticsDay>, rusqlite::Error> {
    let mut statement = connection.prepare(
        "WITH RECURSIVE days(day) AS (
            SELECT COALESCE(
                (SELECT MIN(date(occurred_at, 'unixepoch', 'localtime'))
                 FROM reading_activity_events),
                date('now', 'localtime')
            )
            UNION ALL
            SELECT date(day, '+1 day') FROM days
            WHERE day < date('now', 'localtime')
         )
         SELECT days.day, COALESCE(SUM(reading_activity_events.active_seconds), 0)
         FROM days
         LEFT JOIN reading_activity_events
           ON date(reading_activity_events.occurred_at, 'unixepoch', 'localtime') = days.day
         GROUP BY days.day
         ORDER BY days.day",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(StatisticsDay {
            date: row.get(0)?,
            active_seconds: row.get(1)?,
        })
    })?;
    rows.collect()
}

fn streaks(calendar: &[StatisticsDay]) -> (i64, i64) {
    let mut longest = 0;
    let mut run = 0;
    for day in calendar {
        if day.active_seconds > 0 {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 0;
        }
    }
    let mut current = 0;
    let mut index = calendar.len();
    if calendar.last().is_some_and(|day| day.active_seconds == 0) {
        index = index.saturating_sub(1);
    }
    while index > 0 {
        index -= 1;
        if calendar[index].active_seconds == 0 {
            break;
        }
        current += 1;
    }
    (current, longest)
}

fn daily_goal(connection: &Connection) -> Result<i64, StatisticsError> {
    let value = connection
        .query_row(
            "SELECT value FROM app_metadata WHERE key = 'daily_goal_minutes'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .unwrap_or_else(|| "20".to_owned());
    value
        .parse::<i64>()
        .ok()
        .filter(|minutes| (5..=240).contains(minutes))
        .ok_or(StatisticsError::InvalidDailyGoal)
}

fn distinct_completed(connection: &Connection, column: &str) -> Result<i64, StatisticsError> {
    let sql = match column {
        "author" => {
            "SELECT COUNT(DISTINCT LOWER(TRIM(books.author)))
             FROM reading_sessions JOIN books ON books.id = reading_sessions.book_id
             WHERE books.progress >= 0.995 AND TRIM(books.author) <> ''"
        }
        "series" => {
            "SELECT COUNT(DISTINCT LOWER(TRIM(books.series)))
             FROM reading_sessions JOIN books ON books.id = reading_sessions.book_id
             WHERE books.progress >= 0.995 AND TRIM(books.series) <> ''"
        }
        _ => return Ok(0),
    };
    connection
        .query_row(sql, [], |row| row.get::<_, i64>(0))
        .map_err(Into::into)
}

fn distinct_completed_genres(connection: &Connection) -> Result<i64, StatisticsError> {
    let mut statement = connection.prepare(
        "SELECT DISTINCT books.genres
         FROM reading_sessions JOIN books ON books.id = reading_sessions.book_id
         WHERE books.progress >= 0.995 AND TRIM(books.genres) <> ''",
    )?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
    let mut genres = HashSet::<String>::new();
    for row in rows {
        for genre in row?.split([',', ';']) {
            let normalized = genre.split_whitespace().collect::<Vec<_>>().join(" ");
            if !normalized.is_empty() {
                genres.insert(normalized.to_lowercase());
            }
        }
    }
    Ok(i64::try_from(genres.len()).unwrap_or(i64::MAX))
}

fn unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .try_into()
        .unwrap_or(i64::MAX)
}

fn unix_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn database() -> Connection {
        let connection = Connection::open_in_memory().expect("open database");
        connection
            .execute_batch(
                "PRAGMA foreign_keys = ON;
                 CREATE TABLE app_metadata(key TEXT PRIMARY KEY, value TEXT NOT NULL) STRICT;
                 INSERT INTO app_metadata VALUES('daily_goal_minutes', '20');
                 CREATE TABLE books(
                    id INTEGER PRIMARY KEY, progress REAL NOT NULL,
                    author TEXT NOT NULL DEFAULT '', series TEXT NOT NULL DEFAULT '',
                    genres TEXT NOT NULL DEFAULT ''
                 ) STRICT;
                 CREATE TABLE annotations(id INTEGER PRIMARY KEY) STRICT;
                 CREATE TABLE reading_sessions(
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    token TEXT NOT NULL UNIQUE,
                    book_id INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
                    started_at INTEGER NOT NULL, last_seen_at INTEGER NOT NULL,
                    ended_at INTEGER, last_progress REAL NOT NULL,
                    last_words INTEGER NOT NULL, last_pages INTEGER NOT NULL
                 ) STRICT;
                 CREATE TABLE reading_activity_events(
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    session_id INTEGER NOT NULL REFERENCES reading_sessions(id) ON DELETE CASCADE,
                    occurred_at INTEGER NOT NULL, active_seconds INTEGER NOT NULL,
                    words_read INTEGER NOT NULL, pages_read INTEGER NOT NULL,
                    progress REAL NOT NULL, UNIQUE(session_id, occurred_at)
                 ) STRICT;
                 CREATE TABLE achievement_unlocks(
                    achievement_id TEXT PRIMARY KEY, unlocked_at INTEGER NOT NULL
                 ) STRICT;
                 CREATE TABLE achievement_sync_queue(
                    achievement_id TEXT PRIMARY KEY, unlocked_at INTEGER NOT NULL,
                    attempts INTEGER NOT NULL DEFAULT 0, last_attempt_at INTEGER,
                    last_error TEXT, synced_at INTEGER
                 ) STRICT;
                 INSERT INTO books(id, progress, author, series, genres)
                 VALUES(1, 0.2, 'Author', 'Series', 'Fantasy, Adventure');",
            )
            .expect("create schema");
        connection
    }

    #[test]
    fn ignores_idle_gaps_and_duplicate_events() {
        let connection = database();
        connection
            .execute(
                "INSERT INTO reading_sessions(
                    token, book_id, started_at, last_seen_at, last_progress, last_words, last_pages
                 ) VALUES('token', 1, 100, 100, 0.1, 10, 1)",
                [],
            )
            .expect("session");
        record_activity_at(&connection, "token", true, 0.2, 20, 2, 115).expect("first activity");
        record_activity_at(&connection, "token", true, 0.3, 30, 3, 115)
            .expect("duplicate activity");
        record_activity_at(&connection, "token", true, 0.4, 40, 4, 300).expect("idle activity");
        let totals = aggregate_totals(&connection).expect("totals");
        assert_eq!(totals.0, 15);
        assert_eq!(totals.4, 10);
        assert_eq!(totals.5, 1);
    }

    #[test]
    fn achievement_unlocks_are_idempotent() {
        let connection = database();
        connection
            .execute(
                "INSERT INTO reading_sessions(
                    token, book_id, started_at, last_seen_at, last_progress, last_words, last_pages
                 ) VALUES('token', 1, 100, 100, 0.1, 0, 0)",
                [],
            )
            .expect("session");
        achievements(&connection).expect("first evaluation");
        achievements(&connection).expect("second evaluation");
        let count = connection
            .query_row(
                "SELECT COUNT(*) FROM achievement_unlocks
                 WHERE achievement_id = 'first_book_opened'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("unlock count");
        assert_eq!(count, 1);
        let queued = connection
            .query_row(
                "SELECT COUNT(*) FROM achievement_sync_queue
                 WHERE achievement_id = 'first_book_opened'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("queue count");
        assert_eq!(queued, 1);
    }

    #[test]
    fn achievement_registry_is_large_unique_and_long_term() {
        let ids = ACHIEVEMENTS
            .iter()
            .map(|definition| definition.id)
            .collect::<HashSet<_>>();
        assert_eq!(ACHIEVEMENTS.len(), 42);
        assert_eq!(ids.len(), ACHIEVEMENTS.len());
        assert!(ACHIEVEMENTS
            .iter()
            .any(|item| item.id == "two_hundred_fifty_books_finished"));
        assert!(ACHIEVEMENTS.iter().any(|item| item.id == "year_streak"));
        assert!(ACHIEVEMENTS.iter().any(|item| item.id == "twenty_genres"));
    }

    #[test]
    fn discovery_achievements_count_only_completed_books_and_split_genres() {
        let connection = database();
        connection
            .execute(
                "INSERT INTO reading_sessions(
                    token, book_id, started_at, last_seen_at, last_progress, last_words, last_pages
                 ) VALUES('token', 1, 100, 100, 0.2, 0, 0)",
                [],
            )
            .expect("session");
        assert_eq!(
            distinct_completed(&connection, "author").expect("authors"),
            0
        );
        assert_eq!(distinct_completed_genres(&connection).expect("genres"), 0);

        connection
            .execute("UPDATE books SET progress = 1 WHERE id = 1", [])
            .expect("complete book");
        assert_eq!(
            distinct_completed(&connection, "author").expect("authors"),
            1
        );
        assert_eq!(
            distinct_completed(&connection, "series").expect("series"),
            1
        );
        assert_eq!(distinct_completed_genres(&connection).expect("genres"), 2);
    }
}
