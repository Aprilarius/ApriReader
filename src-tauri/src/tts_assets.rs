use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_EXPORT_PARTS: usize = 5_000;
const MAX_EXPORT_PART_BYTES: u64 = 64 * 1024 * 1024;
const MAX_EXPORT_TOTAL_BYTES: u64 = 6 * 1024 * 1024 * 1024;

static EXPORT_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsCacheProviderSummary {
    pub provider: String,
    pub files: usize,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsCacheSummary {
    pub total_files: usize,
    pub total_bytes: u64,
    pub providers: Vec<TtsCacheProviderSummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsExportStarted {
    pub session_id: String,
    pub expected_parts: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsExportResult {
    pub playlist_path: String,
    pub media_directory: String,
    pub parts: usize,
    pub bytes: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsExportPart {
    pub source_path: String,
    pub title: String,
}

struct ExportSession {
    playlist_path: PathBuf,
    partial_directory: PathBuf,
    final_directory: PathBuf,
    expected_parts: usize,
    bytes: u64,
    entries: Vec<(String, String)>,
}

#[derive(Default)]
pub struct TtsAssetService {
    sessions: Mutex<HashMap<String, ExportSession>>,
}

impl TtsAssetService {
    pub fn cache_summary(&self, cache_dir: &Path) -> TtsCacheSummary {
        cache_summary(cache_dir)
    }

    pub fn clear_cache(
        &self,
        cache_dir: &Path,
        provider: Option<&str>,
    ) -> Result<TtsCacheSummary, String> {
        if let Some(value) = provider {
            validate_provider(value)?;
        }
        let Ok(entries) = fs::read_dir(cache_dir) else {
            return Ok(cache_summary(cache_dir));
        };
        for entry in entries.filter_map(Result::ok) {
            if !entry.file_type().is_ok_and(|value| value.is_file()) {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            let Some((entry_provider, _)) = classify_cache_name(&name) else {
                continue;
            };
            if provider.is_none_or(|value| value == entry_provider) {
                fs::remove_file(entry.path()).map_err(|error| {
                    format!("failed to remove a text-to-speech cache file: {error}")
                })?;
            }
        }
        Ok(cache_summary(cache_dir))
    }

    pub fn begin_export(
        &self,
        playlist_path: &Path,
        expected_parts: usize,
    ) -> Result<TtsExportStarted, String> {
        if !(1..=MAX_EXPORT_PARTS).contains(&expected_parts) {
            return Err(format!(
                "speech export accepts between 1 and {MAX_EXPORT_PARTS} parts"
            ));
        }
        if playlist_path
            .extension()
            .and_then(|value| value.to_str())
            .is_none_or(|value| !value.eq_ignore_ascii_case("m3u8"))
        {
            return Err("speech export destination must be an M3U8 playlist".to_owned());
        }
        let parent = playlist_path
            .parent()
            .filter(|value| value.is_dir())
            .ok_or_else(|| "speech export destination directory is unavailable".to_owned())?;
        let stem = playlist_path
            .file_stem()
            .and_then(|value| value.to_str())
            .map(safe_file_stem)
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "ApriReader-TTS".to_owned());
        let final_directory = unique_directory(parent, &format!("{stem}-media"));
        let partial_directory = final_directory.with_extension("partial");
        fs::create_dir(&partial_directory)
            .map_err(|error| format!("failed to create the speech export directory: {error}"))?;
        let session_id = format!(
            "{:x}-{:x}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            EXPORT_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        );
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "speech export state is unavailable".to_owned())?;
        if sessions.len() >= 2 {
            let _ = fs::remove_dir(&partial_directory);
            return Err("too many speech exports are already active".to_owned());
        }
        sessions.insert(
            session_id.clone(),
            ExportSession {
                playlist_path: playlist_path.to_owned(),
                partial_directory,
                final_directory,
                expected_parts,
                bytes: 0,
                entries: Vec::with_capacity(expected_parts),
            },
        );
        Ok(TtsExportStarted {
            session_id,
            expected_parts,
        })
    }

    pub fn append_export_part(
        &self,
        session_id: &str,
        part: TtsExportPart,
        cache_dir: &Path,
    ) -> Result<usize, String> {
        let source = validate_cached_source(cache_dir, Path::new(&part.source_path))?;
        let metadata = fs::metadata(&source).map_err(|error| error.to_string())?;
        if metadata.len() == 0 || metadata.len() > MAX_EXPORT_PART_BYTES {
            return Err("a generated speech part is empty or too large to export".to_owned());
        }
        let extension = source
            .extension()
            .and_then(|value| value.to_str())
            .ok_or_else(|| "generated speech has no supported extension".to_owned())?;
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "speech export state is unavailable".to_owned())?;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| "speech export session is unavailable".to_owned())?;
        if session.entries.len() >= session.expected_parts {
            return Err("speech export received more parts than expected".to_owned());
        }
        let next_bytes = session
            .bytes
            .checked_add(metadata.len())
            .filter(|value| *value <= MAX_EXPORT_TOTAL_BYTES)
            .ok_or_else(|| "speech export exceeds the 6 GiB limit".to_owned())?;
        let index = session.entries.len() + 1;
        let file_name = format!("{index:05}.{extension}");
        let destination = session.partial_directory.join(&file_name);
        fs::copy(&source, &destination)
            .map_err(|error| format!("failed to copy a generated speech part: {error}"))?;
        session.bytes = next_bytes;
        session
            .entries
            .push((file_name, safe_playlist_title(&part.title, index)));
        Ok(index)
    }

    pub fn finish_export(&self, session_id: &str) -> Result<TtsExportResult, String> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "speech export state is unavailable".to_owned())?;
        let session = sessions
            .remove(session_id)
            .ok_or_else(|| "speech export session is unavailable".to_owned())?;
        if session.entries.len() != session.expected_parts {
            sessions.insert(session_id.to_owned(), session);
            return Err("speech export is incomplete".to_owned());
        }
        let directory_name = session
            .final_directory
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| "speech export directory name is invalid".to_owned())?;
        let mut playlist = String::from("#EXTM3U\n");
        for (file_name, title) in &session.entries {
            playlist.push_str(&format!(
                "#EXTINF:-1,{title}\n{directory_name}/{file_name}\n"
            ));
        }
        let temporary = unique_sidecar_path(&session.playlist_path, "new")?;
        if let Err(error) = write_new_file(&temporary, playlist.as_bytes()) {
            sessions.insert(session_id.to_owned(), session);
            return Err(error);
        }
        if let Err(error) = fs::rename(&session.partial_directory, &session.final_directory) {
            let _ = fs::remove_file(&temporary);
            sessions.insert(session_id.to_owned(), session);
            return Err(format!("failed to finalize speech export media: {error}"));
        }
        if let Err(error) = replace_file_safely(&temporary, &session.playlist_path) {
            let media_rollback = fs::rename(&session.final_directory, &session.partial_directory);
            let _ = fs::remove_file(&temporary);
            if media_rollback.is_ok() {
                sessions.insert(session_id.to_owned(), session);
            }
            return Err(error);
        }
        Ok(TtsExportResult {
            playlist_path: session.playlist_path.to_string_lossy().into_owned(),
            media_directory: session.final_directory.to_string_lossy().into_owned(),
            parts: session.entries.len(),
            bytes: session.bytes,
        })
    }

    pub fn cancel_export(&self, session_id: &str) -> Result<(), String> {
        let session = self
            .sessions
            .lock()
            .map_err(|_| "speech export state is unavailable".to_owned())?
            .remove(session_id);
        if let Some(session) = session {
            fs::remove_dir_all(&session.partial_directory)
                .map_err(|error| format!("failed to remove partial speech export: {error}"))?;
        }
        Ok(())
    }
}

