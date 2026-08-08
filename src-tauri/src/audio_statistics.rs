use crate::statistics::AchievementProgress;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

const MAX_ACTIVE_GAP_SECONDS: i64 = 45;
const MAX_CREDITED_SECONDS: i64 = 30;

#[derive(Debug, Error)]
pub enum AudioStatisticsError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("the selected audiobook does not exist")]
    MissingAudiobook,
    #[error("the audiobook listening session is invalid or has ended")]
    InvalidSession,
    #[error("the audiobook listening activity is invalid")]
    InvalidActivity,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudiobookStatisticsSnapshot {
    pub total_active_seconds: i64,
    pub today_active_seconds: i64,
    pub audiobooks_started: i64,
    pub audiobooks_completed: i64,
}

#[derive(Clone, Copy)]
enum Metric {
    Started,
    Completed,
    ActiveSeconds,
}

struct Definition {
    id: &'static str,
    category: &'static str,
    target: i64,
    metric: Metric,
}

const ACHIEVEMENTS: &[Definition] = &[
    Definition {
        id: "audio_first_listen",
        category: "audio_library",
        target: 1,
        metric: Metric::Started,
    },
    Definition {
        id: "audio_first_finish",
        category: "audio_completion",
        target: 1,
        metric: Metric::Completed,
    },
    Definition {
        id: "audio_ten_finished",
        category: "audio_completion",
        target: 10,
        metric: Metric::Completed,
    },
    Definition {
        id: "audio_30_minutes",
        category: "audio_time",
        target: 30 * 60,
        metric: Metric::ActiveSeconds,
    },
    Definition {
        id: "audio_10_hours",
        category: "audio_time",
        target: 10 * 60 * 60,
        metric: Metric::ActiveSeconds,
    },
    Definition {
        id: "audio_50_hours",
        category: "audio_time",
        target: 50 * 60 * 60,
        metric: Metric::ActiveSeconds,
    },
    Definition {
        id: "audio_100_hours",
        category: "audio_time",
        target: 100 * 60 * 60,
        metric: Metric::ActiveSeconds,
    },
];

pub fn start_session(
    connection: &Connection,
    audiobook_id: i64,
    progress: f64,
) -> Result<String, AudioStatisticsError> {
    validate_progress(progress)?;
    let exists = connection
        .query_row(
            "SELECT 1 FROM audiobooks WHERE id = ?1",
            [audiobook_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()?
        .is_some();
    if !exists {
        return Err(AudioStatisticsError::MissingAudiobook);
    }
    let now = unix_seconds();
    let token = format!("as-{audiobook_id:x}-{now:x}-{:x}", unix_nanos());
    connection.execute(
        "INSERT INTO audiobook_listening_sessions(
            token, audiobook_id, started_at, last_seen_at, last_progress
         ) VALUES (?1, ?2, ?3, ?3, ?4)",
        params![token, audiobook_id, now, progress],
    )?;
    Ok(token)
}

pub fn record_activity(
    connection: &Connection,
    token: &str,
    active: bool,
    progress: f64,
) -> Result<(), AudioStatisticsError> {
    record_activity_at(connection, token, active, progress, unix_seconds())
}

fn record_activity_at(
    connection: &Connection,
    token: &str,
    active: bool,
    progress: f64,
    now: i64,
) -> Result<(), AudioStatisticsError> {
    validate_progress(progress)?;
    let (session_id, last_seen_at) = connection
        .query_row(
            "SELECT id, last_seen_at FROM audiobook_listening_sessions
             WHERE token = ?1 AND ended_at IS NULL",
            [token],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()?
        .ok_or(AudioStatisticsError::InvalidSession)?;
    let elapsed = now.saturating_sub(last_seen_at);
    let credited = if active && elapsed > 0 && elapsed <= MAX_ACTIVE_GAP_SECONDS {
        elapsed.min(MAX_CREDITED_SECONDS)
    } else {
        0
    };
    if credited > 0 {
        connection.execute(
            "INSERT INTO audiobook_listening_events(
                session_id, occurred_at, active_seconds, progress
             ) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(session_id, occurred_at) DO NOTHING",
            params![session_id, now, credited, progress],
        )?;
    }
    connection.execute(
        "UPDATE audiobook_listening_sessions
         SET last_seen_at = ?1, last_progress = ?2 WHERE id = ?3",
        params![now, progress, session_id],
    )?;
    Ok(())
}

pub fn end_session(connection: &Connection, token: &str) -> Result<(), AudioStatisticsError> {
    let changed = connection.execute(
        "UPDATE audiobook_listening_sessions SET ended_at = ?1
         WHERE token = ?2 AND ended_at IS NULL",
        params![unix_seconds(), token],
    )?;
    if changed == 0 {
        return Err(AudioStatisticsError::InvalidSession);
    }
    Ok(())
}

pub fn snapshot(
    connection: &Connection,
) -> Result<AudiobookStatisticsSnapshot, AudioStatisticsError> {
    connection
        .query_row(
            "SELECT
                COALESCE((SELECT SUM(active_seconds) FROM audiobook_listening_events), 0),
                COALESCE((SELECT SUM(active_seconds) FROM audiobook_listening_events
                    WHERE date(occurred_at, 'unixepoch', 'localtime') = date('now', 'localtime')), 0),
                (SELECT COUNT(DISTINCT audiobook_id) FROM audiobook_listening_sessions),
                (SELECT COUNT(*) FROM audiobooks WHERE progress >= 0.995)",
            [],
            |row| Ok(AudiobookStatisticsSnapshot {
                total_active_seconds: row.get(0)?,
                today_active_seconds: row.get(1)?,
                audiobooks_started: row.get(2)?,
                audiobooks_completed: row.get(3)?,
            }),
        )
        .map_err(Into::into)
}

pub fn achievements(
    connection: &Connection,
) -> Result<Vec<AchievementProgress>, AudioStatisticsError> {
    let snapshot = snapshot(connection)?;
    let now = unix_seconds();
    for definition in ACHIEVEMENTS {
        if metric_value(&snapshot, definition.metric) >= definition.target {
            connection.execute(
                "INSERT INTO achievement_unlocks(achievement_id, unlocked_at)
                 VALUES (?1, ?2) ON CONFLICT(achievement_id) DO NOTHING",
                params![definition.id, now],
            )?;
        }
    }
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
                current: metric_value(&snapshot, definition.metric),
                target: definition.target,
                unlocked_at,
            })
        })
        .collect::<Result<Vec<_>, rusqlite::Error>>()
        .map_err(Into::into)
}

