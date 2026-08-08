mod audio_importer;
mod audio_prototype;
mod audio_statistics;
mod azure_tts;
mod cloud_tts;
mod database;
mod fonts;
mod google_tts;
mod importer;
mod metadata;
mod reader;
mod special_reader;
mod statistics;
mod steam;
mod tts;
mod tts_assets;

use audio_prototype::{
    AudioOutputDevice, AudioPlaybackSnapshot, AudioProbeResult, AudioPrototypeCapabilities,
    AudioPrototypeService,
};
use audio_statistics::AudiobookStatisticsSnapshot;
use azure_tts::{
    AzureTtsRegion, AzureTtsSettings, AzureTtsStatus, AzureTtsVoice, PreparedAzureTtsAudio,
};
use cloud_tts::{CloudTtsSettings, CloudTtsStatus, CloudTtsVoice, PreparedCloudTtsAudio};
use database::{
    AnnotationRecord, AudioImportSummary, AudiobookBookmarkRecord, AudiobookChapterRecord,
    AudiobookMetadataInput, AudiobookPartRecord, AudiobookRecord, BookMetadataInput, BookRecord,
    Database, ImportSummary, SearchResult, StartupHealth, WatchedAudioFolder, WatchedFolder,
};
use fonts::ImportedReaderFont;
use google_tts::{GoogleTtsSettings, GoogleTtsStatus, GoogleTtsVoice, PreparedGoogleTtsAudio};
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
use tts::{PreparedTtsAudio, TtsService, TtsVoice};
use tts_assets::{
    TtsAssetService, TtsCacheSummary, TtsExportPart, TtsExportResult, TtsExportStarted,
};

#[cfg(desktop)]
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    WindowEvent,
};

const MAX_PENDING_LAUNCH_FILES: usize = 32;

struct LibraryState {
    database: Mutex<Database>,
}

struct LaunchFilesState {
    paths: Mutex<VecDeque<PathBuf>>,
}

#[derive(serde::Serialize)]
#[serde(tag = "kind", content = "item", rename_all = "camelCase")]
enum OpenedLaunchFile {
    Book(BookRecord),
    Audiobook(AudiobookRecord),
}

struct AudioPrototypeState {
    service: Result<AudioPrototypeService, String>,
}

struct TtsState {
    service: Result<TtsService, String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum AudioCloseBehavior {
    #[default]
    Ask,
    Tray,
    Exit,
}

struct AudioCloseState {
    behavior: Mutex<AudioCloseBehavior>,
}

impl AudioCloseBehavior {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "ask" => Ok(Self::Ask),
            "tray" => Ok(Self::Tray),
            "exit" => Ok(Self::Exit),
            _ => Err("unsupported audio close behavior".to_owned()),
        }
    }
}

fn audio_is_active(state: &AudioPrototypeState) -> bool {
    state
        .service
        .as_ref()
        .ok()
        .and_then(|service| service.snapshot().ok())
        .is_some_and(|snapshot| {
            matches!(snapshot.phase.as_str(), "opening" | "buffering" | "playing")
        })
}

fn pause_audio_for_exit(state: &AudioPrototypeState) {
    if let Ok(service) = &state.service {
        let _ = service.pause();
    }
}

#[tauri::command]
fn set_audio_close_behavior(
    behavior: String,
    state: State<'_, AudioCloseState>,
) -> Result<(), String> {
    let behavior = AudioCloseBehavior::parse(&behavior)?;
    *state
        .behavior
        .lock()
        .map_err(|_| "audio close settings are unavailable".to_owned())? = behavior;
    Ok(())
}

#[tauri::command]
fn resolve_audio_close(
    decision: String,
    remember: bool,
    app: tauri::AppHandle,
    close_state: State<'_, AudioCloseState>,
    audio_state: State<'_, AudioPrototypeState>,
) -> Result<(), String> {
    let decision = AudioCloseBehavior::parse(&decision)?;
    if decision == AudioCloseBehavior::Ask {
        return Err("a close decision must be tray or exit".to_owned());
    }
    if remember {
        *close_state
            .behavior
            .lock()
            .map_err(|_| "audio close settings are unavailable".to_owned())? = decision;
    }
    match decision {
        AudioCloseBehavior::Tray => {
            if let Some(window) = app.get_webview_window("main") {
                window.hide().map_err(|error| error.to_string())?;
            }
        }
        AudioCloseBehavior::Exit => {
            pause_audio_for_exit(&audio_state);
            app.exit(0);
        }
        AudioCloseBehavior::Ask => unreachable!(),
    }
    Ok(())
}