fn write_new_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("failed to create speech export playlist: {error}"))?;
    if let Err(error) = file.write_all(bytes).and_then(|_| file.sync_all()) {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(format!("failed to write speech export playlist: {error}"));
    }
    Ok(())
}

pub(crate) fn persist_cache_file(destination: &Path, bytes: &[u8]) -> Result<(), String> {
    if destination.is_file() {
        return Ok(());
    }
    let temporary = unique_sidecar_path(destination, "cache")?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|error| format!("failed to create a speech cache file: {error}"))?;
    if let Err(error) = file.write_all(bytes).and_then(|_| file.sync_all()) {
        drop(file);
        let _ = fs::remove_file(&temporary);
        return Err(format!("failed to write a speech cache file: {error}"));
    }
    drop(file);
    match fs::rename(&temporary, destination) {
        Ok(()) => Ok(()),
        Err(_) if destination.is_file() => {
            let _ = fs::remove_file(&temporary);
            Ok(())
        }
        Err(error) => {
            let _ = fs::remove_file(&temporary);
            Err(format!("failed to finalize a speech cache file: {error}"))
        }
    }
}

fn replace_file_safely(temporary: &Path, destination: &Path) -> Result<(), String> {
    if !destination.exists() {
        return fs::rename(temporary, destination)
            .map_err(|error| format!("failed to finalize speech export playlist: {error}"));
    }
    if !destination.is_file() {
        return Err("speech export destination is not a replaceable file".to_owned());
    }
    let backup = unique_sidecar_path(destination, "backup")?;
    fs::rename(destination, &backup)
        .map_err(|error| format!("failed to stage the previous playlist: {error}"))?;
    if let Err(error) = fs::rename(temporary, destination) {
        let rollback = fs::rename(&backup, destination);
        return Err(if rollback.is_ok() {
            format!("failed to finalize speech export playlist: {error}")
        } else {
            format!("failed to finalize speech export playlist and restore its backup: {error}")
        });
    }
    let _ = fs::remove_file(backup);
    Ok(())
}