pub fn clear(connection: &Connection) -> Result<(), AudioStatisticsError> {
    connection.execute("DELETE FROM audiobook_listening_sessions", [])?;
    connection.execute(
        "DELETE FROM achievement_unlocks WHERE achievement_id LIKE 'audio_%'",
        [],
    )?;
    Ok(())
}

fn metric_value(snapshot: &AudiobookStatisticsSnapshot, metric: Metric) -> i64 {
    match metric {
        Metric::Started => snapshot.audiobooks_started,
        Metric::Completed => snapshot.audiobooks_completed,
        Metric::ActiveSeconds => snapshot.total_active_seconds,
    }
}

fn validate_progress(progress: f64) -> Result<(), AudioStatisticsError> {
    if progress.is_finite() && (0.0..=1.0).contains(&progress) {
        Ok(())
    } else {
        Err(AudioStatisticsError::InvalidActivity)
    }
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

    fn connection() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE audiobooks(id INTEGER PRIMARY KEY, progress REAL NOT NULL);
             INSERT INTO audiobooks VALUES(1, 1.0);
             CREATE TABLE achievement_unlocks(achievement_id TEXT PRIMARY KEY, unlocked_at INTEGER NOT NULL);
             CREATE TABLE audiobook_listening_sessions(
                id INTEGER PRIMARY KEY, token TEXT UNIQUE NOT NULL,
                audiobook_id INTEGER REFERENCES audiobooks(id), started_at INTEGER NOT NULL,
                last_seen_at INTEGER NOT NULL, last_progress REAL NOT NULL, ended_at INTEGER);
             CREATE TABLE audiobook_listening_events(
                id INTEGER PRIMARY KEY, session_id INTEGER REFERENCES audiobook_listening_sessions(id),
                occurred_at INTEGER NOT NULL, active_seconds INTEGER NOT NULL, progress REAL NOT NULL,
                UNIQUE(session_id, occurred_at));",
        ).unwrap();
        connection
    }

    #[test]
    fn credits_only_bounded_active_listening_and_unlocks_audio_goals() {
        let connection = connection();
        connection.execute(
            "INSERT INTO audiobook_listening_sessions(token, audiobook_id, started_at, last_seen_at, last_progress)
             VALUES('session', 1, 100, 100, 0)", [],
        ).unwrap();
        record_activity_at(&connection, "session", true, 0.5, 110).unwrap();
        record_activity_at(&connection, "session", true, 0.7, 200).unwrap();
        let value = snapshot(&connection).unwrap();
        assert_eq!(value.total_active_seconds, 10);
        assert_eq!(value.audiobooks_completed, 1);
        let achievements = achievements(&connection).unwrap();
        assert!(achievements
            .iter()
            .any(|item| item.id == "audio_first_finish" && item.unlocked_at.is_some()));
    }
}
