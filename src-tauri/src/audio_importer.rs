use sha2::{Digest, Sha256};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};
use thiserror::Error;

pub const MAX_AUDIO_FILE_BYTES: u64 = 20 * 1024 * 1024 * 1024;
pub const MAX_AUDIOBOOK_BYTES: u64 = 100 * 1024 * 1024 * 1024;
pub const MAX_AUDIOBOOK_PARTS: usize = 1_000;
pub const MAX_WATCHED_AUDIO_FILES: usize = 100_000;
pub const MAX_DESCRIPTOR_BYTES: u64 = 2 * 1024 * 1024;
pub const MAX_DESCRIPTOR_ENTRIES: usize = 10_000;

pub const NATIVE_EXTENSIONS: &[&str] = &[
    "aac", "flac", "m4a", "m4b", "mp3", "wav", "wma", "3g2", "3gp", "amr",
];
pub const SYSTEM_CODEC_EXTENSIONS: &[&str] = &[
    "aif", "aiff", "alac", "ape", "caf", "mka", "mpc", "oga", "ogg", "opus", "wv",
];
pub const PLAYLIST_EXTENSIONS: &[&str] = &["cue", "m3u", "m3u8"];
pub const BLOCKED_DRM_EXTENSIONS: &[&str] = &["aax", "aaxc", "m4p"];

#[derive(Debug, Error)]
pub enum AudioImportError {
    #[error("the audio file does not exist or is not a regular file")]
    Missing,
    #[error("unsupported or unsafe audio format")]
    Unsupported,
    #[error("the selected format is DRM-protected and is intentionally not supported")]
    DrmProtected,
    #[error("a playlist or chapter description is not a playable audio part")]
    Descriptor,
    #[error("the playlist or chapter description is invalid or unsafe")]
    InvalidDescriptor,
    #[error("the playlist or chapter description exceeds the 2 MiB safety limit")]
    DescriptorTooLarge,
    #[error("the audio file exceeds the 20 GiB safety limit")]
    FileTooLarge,
    #[error("the audiobook exceeds the 100 GiB safety limit")]
    BookTooLarge,
    #[error("the audiobook contains more than 1,000 parts")]
    TooManyParts,
    #[error("the watched audio folder contains more than 100,000 supported files")]
    TooManyWatchedFiles,
    #[error("file operation failed: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct ValidatedAudioPath {
    pub canonical: PathBuf,
    pub extension: String,
    pub file_size: u64,
    pub support_tier: &'static str,
}

#[derive(Debug, Clone)]
pub struct ImportedAudioPart {
    pub source_path: String,
    pub fingerprint: String,
    pub title: String,
    pub format: String,
    pub file_size: i64,
}

#[derive(Debug, Clone)]
pub struct ImportedAudioGroup {
    pub group_key: String,
    pub title: String,
    pub parts: Vec<ImportedAudioPart>,
}

#[derive(Debug, Clone)]
pub struct AudioGroupCandidate {
    pub paths: Vec<PathBuf>,
    pub group_as_folder: bool,
    pub preserve_order: bool,
    pub group_key_override: Option<String>,
    pub title_override: Option<String>,
    pub chapters: Vec<AudioChapterCandidate>,
}

#[derive(Debug, Clone)]
pub struct AudioChapterCandidate {
    pub source_path: PathBuf,
    pub title: String,
    pub start_seconds: f64,
    pub ordinal: usize,
}

#[derive(Debug, Default)]
pub struct AudioGroupDiscovery {
    pub groups: Vec<AudioGroupCandidate>,
    pub errors: Vec<String>,
}

pub fn supported_audio_path(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .is_some_and(|extension| {
            NATIVE_EXTENSIONS.contains(&extension.as_str())
                || SYSTEM_CODEC_EXTENSIONS.contains(&extension.as_str())
        })
}

pub fn supported_descriptor_path(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .is_some_and(|extension| PLAYLIST_EXTENSIONS.contains(&extension.as_str()))
}

