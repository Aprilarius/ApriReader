mod database;
mod fonts;
mod importer;
mod metadata;
mod reader;
mod special_reader;
mod statistics;
mod steam;

use database::{
    AnnotationRecord, BookMetadataInput, BookRecord, Database, ImportSummary, SearchResult,
    StartupHealth, WatchedFolder,
};
use fonts::ImportedReaderFont;
use metadata::MetadataCandidate;
use reader::DocumentModel;
use special_reader::SpecialDocument;
use statistics::{AchievementProgress, StatisticsSnapshot};
use std::{
    collections::VecDeque,
    ffi::OsString,
    path::{Path, PathBuf},
    sync::Mutex,
};
use steam::{SteamIntegrationStatus, SteamSyncResult};
use tauri::{Emitter, Manager, State};

const MAX_PENDING_LAUNCH_BOOKS: usize = 32;

struct LibraryState {
    database: Mutex<Database>,
}

struct LaunchBooksState {
    paths: Mutex<VecDeque<PathBuf>>,
}

fn with_database<T>(
    state: &State<'_, LibraryState>,
    operation: impl FnOnce(&mut Database) -> Result<T, database::DatabaseError>,
) -> Result<T, String> {
    let mut database = state
        .database
        .lock()
        .map_err(|_| "the local library database is unavailable".to_owned())?;
    operation(&mut database).map_err(|error| error.to_string())
}

#[tauri::command]
fn list_books(state: State<'_, LibraryState>) -> Result<Vec<BookRecord>, String> {
    with_database(&state, Database::list_books)
}

#[tauri::command]
fn remove_books(book_ids: Vec<i64>, state: State<'_, LibraryState>) -> Result<usize, String> {
    with_database(&state, |database| database.remove_books(&book_ids))
}