fn with_audio<T>(
    state: &State<'_, AudioPrototypeState>,
    operation: impl FnOnce(&AudioPrototypeService) -> Result<T, String>,
) -> Result<T, String> {
    match &state.service {
        Ok(service) => operation(service),
        Err(error) => Err(error.clone()),
    }
}

#[tauri::command]
fn audio_prototype_capabilities(
    state: State<'_, AudioPrototypeState>,
) -> AudioPrototypeCapabilities {
    match &state.service {
        Ok(_) => audio_prototype::capabilities(true, None),
        Err(error) => audio_prototype::capabilities(false, Some(error.clone())),
    }
}

#[tauri::command]
fn audio_probe_file(
    path: String,
    state: State<'_, AudioPrototypeState>,
) -> Result<AudioProbeResult, String> {
    with_audio(&state, |service| service.probe(PathBuf::from(path)))
}

#[tauri::command]
fn audio_load_file(
    path: String,
    state: State<'_, AudioPrototypeState>,
) -> Result<AudioPlaybackSnapshot, String> {
    with_audio(&state, |service| service.load(PathBuf::from(path)))
}

#[tauri::command]
fn audio_play(state: State<'_, AudioPrototypeState>) -> Result<AudioPlaybackSnapshot, String> {
    with_audio(&state, AudioPrototypeService::play)
}

#[tauri::command]
fn audio_pause(state: State<'_, AudioPrototypeState>) -> Result<AudioPlaybackSnapshot, String> {
    with_audio(&state, AudioPrototypeService::pause)
}

#[tauri::command]
fn audio_seek(
    seconds: f64,
    state: State<'_, AudioPrototypeState>,
) -> Result<AudioPlaybackSnapshot, String> {
    with_audio(&state, |service| service.seek(seconds))
}

#[tauri::command]
fn audio_set_rate(
    rate: f64,
    state: State<'_, AudioPrototypeState>,
) -> Result<AudioPlaybackSnapshot, String> {
    with_audio(&state, |service| service.set_rate(rate))
}

#[tauri::command]
fn audio_set_volume(
    volume: f64,
    state: State<'_, AudioPrototypeState>,
) -> Result<AudioPlaybackSnapshot, String> {
    with_audio(&state, |service| service.set_volume(volume))
}

#[tauri::command]
fn audio_snapshot(state: State<'_, AudioPrototypeState>) -> Result<AudioPlaybackSnapshot, String> {
    with_audio(&state, AudioPrototypeService::snapshot)
}

#[tauri::command]
fn audio_stop(state: State<'_, AudioPrototypeState>) -> Result<AudioPlaybackSnapshot, String> {
    with_audio(&state, AudioPrototypeService::stop)
}

#[tauri::command]
fn audio_list_output_devices(
    state: State<'_, AudioPrototypeState>,
) -> Result<Vec<AudioOutputDevice>, String> {
    with_audio(&state, AudioPrototypeService::list_output_devices)
}

#[tauri::command]
fn audio_set_output_device(
    device_id: String,
    state: State<'_, AudioPrototypeState>,
) -> Result<AudioPlaybackSnapshot, String> {
    with_audio(&state, |service| service.set_output_device(device_id))
}

fn with_tts<T>(
    state: &State<'_, TtsState>,
    operation: impl FnOnce(&TtsService) -> Result<T, String>,
) -> Result<T, String> {
    match &state.service {
        Ok(service) => operation(service),
        Err(error) => Err(error.clone()),
    }
}

#[tauri::command]
fn tts_list_voices(state: State<'_, TtsState>) -> Result<Vec<TtsVoice>, String> {
    with_tts(&state, TtsService::list_voices)
}