pub fn parse_audio_descriptor(path: &Path) -> Result<AudioGroupCandidate, AudioImportError> {
    if !path.is_file() {
        return Err(AudioImportError::Missing);
    }
    let canonical = path.canonicalize()?;
    if canonical.metadata()?.len() > MAX_DESCRIPTOR_BYTES {
        return Err(AudioImportError::DescriptorTooLarge);
    }
    let bytes = fs::read(&canonical)?;
    if bytes.len() as u64 > MAX_DESCRIPTOR_BYTES {
        return Err(AudioImportError::DescriptorTooLarge);
    }
    let text = String::from_utf8_lossy(&bytes);
    let extension = canonical
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or(AudioImportError::InvalidDescriptor)?;
    match extension.as_str() {
        "m3u" | "m3u8" => parse_m3u(&canonical, &text),
        "cue" => parse_cue(&canonical, &text),
        _ => Err(AudioImportError::InvalidDescriptor),
    }
}

fn parse_m3u(path: &Path, text: &str) -> Result<AudioGroupCandidate, AudioImportError> {
    let mut paths: Vec<PathBuf> = Vec::new();
    for line in text.lines() {
        let line = line.trim().trim_start_matches('\u{feff}');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        paths.push(resolve_descriptor_audio(path, line)?);
        if paths.len() > MAX_DESCRIPTOR_ENTRIES {
            return Err(AudioImportError::TooManyParts);
        }
    }
    descriptor_candidate(path, paths, Vec::new(), None)
}

fn parse_cue(path: &Path, text: &str) -> Result<AudioGroupCandidate, AudioImportError> {
    let mut paths: Vec<PathBuf> = Vec::new();
    let mut chapters = Vec::new();
    let mut current_file: Option<PathBuf> = None;
    let mut current_title: Option<String> = None;
    let mut album_title: Option<String> = None;
    let mut in_track = false;
    for line in text.lines() {
        let line = line.trim().trim_start_matches('\u{feff}');
        let upper = line.to_ascii_uppercase();
        if upper.starts_with("FILE ") {
            let value =
                quoted_or_first_value(&line[5..]).ok_or(AudioImportError::InvalidDescriptor)?;
            let resolved = resolve_descriptor_audio(path, value)?;
            if !paths.iter().any(|item| same_path(item, &resolved)) {
                paths.push(resolved.clone());
            }
            current_file = Some(resolved);
        } else if upper.starts_with("TRACK ") {
            in_track = true;
            current_title = None;
        } else if upper.starts_with("TITLE ") {
            let value =
                quoted_or_first_value(&line[6..]).ok_or(AudioImportError::InvalidDescriptor)?;
            if in_track {
                current_title = Some(bounded_title(Some(value)));
            } else {
                album_title = Some(bounded_title(Some(value)));
            }
        } else if upper.starts_with("INDEX 01 ") {
            let source_path = current_file
                .clone()
                .ok_or(AudioImportError::InvalidDescriptor)?;
            let start_seconds = parse_cue_time(line[9..].trim())?;
            let ordinal = chapters.len();
            chapters.push(AudioChapterCandidate {
                source_path,
                title: current_title
                    .clone()
                    .unwrap_or_else(|| format!("Track {}", ordinal + 1)),
                start_seconds,
                ordinal,
            });
            if chapters.len() > MAX_DESCRIPTOR_ENTRIES {
                return Err(AudioImportError::TooManyParts);
            }
        }
    }
    descriptor_candidate(path, paths, chapters, album_title)
}

fn descriptor_candidate(
    descriptor: &Path,
    mut paths: Vec<PathBuf>,
    chapters: Vec<AudioChapterCandidate>,
    title_override: Option<String>,
) -> Result<AudioGroupCandidate, AudioImportError> {
    let mut seen = BTreeSet::new();
    paths.retain(|path| seen.insert(normalized_path_key(path)));
    if paths.is_empty() || paths.len() > MAX_AUDIOBOOK_PARTS {
        return Err(AudioImportError::InvalidDescriptor);
    }
    Ok(AudioGroupCandidate {
        paths,
        group_as_folder: true,
        preserve_order: true,
        group_key_override: Some(format!("descriptor:{}", normalized_path_key(descriptor))),
        title_override: title_override.or_else(|| {
            descriptor
                .file_stem()
                .and_then(|value| value.to_str())
                .map(|value| bounded_title(Some(value)))
        }),
        chapters,
    })
}

