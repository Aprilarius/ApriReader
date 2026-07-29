mod database;
mod fonts;
mod importer;
mod language_tools;
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
use language_tools::{
    DictionaryResult, InstalledLanguagePackage, LanguagePackageManager, TranslationResult,
};
use metadata::MetadataCandidate;
use reader::DocumentModel;
use special_reader::SpecialDocument;
use statistics::{AchievementProgress, StatisticsSnapshot};
use std::{path::PathBuf, sync::Mutex};
use steam::{SteamIntegrationStatus, SteamSyncResult};
use tauri::{Manager, State};

struct LibraryState {
    database: Mutex<Database>,
}

struct LanguageToolsState {
    manager: Mutex<LanguagePackageManager>,
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

fn with_language_tools<T>(
    state: &State<'_, LanguageToolsState>,
    operation: impl FnOnce(&LanguagePackageManager) -> Result<T, language_tools::LanguageToolsError>,
) -> Result<T, String> {
    let manager = state
        .manager
        .lock()
        .map_err(|_| "the offline language package manager is unavailable".to_owned())?;
    operation(&manager).map_err(|error| error.to_string())
}

#[tauri::command]
fn import_language_package(
    path: String,
    state: State<'_, LanguageToolsState>,
) -> Result<InstalledLanguagePackage, String> {
    with_language_tools(&state, |manager| {
        manager.import(PathBuf::from(path).as_path())
    })
}

#[tauri::command]
fn list_language_packages(
    state: State<'_, LanguageToolsState>,
) -> Result<Vec<InstalledLanguagePackage>, String> {
    with_language_tools(&state, LanguagePackageManager::list)
}

#[tauri::command]
fn lookup_dictionary(
    text: String,
    context: String,
    state: State<'_, LanguageToolsState>,
) -> Result<Vec<DictionaryResult>, String> {
    with_language_tools(&state, |manager| manager.lookup(&text, &context))
}

#[tauri::command]
fn translate_offline(
    package_id: String,
    version: String,
    text: String,
    state: State<'_, LanguageToolsState>,
) -> Result<TranslationResult, String> {
    with_language_tools(&state, |manager| {
        manager.translate(&package_id, &version, &text)
    })
}

#[tauri::command]
fn remove_language_package(
    package_id: String,
    version: String,
    state: State<'_, LanguageToolsState>,
) -> Result<(), String> {
    with_language_tools(&state, |manager| manager.remove(&package_id, &version))
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
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
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
            let language_packages = LanguagePackageManager::new(data_dir.join("language-packages"))
                .map_err(|error| -> Box<dyn std::error::Error> { Box::new(error) })?;
            app.manage(LanguageToolsState {
                manager: Mutex::new(language_packages),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_startup_health,
            import_reader_font,
            list_books,
            remove_books,
            import_books,
            add_watched_folder,
            list_watched_folders,
            scan_watched_folders,
            update_book_metadata,
            set_book_favorite,
            search_metadata,
            apply_metadata_candidate,
            remove_external_cover,
            import_language_package,
            list_language_packages,
            lookup_dictionary,
            translate_offline,
            remove_language_package,
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