#[tauri::command]
fn tts_prepare_section(
    text: String,
    voice_id: String,
    rate: f64,
    app: tauri::AppHandle,
    state: State<'_, TtsState>,
) -> Result<PreparedTtsAudio, String> {
    let cache_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|error| error.to_string())?
        .join("tts");
    with_tts(&state, |service| {
        service.synthesize_to_cache(&text, &voice_id, rate, &cache_dir)
    })
}

#[tauri::command]
fn cloud_tts_status() -> CloudTtsStatus {
    cloud_tts::status()
}

#[tauri::command]
fn cloud_tts_save_key(api_key: String) -> Result<CloudTtsStatus, String> {
    cloud_tts::save_key(&api_key)
}

#[tauri::command]
fn cloud_tts_delete_key() -> Result<CloudTtsStatus, String> {
    cloud_tts::delete_key()
}

#[tauri::command]
fn cloud_tts_list_voices() -> Result<Vec<CloudTtsVoice>, String> {
    cloud_tts::list_voices()
}

#[tauri::command]
fn cloud_tts_prepare_section(
    text: String,
    voice_id: String,
    settings: CloudTtsSettings,
    app: tauri::AppHandle,
) -> Result<PreparedCloudTtsAudio, String> {
    let cache_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|error| error.to_string())?
        .join("tts");
    cloud_tts::prepare(&text, &voice_id, settings, &cache_dir)
}

#[tauri::command]
fn google_tts_status() -> GoogleTtsStatus {
    google_tts::status()
}

#[tauri::command]
fn google_tts_save_key(api_key: String) -> Result<GoogleTtsStatus, String> {
    google_tts::save_key(&api_key)
}

#[tauri::command]
fn google_tts_delete_key() -> Result<GoogleTtsStatus, String> {
    google_tts::delete_key()
}

#[tauri::command]
fn google_tts_list_voices(language_code: Option<String>) -> Result<Vec<GoogleTtsVoice>, String> {
    google_tts::list_voices(language_code.as_deref())
}

#[tauri::command]
fn google_tts_prepare_section(
    text: String,
    voice_id: String,
    language_code: String,
    settings: GoogleTtsSettings,
    app: tauri::AppHandle,
) -> Result<PreparedGoogleTtsAudio, String> {
    let cache_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|error| error.to_string())?
        .join("tts");
    google_tts::prepare(&text, &voice_id, &language_code, settings, &cache_dir)
}

#[tauri::command]
fn azure_tts_status() -> AzureTtsStatus {
    azure_tts::status()
}

#[tauri::command]
fn azure_tts_regions() -> Vec<AzureTtsRegion> {
    azure_tts::regions()
}

#[tauri::command]
fn azure_tts_save_key(api_key: String) -> Result<AzureTtsStatus, String> {
    azure_tts::save_key(&api_key)
}

#[tauri::command]
fn azure_tts_delete_key() -> Result<AzureTtsStatus, String> {
    azure_tts::delete_key()
}

#[tauri::command]
fn azure_tts_list_voices(
    region: String,
    language: Option<String>,
) -> Result<Vec<AzureTtsVoice>, String> {
    azure_tts::list_voices(&region, language.as_deref())
}

#[tauri::command]
fn azure_tts_prepare_section(
    text: String,
    voice_id: String,
    language: String,
    region: String,
    settings: AzureTtsSettings,
    app: tauri::AppHandle,
) -> Result<PreparedAzureTtsAudio, String> {
    let cache_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|error| error.to_string())?
        .join("tts");
    azure_tts::prepare(&text, &voice_id, &language, &region, settings, &cache_dir)
}

fn tts_cache_directory(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_local_data_dir()
        .map_err(|error| error.to_string())?
        .join("tts"))
}

#[tauri::command]
fn tts_cache_summary(
    assets: State<'_, TtsAssetService>,
    app: tauri::AppHandle,
) -> Result<TtsCacheSummary, String> {
    Ok(assets.cache_summary(&tts_cache_directory(&app)?))
}