fn resolve_descriptor_audio(descriptor: &Path, value: &str) -> Result<PathBuf, AudioImportError> {
    let value = value.trim().trim_matches('"');
    if value.is_empty() || value.contains("://") {
        return Err(AudioImportError::InvalidDescriptor);
    }
    let relative = Path::new(value);
    if relative.is_absolute() {
        return Err(AudioImportError::InvalidDescriptor);
    }
    let base = descriptor
        .parent()
        .ok_or(AudioImportError::InvalidDescriptor)?
        .canonicalize()?;
    let canonical = base.join(relative).canonicalize()?;
    if !canonical.starts_with(&base) || !supported_audio_path(&canonical) {
        return Err(AudioImportError::InvalidDescriptor);
    }
    Ok(canonical)
}

fn quoted_or_first_value(value: &str) -> Option<&str> {
    let value = value.trim();
    if let Some(rest) = value.strip_prefix('"') {
        rest.find('"').map(|end| &rest[..end])
    } else {
        value.split_whitespace().next()
    }
}

fn parse_cue_time(value: &str) -> Result<f64, AudioImportError> {
    let mut parts = value.split(':');
    let minutes = parts
        .next()
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or(AudioImportError::InvalidDescriptor)?;
    let seconds = parts
        .next()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value < 60)
        .ok_or(AudioImportError::InvalidDescriptor)?;
    let frames = parts
        .next()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value < 75)
        .ok_or(AudioImportError::InvalidDescriptor)?;
    if parts.next().is_some() {
        return Err(AudioImportError::InvalidDescriptor);
    }
    Ok(minutes as f64 * 60.0 + seconds as f64 + frames as f64 / 75.0)
}

pub fn validate_audio_path(path: &Path) -> Result<ValidatedAudioPath, AudioImportError> {
    if !path.is_file() {
        return Err(AudioImportError::Missing);
    }
    let canonical = path.canonicalize()?;
    let extension = canonical
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or(AudioImportError::Unsupported)?;
    if BLOCKED_DRM_EXTENSIONS.contains(&extension.as_str()) {
        return Err(AudioImportError::DrmProtected);
    }
    if PLAYLIST_EXTENSIONS.contains(&extension.as_str()) {
        return Err(AudioImportError::Descriptor);
    }
    let support_tier = if NATIVE_EXTENSIONS.contains(&extension.as_str()) {
        "windows-native"
    } else if SYSTEM_CODEC_EXTENSIONS.contains(&extension.as_str()) {
        "installed-system-codec"
    } else {
        return Err(AudioImportError::Unsupported);
    };
    let file_size = canonical.metadata()?.len();
    if file_size > MAX_AUDIO_FILE_BYTES {
        return Err(AudioImportError::FileTooLarge);
    }
    Ok(ValidatedAudioPath {
        canonical,
        extension,
        file_size,
        support_tier,
    })
}

pub fn inspect_audio_group(
    paths: &[PathBuf],
    group_as_folder: bool,
    preserve_order: bool,
    title_override: Option<&str>,
) -> Result<ImportedAudioGroup, AudioImportError> {
    if paths.is_empty() {
        return Err(AudioImportError::Missing);
    }
    if paths.len() > MAX_AUDIOBOOK_PARTS {
        return Err(AudioImportError::TooManyParts);
    }
    let mut validated = paths
        .iter()
        .map(|path| validate_audio_path(path))
        .collect::<Result<Vec<_>, _>>()?;
    if !preserve_order {
        validated.sort_by(|left, right| natural_path_cmp(&left.canonical, &right.canonical));
    }
    validated.dedup_by(|left, right| same_path(&left.canonical, &right.canonical));
    validate_group_limits(validated.len(), validated.iter().map(|part| part.file_size))?;

    let folder_group = group_as_folder || validated.len() > 1;
    let title = title_override
        .map(|value| bounded_title(Some(value)))
        .unwrap_or_else(|| group_title(&validated, folder_group));
    let mut parts = Vec::with_capacity(validated.len());
    let mut actual_total_size = 0_u64;
    for part in &validated {
        let (fingerprint, actual_size) = sha256(&part.canonical, MAX_AUDIO_FILE_BYTES)?;
        actual_total_size = actual_total_size
            .checked_add(actual_size)
            .filter(|value| *value <= MAX_AUDIOBOOK_BYTES)
            .ok_or(AudioImportError::BookTooLarge)?;
        parts.push(ImportedAudioPart {
            source_path: part.canonical.to_string_lossy().into_owned(),
            fingerprint,
            title: bounded_title(part.canonical.file_stem().and_then(|value| value.to_str())),
            format: part.extension.to_ascii_uppercase(),
            file_size: i64::try_from(actual_size).unwrap_or(i64::MAX),
        });
    }
    let group_key = if folder_group {
        let parent = validated[0]
            .canonical
            .parent()
            .unwrap_or_else(|| Path::new("."));
        format!("folder:{}", normalized_path_key(parent))
    } else {
        format!("single:{}", parts[0].fingerprint)
    };
    Ok(ImportedAudioGroup {
        group_key,
        title,
        parts,
    })
}

