use crate::audio_importer::{
    discover_manual_audio_groups, discover_watched_audio_groups, inspect_audio_group,
    AudioGroupCandidate, AudioImportError, ImportedAudioGroup, MAX_AUDIOBOOK_BYTES,
    MAX_AUDIOBOOK_PARTS,
};
use crate::audio_statistics::{self, AudioStatisticsError, AudiobookStatisticsSnapshot};
use crate::importer::{inspect_book, supported_book_path, ImportError};
use crate::metadata::{
    download_open_library_cover, image_extension, MetadataCandidate, MetadataError,
    MetadataLanguage, MAX_COVER_BYTES,
};
use crate::reader::{read_document, DocumentModel, ReaderError};
use crate::special_reader::{prepare_special_document, SpecialDocument, SpecialReaderError};
use crate::statistics::{self, AchievementProgress, StatisticsError, StatisticsSnapshot};
use crate::steam::{self, SteamError, SteamIntegrationStatus, SteamSyncResult};
use rusqlite::{params, Connection, OpenFlags, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs,
    io::Read,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error;

const MAX_WATCHED_BOOK_FILES: usize = 100_000;

const MIGRATIONS: &[(i64, &str)] = &[
    (
        1,
        "CREATE TABLE app_metadata (
            key TEXT PRIMARY KEY NOT NULL,
            value TEXT NOT NULL
        ) STRICT;",
    ),
    (
        2,
        "CREATE TABLE books (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source_path TEXT NOT NULL UNIQUE,
            fingerprint TEXT NOT NULL UNIQUE,
            title TEXT NOT NULL,
            author TEXT NOT NULL DEFAULT '',
            format TEXT NOT NULL,
            file_size INTEGER NOT NULL,
            cover_path TEXT,
            added_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            last_seen_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            is_available INTEGER NOT NULL DEFAULT 1 CHECK (is_available IN (0, 1)),
            progress REAL NOT NULL DEFAULT 0 CHECK (progress >= 0 AND progress <= 1)
        ) STRICT;
        CREATE INDEX books_title_idx ON books(title);
        CREATE INDEX books_author_idx ON books(author);
        CREATE TABLE watched_folders (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            path TEXT NOT NULL UNIQUE,
            added_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            last_scanned_at TEXT
        ) STRICT;",
    ),
    (
        3,
        "ALTER TABLE books ADD COLUMN last_section INTEGER NOT NULL DEFAULT 0
            CHECK (last_section >= 0);
        ALTER TABLE books ADD COLUMN section_progress REAL NOT NULL DEFAULT 0
            CHECK (section_progress >= 0 AND section_progress <= 1);",
    ),
    (
        4,
        "CREATE TABLE annotations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            book_id INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
            kind TEXT NOT NULL CHECK (kind IN ('bookmark', 'highlight', 'note', 'quote')),
            section_id TEXT NOT NULL,
            block_index INTEGER NOT NULL CHECK (block_index >= 0),
            start_offset INTEGER NOT NULL CHECK (start_offset >= 0),
            end_offset INTEGER NOT NULL CHECK (end_offset >= start_offset),
            selected_text TEXT NOT NULL DEFAULT '',
            note TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        ) STRICT;
        CREATE INDEX annotations_book_location_idx
            ON annotations(book_id, section_id, block_index);
        CREATE VIRTUAL TABLE book_search USING fts5(
            book_id UNINDEXED,
            section_id UNINDEXED,
            block_index UNINDEXED,
            section_title,
            body,
            tokenize = 'unicode61 remove_diacritics 2'
        );",
    ),
    (
        5,
        "ALTER TABLE books ADD COLUMN subtitle TEXT NOT NULL DEFAULT '';
        ALTER TABLE books ADD COLUMN isbn TEXT NOT NULL DEFAULT '';
        ALTER TABLE books ADD COLUMN publisher TEXT NOT NULL DEFAULT '';
        ALTER TABLE books ADD COLUMN published_year TEXT NOT NULL DEFAULT '';
        ALTER TABLE books ADD COLUMN language TEXT NOT NULL DEFAULT '';
        ALTER TABLE books ADD COLUMN series TEXT NOT NULL DEFAULT '';
        ALTER TABLE books ADD COLUMN description TEXT NOT NULL DEFAULT '';
        ALTER TABLE books ADD COLUMN metadata_source TEXT NOT NULL DEFAULT 'embedded';
        ALTER TABLE books ADD COLUMN metadata_provider_id TEXT;
        ALTER TABLE books ADD COLUMN metadata_updated_at TEXT;
        ALTER TABLE books ADD COLUMN embedded_cover_path TEXT;
        ALTER TABLE books ADD COLUMN cover_source TEXT NOT NULL DEFAULT 'embedded';
        UPDATE books SET embedded_cover_path = cover_path WHERE cover_path IS NOT NULL;
        CREATE TABLE metadata_cache (
            query_key TEXT PRIMARY KEY NOT NULL,
            response_json TEXT NOT NULL,
            fetched_at INTEGER NOT NULL
        ) STRICT;",
    ),
    (
        6,
        "CREATE TABLE reading_sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            token TEXT NOT NULL UNIQUE,
            book_id INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
            started_at INTEGER NOT NULL,
            last_seen_at INTEGER NOT NULL,
            ended_at INTEGER,
            last_progress REAL NOT NULL CHECK (last_progress >= 0 AND last_progress <= 1),
            last_words INTEGER NOT NULL DEFAULT 0 CHECK (last_words >= 0),
            last_pages INTEGER NOT NULL DEFAULT 0 CHECK (last_pages >= 0)
        ) STRICT;
        CREATE INDEX reading_sessions_book_idx ON reading_sessions(book_id, started_at);
        CREATE TABLE reading_activity_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id INTEGER NOT NULL REFERENCES reading_sessions(id) ON DELETE CASCADE,
            occurred_at INTEGER NOT NULL,
            active_seconds INTEGER NOT NULL DEFAULT 0 CHECK (active_seconds >= 0),
            words_read INTEGER NOT NULL DEFAULT 0 CHECK (words_read >= 0),
            pages_read INTEGER NOT NULL DEFAULT 0 CHECK (pages_read >= 0),
            progress REAL NOT NULL CHECK (progress >= 0 AND progress <= 1),
            UNIQUE(session_id, occurred_at)
        ) STRICT;
        CREATE INDEX reading_activity_day_idx
            ON reading_activity_events(occurred_at, session_id);
        CREATE TABLE achievement_unlocks (
            achievement_id TEXT PRIMARY KEY NOT NULL,
            unlocked_at INTEGER NOT NULL
        ) STRICT;
        INSERT INTO app_metadata(key, value) VALUES('daily_goal_minutes', '20')
            ON CONFLICT(key) DO NOTHING;",
    ),
    (
        7,
        "CREATE TABLE achievement_sync_queue (
            achievement_id TEXT PRIMARY KEY NOT NULL,
            unlocked_at INTEGER NOT NULL,
            attempts INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
            last_attempt_at INTEGER,
            last_error TEXT,
            synced_at INTEGER
        ) STRICT;
        CREATE INDEX achievement_sync_pending_idx
            ON achievement_sync_queue(synced_at, unlocked_at);
        INSERT OR IGNORE INTO achievement_sync_queue(achievement_id, unlocked_at)
            SELECT achievement_id, unlocked_at FROM achievement_unlocks;",
    ),
    (
        8,
        "ALTER TABLE books ADD COLUMN last_opened_at INTEGER;
        UPDATE books
            SET last_opened_at = CAST(strftime('%s', last_seen_at) AS INTEGER)
            WHERE progress > 0 AND progress < 0.995;
        CREATE INDEX books_last_opened_idx ON books(last_opened_at DESC);",
    ),
    (
        9,
        "ALTER TABLE books ADD COLUMN is_favorite INTEGER NOT NULL DEFAULT 0
            CHECK (is_favorite IN (0, 1));
        CREATE INDEX books_favorite_idx ON books(is_favorite, title);",
    ),
    (
        10,
        "ALTER TABLE books ADD COLUMN genres TEXT NOT NULL DEFAULT '';",
    ),
    (
        11,
        "CREATE TABLE audiobooks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            group_key TEXT NOT NULL UNIQUE,
            title TEXT NOT NULL,
            author TEXT NOT NULL DEFAULT '',
            cover_path TEXT,
            added_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            is_available INTEGER NOT NULL DEFAULT 1 CHECK (is_available IN (0, 1)),
            total_size INTEGER NOT NULL DEFAULT 0 CHECK (total_size >= 0),
            part_count INTEGER NOT NULL DEFAULT 0 CHECK (part_count >= 0 AND part_count <= 1000),
            total_duration_seconds REAL NOT NULL DEFAULT 0 CHECK (total_duration_seconds >= 0),
            progress REAL NOT NULL DEFAULT 0 CHECK (progress >= 0 AND progress <= 1),
            last_part_index INTEGER NOT NULL DEFAULT 0 CHECK (last_part_index >= 0),
            last_position_seconds REAL NOT NULL DEFAULT 0 CHECK (last_position_seconds >= 0)
        ) STRICT;
        CREATE INDEX audiobooks_title_idx ON audiobooks(title);
        CREATE TABLE audiobook_parts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            audiobook_id INTEGER NOT NULL REFERENCES audiobooks(id) ON DELETE CASCADE,
            source_path TEXT NOT NULL UNIQUE,
            fingerprint TEXT NOT NULL,
            title TEXT NOT NULL,
            format TEXT NOT NULL,
            file_size INTEGER NOT NULL CHECK (file_size >= 0),
            duration_seconds REAL CHECK (duration_seconds IS NULL OR duration_seconds >= 0),
            ordinal INTEGER NOT NULL CHECK (ordinal >= 0 AND ordinal < 1000),
            is_available INTEGER NOT NULL DEFAULT 1 CHECK (is_available IN (0, 1)),
            last_seen_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        ) STRICT;
        CREATE INDEX audiobook_parts_book_order_idx
            ON audiobook_parts(audiobook_id, ordinal);
        CREATE INDEX audiobook_parts_fingerprint_idx
            ON audiobook_parts(fingerprint);
        CREATE TABLE watched_audio_folders (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            path TEXT NOT NULL UNIQUE,
            added_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            last_scanned_at TEXT
        ) STRICT;",
    ),
    (
        12,
        "CREATE TABLE audiobook_bookmarks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            audiobook_id INTEGER NOT NULL REFERENCES audiobooks(id) ON DELETE CASCADE,
            part_index INTEGER NOT NULL CHECK (part_index >= 0 AND part_index < 1000),
            position_seconds REAL NOT NULL CHECK (position_seconds >= 0),
            note TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        ) STRICT;
        CREATE INDEX audiobook_bookmarks_location_idx
            ON audiobook_bookmarks(audiobook_id, part_index, position_seconds);
        CREATE TABLE audiobook_chapters (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            audiobook_id INTEGER NOT NULL REFERENCES audiobooks(id) ON DELETE CASCADE,
            part_index INTEGER NOT NULL CHECK (part_index >= 0 AND part_index < 1000),
            title TEXT NOT NULL,
            start_seconds REAL NOT NULL CHECK (start_seconds >= 0),
            ordinal INTEGER NOT NULL CHECK (ordinal >= 0 AND ordinal < 10000),
            UNIQUE(audiobook_id, ordinal)
        ) STRICT;
        CREATE INDEX audiobook_chapters_location_idx
            ON audiobook_chapters(audiobook_id, part_index, start_seconds);",
    ),
    (
        13,
        "ALTER TABLE audiobooks ADD COLUMN narrator TEXT NOT NULL DEFAULT '';
        ALTER TABLE audiobooks ADD COLUMN series TEXT NOT NULL DEFAULT '';
        ALTER TABLE audiobooks ADD COLUMN genres TEXT NOT NULL DEFAULT '';
        ALTER TABLE audiobooks ADD COLUMN description TEXT NOT NULL DEFAULT '';
        ALTER TABLE audiobooks ADD COLUMN language TEXT NOT NULL DEFAULT '';
        ALTER TABLE audiobooks ADD COLUMN published_year TEXT NOT NULL DEFAULT '';
        ALTER TABLE audiobooks ADD COLUMN metadata_source TEXT NOT NULL DEFAULT 'filename';
        ALTER TABLE audiobooks ADD COLUMN metadata_provider_id TEXT;
        ALTER TABLE audiobooks ADD COLUMN metadata_updated_at TEXT;
        ALTER TABLE audiobooks ADD COLUMN cover_source TEXT NOT NULL DEFAULT 'none';
        CREATE TABLE audiobook_listening_sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            token TEXT NOT NULL UNIQUE,
            audiobook_id INTEGER NOT NULL REFERENCES audiobooks(id) ON DELETE CASCADE,
            started_at INTEGER NOT NULL,
            last_seen_at INTEGER NOT NULL,
            last_progress REAL NOT NULL CHECK(last_progress >= 0 AND last_progress <= 1),
            ended_at INTEGER
        ) STRICT;
        CREATE TABLE audiobook_listening_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id INTEGER NOT NULL REFERENCES audiobook_listening_sessions(id) ON DELETE CASCADE,
            occurred_at INTEGER NOT NULL,
            active_seconds INTEGER NOT NULL CHECK(active_seconds >= 0 AND active_seconds <= 30),
            progress REAL NOT NULL CHECK(progress >= 0 AND progress <= 1),
            UNIQUE(session_id, occurred_at)
        ) STRICT;
        CREATE INDEX audiobook_listening_events_time_idx
            ON audiobook_listening_events(occurred_at);",
    ),
];

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("file operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("the local library failed its integrity check: {0}")]
    IntegrityCheck(String),
    #[error(transparent)]
    Import(#[from] ImportError),
    #[error(transparent)]
    AudioImport(#[from] AudioImportError),
    #[error(transparent)]
    Reader(#[from] ReaderError),
    #[error(transparent)]
    SpecialReader(#[from] SpecialReaderError),
    #[error("the selected watched folder does not exist")]
    MissingFolder,
    #[error("the watched folder contains more than 100,000 supported book files")]
    TooManyWatchedBooks,
    #[error("the selected book does not exist")]
    MissingBook,
    #[error("the selected audiobook part does not exist or is unavailable")]
    MissingAudioPart,
    #[error("the selected audiobook launch file is invalid or resolves ambiguously")]
    InvalidAudioLaunch,
    #[error("the book selection is invalid")]
    InvalidBookSelection,
    #[error("the reading position is invalid")]
    InvalidPosition,
    #[error("the audiobook bookmark is invalid")]
    InvalidAudioBookmark,
    #[error("the annotation is invalid")]
    InvalidAnnotation,
    #[error("the search query is invalid")]
    InvalidSearch,
    #[error("the metadata values are invalid")]
    InvalidMetadata,
    #[error("Open Library allows one unidentified request per second; try again")]
    MetadataRateLimited,
    #[error(transparent)]
    Metadata(#[from] MetadataError),
    #[error(transparent)]
    Statistics(#[from] StatisticsError),
    #[error(transparent)]
    AudioStatistics(#[from] AudioStatisticsError),
    #[error(transparent)]
    Steam(#[from] SteamError),
    #[error("cached metadata is invalid: {0}")]
    MetadataJson(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BookRecord {
    pub id: i64,
    pub source_path: String,
    pub title: String,
    pub author: String,
    pub format: String,
    pub file_size: i64,
    pub cover_path: Option<String>,
    pub added_at: String,
    pub is_available: bool,
    pub progress: f64,
    pub subtitle: String,
    pub isbn: String,
    pub publisher: String,
    pub published_year: String,
    pub language: String,
    pub series: String,
    pub genres: String,
    pub description: String,
    pub metadata_source: String,
    pub metadata_provider_id: Option<String>,
    pub metadata_updated_at: Option<String>,
    pub cover_source: String,
    pub last_opened_at: Option<i64>,
    pub is_favorite: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookMetadataInput {
    pub title: String,
    pub author: String,
    pub subtitle: String,
    pub isbn: String,
    pub publisher: String,
    pub published_year: String,
    pub language: String,
    pub series: String,
    pub genres: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchedFolder {
    pub id: i64,
    pub path: String,
    pub last_scanned_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudiobookRecord {
    pub id: i64,
    pub title: String,
    pub author: String,
    pub cover_path: Option<String>,
    pub added_at: String,
    pub is_available: bool,
    pub total_size: i64,
    pub part_count: i64,
    pub total_duration_seconds: f64,
    pub progress: f64,
    pub last_part_index: i64,
    pub last_position_seconds: f64,
    pub narrator: String,
    pub series: String,
    pub genres: String,
    pub description: String,
    pub language: String,
    pub published_year: String,
    pub metadata_source: String,
    pub metadata_provider_id: Option<String>,
    pub metadata_updated_at: Option<String>,
    pub cover_source: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudiobookMetadataInput {
    pub title: String,
    pub author: String,
    pub narrator: String,
    pub series: String,
    pub genres: String,
    pub description: String,
    pub language: String,
    pub published_year: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudiobookPartRecord {
    pub id: i64,
    pub audiobook_id: i64,
    pub source_path: String,
    pub title: String,
    pub format: String,
    pub file_size: i64,
    pub duration_seconds: Option<f64>,
    pub ordinal: i64,
    pub is_available: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudiobookBookmarkRecord {
    pub id: i64,
    pub audiobook_id: i64,
    pub part_index: i64,
    pub position_seconds: f64,
    pub note: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudiobookChapterRecord {
    pub id: i64,
    pub audiobook_id: i64,
    pub part_index: i64,
    pub title: String,
    pub start_seconds: f64,
    pub ordinal: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchedAudioFolder {
    pub id: i64,
    pub path: String,
    pub last_scanned_at: Option<String>,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioImportSummary {
    pub imported_books: usize,
    pub imported_parts: usize,
    pub duplicate_parts: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}

struct AudioGroupImportResult {
    audiobook_id: Option<i64>,
    imported_book: bool,
    imported_parts: usize,
    duplicate_parts: usize,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub imported: usize,
    pub duplicates: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadingLocator {
    pub section_id: String,
    pub block_index: usize,
    pub start_offset: usize,
    pub end_offset: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationRecord {
    pub id: i64,
    pub book_id: i64,
    pub kind: String,
    pub locator: ReadingLocator,
    pub selected_text: String,
    pub note: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub section_id: String,
    pub section_title: String,
    pub block_index: usize,
    pub excerpt: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupHealth {
    pub previous_exit_unclean: bool,
    pub recovered_from_backup: bool,
    pub quarantined_database: Option<String>,
}

pub struct Database {
    connection: Connection,
    cover_dir: PathBuf,
    backup_dir: PathBuf,
    reader_cache_dir: PathBuf,
    startup_health: StartupHealth,
}

impl Database {
    pub fn open(
        database_path: &Path,
        cover_dir: PathBuf,
        backup_dir: PathBuf,
    ) -> Result<Self, DatabaseError> {
        fs::create_dir_all(&cover_dir)?;
        fs::create_dir_all(&backup_dir)?;
        let reader_cache_dir = cover_dir
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("readers");
        fs::create_dir_all(&reader_cache_dir)?;
        let (connection, recovered_from_backup, quarantined_database) =
            match Self::open_connection(database_path) {
                Ok(connection) => (connection, false, None),
                Err(original_error) if database_path.is_file() => {
                    let Some(backup) = Self::latest_valid_backup(&backup_dir) else {
                        return Err(original_error);
                    };
                    let quarantined = Self::quarantine_database(database_path)?;
                    if let Err(copy_error) = fs::copy(&backup, database_path) {
                        let _ = fs::copy(&quarantined, database_path);
                        return Err(copy_error.into());
                    }
                    match Self::open_connection(database_path) {
                        Ok(connection) => (
                            connection,
                            true,
                            Some(quarantined.to_string_lossy().into_owned()),
                        ),
                        Err(recovery_error) => {
                            let _ = fs::copy(&quarantined, database_path);
                            return Err(recovery_error);
                        }
                    }
                }
                Err(error) => return Err(error),
            };
        let previous_exit_unclean = connection
            .query_row(
                "SELECT value FROM app_metadata WHERE key = 'last_exit_clean'",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .is_some_and(|value| value == "0");
        connection.execute(
            "INSERT INTO app_metadata(key, value) VALUES('last_exit_clean', '0')
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [],
        )?;
        Ok(Self {
            connection,
            cover_dir,
            backup_dir,
            reader_cache_dir,
            startup_health: StartupHealth {
                previous_exit_unclean,
                recovered_from_backup,
                quarantined_database,
            },
        })
    }

    fn open_connection(database_path: &Path) -> Result<Connection, DatabaseError> {
        let mut connection = Connection::open(database_path)?;
        connection.pragma_update(None, "foreign_keys", true)?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        Self::migrate(&mut connection)?;
        let check =
            connection.query_row("PRAGMA quick_check", [], |row| row.get::<_, String>(0))?;
        if check != "ok" {
            return Err(DatabaseError::IntegrityCheck(check));
        }
        Ok(connection)
    }

    fn latest_valid_backup(backup_dir: &Path) -> Option<PathBuf> {
        let mut backups = fs::read_dir(backup_dir)
            .ok()?
            .filter_map(Result::ok)
            .filter(|entry| {
                entry.file_type().is_ok_and(|kind| kind.is_file())
                    && entry.file_name().to_string_lossy().starts_with("library-")
            })
            .map(|entry| entry.path())
            .collect::<Vec<_>>();
        backups.sort();
        backups.into_iter().rev().find(|path| {
            Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
                .and_then(|connection| {
                    connection.query_row("PRAGMA quick_check", [], |row| row.get::<_, String>(0))
                })
                .is_ok_and(|check| check == "ok")
        })
    }

    fn quarantine_database(database_path: &Path) -> Result<PathBuf, DatabaseError> {
        let recovery_dir = database_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("recovery");
        fs::create_dir_all(&recovery_dir)?;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let quarantined = recovery_dir.join(format!("library-corrupt-{timestamp}.db"));
        fs::rename(database_path, &quarantined)?;
        for suffix in ["-wal", "-shm"] {
            let sidecar = PathBuf::from(format!("{}{suffix}", database_path.to_string_lossy()));
            if sidecar.is_file() {
                let target = recovery_dir.join(format!("library-corrupt-{timestamp}.db{suffix}"));
                fs::rename(sidecar, target)?;
            }
        }
        Ok(quarantined)
    }

    pub fn startup_health(&self) -> StartupHealth {
        self.startup_health.clone()
    }

    pub fn mark_clean_shutdown(&self) -> Result<(), DatabaseError> {
        self.connection.execute(
            "INSERT INTO app_metadata(key, value) VALUES('last_exit_clean', '1')
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [],
        )?;
        Ok(())
    }

    fn migrate(connection: &mut Connection) -> Result<(), DatabaseError> {
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY NOT NULL,
                applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            ) STRICT;",
        )?;
        for (version, sql) in MIGRATIONS {
            let applied = connection
                .query_row(
                    "SELECT version FROM schema_migrations WHERE version = ?1",
                    [version],
                    |row| row.get::<_, i64>(0),
                )
                .optional()?
                .is_some();
            if !applied {
                let transaction = connection.transaction()?;
                transaction.execute_batch(sql)?;
                transaction.execute(
                    "INSERT INTO schema_migrations(version) VALUES (?1)",
                    [version],
                )?;
                transaction.commit()?;
            }
        }
        Ok(())
    }

    pub fn list_books(&mut self) -> Result<Vec<BookRecord>, DatabaseError> {
        self.refresh_availability()?;
        let mut statement = self.connection.prepare(
            "SELECT id, source_path, title, author, format, file_size, cover_path,
                    added_at, is_available, progress, subtitle, isbn, publisher,
                    published_year, language, series, genres, description, metadata_source,
                    metadata_provider_id, metadata_updated_at, cover_source,
                    last_opened_at, is_favorite
             FROM books ORDER BY title COLLATE NOCASE, author COLLATE NOCASE",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(BookRecord {
                id: row.get(0)?,
                source_path: row.get(1)?,
                title: row.get(2)?,
                author: row.get(3)?,
                format: row.get(4)?,
                file_size: row.get(5)?,
                cover_path: row.get(6)?,
                added_at: row.get(7)?,
                is_available: row.get::<_, i64>(8)? == 1,
                progress: row.get(9)?,
                subtitle: row.get(10)?,
                isbn: row.get(11)?,
                publisher: row.get(12)?,
                published_year: row.get(13)?,
                language: row.get(14)?,
                series: row.get(15)?,
                genres: row.get(16)?,
                description: row.get(17)?,
                metadata_source: row.get(18)?,
                metadata_provider_id: row.get(19)?,
                metadata_updated_at: row.get(20)?,
                cover_source: row.get(21)?,
                last_opened_at: row.get(22)?,
                is_favorite: row.get::<_, i64>(23)? == 1,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn list_audiobooks(&mut self) -> Result<Vec<AudiobookRecord>, DatabaseError> {
        self.refresh_audio_availability()?;
        let mut statement = self.connection.prepare(
            "SELECT id, title, author, cover_path, added_at, is_available,
                    total_size, part_count, total_duration_seconds, progress,
                    last_part_index, last_position_seconds, narrator, series,
                    genres, description, language, published_year,
                    metadata_source, metadata_provider_id, metadata_updated_at,
                    cover_source
             FROM audiobooks ORDER BY title COLLATE NOCASE, author COLLATE NOCASE",
        )?;
        let rows = statement.query_map([], audiobook_from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    fn audiobook_by_id(&self, audiobook_id: i64) -> Result<AudiobookRecord, DatabaseError> {
        self.connection
            .query_row(
                "SELECT id, title, author, cover_path, added_at, is_available,
                        total_size, part_count, total_duration_seconds, progress,
                        last_part_index, last_position_seconds, narrator, series,
                        genres, description, language, published_year,
                        metadata_source, metadata_provider_id, metadata_updated_at,
                        cover_source
                 FROM audiobooks WHERE id = ?1",
                [audiobook_id],
                audiobook_from_row,
            )
            .optional()?
            .ok_or(DatabaseError::MissingBook)
    }

    pub fn list_audiobook_parts(
        &mut self,
        audiobook_id: i64,
    ) -> Result<Vec<AudiobookPartRecord>, DatabaseError> {
        self.refresh_audio_availability()?;
        let exists = self
            .connection
            .query_row(
                "SELECT id FROM audiobooks WHERE id = ?1",
                [audiobook_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
            .is_some();
        if !exists {
            return Err(DatabaseError::MissingBook);
        }
        let mut statement = self.connection.prepare(
            "SELECT id, audiobook_id, source_path, title, format, file_size,
                    duration_seconds, ordinal, is_available
             FROM audiobook_parts WHERE audiobook_id = ?1
             ORDER BY ordinal, title COLLATE NOCASE",
        )?;
        let rows = statement.query_map([audiobook_id], audiobook_part_from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn save_audiobook_position(
        &mut self,
        audiobook_id: i64,
        part_index: i64,
        position_seconds: f64,
        duration_seconds: f64,
    ) -> Result<AudiobookRecord, DatabaseError> {
        if part_index < 0
            || !position_seconds.is_finite()
            || !duration_seconds.is_finite()
            || position_seconds < 0.0
            || duration_seconds <= 0.0
            || position_seconds > duration_seconds + 1.0
        {
            return Err(DatabaseError::InvalidPosition);
        }
        let transaction = self.connection.transaction()?;
        let part_id = transaction
            .query_row(
                "SELECT id FROM audiobook_parts
                 WHERE audiobook_id = ?1 AND ordinal = ?2 AND is_available = 1",
                params![audiobook_id, part_index],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
            .ok_or(DatabaseError::MissingAudioPart)?;
        transaction.execute(
            "UPDATE audiobook_parts SET duration_seconds = ?1 WHERE id = ?2",
            params![duration_seconds, part_id],
        )?;
        let (part_count, known_count, total_duration, elapsed_before) = transaction.query_row(
            "SELECT COUNT(*), COUNT(duration_seconds),
                    COALESCE(SUM(duration_seconds), 0),
                    COALESCE(SUM(CASE WHEN ordinal < ?2
                                      THEN duration_seconds ELSE 0 END), 0)
             FROM audiobook_parts WHERE audiobook_id = ?1",
            params![audiobook_id, part_index],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, f64>(2)?,
                    row.get::<_, f64>(3)?,
                ))
            },
        )?;
        if part_count == 0 {
            return Err(DatabaseError::MissingBook);
        }
        let bounded_position = position_seconds.min(duration_seconds);
        let progress = if known_count == part_count && total_duration > 0.0 {
            (elapsed_before + bounded_position) / total_duration
        } else {
            (part_index as f64 + bounded_position / duration_seconds) / part_count as f64
        }
        .clamp(0.0, 1.0);
        let changed = transaction.execute(
            "UPDATE audiobooks
             SET total_duration_seconds = ?1, progress = ?2,
                 last_part_index = ?3, last_position_seconds = ?4,
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = ?5",
            params![
                total_duration,
                progress,
                part_index,
                bounded_position,
                audiobook_id
            ],
        )?;
        if changed == 0 {
            return Err(DatabaseError::MissingBook);
        }
        transaction.commit()?;
        self.audiobook_by_id(audiobook_id)
    }

    pub fn list_audiobook_bookmarks(
        &self,
        audiobook_id: i64,
    ) -> Result<Vec<AudiobookBookmarkRecord>, DatabaseError> {
        if self.audiobook_by_id(audiobook_id).is_err() {
            return Err(DatabaseError::MissingBook);
        }
        let mut statement = self.connection.prepare(
            "SELECT id, audiobook_id, part_index, position_seconds, note, created_at
             FROM audiobook_bookmarks WHERE audiobook_id = ?1
             ORDER BY part_index, position_seconds, id",
        )?;
        let rows = statement.query_map([audiobook_id], |row| {
            Ok(AudiobookBookmarkRecord {
                id: row.get(0)?,
                audiobook_id: row.get(1)?,
                part_index: row.get(2)?,
                position_seconds: row.get(3)?,
                note: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn create_audiobook_bookmark(
        &mut self,
        audiobook_id: i64,
        part_index: i64,
        position_seconds: f64,
        note: &str,
    ) -> Result<AudiobookBookmarkRecord, DatabaseError> {
        let normalized_note = note.split_whitespace().collect::<Vec<_>>().join(" ");
        if part_index < 0
            || !position_seconds.is_finite()
            || position_seconds < 0.0
            || normalized_note.chars().count() > 512
        {
            return Err(DatabaseError::InvalidAudioBookmark);
        }
        let duration = self
            .connection
            .query_row(
                "SELECT duration_seconds FROM audiobook_parts
                 WHERE audiobook_id = ?1 AND ordinal = ?2 AND is_available = 1",
                params![audiobook_id, part_index],
                |row| row.get::<_, Option<f64>>(0),
            )
            .optional()?
            .ok_or(DatabaseError::MissingAudioPart)?;
        if duration.is_some_and(|value| position_seconds > value + 1.0) {
            return Err(DatabaseError::InvalidAudioBookmark);
        }
        self.connection.execute(
            "INSERT INTO audiobook_bookmarks(
                audiobook_id, part_index, position_seconds, note
             ) VALUES (?1, ?2, ?3, ?4)",
            params![audiobook_id, part_index, position_seconds, normalized_note],
        )?;
        let id = self.connection.last_insert_rowid();
        self.connection
            .query_row(
                "SELECT id, audiobook_id, part_index, position_seconds, note, created_at
                 FROM audiobook_bookmarks WHERE id = ?1",
                [id],
                |row| {
                    Ok(AudiobookBookmarkRecord {
                        id: row.get(0)?,
                        audiobook_id: row.get(1)?,
                        part_index: row.get(2)?,
                        position_seconds: row.get(3)?,
                        note: row.get(4)?,
                        created_at: row.get(5)?,
                    })
                },
            )
            .map_err(Into::into)
    }

    pub fn delete_audiobook_bookmark(&mut self, bookmark_id: i64) -> Result<(), DatabaseError> {
        if bookmark_id <= 0 {
            return Err(DatabaseError::InvalidAudioBookmark);
        }
        let changed = self.connection.execute(
            "DELETE FROM audiobook_bookmarks WHERE id = ?1",
            [bookmark_id],
        )?;
        if changed == 0 {
            return Err(DatabaseError::InvalidAudioBookmark);
        }
        Ok(())
    }

    pub fn list_audiobook_chapters(
        &self,
        audiobook_id: i64,
    ) -> Result<Vec<AudiobookChapterRecord>, DatabaseError> {
        if self.audiobook_by_id(audiobook_id).is_err() {
            return Err(DatabaseError::MissingBook);
        }
        let mut statement = self.connection.prepare(
            "SELECT id, audiobook_id, part_index, title, start_seconds, ordinal
             FROM audiobook_chapters WHERE audiobook_id = ?1
             ORDER BY ordinal",
        )?;
        let rows = statement.query_map([audiobook_id], |row| {
            Ok(AudiobookChapterRecord {
                id: row.get(0)?,
                audiobook_id: row.get(1)?,
                part_index: row.get(2)?,
                title: row.get(3)?,
                start_seconds: row.get(4)?,
                ordinal: row.get(5)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn update_audiobook_metadata(
        &mut self,
        audiobook_id: i64,
        input: &AudiobookMetadataInput,
    ) -> Result<AudiobookRecord, DatabaseError> {
        validate_audiobook_metadata_input(input)?;
        let changed = self.connection.execute(
            "UPDATE audiobooks SET title = ?1, author = ?2, narrator = ?3,
                series = ?4, genres = ?5, description = ?6, language = ?7,
                published_year = ?8, metadata_source = 'manual',
                metadata_provider_id = NULL, metadata_updated_at = CURRENT_TIMESTAMP,
                updated_at = CURRENT_TIMESTAMP WHERE id = ?9",
            params![
                input.title.trim(),
                input.author.trim(),
                input.narrator.trim(),
                input.series.trim(),
                normalize_genres_input(&input.genres),
                input.description.trim(),
                input.language.trim(),
                input.published_year.trim(),
                audiobook_id,
            ],
        )?;
        if changed == 0 {
            return Err(DatabaseError::MissingBook);
        }
        self.create_backup()?;
        self.audiobook_by_id(audiobook_id)
    }

    pub fn search_audiobook_metadata(
        &mut self,
        audiobook_id: i64,
        explicit_query: &str,
        language: &str,
    ) -> Result<Vec<MetadataCandidate>, DatabaseError> {
        let language = MetadataLanguage::parse(language)?;
        let (title, author) = self
            .connection
            .query_row(
                "SELECT title, author FROM audiobooks WHERE id = ?1",
                [audiobook_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?
            .ok_or(DatabaseError::MissingBook)?;
        let query = if explicit_query.trim().is_empty() {
            format!("{title} {author}")
        } else {
            explicit_query.to_owned()
        };
        let query = normalize_metadata_query(&query).ok_or(DatabaseError::InvalidMetadata)?;
        let now = unix_seconds();
        let cache_key = format!("audio:{}:{}", language.code(), query.to_lowercase());
        if let Some(cached) = self
            .connection
            .query_row(
                "SELECT response_json FROM metadata_cache
                 WHERE query_key = ?1 AND fetched_at >= ?2",
                params![cache_key, now.saturating_sub(30 * 24 * 60 * 60)],
                |row| row.get::<_, String>(0),
            )
            .optional()?
        {
            return serde_json::from_str(&cached).map_err(Into::into);
        }
        self.reserve_metadata_request(false)?;
        let candidates = crate::metadata::search_metadata(&query, language)?;
        self.connection.execute(
            "INSERT INTO metadata_cache(query_key, response_json, fetched_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(query_key) DO UPDATE SET
                response_json = excluded.response_json,
                fetched_at = excluded.fetched_at",
            params![cache_key, serde_json::to_string(&candidates)?, now],
        )?;
        Ok(candidates)
    }

    pub fn apply_audiobook_metadata_candidate(
        &mut self,
        audiobook_id: i64,
        candidate: &MetadataCandidate,
    ) -> Result<AudiobookRecord, DatabaseError> {
        validate_candidate(candidate)?;
        let previous_cover = self
            .connection
            .query_row(
                "SELECT cover_path FROM audiobooks WHERE id = ?1",
                [audiobook_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .ok_or(DatabaseError::MissingBook)?;
        let downloaded_cover = if candidate.provider == "Open Library" {
            if let Some(cover_id) = candidate.cover_id {
                self.reserve_metadata_request(true)?;
                let (bytes, extension) = download_open_library_cover(cover_id)?;
                let path = self.cover_dir.join(format!(
                    "external-audio-{audiobook_id}-{cover_id}.{extension}"
                ));
                fs::write(&path, bytes)?;
                Some(path.to_string_lossy().into_owned())
            } else {
                None
            }
        } else {
            None
        };
        let source = candidate.provider.to_lowercase().replace(' ', "_");
        let changed = self.connection.execute(
            "UPDATE audiobooks SET title = ?1, author = ?2,
                series = CASE WHEN TRIM(?3) = '' THEN series ELSE ?3 END,
                genres = CASE WHEN TRIM(?4) = '' THEN genres ELSE ?4 END,
                language = ?5, published_year = ?6, metadata_source = ?7,
                metadata_provider_id = ?8, metadata_updated_at = CURRENT_TIMESTAMP,
                cover_path = COALESCE(?9, cover_path),
                cover_source = CASE WHEN ?9 IS NULL THEN cover_source ELSE 'open_library' END,
                updated_at = CURRENT_TIMESTAMP WHERE id = ?10",
            params![
                candidate.title.trim(),
                candidate.author.trim(),
                candidate.series.trim(),
                normalize_genres_input(&candidate.genres),
                candidate.language.trim(),
                candidate.published_year.trim(),
                source,
                candidate.provider_id.trim(),
                downloaded_cover,
                audiobook_id,
            ],
        )?;
        if changed == 0 {
            return Err(DatabaseError::MissingBook);
        }
        if let Some(previous) = previous_cover {
            if Some(previous.as_str()) != downloaded_cover.as_deref() {
                remove_managed_external_cover(&self.cover_dir, &previous)?;
            }
        }
        self.create_backup()?;
        self.audiobook_by_id(audiobook_id)
    }

    pub fn set_audiobook_local_cover(
        &mut self,
        audiobook_id: i64,
        source: &Path,
    ) -> Result<AudiobookRecord, DatabaseError> {
        let previous = self
            .connection
            .query_row(
                "SELECT cover_path FROM audiobooks WHERE id = ?1",
                [audiobook_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .ok_or(DatabaseError::MissingBook)?;
        let metadata = fs::metadata(source)?;
        if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_COVER_BYTES as u64 {
            return Err(DatabaseError::InvalidMetadata);
        }
        let mut bytes = Vec::with_capacity(metadata.len() as usize);
        fs::File::open(source)?
            .take(MAX_COVER_BYTES as u64 + 1)
            .read_to_end(&mut bytes)?;
        let extension = image_extension(&bytes).ok_or(MetadataError::InvalidCover)?;
        let destination = self.cover_dir.join(format!(
            "external-audio-{audiobook_id}-local-{}.{}",
            unix_millis(),
            extension
        ));
        fs::write(&destination, &bytes)?;
        let destination = destination.to_string_lossy().into_owned();
        self.connection.execute(
            "UPDATE audiobooks SET cover_path = ?1, cover_source = 'local',
             updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
            params![destination, audiobook_id],
        )?;
        if let Some(previous) = previous {
            if previous != destination {
                remove_managed_external_cover(&self.cover_dir, &previous)?;
            }
        }
        self.create_backup()?;
        self.audiobook_by_id(audiobook_id)
    }

    pub fn import_audiobooks(
        &mut self,
        paths: &[PathBuf],
    ) -> Result<AudioImportSummary, DatabaseError> {
        let discovery = discover_manual_audio_groups(paths);
        self.import_audio_groups(discovery.groups, discovery.errors)
    }

    pub fn import_audiobook_for_open(
        &mut self,
        path: &Path,
    ) -> Result<AudiobookRecord, DatabaseError> {
        let discovery = discover_manual_audio_groups(&[path.to_path_buf()]);
        if !discovery.errors.is_empty() || discovery.groups.len() != 1 {
            return Err(DatabaseError::InvalidAudioLaunch);
        }
        let candidate = discovery
            .groups
            .into_iter()
            .next()
            .ok_or(DatabaseError::InvalidAudioLaunch)?;
        let mut group = inspect_audio_group(
            &candidate.paths,
            candidate.group_as_folder,
            candidate.preserve_order,
            candidate.title_override.as_deref(),
        )?;
        if let Some(group_key) = &candidate.group_key_override {
            group.group_key.clone_from(group_key);
        }
        let first_fingerprint = group
            .parts
            .first()
            .map(|part| part.fingerprint.clone())
            .ok_or(DatabaseError::InvalidAudioLaunch)?;
        let result = self.import_audio_group(&group)?;
        let audiobook_id = match result.audiobook_id {
            Some(id) => id,
            None => self
                .connection
                .query_row(
                    "SELECT audiobook_id FROM audiobook_parts
                     WHERE fingerprint = ?1
                     ORDER BY is_available DESC, id LIMIT 1",
                    [&first_fingerprint],
                    |row| row.get::<_, i64>(0),
                )
                .optional()?
                .ok_or(DatabaseError::InvalidAudioLaunch)?,
        };
        if candidate.preserve_order {
            self.replace_audiobook_chapters(audiobook_id, &candidate.chapters)?;
        }
        if result.imported_parts > 0 {
            self.create_backup()?;
        }
        self.audiobook_by_id(audiobook_id)
    }

    pub fn add_watched_audio_folder(
        &mut self,
        path: &Path,
    ) -> Result<AudioImportSummary, DatabaseError> {
        if !path.is_dir() {
            return Err(DatabaseError::MissingFolder);
        }
        let canonical = path.canonicalize()?;
        self.connection.execute(
            "INSERT INTO watched_audio_folders(path) VALUES (?1)
             ON CONFLICT(path) DO NOTHING",
            [canonical.to_string_lossy().as_ref()],
        )?;
        let groups = discover_watched_audio_groups(&canonical)?;
        let summary = self.import_audio_groups(groups, Vec::new())?;
        self.connection.execute(
            "UPDATE watched_audio_folders
             SET last_scanned_at = CURRENT_TIMESTAMP WHERE path = ?1",
            [canonical.to_string_lossy().as_ref()],
        )?;
        Ok(summary)
    }

    pub fn list_watched_audio_folders(&self) -> Result<Vec<WatchedAudioFolder>, DatabaseError> {
        let mut statement = self.connection.prepare(
            "SELECT id, path, last_scanned_at
             FROM watched_audio_folders ORDER BY path COLLATE NOCASE",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(WatchedAudioFolder {
                id: row.get(0)?,
                path: row.get(1)?,
                last_scanned_at: row.get(2)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn scan_watched_audio_folders(&mut self) -> Result<AudioImportSummary, DatabaseError> {
        let folders = self
            .list_watched_audio_folders()?
            .into_iter()
            .map(|folder| PathBuf::from(folder.path))
            .collect::<Vec<_>>();
        let mut total = AudioImportSummary::default();
        for folder in folders {
            if !folder.is_dir() {
                total.failed += 1;
                total
                    .errors
                    .push(format!("{}: folder is unavailable", folder.display()));
                continue;
            }
            match discover_watched_audio_groups(&folder) {
                Ok(groups) => {
                    let summary = self.import_audio_groups(groups, Vec::new())?;
                    merge_audio_summary(&mut total, summary);
                    self.connection.execute(
                        "UPDATE watched_audio_folders
                         SET last_scanned_at = CURRENT_TIMESTAMP WHERE path = ?1",
                        [folder.to_string_lossy().as_ref()],
                    )?;
                }
                Err(error) => {
                    total.failed += 1;
                    total.errors.push(format!("{}: {error}", folder.display()));
                }
            }
        }
        Ok(total)
    }

    fn import_audio_groups(
        &mut self,
        groups: Vec<AudioGroupCandidate>,
        discovery_errors: Vec<String>,
    ) -> Result<AudioImportSummary, DatabaseError> {
        let mut summary = AudioImportSummary {
            failed: discovery_errors.len(),
            errors: discovery_errors,
            ..AudioImportSummary::default()
        };
        for candidate in groups {
            match inspect_audio_group(
                &candidate.paths,
                candidate.group_as_folder,
                candidate.preserve_order,
                candidate.title_override.as_deref(),
            ) {
                Ok(mut group) => {
                    if let Some(group_key) = &candidate.group_key_override {
                        group.group_key.clone_from(group_key);
                    }
                    match self.import_audio_group(&group) {
                        Ok(result) => {
                            if candidate.preserve_order {
                                if let Some(audiobook_id) = result.audiobook_id {
                                    self.replace_audiobook_chapters(
                                        audiobook_id,
                                        &candidate.chapters,
                                    )?;
                                }
                            }
                            summary.imported_books += usize::from(result.imported_book);
                            summary.imported_parts += result.imported_parts;
                            summary.duplicate_parts += result.duplicate_parts;
                        }
                        Err(error) => {
                            summary.failed += 1;
                            summary.errors.push(format!(
                                "{}: {error}",
                                candidate
                                    .paths
                                    .first()
                                    .map(|path| path.to_string_lossy())
                                    .unwrap_or_default()
                            ));
                        }
                    }
                }
                Err(error) => {
                    summary.failed += 1;
                    summary.errors.push(format!(
                        "{}: {error}",
                        candidate
                            .paths
                            .first()
                            .map(|path| path.to_string_lossy())
                            .unwrap_or_default()
                    ));
                }
            }
        }
        if summary.imported_parts > 0 {
            self.create_backup()?;
        }
        Ok(summary)
    }

    fn import_audio_group(
        &mut self,
        group: &ImportedAudioGroup,
    ) -> Result<AudioGroupImportResult, DatabaseError> {
        let transaction = self.connection.transaction()?;
        let exact_group = transaction
            .query_row(
                "SELECT id FROM audiobooks WHERE group_key = ?1",
                [&group.group_key],
                |row| row.get::<_, i64>(0),
            )
            .optional()?;
        let mut relocated_group = None;
        if exact_group.is_none() {
            for part in &group.parts {
                relocated_group =
                    audio_part_location_by_fingerprint(&transaction, &part.fingerprint, false)?
                        .map(|(_, audiobook_id)| audiobook_id);
                if relocated_group.is_some() {
                    break;
                }
            }
        }
        if exact_group.is_none()
            && relocated_group.is_none()
            && group.group_key.starts_with("single:")
            && audio_part_location_by_fingerprint(&transaction, &group.parts[0].fingerprint, true)?
                .is_some()
        {
            return Ok(AudioGroupImportResult {
                audiobook_id: None,
                imported_book: false,
                imported_parts: 0,
                duplicate_parts: 1,
            });
        }
        let (audiobook_id, imported_book) = match exact_group.or(relocated_group) {
            Some(id) => {
                transaction.execute(
                    "UPDATE audiobooks SET group_key = ?1, title = ?2,
                         updated_at = CURRENT_TIMESTAMP WHERE id = ?3",
                    params![group.group_key, group.title, id],
                )?;
                (id, false)
            }
            None => {
                transaction.execute(
                    "INSERT INTO audiobooks(group_key, title) VALUES (?1, ?2)",
                    params![group.group_key, group.title],
                )?;
                (transaction.last_insert_rowid(), true)
            }
        };

        let mut imported_parts = 0;
        let mut duplicate_parts = 0;
        for (ordinal, part) in group.parts.iter().enumerate() {
            let existing = transaction
                .query_row(
                    "SELECT id, fingerprint FROM audiobook_parts WHERE source_path = ?1",
                    [&part.source_path],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
                )
                .optional()?;
            if let Some((part_id, existing_fingerprint)) = existing {
                transaction.execute(
                    "UPDATE audiobook_parts SET audiobook_id = ?1, fingerprint = ?2,
                         title = ?3, format = ?4, file_size = ?5, ordinal = ?6,
                         is_available = 1, last_seen_at = CURRENT_TIMESTAMP
                     WHERE id = ?7",
                    params![
                        audiobook_id,
                        part.fingerprint,
                        part.title,
                        part.format,
                        part.file_size,
                        ordinal as i64,
                        part_id,
                    ],
                )?;
                if existing_fingerprint == part.fingerprint {
                    duplicate_parts += 1;
                } else {
                    imported_parts += 1;
                }
                continue;
            }

            if group.group_key.starts_with("single:")
                && audio_part_location_by_fingerprint(&transaction, &part.fingerprint, true)?
                    .is_some()
            {
                duplicate_parts += 1;
                continue;
            }

            let relocated_part =
                audio_part_location_by_fingerprint(&transaction, &part.fingerprint, false)?
                    .map(|(part_id, _)| part_id);
            if let Some(part_id) = relocated_part {
                transaction.execute(
                    "UPDATE audiobook_parts SET audiobook_id = ?1, source_path = ?2,
                         title = ?3, format = ?4, file_size = ?5, ordinal = ?6,
                         is_available = 1, last_seen_at = CURRENT_TIMESTAMP
                     WHERE id = ?7",
                    params![
                        audiobook_id,
                        part.source_path,
                        part.title,
                        part.format,
                        part.file_size,
                        ordinal as i64,
                        part_id,
                    ],
                )?;
                imported_parts += 1;
            } else {
                transaction.execute(
                    "INSERT INTO audiobook_parts(
                        audiobook_id, source_path, fingerprint, title, format,
                        file_size, ordinal
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        audiobook_id,
                        part.source_path,
                        part.fingerprint,
                        part.title,
                        part.format,
                        part.file_size,
                        ordinal as i64,
                    ],
                )?;
                imported_parts += 1;
            }
        }
        let (stored_part_count, stored_total_size) = transaction.query_row(
            "SELECT COUNT(*), COALESCE(SUM(file_size), 0)
             FROM audiobook_parts WHERE audiobook_id = ?1",
            [audiobook_id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )?;
        if stored_part_count > MAX_AUDIOBOOK_PARTS as i64 {
            return Err(AudioImportError::TooManyParts.into());
        }
        if stored_total_size > MAX_AUDIOBOOK_BYTES as i64 {
            return Err(AudioImportError::BookTooLarge.into());
        }
        transaction.execute(
            "UPDATE audiobooks SET
                total_size = COALESCE((SELECT SUM(file_size) FROM audiobook_parts
                                       WHERE audiobook_id = ?1), 0),
                part_count = (SELECT COUNT(*) FROM audiobook_parts
                              WHERE audiobook_id = ?1),
                is_available = CASE
                    WHEN (SELECT COUNT(*) FROM audiobook_parts WHERE audiobook_id = ?1) > 0
                     AND (SELECT MIN(is_available) FROM audiobook_parts WHERE audiobook_id = ?1) = 1
                    THEN 1 ELSE 0 END,
                updated_at = CURRENT_TIMESTAMP
             WHERE id = ?1",
            [audiobook_id],
        )?;
        transaction.execute(
            "DELETE FROM audiobooks
             WHERE id != ?1
               AND NOT EXISTS (SELECT 1 FROM audiobook_parts
                               WHERE audiobook_id = audiobooks.id)",
            [audiobook_id],
        )?;
        transaction.commit()?;
        Ok(AudioGroupImportResult {
            audiobook_id: Some(audiobook_id),
            imported_book,
            imported_parts,
            duplicate_parts,
        })
    }

    fn replace_audiobook_chapters(
        &mut self,
        audiobook_id: i64,
        chapters: &[crate::audio_importer::AudioChapterCandidate],
    ) -> Result<(), DatabaseError> {
        let transaction = self.connection.transaction()?;
        transaction.execute(
            "DELETE FROM audiobook_chapters WHERE audiobook_id = ?1",
            [audiobook_id],
        )?;
        for chapter in chapters {
            let part_index = transaction
                .query_row(
                    "SELECT ordinal FROM audiobook_parts
                     WHERE audiobook_id = ?1 AND source_path = ?2",
                    params![audiobook_id, chapter.source_path.to_string_lossy().as_ref()],
                    |row| row.get::<_, i64>(0),
                )
                .optional()?
                .ok_or(DatabaseError::MissingAudioPart)?;
            transaction.execute(
                "INSERT INTO audiobook_chapters(
                    audiobook_id, part_index, title, start_seconds, ordinal
                 ) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    audiobook_id,
                    part_index,
                    chapter.title,
                    chapter.start_seconds,
                    chapter.ordinal as i64
                ],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn remove_books(&mut self, book_ids: &[i64]) -> Result<usize, DatabaseError> {
        let book_ids = book_ids
            .iter()
            .copied()
            .filter(|id| *id > 0)
            .collect::<BTreeSet<_>>();
        if book_ids.is_empty() || book_ids.len() > 10_000 {
            return Err(DatabaseError::InvalidBookSelection);
        }

        let transaction = self.connection.transaction()?;
        let mut removed = 0;
        let mut cached_paths = BTreeSet::new();
        let mut reader_cache_keys = BTreeSet::new();
        for book_id in book_ids {
            let record = transaction
                .query_row(
                    "SELECT fingerprint, cover_path, embedded_cover_path
                     FROM books WHERE id = ?1",
                    [book_id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, Option<String>>(1)?,
                            row.get::<_, Option<String>>(2)?,
                        ))
                    },
                )
                .optional()?;
            let Some((fingerprint, cover_path, embedded_cover_path)) = record else {
                continue;
            };
            transaction.execute("DELETE FROM book_search WHERE book_id = ?1", [book_id])?;
            removed += transaction.execute("DELETE FROM books WHERE id = ?1", [book_id])?;
            cached_paths.extend(cover_path);
            cached_paths.extend(embedded_cover_path);
            reader_cache_keys.insert(
                fingerprint
                    .get(..24)
                    .unwrap_or(fingerprint.as_str())
                    .to_owned(),
            );
        }
        transaction.commit()?;

        for path in cached_paths {
            remove_managed_file(&self.cover_dir, Path::new(&path));
        }
        for key in reader_cache_keys {
            remove_managed_directory(&self.reader_cache_dir, &key);
        }
        if removed > 0 {
            self.create_backup()?;
        }
        Ok(removed)
    }

    pub fn update_book_metadata(
        &mut self,
        book_id: i64,
        input: &BookMetadataInput,
    ) -> Result<BookRecord, DatabaseError> {
        validate_metadata_input(input)?;
        let changed = self.connection.execute(
            "UPDATE books SET
                title = ?1, author = ?2, subtitle = ?3, isbn = ?4,
                publisher = ?5, published_year = ?6, language = ?7,
                series = ?8, genres = ?9, description = ?10, metadata_source = 'manual',
                metadata_provider_id = NULL, metadata_updated_at = CURRENT_TIMESTAMP
             WHERE id = ?11",
            params![
                input.title.trim(),
                input.author.trim(),
                input.subtitle.trim(),
                input.isbn.trim(),
                input.publisher.trim(),
                input.published_year.trim(),
                input.language.trim(),
                input.series.trim(),
                normalize_genres_input(&input.genres),
                input.description.trim(),
                book_id,
            ],
        )?;
        if changed == 0 {
            return Err(DatabaseError::MissingBook);
        }
        self.create_backup()?;
        self.book_by_id(book_id)
    }

    pub fn set_book_favorite(
        &mut self,
        book_id: i64,
        favorite: bool,
    ) -> Result<BookRecord, DatabaseError> {
        let changed = self.connection.execute(
            "UPDATE books SET is_favorite = ?1 WHERE id = ?2",
            params![i64::from(favorite), book_id],
        )?;
        if changed == 0 {
            return Err(DatabaseError::MissingBook);
        }
        self.create_backup()?;
        self.book_by_id(book_id)
    }

    pub fn search_metadata(
        &mut self,
        book_id: i64,
        explicit_query: &str,
        language: &str,
    ) -> Result<Vec<MetadataCandidate>, DatabaseError> {
        let language = MetadataLanguage::parse(language)?;
        let (title, author, isbn) = self
            .connection
            .query_row(
                "SELECT title, author, isbn FROM books WHERE id = ?1",
                [book_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?
            .ok_or(DatabaseError::MissingBook)?;
        let query = if explicit_query.trim().is_empty() {
            if !isbn.trim().is_empty() {
                isbn
            } else {
                format!("{title} {author}")
            }
        } else {
            explicit_query.to_owned()
        };
        let query = normalize_metadata_query(&query).ok_or(DatabaseError::InvalidMetadata)?;
        let now = unix_seconds();
        let cache_key = format!("{}:{}", language.code(), query.to_lowercase());
        if let Some(cached) = self
            .connection
            .query_row(
                "SELECT response_json FROM metadata_cache
                 WHERE query_key = ?1 AND fetched_at >= ?2",
                params![cache_key, now.saturating_sub(30 * 24 * 60 * 60)],
                |row| row.get::<_, String>(0),
            )
            .optional()?
        {
            return serde_json::from_str(&cached).map_err(Into::into);
        }
        self.reserve_metadata_request(false)?;
        let candidates = crate::metadata::search_metadata(&query, language)?;
        let json = serde_json::to_string(&candidates)?;
        self.connection.execute(
            "INSERT INTO metadata_cache(query_key, response_json, fetched_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(query_key) DO UPDATE SET
                response_json = excluded.response_json,
                fetched_at = excluded.fetched_at",
            params![cache_key, json, now],
        )?;
        Ok(candidates)
    }

    pub fn apply_metadata_candidate(
        &mut self,
        book_id: i64,
        candidate: &MetadataCandidate,
    ) -> Result<BookRecord, DatabaseError> {
        validate_candidate(candidate)?;
        let previous_external = self
            .connection
            .query_row(
                "SELECT cover_path FROM books
                 WHERE id = ?1 AND cover_source != 'embedded'",
                [book_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten();
        let downloaded_cover = if candidate.provider == "Open Library" {
            let Some(cover_id) = candidate.cover_id else {
                return self.apply_metadata_without_cover(book_id, candidate, previous_external);
            };
            self.reserve_metadata_request(true)?;
            let (bytes, extension) = download_open_library_cover(cover_id)?;
            let path = self
                .cover_dir
                .join(format!("external-{book_id}-{cover_id}.{extension}"));
            fs::write(&path, bytes)?;
            Some(path.to_string_lossy().into_owned())
        } else {
            None
        };
        self.apply_metadata_with_cover(book_id, candidate, previous_external, downloaded_cover)
    }

    fn apply_metadata_without_cover(
        &mut self,
        book_id: i64,
        candidate: &MetadataCandidate,
        previous_external: Option<String>,
    ) -> Result<BookRecord, DatabaseError> {
        self.apply_metadata_with_cover(book_id, candidate, previous_external, None)
    }

    fn apply_metadata_with_cover(
        &mut self,
        book_id: i64,
        candidate: &MetadataCandidate,
        previous_external: Option<String>,
        downloaded_cover: Option<String>,
    ) -> Result<BookRecord, DatabaseError> {
        let metadata_source = if candidate.provider == "ФантЛаб" {
            "fantlab"
        } else {
            "open_library"
        };
        let changed = self.connection.execute(
            "UPDATE books SET
                title = ?1, author = ?2, isbn = ?3, publisher = ?4,
                published_year = ?5, language = ?6,
                genres = CASE WHEN TRIM(?7) = '' THEN genres ELSE ?7 END,
                series = CASE WHEN TRIM(?8) = '' THEN series ELSE ?8 END,
                metadata_source = ?9, metadata_provider_id = ?10,
                metadata_updated_at = CURRENT_TIMESTAMP,
                cover_path = COALESCE(?11, cover_path),
                cover_source = CASE WHEN ?11 IS NULL THEN cover_source
                                    ELSE 'open_library' END
             WHERE id = ?12",
            params![
                candidate.title.trim(),
                candidate.author.trim(),
                candidate.isbn.trim(),
                candidate.publisher.trim(),
                candidate.published_year.trim(),
                candidate.language.trim(),
                normalize_genres_input(&candidate.genres),
                candidate.series.trim(),
                metadata_source,
                candidate.provider_id.trim(),
                downloaded_cover,
                book_id,
            ],
        )?;
        if changed == 0 {
            return Err(DatabaseError::MissingBook);
        }
        if let Some(previous) = previous_external {
            if Some(previous.as_str()) != downloaded_cover.as_deref() {
                remove_managed_external_cover(&self.cover_dir, &previous)?;
            }
        }
        self.create_backup()?;
        self.book_by_id(book_id)
    }

    pub fn set_local_cover(
        &mut self,
        book_id: i64,
        source: &Path,
    ) -> Result<BookRecord, DatabaseError> {
        let previous_cover = self
            .connection
            .query_row(
                "SELECT cover_source, cover_path FROM books WHERE id = ?1",
                [book_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
            )
            .optional()?
            .ok_or(DatabaseError::MissingBook)?;
        let _source_extension = source
            .extension()
            .and_then(|value| value.to_str())
            .map(str::to_ascii_lowercase)
            .filter(|value| matches!(value.as_str(), "jpg" | "jpeg" | "png" | "webp"))
            .ok_or(DatabaseError::InvalidMetadata)?;
        let metadata = fs::metadata(source)?;
        if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_COVER_BYTES as u64 {
            return Err(DatabaseError::InvalidMetadata);
        }
        let mut bytes = Vec::with_capacity(metadata.len() as usize);
        fs::File::open(source)?
            .take(MAX_COVER_BYTES as u64 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > MAX_COVER_BYTES {
            return Err(DatabaseError::InvalidMetadata);
        }
        let extension = image_extension(&bytes).ok_or(MetadataError::InvalidCover)?;
        let destination = self.cover_dir.join(format!(
            "external-{book_id}-local-{}.{}",
            unix_millis(),
            extension
        ));
        fs::write(&destination, &bytes)?;
        let destination_string = destination.to_string_lossy().into_owned();
        let changed = self.connection.execute(
            "UPDATE books SET cover_path = ?1, cover_source = 'local' WHERE id = ?2",
            params![destination_string, book_id],
        )?;
        if changed == 0 {
            remove_managed_file(&self.cover_dir, &destination);
            return Err(DatabaseError::MissingBook);
        }
        if previous_cover.0 != "embedded" {
            if let Some(previous) = previous_cover.1 {
                if previous != destination_string {
                    remove_managed_external_cover(&self.cover_dir, &previous)?;
                }
            }
        }
        self.create_backup()?;
        self.book_by_id(book_id)
    }

    fn reserve_metadata_request(&self, wait: bool) -> Result<(), DatabaseError> {
        let now = unix_millis();
        let last_request = self
            .connection
            .query_row(
                "SELECT value FROM app_metadata WHERE key = 'open_library_last_request'",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .and_then(|value| value.parse::<u128>().ok())
            .unwrap_or_default();
        let elapsed = now.saturating_sub(last_request);
        if elapsed < 1_000 {
            if !wait {
                return Err(DatabaseError::MetadataRateLimited);
            }
            std::thread::sleep(std::time::Duration::from_millis(
                u64::try_from(1_000 - elapsed).unwrap_or(1_000),
            ));
        }
        self.connection.execute(
            "INSERT INTO app_metadata(key, value)
             VALUES ('open_library_last_request', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [unix_millis().to_string()],
        )?;
        Ok(())
    }

    pub fn remove_external_cover(&mut self, book_id: i64) -> Result<BookRecord, DatabaseError> {
        let (cover_source, cover_path) = self
            .connection
            .query_row(
                "SELECT cover_source, cover_path FROM books WHERE id = ?1",
                [book_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
            )
            .optional()?
            .ok_or(DatabaseError::MissingBook)?;
        if cover_source != "embedded" {
            self.connection.execute(
                "UPDATE books SET cover_path = embedded_cover_path,
                    cover_source = 'embedded' WHERE id = ?1",
                [book_id],
            )?;
            if let Some(path) = cover_path {
                remove_managed_external_cover(&self.cover_dir, &path)?;
            }
            self.create_backup()?;
        }
        self.book_by_id(book_id)
    }

    fn book_by_id(&self, book_id: i64) -> Result<BookRecord, DatabaseError> {
        self.connection
            .query_row(
                "SELECT id, source_path, title, author, format, file_size, cover_path,
                        added_at, is_available, progress, subtitle, isbn, publisher,
                        published_year, language, series, genres, description, metadata_source,
                        metadata_provider_id, metadata_updated_at, cover_source,
                        last_opened_at, is_favorite
                 FROM books WHERE id = ?1",
                [book_id],
                book_record_from_row,
            )
            .optional()?
            .ok_or(DatabaseError::MissingBook)
    }

    pub fn load_document(&mut self, book_id: i64) -> Result<DocumentModel, DatabaseError> {
        let record = self
            .connection
            .query_row(
                "SELECT source_path, title, author, format, progress,
                        last_section, section_progress
                 FROM books WHERE id = ?1",
                [book_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, f64>(4)?,
                        row.get::<_, i64>(5)?,
                        row.get::<_, f64>(6)?,
                    ))
                },
            )
            .optional()?
            .ok_or(DatabaseError::MissingBook)?;
        let sections = read_document(Path::new(&record.0))?;
        let last_section = usize::try_from(record.5)
            .unwrap_or_default()
            .min(sections.len().saturating_sub(1));
        let document = DocumentModel {
            book_id,
            title: record.1,
            author: record.2,
            format: record.3,
            sections,
            progress: record.4,
            last_section,
            section_progress: record.6,
        };
        self.index_document(&document)?;
        self.mark_book_opened(book_id)?;
        Ok(document)
    }

    pub fn load_special_document(
        &mut self,
        book_id: i64,
    ) -> Result<SpecialDocument, DatabaseError> {
        let record = self
            .connection
            .query_row(
                "SELECT source_path, fingerprint, title, author, format, progress,
                        last_section
                 FROM books WHERE id = ?1",
                [book_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, f64>(5)?,
                        row.get::<_, i64>(6)?,
                    ))
                },
            )
            .optional()?
            .ok_or(DatabaseError::MissingBook)?;
        let document = prepare_special_document(
            book_id,
            record.2,
            record.3,
            record.4,
            Path::new(&record.0),
            &record.1,
            record.5,
            usize::try_from(record.6).unwrap_or_default(),
            &self.reader_cache_dir,
        )
        .map_err(DatabaseError::from)?;
        self.mark_book_opened(book_id)?;
        Ok(document)
    }

    pub fn save_reading_position(
        &mut self,
        book_id: i64,
        section: usize,
        section_progress: f64,
        progress: f64,
    ) -> Result<(), DatabaseError> {
        if !section_progress.is_finite()
            || !progress.is_finite()
            || !(0.0..=1.0).contains(&section_progress)
            || !(0.0..=1.0).contains(&progress)
        {
            return Err(DatabaseError::InvalidPosition);
        }
        let changed = self.connection.execute(
            "UPDATE books
             SET last_section = ?1, section_progress = ?2, progress = ?3,
                 last_seen_at = CURRENT_TIMESTAMP,
                 last_opened_at = CAST(strftime('%s', 'now') AS INTEGER)
             WHERE id = ?4",
            params![
                i64::try_from(section).unwrap_or(i64::MAX),
                section_progress,
                progress,
                book_id
            ],
        )?;
        if changed == 0 {
            return Err(DatabaseError::MissingBook);
        }
        Ok(())
    }

    fn mark_book_opened(&mut self, book_id: i64) -> Result<(), DatabaseError> {
        let changed = self.connection.execute(
            "UPDATE books
             SET last_opened_at = CAST(strftime('%s', 'now') AS INTEGER)
             WHERE id = ?1",
            [book_id],
        )?;
        if changed == 0 {
            return Err(DatabaseError::MissingBook);
        }
        Ok(())
    }

    pub fn start_reading_session(
        &self,
        book_id: i64,
        progress: f64,
        words: i64,
        pages: i64,
    ) -> Result<String, DatabaseError> {
        statistics::start_session(&self.connection, book_id, progress, words, pages)
            .map_err(Into::into)
    }

    pub fn record_reading_activity(
        &self,
        token: &str,
        active: bool,
        progress: f64,
        words: i64,
        pages: i64,
    ) -> Result<(), DatabaseError> {
        statistics::record_activity(&self.connection, token, active, progress, words, pages)
            .map_err(Into::into)
    }

    pub fn end_reading_session(&self, token: &str) -> Result<(), DatabaseError> {
        statistics::end_session(&self.connection, token).map_err(Into::into)
    }

    pub fn start_audiobook_session(
        &self,
        audiobook_id: i64,
        progress: f64,
    ) -> Result<String, DatabaseError> {
        audio_statistics::start_session(&self.connection, audiobook_id, progress)
            .map_err(Into::into)
    }

    pub fn record_audiobook_activity(
        &self,
        token: &str,
        active: bool,
        progress: f64,
    ) -> Result<(), DatabaseError> {
        audio_statistics::record_activity(&self.connection, token, active, progress)
            .map_err(Into::into)
    }

    pub fn end_audiobook_session(&self, token: &str) -> Result<(), DatabaseError> {
        audio_statistics::end_session(&self.connection, token).map_err(Into::into)
    }

    pub fn audiobook_statistics_snapshot(
        &self,
    ) -> Result<AudiobookStatisticsSnapshot, DatabaseError> {
        audio_statistics::snapshot(&self.connection).map_err(Into::into)
    }

    pub fn audiobook_achievements(&self) -> Result<Vec<AchievementProgress>, DatabaseError> {
        audio_statistics::achievements(&self.connection).map_err(Into::into)
    }

    pub fn statistics_snapshot(&self) -> Result<StatisticsSnapshot, DatabaseError> {
        statistics::snapshot(&self.connection).map_err(Into::into)
    }

    pub fn achievements(&self) -> Result<Vec<AchievementProgress>, DatabaseError> {
        statistics::achievements(&self.connection).map_err(Into::into)
    }

    pub fn set_daily_goal(&self, minutes: i64) -> Result<(), DatabaseError> {
        statistics::set_daily_goal(&self.connection, minutes).map_err(Into::into)
    }

    pub fn clear_reading_statistics(&self) -> Result<(), DatabaseError> {
        statistics::clear(&self.connection)?;
        audio_statistics::clear(&self.connection).map_err(Into::into)
    }

    pub fn steam_integration_status(&self) -> Result<SteamIntegrationStatus, DatabaseError> {
        steam::status(&self.connection).map_err(Into::into)
    }

    pub fn sync_steam_achievements(&self) -> Result<SteamSyncResult, DatabaseError> {
        steam::synchronize(&self.connection).map_err(Into::into)
    }

    fn index_document(&mut self, document: &DocumentModel) -> Result<(), DatabaseError> {
        let transaction = self.connection.transaction()?;
        transaction.execute(
            "DELETE FROM book_search WHERE book_id = ?1",
            [document.book_id],
        )?;
        {
            let mut statement = transaction.prepare(
                "INSERT INTO book_search(
                    book_id, section_id, block_index, section_title, body
                 ) VALUES (?1, ?2, ?3, ?4, ?5)",
            )?;
            for section in &document.sections {
                for (block_index, block) in section.blocks.iter().enumerate() {
                    if !block.text.is_empty() {
                        statement.execute(params![
                            document.book_id,
                            section.id,
                            i64::try_from(block_index).unwrap_or(i64::MAX),
                            section.title,
                            block.text
                        ])?;
                    }
                }
            }
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn search_book(
        &self,
        book_id: i64,
        query: &str,
    ) -> Result<Vec<SearchResult>, DatabaseError> {
        let match_query = fts_query(query).ok_or(DatabaseError::InvalidSearch)?;
        let mut statement = self.connection.prepare(
            "SELECT section_id, section_title, block_index,
                    snippet(book_search, 4, '‹', '›', ' … ', 18)
             FROM book_search
             WHERE book_search MATCH ?1 AND book_id = ?2
             ORDER BY rank
             LIMIT 100",
        )?;
        let rows = statement.query_map(params![match_query, book_id], |row| {
            let index = row.get::<_, i64>(2)?;
            Ok(SearchResult {
                section_id: row.get(0)?,
                section_title: row.get(1)?,
                block_index: usize::try_from(index).unwrap_or_default(),
                excerpt: row.get(3)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn list_annotations(&self, book_id: i64) -> Result<Vec<AnnotationRecord>, DatabaseError> {
        let mut statement = self.connection.prepare(
            "SELECT id, book_id, kind, section_id, block_index, start_offset,
                    end_offset, selected_text, note, created_at, updated_at
             FROM annotations
             WHERE book_id = ?1
             ORDER BY section_id, block_index, start_offset, id",
        )?;
        let rows = statement.query_map([book_id], annotation_from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_annotation(
        &mut self,
        book_id: i64,
        kind: &str,
        section_id: &str,
        block_index: usize,
        start_offset: usize,
        end_offset: usize,
        selected_text: &str,
        note: &str,
    ) -> Result<AnnotationRecord, DatabaseError> {
        if !matches!(kind, "bookmark" | "highlight" | "note" | "quote")
            || section_id.trim().is_empty()
            || end_offset < start_offset
            || selected_text.chars().count() > 4_000
            || note.chars().count() > 20_000
            || (kind != "bookmark" && selected_text.trim().is_empty())
        {
            return Err(DatabaseError::InvalidAnnotation);
        }
        let exists = self
            .connection
            .query_row("SELECT id FROM books WHERE id = ?1", [book_id], |row| {
                row.get::<_, i64>(0)
            })
            .optional()?
            .is_some();
        if !exists {
            return Err(DatabaseError::MissingBook);
        }
        self.connection.execute(
            "INSERT INTO annotations(
                book_id, kind, section_id, block_index, start_offset, end_offset,
                selected_text, note
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                book_id,
                kind,
                section_id,
                i64::try_from(block_index).unwrap_or(i64::MAX),
                i64::try_from(start_offset).unwrap_or(i64::MAX),
                i64::try_from(end_offset).unwrap_or(i64::MAX),
                selected_text.trim(),
                note.trim()
            ],
        )?;
        let id = self.connection.last_insert_rowid();
        self.annotation(id)?.ok_or(DatabaseError::InvalidAnnotation)
    }

    pub fn update_annotation_note(
        &mut self,
        annotation_id: i64,
        note: &str,
    ) -> Result<AnnotationRecord, DatabaseError> {
        if note.chars().count() > 20_000 {
            return Err(DatabaseError::InvalidAnnotation);
        }
        let changed = self.connection.execute(
            "UPDATE annotations
             SET note = ?1, updated_at = CURRENT_TIMESTAMP
             WHERE id = ?2",
            params![note.trim(), annotation_id],
        )?;
        if changed == 0 {
            return Err(DatabaseError::InvalidAnnotation);
        }
        self.annotation(annotation_id)?
            .ok_or(DatabaseError::InvalidAnnotation)
    }

    pub fn delete_annotation(&mut self, annotation_id: i64) -> Result<(), DatabaseError> {
        let changed = self
            .connection
            .execute("DELETE FROM annotations WHERE id = ?1", [annotation_id])?;
        if changed == 0 {
            return Err(DatabaseError::InvalidAnnotation);
        }
        Ok(())
    }

    pub fn export_annotations(&self, book_id: i64, path: &Path) -> Result<(), DatabaseError> {
        let title = self
            .connection
            .query_row("SELECT title FROM books WHERE id = ?1", [book_id], |row| {
                row.get::<_, String>(0)
            })
            .optional()?
            .ok_or(DatabaseError::MissingBook)?;
        let annotations = self.list_annotations(book_id)?;
        let mut output = format!("# {title}\n\n");
        for annotation in annotations {
            output.push_str(&format!(
                "## {} · {} · block {}\n\n",
                annotation.kind,
                annotation.locator.section_id,
                annotation.locator.block_index + 1
            ));
            if !annotation.selected_text.is_empty() {
                for line in annotation.selected_text.lines() {
                    output.push_str(&format!("> {line}\n"));
                }
                output.push('\n');
            }
            if !annotation.note.is_empty() {
                output.push_str(&annotation.note);
                output.push_str("\n\n");
            }
        }
        fs::write(path, output)?;
        Ok(())
    }

    fn annotation(&self, id: i64) -> Result<Option<AnnotationRecord>, DatabaseError> {
        self.connection
            .query_row(
                "SELECT id, book_id, kind, section_id, block_index, start_offset,
                        end_offset, selected_text, note, created_at, updated_at
                 FROM annotations WHERE id = ?1",
                [id],
                annotation_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn import_paths(&mut self, paths: &[PathBuf]) -> Result<ImportSummary, DatabaseError> {
        let mut summary = ImportSummary::default();
        for path in paths {
            match self.import_one(path) {
                Ok((_, true)) => summary.imported += 1,
                Ok((_, false)) => summary.duplicates += 1,
                Err(error) => {
                    summary.failed += 1;
                    summary
                        .errors
                        .push(format!("{}: {error}", path.to_string_lossy()));
                }
            }
        }
        if summary.imported > 0 {
            self.create_backup()?;
        }
        Ok(summary)
    }

    pub fn import_book_for_open(&mut self, path: &Path) -> Result<BookRecord, DatabaseError> {
        let (book_id, imported) = self.import_one(path)?;
        if imported {
            self.create_backup()?;
        }
        self.book_by_id(book_id)
    }

    fn import_one(&mut self, path: &Path) -> Result<(i64, bool), DatabaseError> {
        let book = inspect_book(path, &self.cover_dir)?;
        let duplicate = self
            .connection
            .query_row(
                "SELECT id, source_path FROM books WHERE fingerprint = ?1",
                [&book.fingerprint],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;
        if let Some((book_id, existing_source)) = duplicate {
            if !Path::new(&existing_source).is_file() && existing_source != book.source_path {
                self.connection.execute(
                    "UPDATE books SET
                        source_path = ?1, file_size = ?2, format = ?3,
                        embedded_cover_path = ?4,
                        cover_path = CASE WHEN cover_source != 'embedded'
                                          THEN cover_path ELSE ?4 END,
                        last_seen_at = CURRENT_TIMESTAMP, is_available = 1
                     WHERE id = ?5",
                    params![
                        book.source_path,
                        book.file_size,
                        book.format,
                        book.cover_path,
                        book_id,
                    ],
                )?;
                return Ok((book_id, true));
            }
            return Ok((book_id, false));
        }
        let source_path = book.source_path.clone();
        self.connection.execute(
            "INSERT INTO books (
                source_path, fingerprint, title, author, format, file_size,
                genres, cover_path, embedded_cover_path
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
             ON CONFLICT(source_path) DO UPDATE SET
                fingerprint = excluded.fingerprint,
                title = CASE WHEN books.metadata_source = 'embedded'
                             THEN excluded.title ELSE books.title END,
                author = CASE WHEN books.metadata_source = 'embedded'
                              THEN excluded.author ELSE books.author END,
                genres = CASE WHEN books.metadata_source = 'embedded'
                              THEN excluded.genres ELSE books.genres END,
                format = excluded.format,
                file_size = excluded.file_size,
                embedded_cover_path = excluded.cover_path,
                cover_path = CASE WHEN books.cover_source != 'embedded'
                                  THEN books.cover_path ELSE excluded.cover_path END,
                last_seen_at = CURRENT_TIMESTAMP,
                is_available = 1",
            params![
                book.source_path,
                book.fingerprint,
                book.title,
                book.author,
                book.format,
                book.file_size,
                book.genres,
                book.cover_path,
            ],
        )?;
        let book_id = self.connection.query_row(
            "SELECT id FROM books WHERE source_path = ?1",
            [source_path],
            |row| row.get::<_, i64>(0),
        )?;
        Ok((book_id, true))
    }

    pub fn add_watched_folder(&mut self, path: &Path) -> Result<ImportSummary, DatabaseError> {
        if !path.is_dir() {
            return Err(DatabaseError::MissingFolder);
        }
        let canonical = path.canonicalize()?;
        self.connection.execute(
            "INSERT INTO watched_folders(path) VALUES (?1)
             ON CONFLICT(path) DO NOTHING",
            [canonical.to_string_lossy().as_ref()],
        )?;
        let summary = self.scan_folder(&canonical)?;
        self.connection.execute(
            "UPDATE watched_folders SET last_scanned_at = CURRENT_TIMESTAMP WHERE path = ?1",
            [canonical.to_string_lossy().as_ref()],
        )?;
        Ok(summary)
    }

    pub fn list_watched_folders(&self) -> Result<Vec<WatchedFolder>, DatabaseError> {
        let mut statement = self.connection.prepare(
            "SELECT id, path, last_scanned_at FROM watched_folders ORDER BY path COLLATE NOCASE",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(WatchedFolder {
                id: row.get(0)?,
                path: row.get(1)?,
                last_scanned_at: row.get(2)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn scan_watched_folders(&mut self) -> Result<ImportSummary, DatabaseError> {
        let folders = self
            .list_watched_folders()?
            .into_iter()
            .map(|folder| PathBuf::from(folder.path))
            .collect::<Vec<_>>();
        let mut total = ImportSummary::default();
        for folder in folders {
            if !folder.is_dir() {
                total.failed += 1;
                total
                    .errors
                    .push(format!("{}: folder is unavailable", folder.display()));
                continue;
            }
            let result = self.scan_folder(&folder)?;
            total.imported += result.imported;
            total.duplicates += result.duplicates;
            total.failed += result.failed;
            total.errors.extend(result.errors);
            self.connection.execute(
                "UPDATE watched_folders SET last_scanned_at = CURRENT_TIMESTAMP WHERE path = ?1",
                [folder.to_string_lossy().as_ref()],
            )?;
        }
        Ok(total)
    }

    fn scan_folder(&mut self, folder: &Path) -> Result<ImportSummary, DatabaseError> {
        let mut files = Vec::new();
        collect_supported_files(folder, 0, &mut files)?;
        self.import_paths(&files)
    }

    fn refresh_availability(&mut self) -> Result<(), DatabaseError> {
        let paths = {
            let mut statement = self
                .connection
                .prepare("SELECT id, source_path FROM books")?;
            let rows = statement
                .query_map([], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            rows
        };
        let transaction = self.connection.transaction()?;
        for (id, path) in paths {
            transaction.execute(
                "UPDATE books SET is_available = ?1, last_seen_at = CURRENT_TIMESTAMP WHERE id = ?2",
                params![i64::from(Path::new(&path).is_file()), id],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    fn refresh_audio_availability(&mut self) -> Result<(), DatabaseError> {
        let paths = {
            let mut statement = self
                .connection
                .prepare("SELECT id, source_path FROM audiobook_parts")?;
            let rows = statement
                .query_map([], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            rows
        };
        let transaction = self.connection.transaction()?;
        for (id, path) in paths {
            transaction.execute(
                "UPDATE audiobook_parts SET is_available = ?1,
                     last_seen_at = CURRENT_TIMESTAMP WHERE id = ?2",
                params![i64::from(Path::new(&path).is_file()), id],
            )?;
        }
        transaction.execute_batch(
            "UPDATE audiobooks SET
                is_available = CASE
                    WHEN (SELECT COUNT(*) FROM audiobook_parts
                          WHERE audiobook_id = audiobooks.id) > 0
                     AND (SELECT MIN(is_available) FROM audiobook_parts
                          WHERE audiobook_id = audiobooks.id) = 1
                    THEN 1 ELSE 0 END,
                total_size = COALESCE((SELECT SUM(file_size) FROM audiobook_parts
                                       WHERE audiobook_id = audiobooks.id), 0),
                part_count = (SELECT COUNT(*) FROM audiobook_parts
                              WHERE audiobook_id = audiobooks.id);",
        )?;
        transaction.commit()?;
        Ok(())
    }

    fn create_backup(&mut self) -> Result<(), DatabaseError> {
        self.connection
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let target = self.backup_dir.join(format!("library-{timestamp}.db"));
        self.connection
            .execute("VACUUM INTO ?1", [target.to_string_lossy().as_ref()])?;
        let mut backups = fs::read_dir(&self.backup_dir)?
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().starts_with("library-"))
            .collect::<Vec<_>>();
        backups.sort_by_key(|entry| entry.file_name());
        let remove_count = backups.len().saturating_sub(10);
        for old in backups.into_iter().take(remove_count) {
            fs::remove_file(old.path())?;
        }
        Ok(())
    }

    #[cfg(test)]
    fn schema_version(&self) -> Result<i64, rusqlite::Error> {
        self.connection.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
    }
}

fn annotation_from_row(row: &rusqlite::Row<'_>) -> Result<AnnotationRecord, rusqlite::Error> {
    let block_index = row.get::<_, i64>(4)?;
    let start_offset = row.get::<_, i64>(5)?;
    let end_offset = row.get::<_, i64>(6)?;
    Ok(AnnotationRecord {
        id: row.get(0)?,
        book_id: row.get(1)?,
        kind: row.get(2)?,
        locator: ReadingLocator {
            section_id: row.get(3)?,
            block_index: usize::try_from(block_index).unwrap_or_default(),
            start_offset: usize::try_from(start_offset).unwrap_or_default(),
            end_offset: usize::try_from(end_offset).unwrap_or_default(),
        },
        selected_text: row.get(7)?,
        note: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

fn audiobook_from_row(row: &rusqlite::Row<'_>) -> Result<AudiobookRecord, rusqlite::Error> {
    Ok(AudiobookRecord {
        id: row.get(0)?,
        title: row.get(1)?,
        author: row.get(2)?,
        cover_path: row.get(3)?,
        added_at: row.get(4)?,
        is_available: row.get::<_, i64>(5)? == 1,
        total_size: row.get(6)?,
        part_count: row.get(7)?,
        total_duration_seconds: row.get(8)?,
        progress: row.get(9)?,
        last_part_index: row.get(10)?,
        last_position_seconds: row.get(11)?,
        narrator: row.get(12)?,
        series: row.get(13)?,
        genres: row.get(14)?,
        description: row.get(15)?,
        language: row.get(16)?,
        published_year: row.get(17)?,
        metadata_source: row.get(18)?,
        metadata_provider_id: row.get(19)?,
        metadata_updated_at: row.get(20)?,
        cover_source: row.get(21)?,
    })
}

fn audiobook_part_from_row(
    row: &rusqlite::Row<'_>,
) -> Result<AudiobookPartRecord, rusqlite::Error> {
    Ok(AudiobookPartRecord {
        id: row.get(0)?,
        audiobook_id: row.get(1)?,
        source_path: row.get(2)?,
        title: row.get(3)?,
        format: row.get(4)?,
        file_size: row.get(5)?,
        duration_seconds: row.get(6)?,
        ordinal: row.get(7)?,
        is_available: row.get::<_, i64>(8)? == 1,
    })
}

fn merge_audio_summary(target: &mut AudioImportSummary, source: AudioImportSummary) {
    target.imported_books += source.imported_books;
    target.imported_parts += source.imported_parts;
    target.duplicate_parts += source.duplicate_parts;
    target.failed += source.failed;
    target.errors.extend(source.errors);
}

fn audio_part_location_by_fingerprint(
    transaction: &Transaction<'_>,
    fingerprint: &str,
    source_is_available: bool,
) -> Result<Option<(i64, i64)>, rusqlite::Error> {
    let mut statement = transaction.prepare(
        "SELECT id, audiobook_id, source_path
         FROM audiobook_parts WHERE fingerprint = ?1 ORDER BY id",
    )?;
    let locations = statement
        .query_map([fingerprint], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(locations
        .into_iter()
        .find(|(_, _, path)| Path::new(path).is_file() == source_is_available)
        .map(|(part_id, audiobook_id, _)| (part_id, audiobook_id)))
}

fn book_record_from_row(row: &rusqlite::Row<'_>) -> Result<BookRecord, rusqlite::Error> {
    Ok(BookRecord {
        id: row.get(0)?,
        source_path: row.get(1)?,
        title: row.get(2)?,
        author: row.get(3)?,
        format: row.get(4)?,
        file_size: row.get(5)?,
        cover_path: row.get(6)?,
        added_at: row.get(7)?,
        is_available: row.get::<_, i64>(8)? == 1,
        progress: row.get(9)?,
        subtitle: row.get(10)?,
        isbn: row.get(11)?,
        publisher: row.get(12)?,
        published_year: row.get(13)?,
        language: row.get(14)?,
        series: row.get(15)?,
        genres: row.get(16)?,
        description: row.get(17)?,
        metadata_source: row.get(18)?,
        metadata_provider_id: row.get(19)?,
        metadata_updated_at: row.get(20)?,
        cover_source: row.get(21)?,
        last_opened_at: row.get(22)?,
        is_favorite: row.get::<_, i64>(23)? == 1,
    })
}

fn validate_metadata_input(input: &BookMetadataInput) -> Result<(), DatabaseError> {
    let values = [
        (&input.title, 512_usize),
        (&input.author, 512),
        (&input.subtitle, 512),
        (&input.isbn, 64),
        (&input.publisher, 512),
        (&input.published_year, 32),
        (&input.language, 64),
        (&input.series, 512),
        (&input.genres, 1_024),
        (&input.description, 16_384),
    ];
    if input.title.trim().is_empty()
        || values
            .iter()
            .any(|(value, limit)| value.chars().count() > *limit)
    {
        return Err(DatabaseError::InvalidMetadata);
    }
    Ok(())
}

fn validate_audiobook_metadata_input(input: &AudiobookMetadataInput) -> Result<(), DatabaseError> {
    let values = [
        (&input.title, 512_usize),
        (&input.author, 512),
        (&input.narrator, 512),
        (&input.series, 512),
        (&input.genres, 1_024),
        (&input.description, 16_384),
        (&input.language, 64),
        (&input.published_year, 32),
    ];
    if input.title.trim().is_empty()
        || values
            .iter()
            .any(|(value, limit)| value.chars().count() > *limit)
    {
        return Err(DatabaseError::InvalidMetadata);
    }
    Ok(())
}

fn validate_candidate(candidate: &MetadataCandidate) -> Result<(), DatabaseError> {
    let valid_provider_id = match candidate.provider.as_str() {
        "Open Library" => {
            candidate.provider_id.starts_with("/works/")
                || candidate.provider_id.starts_with("/books/")
        }
        "ФантЛаб" => candidate
            .provider_id
            .strip_prefix("edition:")
            .is_some_and(|value| {
                !value.is_empty() && value.chars().all(|character| character.is_ascii_digit())
            }),
        _ => false,
    };
    if !valid_provider_id
        || candidate.title.trim().is_empty()
        || candidate.title.chars().count() > 512
        || candidate.author.chars().count() > 512
        || candidate.isbn.chars().count() > 64
        || candidate.publisher.chars().count() > 512
        || candidate.published_year.chars().count() > 32
        || candidate.language.chars().count() > 64
        || candidate.series.chars().count() > 512
        || candidate.genres.chars().count() > 1_024
        || candidate.provider_id.chars().count() > 128
    {
        return Err(DatabaseError::InvalidMetadata);
    }
    Ok(())
}

fn normalize_genres_input(genres: &str) -> String {
    let mut normalized = Vec::<String>::new();
    for value in genres.split([',', ';']) {
        let value = value.split_whitespace().collect::<Vec<_>>().join(" ");
        if value.is_empty()
            || value.chars().count() > 64
            || normalized
                .iter()
                .any(|genre| genre.to_lowercase() == value.to_lowercase())
        {
            continue;
        }
        normalized.push(value);
        if normalized.len() == 12 {
            break;
        }
    }
    normalized.join(", ")
}

fn normalize_metadata_query(query: &str) -> Option<String> {
    let query = query.split_whitespace().collect::<Vec<_>>().join(" ");
    if query.is_empty() || query.chars().count() > 256 {
        None
    } else {
        Some(query)
    }
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn remove_managed_external_cover(cover_dir: &Path, path: &str) -> Result<(), DatabaseError> {
    let path = Path::new(path);
    let managed_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("external-"));
    if managed_name && path.parent() == Some(cover_dir) && path.is_file() {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn remove_managed_file(root: &Path, path: &Path) {
    if path.parent() == Some(root) && path.is_file() {
        let _ = fs::remove_file(path);
    }
}

fn remove_managed_directory(root: &Path, name: &str) {
    let safe_name = !name.is_empty() && name.chars().all(|character| character.is_ascii_hexdigit());
    if safe_name {
        let path = root.join(name);
        if path.parent() == Some(root) && path.is_dir() {
            let _ = fs::remove_dir_all(path);
        }
    }
}

fn fts_query(query: &str) -> Option<String> {
    let tokens = query
        .split_whitespace()
        .map(|token| {
            token
                .chars()
                .filter(|character| character.is_alphanumeric())
                .collect::<String>()
        })
        .filter(|token| !token.is_empty())
        .take(12)
        .collect::<Vec<_>>();
    (!tokens.is_empty()).then(|| {
        tokens
            .into_iter()
            .map(|token| format!("\"{token}\""))
            .collect::<Vec<_>>()
            .join(" AND ")
    })
}

fn collect_supported_files(
    folder: &Path,
    depth: usize,
    files: &mut Vec<PathBuf>,
) -> Result<(), DatabaseError> {
    collect_supported_files_bounded(folder, depth, files, MAX_WATCHED_BOOK_FILES)
}

fn collect_supported_files_bounded(
    folder: &Path,
    depth: usize,
    files: &mut Vec<PathBuf>,
    max_files: usize,
) -> Result<(), DatabaseError> {
    if depth > 12 {
        return Ok(());
    }
    for entry in fs::read_dir(folder)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            collect_supported_files_bounded(&entry.path(), depth + 1, files, max_files)?;
        } else if file_type.is_file() && supported_book_path(&entry.path()) {
            if files.len() == max_files {
                return Err(DatabaseError::TooManyWatchedBooks);
            }
            files.push(entry.path());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::io::Write;

    const TEST_PNG: &[u8] = &[137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82];

    fn test_database(directory: &Path) -> Database {
        Database::open(
            &directory.join("library.db"),
            directory.join("covers"),
            directory.join("backups"),
        )
        .expect("database")
    }

    fn write_test_epub(path: &Path) {
        let file = fs::File::create(path).expect("EPUB");
        let mut archive = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        archive
            .start_file("META-INF/container.xml", options)
            .expect("container");
        archive
            .write_all(br#"<container><rootfile full-path="OPS/book.opf"/></container>"#)
            .expect("container content");
        archive.start_file("OPS/book.opf", options).expect("OPF");
        archive
            .write_all(
                br#"<package><metadata><dc:title>Matrix EPUB</dc:title></metadata><manifest><item id="chapter" href="chapter.xhtml"/></manifest><spine><itemref idref="chapter"/></spine></package>"#,
            )
            .expect("OPF content");
        archive
            .start_file("OPS/chapter.xhtml", options)
            .expect("chapter");
        archive
            .write_all(b"<h1>EPUB chapter</h1><p>Safe EPUB text.</p>")
            .expect("chapter content");
        archive.finish().expect("EPUB archive");
    }

    fn write_test_docx(path: &Path) {
        let file = fs::File::create(path).expect("DOCX");
        let mut archive = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        archive
            .start_file("word/document.xml", options)
            .expect("document");
        archive
            .write_all(
                br#"<w:document xmlns:w="urn:test"><w:body><w:p><w:r><w:t>Safe DOCX text.</w:t></w:r></w:p></w:body></w:document>"#,
            )
            .expect("document content");
        archive.finish().expect("DOCX archive");
    }

    fn write_test_cbz(path: &Path) {
        let file = fs::File::create(path).expect("CBZ");
        let mut archive = zip::ZipWriter::new(file);
        archive
            .start_file("page-001.png", zip::write::SimpleFileOptions::default())
            .expect("page");
        archive.write_all(TEST_PNG).expect("page content");
        archive.finish().expect("CBZ archive");
    }

    fn write_test_cbr(path: &Path) {
        use rars::{
            rar15_40::{write_stored_archive, StoredEntry, WriterOptions},
            version::ArchiveVersion,
            FeatureSet,
        };
        let entries = [StoredEntry {
            name: b"page-001.png",
            data: TEST_PNG,
            file_time: 0,
            file_attr: 0,
            host_os: 2,
            password: None,
            file_comment: None,
        }];
        let bytes = write_stored_archive(
            &entries,
            WriterOptions::new(ArchiveVersion::Rar29, FeatureSet::store_only()),
        )
        .expect("RAR");
        fs::write(path, bytes).expect("CBR");
    }

    #[test]
    fn applies_migrations_idempotently() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let database = test_database(directory.path());
        assert_eq!(database.schema_version().expect("schema version"), 13);
        drop(database);
        let database = test_database(directory.path());
        assert_eq!(database.schema_version().expect("schema version"), 13);
    }

    #[test]
    fn records_clean_and_unclean_restarts_locally() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let database = test_database(directory.path());
        assert!(!database.startup_health().previous_exit_unclean);
        drop(database);

        let database = test_database(directory.path());
        assert!(database.startup_health().previous_exit_unclean);
        database.mark_clean_shutdown().expect("clean shutdown");
        drop(database);

        let database = test_database(directory.path());
        assert!(!database.startup_health().previous_exit_unclean);
    }

    #[test]
    fn restores_latest_valid_backup_and_preserves_corrupt_database() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source = directory.path().join("Recovery fixture.txt");
        fs::write(&source, "safe recovery fixture").expect("fixture");
        let mut database = test_database(directory.path());
        database
            .import_paths(std::slice::from_ref(&source))
            .expect("import and backup");
        drop(database);

        fs::write(
            directory.path().join("library.db"),
            b"not a sqlite database",
        )
        .expect("corrupt database");
        let mut recovered = test_database(directory.path());
        let health = recovered.startup_health();
        assert!(health.recovered_from_backup);
        assert_eq!(recovered.list_books().expect("restored books").len(), 1);
        let quarantined = health.quarantined_database.expect("quarantine path");
        assert_eq!(
            fs::read(quarantined).expect("preserved corrupt database"),
            b"not a sqlite database"
        );
        assert_eq!(
            fs::read_to_string(source).expect("source book remains unchanged"),
            "safe recovery fixture"
        );
    }

    #[test]
    fn imports_a_file_once_and_creates_backup() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source = directory.path().join("Quiet Book.txt");
        let fixture = b"A safe synthetic fixture.";
        fs::write(&source, fixture).expect("fixture");
        let mut database = test_database(directory.path());
        let first = database
            .import_paths(std::slice::from_ref(&source))
            .expect("first import");
        let second = database
            .import_paths(std::slice::from_ref(&source))
            .expect("second import");
        assert_eq!(first.imported, 1);
        assert_eq!(second.duplicates, 1);
        assert_eq!(database.list_books().expect("books").len(), 1);
        assert!(fs::read_dir(directory.path().join("backups"))
            .expect("backups")
            .next()
            .is_some());
        assert_eq!(fs::read(&source).expect("source remains readable"), fixture);
    }

    #[test]
    fn shell_open_imports_once_and_reuses_the_existing_book_record() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source = directory.path().join("Explorer Book.txt");
        let fixture = b"A safe Explorer fixture.";
        fs::write(&source, fixture).expect("fixture");
        let mut database = test_database(directory.path());

        let first = database
            .import_book_for_open(&source)
            .expect("first shell open");
        let second = database
            .import_book_for_open(&source)
            .expect("second shell open");

        assert_eq!(first.id, second.id);
        assert_eq!(database.list_books().expect("books").len(), 1);
        assert_eq!(fs::read(&source).expect("source remains readable"), fixture);
    }

    #[test]
    fn shell_open_reconnects_an_unavailable_duplicate_to_its_new_path() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let original = directory.path().join("Original.txt");
        let relocated = directory.path().join("Relocated.txt");
        let fixture = b"The same safely relocated book.";
        fs::write(&original, fixture).expect("original fixture");
        let mut database = test_database(directory.path());
        let imported = database
            .import_book_for_open(&original)
            .expect("initial import");

        fs::remove_file(&original).expect("simulate moved source");
        fs::write(&relocated, fixture).expect("relocated fixture");
        let reopened = database
            .import_book_for_open(&relocated)
            .expect("reconnect duplicate");

        assert_eq!(reopened.id, imported.id);
        assert_eq!(
            Path::new(&reopened.source_path),
            relocated.canonicalize().unwrap()
        );
        assert!(reopened.is_available);
        assert_eq!(fs::read(relocated).expect("relocated source"), fixture);
    }

    #[test]
    fn public_format_matrix_opens_valid_synthetic_books_without_source_changes() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let fixtures = directory.path().join("fixtures");
        fs::create_dir(&fixtures).expect("fixtures");
        let txt = fixtures.join("matrix.txt");
        let html = fixtures.join("matrix.html");
        let markdown = fixtures.join("matrix.md");
        let epub = fixtures.join("matrix.epub");
        let fb2 = fixtures.join("matrix.fb2");
        let docx = fixtures.join("matrix.docx");
        let pdf = fixtures.join("matrix.pdf");
        let cbz = fixtures.join("matrix.cbz");
        let cbr = fixtures.join("matrix.cbr");
        fs::write(&txt, "Safe TXT matrix text.").expect("TXT");
        fs::write(
            &html,
            "<h1>HTML matrix</h1><script src='https://invalid.test/x.js'></script><p>Safe HTML text.</p>",
        )
        .expect("HTML");
        fs::write(&markdown, "# Markdown matrix\n\nSafe Markdown text.").expect("Markdown");
        write_test_epub(&epub);
        fs::write(
            &fb2,
            r#"<?xml version="1.0"?><FictionBook><body><section><title><p>FB2 matrix</p></title><p>Safe FB2 text.</p></section></body></FictionBook>"#,
        )
        .expect("FB2");
        write_test_docx(&docx);
        fs::write(&pdf, b"%PDF-1.4\n% synthetic matrix\n%%EOF").expect("PDF");
        write_test_cbz(&cbz);
        write_test_cbr(&cbr);
        let paths = vec![txt, html, markdown, epub, fb2, docx, pdf, cbz, cbr];
        let originals = paths
            .iter()
            .map(|path| (path.clone(), fs::read(path).expect("source bytes")))
            .collect::<BTreeMap<_, _>>();

        let mut database = test_database(directory.path());
        let summary = database.import_paths(&paths).expect("matrix import");
        assert_eq!(summary.imported, 9);
        assert_eq!(summary.failed, 0);
        let books = database.list_books().expect("matrix books");
        assert_eq!(books.len(), 9);
        for book in books {
            match book.format.as_str() {
                "TXT" | "HTML" | "MD" | "EPUB" | "FB2" | "DOCX" => {
                    let document = database.load_document(book.id).expect("reflow document");
                    assert!(!document.sections.is_empty());
                    let text = document
                        .sections
                        .iter()
                        .flat_map(|section| &section.blocks)
                        .map(|block| block.text.as_str())
                        .collect::<Vec<_>>()
                        .join(" ");
                    assert!(!text.contains("invalid.test"));
                }
                "PDF" => {
                    let document = database
                        .load_special_document(book.id)
                        .expect("PDF document");
                    let cached = document.source_path.expect("cached PDF");
                    assert_eq!(
                        fs::read(cached).expect("cached PDF bytes"),
                        fs::read(&book.source_path).expect("source PDF bytes")
                    );
                }
                "CBZ" | "CBR" => {
                    let document = database
                        .load_special_document(book.id)
                        .expect("comic document");
                    assert_eq!(document.pages.len(), 1);
                }
                format => panic!("unexpected matrix format: {format}"),
            }
        }
        for (path, expected) in originals {
            assert_eq!(fs::read(path).expect("unchanged source"), expected);
        }
    }

    #[test]
    fn malformed_format_matrix_fails_safely_and_keeps_fixture_bytes() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let fixtures = [
            ("broken.txt", b"".as_slice()),
            (
                "broken.html",
                b"<script src='https://invalid.test/payload.js'>run()</script>".as_slice(),
            ),
            ("broken.md", b"---".as_slice()),
            ("broken.epub", b"not a ZIP archive".as_slice()),
            ("broken.fb2", b"not FictionBook XML".as_slice()),
            ("broken.docx", b"not a DOCX archive".as_slice()),
            ("broken.pdf", b"not a PDF".as_slice()),
            ("broken.cbz", b"not a CBZ archive".as_slice()),
            ("broken.cbr", b"not a CBR archive".as_slice()),
        ];
        let paths = fixtures
            .iter()
            .map(|(name, bytes)| {
                let path = directory.path().join(name);
                fs::write(&path, bytes).expect("malformed fixture");
                (path, bytes.to_vec())
            })
            .collect::<Vec<_>>();

        for (path, original) in &paths {
            let extension = path
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_ascii_uppercase();
            let failed_safely = match extension.as_str() {
                "TXT" | "HTML" | "MD" | "EPUB" | "FB2" | "DOCX" => read_document(path).is_err(),
                "PDF" | "CBZ" | "CBR" => prepare_special_document(
                    1,
                    "Broken fixture".to_owned(),
                    String::new(),
                    extension,
                    path,
                    "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
                    0.0,
                    0,
                    &directory.path().join("reader-cache"),
                )
                .is_err(),
                _ => unreachable!(),
            };
            assert!(failed_safely, "{} should fail safely", path.display());
            assert_eq!(fs::read(path).expect("fixture remains"), *original);
        }
    }

    #[test]
    fn removes_a_book_record_and_local_derivatives_but_keeps_the_source() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source = directory.path().join("Removable Book.txt");
        fs::write(&source, "source content must remain").expect("fixture");
        let mut database = test_database(directory.path());
        database
            .import_paths(std::slice::from_ref(&source))
            .expect("import");
        let book_id = database.list_books().expect("books")[0].id;
        let fingerprint = database
            .connection
            .query_row(
                "SELECT fingerprint FROM books WHERE id = ?1",
                [book_id],
                |row| row.get::<_, String>(0),
            )
            .expect("fingerprint");
        database
            .connection
            .execute(
                "INSERT INTO annotations(
                    book_id, kind, section_id, block_index, start_offset, end_offset
                 ) VALUES (?1, 'bookmark', 'section-1', 0, 0, 0)",
                [book_id],
            )
            .expect("annotation");
        database
            .connection
            .execute(
                "INSERT INTO book_search(
                    book_id, section_id, block_index, section_title, body
                 ) VALUES (?1, 'section-1', 0, 'Fixture', 'indexed text')",
                [book_id],
            )
            .expect("search index");
        let reader_cache = database
            .reader_cache_dir
            .join(fingerprint.get(..24).unwrap_or(&fingerprint));
        fs::create_dir_all(&reader_cache).expect("reader cache");
        fs::write(reader_cache.join("page.bin"), b"cache").expect("cache file");

        assert_eq!(database.remove_books(&[book_id]).expect("remove"), 1);
        assert!(database.list_books().expect("books").is_empty());
        assert_eq!(
            database
                .connection
                .query_row("SELECT COUNT(*) FROM annotations", [], |row| {
                    row.get::<_, i64>(0)
                })
                .expect("annotation count"),
            0
        );
        assert_eq!(
            database
                .connection
                .query_row("SELECT COUNT(*) FROM book_search", [], |row| {
                    row.get::<_, i64>(0)
                })
                .expect("search count"),
            0
        );
        assert!(!reader_cache.exists());
        assert_eq!(
            fs::read_to_string(source).expect("source remains"),
            "source content must remain"
        );
    }

    #[test]
    fn removes_several_unique_book_ids_as_one_batch() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let first = directory.path().join("First removable.txt");
        let second = directory.path().join("Second removable.txt");
        fs::write(&first, "first source").expect("first fixture");
        fs::write(&second, "second source").expect("second fixture");
        let mut database = test_database(directory.path());
        database
            .import_paths(&[first.clone(), second.clone()])
            .expect("import");
        let ids = database
            .list_books()
            .expect("books")
            .into_iter()
            .map(|book| book.id)
            .collect::<Vec<_>>();

        assert_eq!(
            database
                .remove_books(&[ids[0], ids[1], ids[1], i64::MAX])
                .expect("batch remove"),
            2
        );
        assert!(database.list_books().expect("books").is_empty());
        assert!(first.is_file());
        assert!(second.is_file());
        assert!(matches!(
            database.remove_books(&[]),
            Err(DatabaseError::InvalidBookSelection)
        ));
    }

    #[test]
    fn updates_a_changed_book_at_the_same_source_path() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source = directory.path().join("Living document.txt");
        fs::write(&source, "first revision").expect("first revision");
        let mut database = test_database(directory.path());
        let first = database
            .import_paths(std::slice::from_ref(&source))
            .expect("first import");
        fs::write(&source, "second revision").expect("second revision");
        let second = database
            .import_paths(std::slice::from_ref(&source))
            .expect("updated import");
        assert_eq!(first.imported, 1);
        assert_eq!(second.imported, 1);
        assert_eq!(database.list_books().expect("books").len(), 1);
        assert_eq!(
            fs::read_to_string(&source).expect("updated source"),
            "second revision"
        );
    }

    #[test]
    fn manual_metadata_is_local_and_survives_source_rescan() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source = directory.path().join("Original title.txt");
        fs::write(&source, "first revision").expect("fixture");
        let mut database = test_database(directory.path());
        database
            .import_paths(std::slice::from_ref(&source))
            .expect("import");
        let book_id = database.list_books().expect("books")[0].id;
        let edited = database
            .update_book_metadata(
                book_id,
                &BookMetadataInput {
                    title: "A corrected title".to_owned(),
                    author: "Local Author".to_owned(),
                    subtitle: "Subtitle".to_owned(),
                    isbn: "9780000000001".to_owned(),
                    publisher: "Local Press".to_owned(),
                    published_year: "2026".to_owned(),
                    language: "eng".to_owned(),
                    series: "A Series".to_owned(),
                    genres: "Science Fiction; Adventure, science fiction".to_owned(),
                    description: "Stored only in the local library.".to_owned(),
                },
            )
            .expect("metadata");
        assert_eq!(edited.metadata_source, "manual");
        fs::write(&source, "second revision").expect("changed source");
        database
            .import_paths(std::slice::from_ref(&source))
            .expect("rescan");
        let restored = database.list_books().expect("books").remove(0);
        assert_eq!(restored.title, "A corrected title");
        assert_eq!(restored.author, "Local Author");
        assert_eq!(restored.genres, "Science Fiction, Adventure");
        assert_eq!(
            fs::read_to_string(source).expect("source remains unchanged"),
            "second revision"
        );
    }

    #[test]
    fn favorite_marker_is_local_and_persistent() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source = directory.path().join("Favorite fixture.txt");
        fs::write(&source, "Favorite fixture.").expect("fixture");
        let mut database = test_database(directory.path());
        database
            .import_paths(std::slice::from_ref(&source))
            .expect("import");
        let book_id = database.list_books().expect("books")[0].id;
        assert!(!database.list_books().expect("books")[0].is_favorite);

        let favorite = database.set_book_favorite(book_id, true).expect("favorite");
        assert!(favorite.is_favorite);
        drop(database);

        let mut reopened = test_database(directory.path());
        assert!(reopened.list_books().expect("books")[0].is_favorite);
        let restored = reopened
            .set_book_favorite(book_id, false)
            .expect("remove favorite");
        assert!(!restored.is_favorite);
        assert_eq!(
            fs::read_to_string(source).expect("source remains unchanged"),
            "Favorite fixture."
        );
    }

    #[test]
    fn metadata_search_uses_fresh_local_cache_without_network() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source = directory.path().join("Cached lookup.txt");
        fs::write(&source, "fixture").expect("fixture");
        let mut database = test_database(directory.path());
        database
            .import_paths(std::slice::from_ref(&source))
            .expect("import");
        let book_id = database.list_books().expect("books")[0].id;
        let cached = vec![MetadataCandidate {
            provider: "Open Library".to_owned(),
            provider_id: "/works/OL1W".to_owned(),
            title: "Cached lookup".to_owned(),
            author: "Cached Author".to_owned(),
            isbn: String::new(),
            publisher: String::new(),
            published_year: String::new(),
            language: String::new(),
            series: String::new(),
            genres: "Mystery".to_owned(),
            cover_id: None,
        }];
        database
            .connection
            .execute(
                "INSERT INTO metadata_cache(query_key, response_json, fetched_at)
                 VALUES (?1, ?2, ?3)",
                params![
                    "en:cached lookup",
                    serde_json::to_string(&cached).expect("json"),
                    unix_seconds()
                ],
            )
            .expect("cache");
        let results = database
            .search_metadata(book_id, "Cached lookup", "en")
            .expect("cached results");
        assert_eq!(results, cached);
    }

    #[test]
    fn local_cover_is_validated_copied_and_restored_without_touching_source() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source = directory.path().join("Local cover.txt");
        let cover = directory.path().join("chosen.png");
        fs::write(&source, "fixture").expect("fixture");
        fs::write(&cover, b"\x89PNG\r\n\x1a\npayload").expect("cover fixture");
        let mut database = test_database(directory.path());
        database
            .import_paths(std::slice::from_ref(&source))
            .expect("import");
        let book_id = database.list_books().expect("books")[0].id;

        let updated = database
            .set_local_cover(book_id, &cover)
            .expect("local cover");
        assert_eq!(updated.cover_source, "local");
        let cached_cover = PathBuf::from(updated.cover_path.expect("cover path"));
        assert!(cached_cover.is_file());
        assert_ne!(cached_cover, cover);

        let restored = database
            .remove_external_cover(book_id)
            .expect("restore embedded cover");
        assert_eq!(restored.cover_source, "embedded");
        assert!(!cached_cover.exists());
        assert_eq!(fs::read_to_string(&source).expect("source"), "fixture");
        assert!(cover.is_file());
    }

    #[test]
    fn loads_text_and_persists_a_valid_reading_position() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source = directory.path().join("Readable.txt");
        fs::write(&source, "First paragraph.\n\nSecond paragraph.").expect("fixture");
        let mut database = test_database(directory.path());
        database
            .import_paths(std::slice::from_ref(&source))
            .expect("import");
        let book_id = database.list_books().expect("books")[0].id;
        assert_eq!(
            database.list_books().expect("books")[0].last_opened_at,
            None
        );
        let document = database.load_document(book_id).expect("document");
        assert_eq!(document.sections[0].blocks.len(), 2);
        assert!(database.list_books().expect("books")[0]
            .last_opened_at
            .is_some());
        database
            .save_reading_position(book_id, 0, 0.5, 0.5)
            .expect("position");
        let restored = database.load_document(book_id).expect("restored");
        assert_eq!(restored.section_progress, 0.5);
        assert_eq!(restored.progress, 0.5);
        assert!(database
            .save_reading_position(book_id, 0, 1.5, 1.0)
            .is_err());
    }

    #[test]
    fn failed_open_does_not_add_a_book_to_reading_now() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source = directory.path().join("Unavailable.txt");
        fs::write(&source, "Temporary fixture.").expect("fixture");
        let mut database = test_database(directory.path());
        database
            .import_paths(std::slice::from_ref(&source))
            .expect("import");
        let book_id = database.list_books().expect("books")[0].id;
        fs::remove_file(&source).expect("remove fixture");

        assert!(database.load_document(book_id).is_err());
        assert_eq!(
            database.list_books().expect("books")[0].last_opened_at,
            None
        );
    }

    #[test]
    fn indexes_text_and_persists_annotations_with_stable_locators() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source = directory.path().join("Searchable.md");
        fs::write(
            &source,
            "# First\n\nA quiet brass lamp.\n\n# Second\n\nAnother room.",
        )
        .expect("fixture");
        let mut database = test_database(directory.path());
        database
            .import_paths(std::slice::from_ref(&source))
            .expect("import");
        let book_id = database.list_books().expect("books")[0].id;
        database.load_document(book_id).expect("document");

        let results = database.search_book(book_id, "brass lamp").expect("search");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].section_id, "section-1");
        assert!(results[0].excerpt.contains('‹'));

        let annotation = database
            .create_annotation(
                book_id,
                "note",
                "section-1",
                0,
                8,
                18,
                "brass lamp",
                "Remember this image.",
            )
            .expect("annotation");
        assert_eq!(annotation.locator.block_index, 0);
        assert_eq!(annotation.selected_text, "brass lamp");
        let updated = database
            .update_annotation_note(annotation.id, "A revised note.")
            .expect("updated note");
        assert_eq!(updated.note, "A revised note.");

        let export = directory.path().join("notes.md");
        database
            .export_annotations(book_id, &export)
            .expect("export");
        let exported = fs::read_to_string(export).expect("exported markdown");
        assert!(exported.contains("> brass lamp"));
        assert!(exported.contains("A revised note."));

        database
            .delete_annotation(annotation.id)
            .expect("delete annotation");
        assert!(database
            .list_annotations(book_id)
            .expect("annotations")
            .is_empty());
        assert_eq!(
            fs::read_to_string(source).expect("source remains unchanged"),
            "# First\n\nA quiet brass lamp.\n\n# Second\n\nAnother room."
        );
    }

    #[test]
    fn rejects_malformed_search_and_annotation_payloads() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source = directory.path().join("Safe.txt");
        fs::write(&source, "Safe text").expect("fixture");
        let mut database = test_database(directory.path());
        database
            .import_paths(std::slice::from_ref(&source))
            .expect("import");
        let book_id = database.list_books().expect("books")[0].id;
        database.load_document(book_id).expect("document");
        assert!(database.search_book(book_id, "\" *").is_err());
        assert!(database
            .create_annotation(book_id, "script", "text", 0, 0, 1, "S", "")
            .is_err());
        assert!(database
            .create_annotation(book_id, "highlight", "text", 0, 4, 2, "text", "")
            .is_err());
    }

    #[test]
    fn watched_folder_skips_symlinks_and_imports_supported_files() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let watched = directory.path().join("watched");
        fs::create_dir(&watched).expect("watched");
        fs::write(watched.join("One.md"), "# One").expect("markdown");
        fs::write(watched.join("ignore.exe"), "no").expect("ignored");
        let mut database = test_database(directory.path());
        let summary = database.add_watched_folder(&watched).expect("scan");
        assert_eq!(summary.imported, 1);
        assert_eq!(database.list_watched_folders().expect("folders").len(), 1);
    }

    #[test]
    fn watched_folder_scan_has_a_file_count_limit() {
        let directory = tempfile::tempdir().expect("temporary directory");
        for name in ["one.txt", "two.epub", "three.md"] {
            fs::write(directory.path().join(name), b"fixture").expect("fixture");
        }
        let mut files = Vec::new();
        let result = collect_supported_files_bounded(directory.path(), 0, &mut files, 2);
        assert!(matches!(result, Err(DatabaseError::TooManyWatchedBooks)));
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn imports_and_naturally_orders_a_multi_part_audiobook() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let audiobook = directory.path().join("The Long Journey");
        fs::create_dir(&audiobook).expect("audiobook directory");
        let part_ten = audiobook.join("Part 10.mp3");
        let part_two = audiobook.join("Part 2.mp3");
        fs::write(&part_ten, b"synthetic audio ten").expect("part ten");
        fs::write(&part_two, b"synthetic audio two").expect("part two");
        let mut database = test_database(directory.path());

        let first = database
            .import_audiobooks(std::slice::from_ref(&audiobook))
            .expect("audio import");
        assert_eq!(first.imported_books, 1);
        assert_eq!(first.imported_parts, 2);
        assert_eq!(first.failed, 0);
        let books = database.list_audiobooks().expect("audiobooks");
        assert_eq!(books.len(), 1);
        assert_eq!(books[0].title, "The Long Journey");
        assert_eq!(books[0].part_count, 2);
        assert_eq!(books[0].total_size, 38);
        let parts = database.list_audiobook_parts(books[0].id).expect("parts");
        assert_eq!(parts[0].title, "Part 2");
        assert_eq!(parts[0].ordinal, 0);
        assert_eq!(parts[1].title, "Part 10");

        let second = database
            .import_audiobooks(std::slice::from_ref(&audiobook))
            .expect("duplicate scan");
        assert_eq!(second.imported_books, 0);
        assert_eq!(second.imported_parts, 0);
        assert_eq!(second.duplicate_parts, 2);
        assert_eq!(database.list_audiobooks().expect("audiobooks").len(), 1);
        assert_eq!(fs::read(&part_two).expect("source"), b"synthetic audio two");
    }

    #[test]
    fn persists_bounded_audiobook_position_and_observed_part_durations() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let audiobook = directory.path().join("Measured Book");
        fs::create_dir(&audiobook).expect("audiobook directory");
        fs::write(audiobook.join("01.mp3"), b"first audio").expect("first part");
        fs::write(audiobook.join("02.mp3"), b"second audio").expect("second part");
        let mut database = test_database(directory.path());
        database
            .import_audiobooks(std::slice::from_ref(&audiobook))
            .expect("audio import");
        let book_id = database.list_audiobooks().expect("books")[0].id;

        let first = database
            .save_audiobook_position(book_id, 0, 50.0, 100.0)
            .expect("first position");
        assert!((first.progress - 0.25).abs() < 0.0001);
        assert_eq!(first.last_part_index, 0);
        assert_eq!(first.last_position_seconds, 50.0);

        let second = database
            .save_audiobook_position(book_id, 1, 25.0, 100.0)
            .expect("second position");
        assert!((second.progress - 0.625).abs() < 0.0001);
        assert_eq!(second.total_duration_seconds, 200.0);
        assert_eq!(second.last_part_index, 1);
        let parts = database.list_audiobook_parts(book_id).expect("parts");
        assert_eq!(parts[0].duration_seconds, Some(100.0));
        assert_eq!(parts[1].duration_seconds, Some(100.0));

        assert!(database
            .save_audiobook_position(book_id, 1, f64::NAN, 100.0)
            .is_err());
        assert!(database
            .save_audiobook_position(book_id, 8, 0.0, 100.0)
            .is_err());
    }

    #[test]
    fn updates_extended_audiobook_metadata_without_touching_source_audio() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source = directory.path().join("Metadata Story.mp3");
        let fixture = b"synthetic audio metadata fixture";
        fs::write(&source, fixture).expect("audio fixture");
        let mut database = test_database(directory.path());
        database
            .import_audiobooks(std::slice::from_ref(&source))
            .expect("audio import");
        let book = database.list_audiobooks().expect("books").remove(0);

        let updated = database
            .update_audiobook_metadata(
                book.id,
                &AudiobookMetadataInput {
                    title: "The Metadata Story".into(),
                    author: "Alex Writer".into(),
                    narrator: "Maria Voice".into(),
                    series: "Homeward".into(),
                    genres: " Fiction, Journey, fiction ".into(),
                    description: "A local description.".into(),
                    language: "en".into(),
                    published_year: "2026".into(),
                },
            )
            .expect("metadata update");

        assert_eq!(updated.narrator, "Maria Voice");
        assert_eq!(updated.series, "Homeward");
        assert_eq!(updated.genres, "Fiction, Journey");
        assert_eq!(updated.metadata_source, "manual");
        assert_eq!(fs::read(source).expect("source remains readable"), fixture);
    }

    #[test]
    fn shell_open_imports_audio_once_and_reuses_the_existing_audiobook() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source = directory.path().join("Explorer Story.m4b");
        let fixture = b"synthetic shell-open audiobook";
        fs::write(&source, fixture).expect("audio fixture");
        let mut database = test_database(directory.path());

        let first = database
            .import_audiobook_for_open(&source)
            .expect("first shell open");
        let second = database
            .import_audiobook_for_open(&source)
            .expect("second shell open");

        assert_eq!(first.id, second.id);
        assert_eq!(database.list_audiobooks().expect("books").len(), 1);
        assert_eq!(
            database
                .list_audiobook_parts(first.id)
                .expect("parts")
                .len(),
            1
        );
        assert_eq!(fs::read(source).expect("source remains readable"), fixture);
    }

    #[test]
    fn imports_cue_chapters_and_manages_local_audio_bookmarks() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let audio = directory.path().join("narration.flac");
        let cue = directory.path().join("narration.cue");
        fs::write(&audio, b"synthetic narration").expect("audio");
        fs::write(
            &cue,
            "TITLE \"Night Library\"\nFILE \"narration.flac\" WAVE\nTRACK 01 AUDIO\nTITLE \"Door\"\nINDEX 01 00:00:00\nTRACK 02 AUDIO\nTITLE \"Reading Room\"\nINDEX 01 00:30:00\n",
        )
        .expect("cue");
        let mut database = test_database(directory.path());
        let summary = database
            .import_audiobooks(std::slice::from_ref(&cue))
            .expect("descriptor import");
        assert_eq!(summary.imported_books, 1);
        let book = database.list_audiobooks().expect("books")[0].clone();
        assert_eq!(book.title, "Night Library");
        let chapters = database.list_audiobook_chapters(book.id).expect("chapters");
        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[1].title, "Reading Room");
        assert_eq!(chapters[1].start_seconds, 30.0);

        database
            .save_audiobook_position(book.id, 0, 15.0, 60.0)
            .expect("observed duration");
        let bookmark = database
            .create_audiobook_bookmark(book.id, 0, 15.0, "  Important   passage  ")
            .expect("bookmark");
        assert_eq!(bookmark.note, "Important passage");
        assert_eq!(database.list_audiobook_bookmarks(book.id).unwrap().len(), 1);
        database
            .delete_audiobook_bookmark(bookmark.id)
            .expect("delete bookmark");
        assert!(database
            .list_audiobook_bookmarks(book.id)
            .unwrap()
            .is_empty());
        assert!(database
            .create_audiobook_bookmark(book.id, 0, 70.0, "invalid")
            .is_err());
    }

    #[test]
    fn watched_audio_root_keeps_single_files_separate_and_groups_subfolders() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let watched = directory.path().join("audio");
        let nested = watched.join("Nested Book");
        fs::create_dir_all(&nested).expect("watched folders");
        fs::write(watched.join("First.m4b"), b"first single").expect("single");
        fs::write(watched.join("Second.mp3"), b"second single").expect("single");
        let nested_one = nested.join("01.mp3");
        let nested_two = nested.join("02.mp3");
        fs::write(&nested_one, b"nested one").expect("nested part");
        fs::write(&nested_two, b"nested two").expect("nested part");
        fs::write(watched.join("ignored.exe"), b"ignored").expect("ignored");
        let mut database = test_database(directory.path());

        let summary = database
            .add_watched_audio_folder(&watched)
            .expect("watched audio import");
        assert_eq!(summary.imported_books, 3);
        assert_eq!(summary.imported_parts, 4);
        assert_eq!(summary.failed, 0);
        assert_eq!(
            database
                .list_watched_audio_folders()
                .expect("watched folders")
                .len(),
            1
        );
        let books = database.list_audiobooks().expect("audiobooks");
        assert_eq!(books.len(), 3);
        let nested_book = books
            .iter()
            .find(|book| book.title == "Nested Book")
            .expect("nested book");
        assert_eq!(nested_book.part_count, 2);

        fs::remove_file(&nested_two).expect("remove one part");
        let books = database.list_audiobooks().expect("availability refresh");
        let nested_book = books
            .iter()
            .find(|book| book.title == "Nested Book")
            .expect("nested book");
        assert!(!nested_book.is_available);
        let parts = database
            .list_audiobook_parts(nested_book.id)
            .expect("parts");
        assert_eq!(parts.iter().filter(|part| !part.is_available).count(), 1);
        let rescan = database
            .scan_watched_audio_folders()
            .expect("watched rescan");
        assert_eq!(rescan.imported_books, 0);
        assert_eq!(database.list_audiobooks().expect("audiobooks").len(), 3);
        assert!(nested_one.is_file());
    }

    #[test]
    fn changed_single_audio_source_updates_without_leaving_an_empty_book() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source = directory.path().join("Living Audio.mp3");
        fs::write(&source, b"first audio revision").expect("first revision");
        let mut database = test_database(directory.path());
        database
            .import_audiobooks(std::slice::from_ref(&source))
            .expect("first import");
        fs::write(&source, b"second audio revision").expect("second revision");
        let updated = database
            .import_audiobooks(std::slice::from_ref(&source))
            .expect("updated import");
        assert_eq!(updated.imported_parts, 1);
        assert_eq!(database.list_audiobooks().expect("audiobooks").len(), 1);
        let book = database.list_audiobooks().expect("audiobooks").remove(0);
        assert_eq!(
            database.list_audiobook_parts(book.id).expect("parts").len(),
            1
        );
        assert_eq!(fs::read(source).expect("source"), b"second audio revision");
    }

    #[test]
    fn duplicate_single_audio_content_does_not_create_a_second_book() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let first_dir = directory.path().join("first");
        let second_dir = directory.path().join("second");
        fs::create_dir(&first_dir).expect("first directory");
        fs::create_dir(&second_dir).expect("second directory");
        let first = first_dir.join("Original.mp3");
        let copy = second_dir.join("Copy.mp3");
        fs::write(&first, b"identical audio content").expect("first");
        fs::write(&copy, b"identical audio content").expect("copy");
        let mut database = test_database(directory.path());
        database
            .import_audiobooks(std::slice::from_ref(&first))
            .expect("first import");
        let duplicate = database
            .import_audiobooks(std::slice::from_ref(&copy))
            .expect("duplicate import");
        assert_eq!(duplicate.imported_books, 0);
        assert_eq!(duplicate.imported_parts, 0);
        assert_eq!(duplicate.duplicate_parts, 1);
        let books = database.list_audiobooks().expect("audiobooks");
        assert_eq!(books.len(), 1);
        assert_eq!(
            database
                .list_audiobook_parts(books[0].id)
                .expect("parts")
                .len(),
            1
        );
        assert!(first.is_file());
        assert!(copy.is_file());
    }

    #[test]
    fn relocated_multi_part_book_reconnects_by_content() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let original = directory.path().join("Original Audio");
        let relocated = directory.path().join("Relocated Audio");
        fs::create_dir(&original).expect("original directory");
        let original_one = original.join("01.mp3");
        let original_two = original.join("02.mp3");
        fs::write(&original_one, b"relocated part one").expect("part one");
        fs::write(&original_two, b"relocated part two").expect("part two");
        let mut database = test_database(directory.path());
        database
            .import_audiobooks(std::slice::from_ref(&original))
            .expect("initial import");

        fs::remove_dir_all(&original).expect("remove original folder");
        fs::create_dir(&relocated).expect("relocated directory");
        fs::write(relocated.join("01.mp3"), b"relocated part one").expect("part one");
        fs::write(relocated.join("02.mp3"), b"relocated part two").expect("part two");
        let result = database
            .import_audiobooks(std::slice::from_ref(&relocated))
            .expect("relocated import");
        assert_eq!(result.imported_books, 0);
        assert_eq!(result.imported_parts, 2);
        let books = database.list_audiobooks().expect("audiobooks");
        assert_eq!(books.len(), 1);
        assert_eq!(books[0].title, "Relocated Audio");
        assert!(books[0].is_available);
        let parts = database.list_audiobook_parts(books[0].id).expect("parts");
        assert_eq!(parts.len(), 2);
        assert!(parts
            .iter()
            .all(|part| part.source_path.contains("Relocated Audio")));
    }
}