#[tauri::command]
fn tts_clear_cache(
    provider: Option<String>,
    assets: State<'_, TtsAssetService>,
    app: tauri::AppHandle,
) -> Result<TtsCacheSummary, String> {
    assets.clear_cache(&tts_cache_directory(&app)?, provider.as_deref())
}

#[tauri::command]
fn tts_begin_export(
    playlist_path: String,
    expected_parts: usize,
    assets: State<'_, TtsAssetService>,
) -> Result<TtsExportStarted, String> {
    assets.begin_export(Path::new(&playlist_path), expected_parts)
}

#[tauri::command]
fn tts_append_export_part(
    session_id: String,
    part: TtsExportPart,
    assets: State<'_, TtsAssetService>,
    app: tauri::AppHandle,
) -> Result<usize, String> {
    assets.append_export_part(&session_id, part, &tts_cache_directory(&app)?)
}

#[tauri::command]
fn tts_finish_export(
    session_id: String,
    assets: State<'_, TtsAssetService>,
) -> Result<TtsExportResult, String> {
    assets.finish_export(&session_id)
}

#[tauri::command]
fn tts_cancel_export(session_id: String, assets: State<'_, TtsAssetService>) -> Result<(), String> {
    assets.cancel_export(&session_id)
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
fn list_audiobooks(state: State<'_, LibraryState>) -> Result<Vec<AudiobookRecord>, String> {
    with_database(&state, Database::list_audiobooks)
}

#[tauri::command]
fn list_audiobook_parts(
    audiobook_id: i64,
    state: State<'_, LibraryState>,
) -> Result<Vec<AudiobookPartRecord>, String> {
    with_database(&state, |database| {
        database.list_audiobook_parts(audiobook_id)
    })
}

#[tauri::command]
fn save_audiobook_position(
    audiobook_id: i64,
    part_index: i64,
    position_seconds: f64,
    duration_seconds: f64,
    state: State<'_, LibraryState>,
) -> Result<AudiobookRecord, String> {
    with_database(&state, |database| {
        database.save_audiobook_position(
            audiobook_id,
            part_index,
            position_seconds,
            duration_seconds,
        )
    })
}

#[tauri::command]
fn list_audiobook_bookmarks(
    audiobook_id: i64,
    state: State<'_, LibraryState>,
) -> Result<Vec<AudiobookBookmarkRecord>, String> {
    with_database(&state, |database| {
        database.list_audiobook_bookmarks(audiobook_id)
    })
}

#[tauri::command]
fn create_audiobook_bookmark(
    audiobook_id: i64,
    part_index: i64,
    position_seconds: f64,
    note: String,
    state: State<'_, LibraryState>,
) -> Result<AudiobookBookmarkRecord, String> {
    with_database(&state, |database| {
        database.create_audiobook_bookmark(audiobook_id, part_index, position_seconds, &note)
    })
}

#[tauri::command]
fn delete_audiobook_bookmark(
    bookmark_id: i64,
    state: State<'_, LibraryState>,
) -> Result<(), String> {
    with_database(&state, |database| {
        database.delete_audiobook_bookmark(bookmark_id)
    })
}

#[tauri::command]
fn list_audiobook_chapters(
    audiobook_id: i64,
    state: State<'_, LibraryState>,
) -> Result<Vec<AudiobookChapterRecord>, String> {
    with_database(&state, |database| {
        database.list_audiobook_chapters(audiobook_id)
    })
}

#[tauri::command]
fn update_audiobook_metadata(
    audiobook_id: i64,
    metadata: AudiobookMetadataInput,
    state: State<'_, LibraryState>,
) -> Result<AudiobookRecord, String> {
    with_database(&state, |database| {
        database.update_audiobook_metadata(audiobook_id, &metadata)
    })
}

#[tauri::command]
fn search_audiobook_metadata(
    audiobook_id: i64,
    query: String,
    language: String,
    state: State<'_, LibraryState>,
) -> Result<Vec<MetadataCandidate>, String> {
    with_database(&state, |database| {
        database.search_audiobook_metadata(audiobook_id, &query, &language)
    })
}

#[tauri::command]
fn apply_audiobook_metadata_candidate(
    audiobook_id: i64,
    candidate: MetadataCandidate,
    state: State<'_, LibraryState>,
) -> Result<AudiobookRecord, String> {
    with_database(&state, |database| {
        database.apply_audiobook_metadata_candidate(audiobook_id, &candidate)
    })
}

#[tauri::command]
fn set_audiobook_local_cover(
    audiobook_id: i64,
    path: String,
    state: State<'_, LibraryState>,
) -> Result<AudiobookRecord, String> {
    with_database(&state, |database| {
        database.set_audiobook_local_cover(audiobook_id, Path::new(&path))
    })
}

#[tauri::command]
fn import_audiobooks(
    paths: Vec<String>,
    state: State<'_, LibraryState>,
) -> Result<AudioImportSummary, String> {
    let paths = paths.into_iter().map(PathBuf::from).collect::<Vec<_>>();
    with_database(&state, |database| database.import_audiobooks(&paths))
}

#[tauri::command]
fn add_watched_audio_folder(
    path: String,
    state: State<'_, LibraryState>,
) -> Result<AudioImportSummary, String> {
    with_database(&state, |database| {
        database.add_watched_audio_folder(PathBuf::from(path).as_path())
    })
}

#[tauri::command]
fn list_watched_audio_folders(
    state: State<'_, LibraryState>,
) -> Result<Vec<WatchedAudioFolder>, String> {
    with_database(&state, |database| database.list_watched_audio_folders())
}

#[tauri::command]
fn scan_watched_audio_folders(
    state: State<'_, LibraryState>,
) -> Result<AudioImportSummary, String> {
    with_database(&state, Database::scan_watched_audio_folders)
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
fn take_launch_paths(state: State<'_, LaunchFilesState>) -> Result<Vec<String>, String> {
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
fn open_launch_path(
    path: String,
    state: State<'_, LibraryState>,
) -> Result<OpenedLaunchFile, String> {
    with_database(&state, |database| {
        let path = PathBuf::from(path);
        if importer::supported_book_path(&path) {
            database
                .import_book_for_open(&path)
                .map(OpenedLaunchFile::Book)
        } else if audio_importer::supported_audio_path(&path)
            || audio_importer::supported_descriptor_path(&path)
        {
            database
                .import_audiobook_for_open(&path)
                .map(OpenedLaunchFile::Audiobook)
        } else {
            Err(database::DatabaseError::InvalidAudioLaunch)
        }
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
    language: String,
    state: State<'_, LibraryState>,
) -> Result<Vec<MetadataCandidate>, String> {
    with_database(&state, |database| {
        database.search_metadata(book_id, &query, &language)
    })
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
fn set_local_cover(
    book_id: i64,
    path: String,
    state: State<'_, LibraryState>,
) -> Result<BookRecord, String> {
    with_database(&state, |database| {
        database.set_local_cover(book_id, PathBuf::from(path).as_path())
    })
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
fn start_audiobook_session(
    audiobook_id: i64,
    progress: f64,
    state: State<'_, LibraryState>,
) -> Result<String, String> {
    with_database(&state, |database| {
        database.start_audiobook_session(audiobook_id, progress)
    })
}

#[tauri::command]
fn record_audiobook_activity(
    token: String,
    active: bool,
    progress: f64,
    state: State<'_, LibraryState>,
) -> Result<(), String> {
    with_database(&state, |database| {
        database.record_audiobook_activity(&token, active, progress)
    })
}

#[tauri::command]
fn end_audiobook_session(token: String, state: State<'_, LibraryState>) -> Result<(), String> {
    with_database(&state, |database| database.end_audiobook_session(&token))
}

#[tauri::command]
fn get_audiobook_statistics(
    state: State<'_, LibraryState>,
) -> Result<AudiobookStatisticsSnapshot, String> {
    with_database(&state, |database| database.audiobook_statistics_snapshot())
}

#[tauri::command]
fn get_audiobook_achievements(
    state: State<'_, LibraryState>,
) -> Result<Vec<AchievementProgress>, String> {
    with_database(&state, |database| database.audiobook_achievements())
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

#[cfg(desktop)]
fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
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
    let initial_paths = collect_launch_paths(std::env::args_os().skip(1), &initial_cwd);
    let mut builder = tauri::Builder::default().manage(LaunchFilesState {
        paths: Mutex::new(initial_paths.into()),
    });
    builder = builder.manage(AudioPrototypeState {
        service: AudioPrototypeService::start(),
    });
    builder = builder.manage(TtsState {
        service: TtsService::start(),
    });
    builder = builder.manage(TtsAssetService::default());
    builder = builder.manage(AudioCloseState {
        behavior: Mutex::new(AudioCloseBehavior::Ask),
    });

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            let launch_paths = collect_launch_paths(
                argv.into_iter().skip(1).map(OsString::from),
                Path::new(&cwd),
            );
            if !launch_paths.is_empty() {
                if let Ok(mut pending) = app.state::<LaunchFilesState>().paths.lock() {
                    for path in launch_paths {
                        if pending.len() == MAX_PENDING_LAUNCH_FILES {
                            pending.pop_front();
                        }
                        if !pending.iter().any(|queued| same_launch_path(queued, &path)) {
                            pending.push_back(path);
                        }
                    }
                }
                let _ = app.emit("open-file-paths", ());
            }
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }));
        builder = builder.on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }
            if let WindowEvent::CloseRequested { api, .. } = event {
                let audio_state = window.state::<AudioPrototypeState>();
                if !audio_is_active(&audio_state) {
                    api.prevent_close();
                    pause_audio_for_exit(&audio_state);
                    window.app_handle().exit(0);
                    return;
                }
                let close_state = window.state::<AudioCloseState>();
                let behavior = close_state
                    .behavior
                    .lock()
                    .map(|value| *value)
                    .unwrap_or(AudioCloseBehavior::Ask);
                match behavior {
                    AudioCloseBehavior::Ask => {
                        api.prevent_close();
                        let _ = window.emit("audio-close-requested", ());
                    }
                    AudioCloseBehavior::Tray => {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                    AudioCloseBehavior::Exit => {
                        api.prevent_close();
                        pause_audio_for_exit(&audio_state);
                        window.app_handle().exit(0);
                    }
                }
            }
        });
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

            #[cfg(desktop)]
            {
                let open_item =
                    MenuItem::with_id(app, "tray-open", "Открыть ApriReader", true, None::<&str>)?;
                let playback_item = MenuItem::with_id(
                    app,
                    "tray-play-pause",
                    "Воспроизвести / пауза",
                    true,
                    None::<&str>,
                )?;
                let exit_item =
                    MenuItem::with_id(app, "tray-exit", "Закрыть ApriReader", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&open_item, &playback_item, &exit_item])?;
                let mut tray = TrayIconBuilder::with_id("aprireader-main")
                    .menu(&menu)
                    .tooltip("ApriReader")
                    .show_menu_on_left_click(false)
                    .on_menu_event(|app, event| match event.id().as_ref() {
                        "tray-open" => show_main_window(app),
                        "tray-play-pause" => {
                            let state = app.state::<AudioPrototypeState>();
                            if let Ok(service) = &state.service {
                                if let Ok(snapshot) = service.snapshot() {
                                    if snapshot.phase == "playing" {
                                        let _ = service.pause();
                                    } else if snapshot.path.is_some() {
                                        let _ = service.play();
                                    }
                                }
                            }
                        }
                        "tray-exit" => {
                            pause_audio_for_exit(&app.state::<AudioPrototypeState>());
                            app.exit(0);
                        }
                        _ => {}
                    })
                    .on_tray_icon_event(|tray, event| {
                        if matches!(
                            event,
                            TrayIconEvent::Click {
                                button: MouseButton::Left,
                                button_state: MouseButtonState::Up,
                                ..
                            } | TrayIconEvent::DoubleClick {
                                button: MouseButton::Left,
                                ..
                            }
                        ) {
                            show_main_window(tray.app_handle());
                        }
                    });
                if let Some(icon) = app.default_window_icon() {
                    tray = tray.icon(icon.clone());
                }
                tray.build(app)?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            audio_prototype_capabilities,
            audio_probe_file,
            audio_load_file,
            audio_play,
            audio_pause,
            audio_seek,
            audio_set_rate,
            audio_set_volume,
            audio_snapshot,
            audio_stop,
            audio_list_output_devices,
            audio_set_output_device,
            tts_list_voices,
            tts_prepare_section,
            cloud_tts_status,
            cloud_tts_save_key,
            cloud_tts_delete_key,
            cloud_tts_list_voices,
            cloud_tts_prepare_section,
            google_tts_status,
            google_tts_save_key,
            google_tts_delete_key,
            google_tts_list_voices,
            google_tts_prepare_section,
            azure_tts_status,
            azure_tts_regions,
            azure_tts_save_key,
            azure_tts_delete_key,
            azure_tts_list_voices,
            azure_tts_prepare_section,
            tts_cache_summary,
            tts_clear_cache,
            tts_begin_export,
            tts_append_export_part,
            tts_finish_export,
            tts_cancel_export,
            set_audio_close_behavior,
            resolve_audio_close,
            get_startup_health,
            import_reader_font,
            list_books,
            list_audiobooks,
            list_audiobook_parts,
            save_audiobook_position,
            list_audiobook_bookmarks,
            create_audiobook_bookmark,
            delete_audiobook_bookmark,
            list_audiobook_chapters,
            update_audiobook_metadata,
            search_audiobook_metadata,
            apply_audiobook_metadata_candidate,
            set_audiobook_local_cover,
            import_audiobooks,
            add_watched_audio_folder,
            list_watched_audio_folders,
            scan_watched_audio_folders,
            remove_books,
            import_books,
            take_launch_paths,
            open_launch_path,
            add_watched_folder,
            list_watched_folders,
            scan_watched_folders,
            update_book_metadata,
            set_book_favorite,
            search_metadata,
            apply_metadata_candidate,
            remove_external_cover,
            set_local_cover,
            load_document,
            load_special_document,
            save_reading_position,
            start_reading_session,
            record_reading_activity,
            end_reading_session,
            start_audiobook_session,
            record_audiobook_activity,
            end_audiobook_session,
            get_statistics,
            get_audiobook_statistics,
            get_achievements,
            get_audiobook_achievements,
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

fn collect_launch_paths(args: impl IntoIterator<Item = OsString>, cwd: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::<PathBuf>::new();
    for argument in args {
        if paths.len() == MAX_PENDING_LAUNCH_FILES {
            break;
        }
        let path = PathBuf::from(argument);
        if !supported_launch_path(&path) {
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

fn supported_launch_path(path: &Path) -> bool {
    importer::supported_book_path(path)
        || audio_importer::supported_audio_path(path)
        || audio_importer::supported_descriptor_path(path)
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
        let paths = collect_launch_paths(
            [
                OsString::from("novel.epub"),
                OsString::from("--flag"),
                OsString::from("malware.exe"),
                OsString::from("NOVEL.EPUB"),
                OsString::from(r"C:\Audio\story.M4B"),
                OsString::from("chapters.cue"),
                OsString::from("locked.aax"),
            ],
            cwd,
        );
        assert_eq!(
            paths,
            vec![
                cwd.join("novel.epub"),
                PathBuf::from(r"C:\Audio\story.M4B"),
                cwd.join("chapters.cue"),
            ]
        );
    }

    #[test]
    fn bounds_the_launch_queue() {
        let args = (0..64).map(|index| OsString::from(format!("book-{index}.txt")));
        let paths = collect_launch_paths(args, Path::new(r"C:\Books"));
        assert_eq!(paths.len(), MAX_PENDING_LAUNCH_FILES);
    }

    #[test]
    fn accepts_only_explicit_audio_close_behaviors() {
        assert_eq!(
            AudioCloseBehavior::parse("ask").unwrap(),
            AudioCloseBehavior::Ask
        );
        assert_eq!(
            AudioCloseBehavior::parse("tray").unwrap(),
            AudioCloseBehavior::Tray
        );
        assert_eq!(
            AudioCloseBehavior::parse("exit").unwrap(),
            AudioCloseBehavior::Exit
        );
        assert!(AudioCloseBehavior::parse("minimize").is_err());
    }
}