pub fn discover_manual_audio_groups(paths: &[PathBuf]) -> AudioGroupDiscovery {
    let mut discovery = AudioGroupDiscovery::default();
    let mut explicit_files = BTreeMap::<String, Vec<PathBuf>>::new();
    let mut seen = BTreeSet::new();
    for path in paths.iter().take(10_000) {
        if path.is_dir() {
            match discover_directory_groups(path, true, &mut discovery.groups) {
                Ok(()) => {}
                Err(error) => discovery
                    .errors
                    .push(format!("{}: {error}", path.to_string_lossy())),
            }
        } else if path.is_file() {
            match path.canonicalize() {
                Ok(canonical) if supported_audio_path(&canonical) => {
                    let key = canonical
                        .parent()
                        .map(normalized_path_key)
                        .unwrap_or_default();
                    if seen.insert(normalized_path_key(&canonical)) {
                        explicit_files.entry(key).or_default().push(canonical);
                    }
                }
                Ok(canonical) if supported_descriptor_path(&canonical) => {
                    match parse_audio_descriptor(&canonical) {
                        Ok(candidate) => discovery.groups.push(candidate),
                        Err(error) => discovery
                            .errors
                            .push(format!("{}: {error}", path.to_string_lossy())),
                    }
                }
                Ok(_) => discovery.errors.push(format!(
                    "{}: unsupported or unsafe audio format",
                    path.to_string_lossy()
                )),
                Err(error) => discovery
                    .errors
                    .push(format!("{}: {error}", path.to_string_lossy())),
            }
        } else {
            discovery.errors.push(format!(
                "{}: audio source is unavailable",
                path.to_string_lossy()
            ));
        }
    }
    discovery.groups.extend(
        explicit_files
            .into_values()
            .map(|paths| AudioGroupCandidate {
                group_as_folder: paths.len() > 1,
                paths,
                preserve_order: false,
                group_key_override: None,
                title_override: None,
                chapters: Vec::new(),
            }),
    );
    deduplicate_groups(&mut discovery.groups);
    discovery
}

pub fn discover_watched_audio_groups(
    root: &Path,
) -> Result<Vec<AudioGroupCandidate>, AudioImportError> {
    let canonical = root.canonicalize()?;
    let mut groups = Vec::new();
    discover_directory_groups(&canonical, false, &mut groups)?;
    deduplicate_groups(&mut groups);
    Ok(groups)
}

fn discover_directory_groups(
    directory: &Path,
    group_root_files: bool,
    groups: &mut Vec<AudioGroupCandidate>,
) -> Result<(), AudioImportError> {
    let mut count = groups.iter().map(|group| group.paths.len()).sum::<usize>();
    discover_directory_groups_inner(directory, 0, group_root_files, groups, &mut count)
}