#[tauri::command]
fn import_reader_font(path: String, app: tauri::AppHandle) -> Result<ImportedReaderFont, String> {
    let destination = app
        .path()
        .app_local_data_dir()
        .map_err(|error| error.to_string())?
        .join("fonts");
    fonts::import_reader_font(PathBuf::from(path).as_path(), &destination)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn get_startup_health(state: State<'_, LibraryState>) -> Result<StartupHealth, String> {
    with_database(&state, |database| Ok(database.startup_health()))
}

#[tauri::command]
fn import_books(
    paths: Vec<String>,
    state: State<'_, LibraryState>,
) -> Result<ImportSummary, String> {
    let paths = paths.into_iter().map(PathBuf::from).collect::<Vec<_>>();
    with_database(&state, |database| database.import_paths(&paths))
}

#[tauri::command]
fn take_launch_book_paths(state: State<'_, LaunchBooksState>) -> Result<Vec<String>, String> {
    let mut paths = state
        .paths
        .lock()
        .map_err(|_| "the launch-file queue is unavailable".to_owned())?;
    Ok(paths
        .drain(..)
        .map(|path| path.to_string_lossy().into_owned())
        .collect())
}

#[tauri::command]
fn open_book_path(path: String, state: State<'_, LibraryState>) -> Result<BookRecord, String> {
    with_database(&state, |database| {
        database.import_book_for_open(PathBuf::from(path).as_path())
    })
}

#[tauri::command]
fn add_watched_folder(
    path: String,
    state: State<'_, LibraryState>,
) -> Result<ImportSummary, String> {
    with_database(&state, |database| {
        database.add_watched_folder(PathBuf::from(path).as_path())
    })
}

#[tauri::command]
fn list_watched_folders(state: State<'_, LibraryState>) -> Result<Vec<WatchedFolder>, String> {
    with_database(&state, |database| database.list_watched_folders())
}

#[tauri::command]
fn scan_watched_folders(state: State<'_, LibraryState>) -> Result<ImportSummary, String> {
    with_database(&state, Database::scan_watched_folders)
}

#[tauri::command]
fn update_book_metadata(
    book_id: i64,
    metadata: BookMetadataInput,
    state: State<'_, LibraryState>,
) -> Result<BookRecord, String> {
    with_database(&state, |database| {
        database.update_book_metadata(book_id, &metadata)
    })
}

#[tauri::command]
fn set_book_favorite(
    book_id: i64,
    favorite: bool,
    state: State<'_, LibraryState>,
) -> Result<BookRecord, String> {
    with_database(&state, |database| {
        database.set_book_favorite(book_id, favorite)
    })
}

#[tauri::command]
fn search_metadata(
    book_id: i64,
    query: String,
    state: State<'_, LibraryState>,
) -> Result<Vec<MetadataCandidate>, String> {
    with_database(&state, |database| database.search_metadata(book_id, &query))
}

#[tauri::command]
fn apply_metadata_candidate(
    book_id: i64,
    candidate: MetadataCandidate,
    state: State<'_, LibraryState>,
) -> Result<BookRecord, String> {
    with_database(&state, |database| {
        database.apply_metadata_candidate(book_id, &candidate)
    })
}

#[tauri::command]
fn remove_external_cover(
    book_id: i64,
    state: State<'_, LibraryState>,
) -> Result<BookRecord, String> {
    with_database(&state, |database| database.remove_external_cover(book_id))
}

#[tauri::command]
fn load_document(book_id: i64, state: State<'_, LibraryState>) -> Result<DocumentModel, String> {
    with_database(&state, |database| database.load_document(book_id))
}

#[tauri::command]
fn load_special_document(
    book_id: i64,
    state: State<'_, LibraryState>,
) -> Result<SpecialDocument, String> {
    with_database(&state, |database| database.load_special_document(book_id))
}

#[tauri::command]
fn save_reading_position(
    book_id: i64,
    section: usize,
    section_progress: f64,
    progress: f64,
    state: State<'_, LibraryState>,
) -> Result<(), String> {
    with_database(&state, |database| {
        database.save_reading_position(book_id, section, section_progress, progress)
    })
}

#[tauri::command]
fn start_reading_session(
    book_id: i64,
    progress: f64,
    words: i64,
    pages: i64,
    state: State<'_, LibraryState>,
) -> Result<String, String> {
    with_database(&state, |database| {
        database.start_reading_session(book_id, progress, words, pages)
    })
}

#[tauri::command]
fn record_reading_activity(
    token: String,
    active: bool,
    progress: f64,
    words: i64,
    pages: i64,
    state: State<'_, LibraryState>,
) -> Result<(), String> {
    with_database(&state, |database| {
        database.record_reading_activity(&token, active, progress, words, pages)
    })
}

#[tauri::command]
fn end_reading_session(token: String, state: State<'_, LibraryState>) -> Result<(), String> {
    with_database(&state, |database| database.end_reading_session(&token))
}

#[tauri::command]
fn get_statistics(state: State<'_, LibraryState>) -> Result<StatisticsSnapshot, String> {
    with_database(&state, |database| database.statistics_snapshot())
}

#[tauri::command]
fn get_achievements(state: State<'_, LibraryState>) -> Result<Vec<AchievementProgress>, String> {
    with_database(&state, |database| database.achievements())
}

#[tauri::command]
fn set_daily_goal(minutes: i64, state: State<'_, LibraryState>) -> Result<(), String> {
    with_database(&state, |database| database.set_daily_goal(minutes))
}

#[tauri::command]
fn clear_reading_statistics(state: State<'_, LibraryState>) -> Result<(), String> {
    with_database(&state, |database| database.clear_reading_statistics())
}

#[tauri::command]
fn get_steam_integration_status(
    state: State<'_, LibraryState>,
) -> Result<SteamIntegrationStatus, String> {
    with_database(&state, |database| database.steam_integration_status())
}

#[tauri::command]
fn sync_steam_achievements(state: State<'_, LibraryState>) -> Result<SteamSyncResult, String> {
    with_database(&state, |database| database.sync_steam_achievements())
}

#[tauri::command]
fn search_book(
    book_id: i64,
    query: String,
    state: State<'_, LibraryState>,
) -> Result<Vec<SearchResult>, String> {
    with_database(&state, |database| database.search_book(book_id, &query))
}

#[tauri::command]
fn list_annotations(
    book_id: i64,
    state: State<'_, LibraryState>,
) -> Result<Vec<AnnotationRecord>, String> {
    with_database(&state, |database| database.list_annotations(book_id))
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn create_annotation(
    book_id: i64,
    kind: String,
    section_id: String,
    block_index: usize,
    start_offset: usize,
    end_offset: usize,
    selected_text: String,
    note: String,
    state: State<'_, LibraryState>,
) -> Result<AnnotationRecord, String> {
    with_database(&state, |database| {
        database.create_annotation(
            book_id,
            &kind,
            &section_id,
            block_index,
            start_offset,
            end_offset,
            &selected_text,
            &note,
        )
    })
}

#[tauri::command]
fn update_annotation_note(
    annotation_id: i64,
    note: String,
    state: State<'_, LibraryState>,
) -> Result<AnnotationRecord, String> {
    with_database(&state, |database| {
        database.update_annotation_note(annotation_id, &note)
    })
}

#[tauri::command]
fn delete_annotation(annotation_id: i64, state: State<'_, LibraryState>) -> Result<(), String> {
    with_database(&state, |database| database.delete_annotation(annotation_id))
}

#[tauri::command]
fn export_annotations(
    book_id: i64,
    path: String,
    state: State<'_, LibraryState>,
) -> Result<(), String> {
    with_database(&state, |database| {
        database.export_annotations(book_id, PathBuf::from(path).as_path())
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let initial_cwd = std::env::current_dir().unwrap_or_default();
    let initial_paths = collect_launch_book_paths(std::env::args_os().skip(1), &initial_cwd);
    let mut builder = tauri::Builder::default().manage(LaunchBooksState {
        paths: Mutex::new(initial_paths.into()),
    });

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            let launch_paths = collect_launch_book_paths(
                argv.into_iter().skip(1).map(OsString::from),
                Path::new(&cwd),
            );
            if !launch_paths.is_empty() {
                if let Ok(mut pending) = app.state::<LaunchBooksState>().paths.lock() {
                    for path in launch_paths {
                        if pending.len() == MAX_PENDING_LAUNCH_BOOKS {
                            pending.pop_front();
                        }
                        if !pending.iter().any(|queued| same_launch_path(queued, &path)) {
                            pending.push_back(path);
                        }
                    }
                }
                let _ = app.emit("open-book-paths", ());
            }
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }));
    }

    let app = builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app.path().app_local_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let database = Database::open(
                &data_dir.join("library.db"),
                data_dir.join("covers"),
                data_dir.join("backups"),
            )
            .map_err(|error| -> Box<dyn std::error::Error> { Box::new(error) })?;
            app.manage(LibraryState {
                database: Mutex::new(database),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_startup_health,
            import_reader_font,
            list_books,
            remove_books,
            import_books,
            take_launch_book_paths,
            open_book_path,
            add_watched_folder,
            list_watched_folders,
            scan_watched_folders,
            update_book_metadata,
            set_book_favorite,
            search_metadata,
            apply_metadata_candidate,
            remove_external_cover,
            load_document,
            load_special_document,
            save_reading_position,
            start_reading_session,
            record_reading_activity,
            end_reading_session,
            get_statistics,
            get_achievements,
            set_daily_goal,
            clear_reading_statistics,
            get_steam_integration_status,
            sync_steam_achievements,
            search_book,
            list_annotations,
            create_annotation,
            update_annotation_note,
            delete_annotation,
            export_annotations
        ])
        .build(tauri::generate_context!())
        .expect("failed to build ApriReader");
    app.run(|app_handle, event| {
        if matches!(event, tauri::RunEvent::Exit) {
            if let Some(state) = app_handle.try_state::<LibraryState>() {
                if let Ok(database) = state.database.lock() {
                    let _ = database.mark_clean_shutdown();
                }
            }
        }
    });
}