fn unique_sidecar_path(path: &Path, label: &str) -> Result<PathBuf, String> {
    let parent = path
        .parent()
        .ok_or_else(|| "speech export destination has no parent directory".to_owned())?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "speech export destination name is invalid".to_owned())?;
    let sequence = EXPORT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    for suffix in 0..10_000usize {
        let candidate = parent.join(format!(
            ".{file_name}.aprireader-{label}-{sequence:x}-{suffix:x}"
        ));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err("unable to allocate a unique speech export sidecar file".to_owned())
}

fn cache_summary(cache_dir: &Path) -> TtsCacheSummary {
    let mut values = [
        ("local", 0usize, 0u64),
        ("elevenlabs", 0, 0),
        ("google", 0, 0),
        ("azure", 0, 0),
    ];
    if let Ok(entries) = fs::read_dir(cache_dir) {
        for entry in entries.filter_map(Result::ok) {
            if !entry.file_type().is_ok_and(|value| value.is_file()) {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            let Some((provider, _)) = classify_cache_name(&name) else {
                continue;
            };
            let bytes = entry.metadata().map(|value| value.len()).unwrap_or(0);
            if let Some(value) = values.iter_mut().find(|value| value.0 == provider) {
                value.1 += 1;
                value.2 = value.2.saturating_add(bytes);
            }
        }
    }
    let providers = values
        .into_iter()
        .map(|(provider, files, bytes)| TtsCacheProviderSummary {
            provider: provider.to_owned(),
            files,
            bytes,
        })
        .collect::<Vec<_>>();
    TtsCacheSummary {
        total_files: providers.iter().map(|value| value.files).sum(),
        total_bytes: providers.iter().map(|value| value.bytes).sum(),
        providers,
    }
}

fn validate_provider(provider: &str) -> Result<(), String> {
    matches!(provider, "local" | "elevenlabs" | "google" | "azure")
        .then_some(())
        .ok_or_else(|| "unknown text-to-speech cache provider".to_owned())
}

fn classify_cache_name(name: &str) -> Option<(&'static str, &'static str)> {
    let candidates = [
        ("cloud-tts-", ".mp3", "elevenlabs"),
        ("google-tts-", ".mp3", "google"),
        ("azure-tts-", ".mp3", "azure"),
        ("tts-", ".wav", "local"),
    ];
    for (prefix, extension, provider) in candidates {
        let Some(digest) = name
            .strip_prefix(prefix)
            .and_then(|value| value.strip_suffix(extension))
        else {
            continue;
        };
        if digest.len() == 64 && digest.bytes().all(|value| value.is_ascii_hexdigit()) {
            return Some((provider, extension));
        }
    }
    None
}

fn validate_cached_source(cache_dir: &Path, source: &Path) -> Result<PathBuf, String> {
    let cache = fs::canonicalize(cache_dir)
        .map_err(|_| "text-to-speech cache directory is unavailable".to_owned())?;
    let source = fs::canonicalize(source)
        .map_err(|_| "generated speech cache file is unavailable".to_owned())?;
    if source.parent() != Some(cache.as_path())
        || !source.is_file()
        || source
            .file_name()
            .and_then(|value| value.to_str())
            .and_then(classify_cache_name)
            .is_none()
    {
        return Err("speech export accepts only validated ApriReader cache files".to_owned());
    }
    Ok(source)
}

fn unique_directory(parent: &Path, stem: &str) -> PathBuf {
    for suffix in 0..10_000usize {
        let name = if suffix == 0 {
            stem.to_owned()
        } else {
            format!("{stem}-{}", suffix + 1)
        };
        let candidate = parent.join(name);
        if !candidate.exists() && !candidate.with_extension("partial").exists() {
            return candidate;
        }
    }
    parent.join(format!(
        "{stem}-{}",
        EXPORT_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ))
}

fn safe_file_stem(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_control() && !r#"<>:"/\|?*"#.contains(*character))
        .take(80)
        .collect::<String>()
        .trim()
        .to_owned()
}

fn safe_playlist_title(value: &str, index: usize) -> String {
    let value = value
        .chars()
        .filter(|character| !character.is_control() && *character != ',')
        .take(160)
        .collect::<String>();
    if value.trim().is_empty() {
        format!("Part {index}")
    } else {
        value.trim().to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_only_exact_bounded_cache_names() {
        assert_eq!(
            classify_cache_name(&format!("tts-{}.wav", "a".repeat(64)))
                .unwrap()
                .0,
            "local"
        );
        assert_eq!(
            classify_cache_name(&format!("azure-tts-{}.mp3", "0".repeat(64)))
                .unwrap()
                .0,
            "azure"
        );
        assert!(classify_cache_name("tts-../../book.wav").is_none());
        assert!(classify_cache_name(&format!("tts-{}.exe", "a".repeat(64))).is_none());
    }

    #[test]
    fn sanitizes_export_names_and_playlist_labels() {
        assert_eq!(safe_file_stem("A: book?"), "A book");
        assert_eq!(safe_playlist_title("Chapter, one\n", 1), "Chapter one");
        assert_eq!(safe_playlist_title("\n", 4), "Part 4");
    }

    #[test]
    fn concurrent_cache_writers_do_not_collide_or_leave_sidecars() {
        let root = tempfile::tempdir().unwrap();
        let destination = root.path().join("tts-cache.wav");
        let destination_one = destination.clone();
        let destination_two = destination.clone();
        let first = std::thread::spawn(move || persist_cache_file(&destination_one, b"RIFF-one"));
        let second = std::thread::spawn(move || persist_cache_file(&destination_two, b"RIFF-two"));
        first.join().unwrap().unwrap();
        second.join().unwrap().unwrap();
        let content = fs::read(&destination).unwrap();
        assert!(content == b"RIFF-one" || content == b"RIFF-two");
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    }

    #[test]
    fn exports_only_a_validated_cache_file_and_playlist() {
        let root = std::env::temp_dir().join(format!(
            "aprireader-tts-export-test-{}",
            EXPORT_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let cache = root.join("cache");
        let output = root.join("output");
        fs::create_dir_all(&cache).unwrap();
        fs::create_dir_all(&output).unwrap();
        let source = cache.join(format!("tts-{}.wav", "a".repeat(64)));
        fs::write(&source, b"RIFF-test").unwrap();
        let playlist = output.join("Book.m3u8");
        let unrelated_temporary = output.join("Book.m3u8.tmp");
        fs::write(&playlist, "old playlist").unwrap();
        fs::write(&unrelated_temporary, "user file").unwrap();
        let service = TtsAssetService::default();
        let started = service.begin_export(&playlist, 1).unwrap();
        assert_eq!(
            service
                .append_export_part(
                    &started.session_id,
                    TtsExportPart {
                        source_path: source.to_string_lossy().into_owned(),
                        title: "Chapter, one".to_owned(),
                    },
                    &cache,
                )
                .unwrap(),
            1
        );
        let result = service.finish_export(&started.session_id).unwrap();
        assert_eq!(result.parts, 1);
        assert!(Path::new(&result.media_directory)
            .join("00001.wav")
            .is_file());
        let content = fs::read_to_string(&playlist).unwrap();
        assert!(content.starts_with("#EXTM3U\n"));
        assert!(content.contains("#EXTINF:-1,Chapter one"));
        assert!(!content.contains(source.to_string_lossy().as_ref()));
        assert_eq!(
            fs::read_to_string(unrelated_temporary).unwrap(),
            "user file"
        );
        fs::remove_dir_all(root).unwrap();
    }
}