fn discover_directory_groups_inner(
    directory: &Path,
    depth: usize,
    group_this_directory: bool,
    groups: &mut Vec<AudioGroupCandidate>,
    count: &mut usize,
) -> Result<(), AudioImportError> {
    if depth > 12 {
        return Ok(());
    }
    let mut local_files = Vec::new();
    let mut child_directories = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            child_directories.push(entry.path());
        } else if file_type.is_file() && supported_audio_path(&entry.path()) {
            *count += 1;
            if *count > MAX_WATCHED_AUDIO_FILES {
                return Err(AudioImportError::TooManyWatchedFiles);
            }
            local_files.push(entry.path());
        }
    }
    local_files.sort_by(|left, right| natural_path_cmp(left, right));
    if group_this_directory {
        if !local_files.is_empty() {
            groups.push(AudioGroupCandidate {
                paths: local_files,
                group_as_folder: true,
                preserve_order: false,
                group_key_override: None,
                title_override: None,
                chapters: Vec::new(),
            });
        }
    } else {
        groups.extend(local_files.into_iter().map(|path| AudioGroupCandidate {
            paths: vec![path],
            group_as_folder: false,
            preserve_order: false,
            group_key_override: None,
            title_override: None,
            chapters: Vec::new(),
        }));
    }
    child_directories.sort_by(|left, right| natural_path_cmp(left, right));
    for child in child_directories {
        discover_directory_groups_inner(&child, depth + 1, true, groups, count)?;
    }
    Ok(())
}

fn deduplicate_groups(groups: &mut Vec<AudioGroupCandidate>) {
    let mut seen = BTreeSet::new();
    for group in groups.iter_mut() {
        group.paths.retain(|path| {
            path.canonicalize()
                .ok()
                .is_some_and(|path| seen.insert(normalized_path_key(&path)))
        });
    }
    groups.retain(|group| !group.paths.is_empty());
}

fn group_title(parts: &[ValidatedAudioPath], multi_part: bool) -> String {
    if multi_part {
        bounded_title(
            parts[0]
                .canonical
                .parent()
                .and_then(Path::file_name)
                .and_then(|value| value.to_str()),
        )
    } else {
        bounded_title(
            parts[0]
                .canonical
                .file_stem()
                .and_then(|value| value.to_str()),
        )
    }
}

fn bounded_title(value: Option<&str>) -> String {
    let value = value.unwrap_or("Untitled");
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let title = normalized.chars().take(512).collect::<String>();
    if title.is_empty() {
        "Untitled".to_owned()
    } else {
        title
    }
}

fn validate_group_limits(
    part_count: usize,
    sizes: impl IntoIterator<Item = u64>,
) -> Result<u64, AudioImportError> {
    if part_count > MAX_AUDIOBOOK_PARTS {
        return Err(AudioImportError::TooManyParts);
    }
    sizes.into_iter().try_fold(0_u64, |total, size| {
        total
            .checked_add(size)
            .filter(|value| *value <= MAX_AUDIOBOOK_BYTES)
            .ok_or(AudioImportError::BookTooLarge)
    })
}

fn sha256(path: &Path, max_size: u64) -> Result<(String, u64), AudioImportError> {
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut total = 0_u64;
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        total = total
            .checked_add(count as u64)
            .ok_or(AudioImportError::FileTooLarge)?;
        if total > max_size {
            return Err(AudioImportError::FileTooLarge);
        }
        digest.update(&buffer[..count]);
    }
    Ok((
        digest
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
        total,
    ))
}

fn normalized_path_key(path: &Path) -> String {
    let value = path.to_string_lossy();
    #[cfg(windows)]
    {
        value.to_ascii_lowercase()
    }
    #[cfg(not(windows))]
    {
        value.into_owned()
    }
}

fn same_path(left: &Path, right: &Path) -> bool {
    normalized_path_key(left) == normalized_path_key(right)
}

fn natural_path_cmp(left: &Path, right: &Path) -> Ordering {
    let left = left
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let right = right
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    natural_cmp(left, right).then_with(|| left.to_lowercase().cmp(&right.to_lowercase()))
}