fn collect_launch_book_paths(args: impl IntoIterator<Item = OsString>, cwd: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::<PathBuf>::new();
    for argument in args {
        if paths.len() == MAX_PENDING_LAUNCH_BOOKS {
            break;
        }
        let path = PathBuf::from(argument);
        if !importer::supported_book_path(&path) {
            continue;
        }
        let resolved = if path.is_absolute() {
            path
        } else {
            cwd.join(path)
        };
        if !paths
            .iter()
            .any(|queued| same_launch_path(queued, &resolved))
        {
            paths.push(resolved);
        }
    }
    paths
}

fn same_launch_path(left: &Path, right: &Path) -> bool {
    #[cfg(windows)]
    {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    }
    #[cfg(not(windows))]
    {
        left == right
    }
}

#[cfg(test)]
mod launch_tests {
    use super::*;

    #[test]
    fn accepts_only_supported_launch_paths_and_resolves_relative_arguments() {
        let cwd = Path::new(r"C:\Books");
        let paths = collect_launch_book_paths(
            [
                OsString::from("novel.epub"),
                OsString::from("--flag"),
                OsString::from("malware.exe"),
                OsString::from("NOVEL.EPUB"),
            ],
            cwd,
        );
        assert_eq!(paths, vec![cwd.join("novel.epub")]);
    }

    #[test]
    fn bounds_the_launch_queue() {
        let args = (0..64).map(|index| OsString::from(format!("book-{index}.txt")));
        let paths = collect_launch_book_paths(args, Path::new(r"C:\Books"));
        assert_eq!(paths.len(), MAX_PENDING_LAUNCH_BOOKS);
    }
}