fn natural_cmp(left: &str, right: &str) -> Ordering {
    let mut left = left.chars().peekable();
    let mut right = right.chars().peekable();
    loop {
        match (left.peek(), right.peek()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(a), Some(b)) if a.is_ascii_digit() && b.is_ascii_digit() => {
                let left_number = take_number(&mut left);
                let right_number = take_number(&mut right);
                match left_number.cmp(&right_number) {
                    Ordering::Equal => {}
                    ordering => return ordering,
                }
            }
            _ => {
                let a = left.next().unwrap_or_default().to_ascii_lowercase();
                let b = right.next().unwrap_or_default().to_ascii_lowercase();
                match a.cmp(&b) {
                    Ordering::Equal => {}
                    ordering => return ordering,
                }
            }
        }
    }
}

fn take_number(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> u64 {
    let mut number = 0_u64;
    while chars.peek().is_some_and(|value| value.is_ascii_digit()) {
        let digit = chars
            .next()
            .and_then(|value| value.to_digit(10))
            .unwrap_or(0) as u64;
        number = number.saturating_mul(10).saturating_add(digit);
    }
    number
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_order_keeps_numbered_parts_in_listening_order() {
        let mut names = ["part 10.mp3", "part 2.mp3", "part 01.mp3"];
        names.sort_by(|left, right| natural_cmp(left, right));
        assert_eq!(names, ["part 01.mp3", "part 2.mp3", "part 10.mp3"]);
    }

    #[test]
    fn validates_group_limits_without_overflow() {
        let sizes = [MAX_AUDIO_FILE_BYTES; 5];
        let total = validate_group_limits(sizes.len(), sizes);
        assert_eq!(total.expect("exact 100 GiB"), MAX_AUDIOBOOK_BYTES);
        assert!(matches!(
            validate_group_limits(6, [MAX_AUDIO_FILE_BYTES; 5].into_iter().chain([1])),
            Err(AudioImportError::BookTooLarge)
        ));
        assert!(matches!(
            validate_group_limits(MAX_AUDIOBOOK_PARTS + 1, std::iter::empty()),
            Err(AudioImportError::TooManyParts)
        ));
    }

    #[test]
    fn parses_local_m3u_order_and_rejects_paths_outside_its_folder() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let playlist_directory = directory.path().join("playlist");
        fs::create_dir(&playlist_directory).expect("playlist directory");
        let first = playlist_directory.join("10.mp3");
        let second = playlist_directory.join("02.mp3");
        fs::write(&first, b"first").expect("audio");
        fs::write(&second, b"second").expect("audio");
        let playlist = playlist_directory.join("Listening order.m3u8");
        fs::write(&playlist, "#EXTM3U\n10.mp3\n02.mp3\n").expect("playlist");

        let candidate = parse_audio_descriptor(&playlist).expect("safe playlist");
        assert!(candidate.preserve_order);
        assert_eq!(
            candidate.paths,
            vec![
                first.canonicalize().unwrap(),
                second.canonicalize().unwrap()
            ]
        );

        let outside = directory.path().join("outside.mp3");
        fs::write(&outside, b"outside").expect("outside fixture");
        fs::write(&playlist, "../outside.mp3\n").expect("unsafe playlist");
        assert!(matches!(
            parse_audio_descriptor(&playlist),
            Err(AudioImportError::InvalidDescriptor)
        ));
    }

    #[test]
    fn parses_cue_titles_and_frame_accurate_chapter_offsets() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let audio = directory.path().join("book.flac");
        fs::write(&audio, b"audio").expect("audio");
        let cue = directory.path().join("book.cue");
        fs::write(
            &cue,
            "TITLE \"A Quiet Book\"\nFILE \"book.flac\" WAVE\n  TRACK 01 AUDIO\n    TITLE \"Arrival\"\n    INDEX 01 00:00:00\n  TRACK 02 AUDIO\n    TITLE \"The Hall\"\n    INDEX 01 01:02:37\n",
        )
        .expect("cue");

        let candidate = parse_audio_descriptor(&cue).expect("cue descriptor");
        assert_eq!(candidate.title_override.as_deref(), Some("A Quiet Book"));
        assert_eq!(candidate.paths.len(), 1);
        assert_eq!(candidate.chapters.len(), 2);
        assert_eq!(candidate.chapters[1].title, "The Hall");
        assert!((candidate.chapters[1].start_seconds - 62.493_333).abs() < 0.001);
    }
}
