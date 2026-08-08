use base64::{engine::general_purpose::STANDARD, Engine as _};
use quick_xml::{events::Event, Reader};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Seek},
    path::{Component, Path},
};
use thiserror::Error;
use zip::ZipArchive;

const MAX_BOOK_SIZE: u64 = 2 * 1024 * 1024 * 1024;
const MAX_COMIC_BOOK_SIZE: u64 = 4 * 1024 * 1024 * 1024;
const MAX_XML_SIZE: u64 = 4 * 1024 * 1024;
const MAX_COVER_SIZE: u64 = 10 * 1024 * 1024;
const MAX_METADATA_FIELD_CHARS: usize = 512;
const SUPPORTED: &[&str] = &[
    "epub", "fb2", "txt", "html", "htm", "md", "markdown", "pdf", "cbz", "cbr", "docx",
];

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("unsupported book format")]
    Unsupported,
    #[error("the file does not exist or is not a regular file")]
    Missing,
    #[error("the file is larger than the safety limit for this format")]
    TooLarge,
    #[error("the file is damaged or cannot be read: {0}")]
    Io(#[from] std::io::Error),
    #[error("the archive is invalid: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("embedded metadata is invalid: {0}")]
    Xml(#[from] quick_xml::Error),
    #[error("the file does not contain a valid document for its format")]
    InvalidFormat,
}

pub struct ImportedBook {
    pub source_path: String,
    pub fingerprint: String,
    pub title: String,
    pub author: String,
    pub genres: String,
    pub format: String,
    pub file_size: i64,
    pub cover_path: Option<String>,
}

#[derive(Default)]
struct Metadata {
    title: Option<String>,
    author: Option<String>,
    genres: Vec<String>,
    cover_bytes: Option<Vec<u8>>,
    cover_extension: Option<String>,
}

pub fn supported_book_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| SUPPORTED.contains(&extension.to_ascii_lowercase().as_str()))
}

pub fn inspect_book(path: &Path, cover_dir: &Path) -> Result<ImportedBook, ImportError> {
    if !path.is_file() {
        return Err(ImportError::Missing);
    }
    if !supported_book_path(path) {
        return Err(ImportError::Unsupported);
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let max_size = max_book_size(&extension);
    let file_size = fs::metadata(path)?.len();
    if file_size > max_size {
        return Err(ImportError::TooLarge);
    }
    let fingerprint = sha256(path, max_size)?;
    let metadata = match extension.as_str() {
        "epub" => inspect_epub(path)?,
        "fb2" => inspect_fb2(path)?,
        _ => Metadata::default(),
    };
    let fallback_title = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("Untitled")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(MAX_METADATA_FIELD_CHARS)
        .collect::<String>();
    let cover_path = save_cover(
        &fingerprint,
        metadata.cover_bytes.as_deref(),
        metadata.cover_extension.as_deref(),
        cover_dir,
    )?;
    Ok(ImportedBook {
        source_path: path.canonicalize()?.to_string_lossy().into_owned(),
        fingerprint,
        title: bounded_metadata(metadata.title).unwrap_or(fallback_title),
        author: bounded_metadata(metadata.author).unwrap_or_default(),
        genres: normalize_genres(metadata.genres),
        format: extension.to_ascii_uppercase(),
        file_size: i64::try_from(file_size).unwrap_or(i64::MAX),
        cover_path,
    })
}

fn max_book_size(extension: &str) -> u64 {
    match extension {
        "cbz" | "cbr" => MAX_COMIC_BOOK_SIZE,
        _ => MAX_BOOK_SIZE,
    }
}

fn sha256(path: &Path, max_size: u64) -> Result<String, ImportError> {
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
            .ok_or(ImportError::TooLarge)?;
        if total > max_size {
            return Err(ImportError::TooLarge);
        }
        digest.update(&buffer[..count]);
    }
    Ok(digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn inspect_epub(path: &Path) -> Result<Metadata, ImportError> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    let container = read_zip_entry(&mut archive, "META-INF/container.xml", MAX_XML_SIZE)?;
    let container_text = String::from_utf8_lossy(&container);
    let opf_path = attribute_value(&container_text, "full-path").ok_or(ImportError::Missing)?;
    if !safe_archive_path(Path::new(&opf_path)) {
        return Err(ImportError::Missing);
    }
    let opf = read_zip_entry(&mut archive, &opf_path, MAX_XML_SIZE)?;
    let mut metadata = parse_epub_opf(&opf);
    if let Some(cover_href) = metadata.cover_extension.take() {
        let base = Path::new(&opf_path)
            .parent()
            .unwrap_or_else(|| Path::new(""));
        let cover_archive_path = base.join(&cover_href);
        if safe_archive_path(&cover_archive_path) {
            let cover_name = cover_archive_path.to_string_lossy().replace('\\', "/");
            if let Ok(bytes) = read_zip_entry(&mut archive, &cover_name, MAX_COVER_SIZE) {
                metadata.cover_extension = image_extension(&bytes).map(str::to_owned);
                metadata.cover_bytes = Some(bytes);
            }
        }
    }
    Ok(metadata)
}

fn parse_epub_opf(xml: &[u8]) -> Metadata {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    let mut metadata = Metadata::default();
    let mut current = String::new();
    let mut cover_id = None;
    let mut cover_items = Vec::<(String, String, String)>::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) | Ok(Event::Empty(event)) => {
                let name = local_name(event.name().as_ref());
                current = name.clone();
                if name == "meta" {
                    let attrs = attributes(&event);
                    if attrs.get("name").is_some_and(|value| value == "cover") {
                        cover_id = attrs.get("content").cloned();
                    }
                } else if name == "item" {
                    let attrs = attributes(&event);
                    cover_items.push((
                        attrs.get("id").cloned().unwrap_or_default(),
                        attrs.get("href").cloned().unwrap_or_default(),
                        attrs.get("properties").cloned().unwrap_or_default(),
                    ));
                }
            }
            Ok(Event::Text(text)) => {
                let value = text.decode().unwrap_or_default().trim().to_owned();
                if current == "title" && metadata.title.is_none() {
                    metadata.title = Some(value);
                } else if current == "creator" && metadata.author.is_none() {
                    metadata.author = Some(value);
                } else if current == "subject" {
                    metadata.genres.push(value);
                }
            }
            Ok(Event::End(_)) => current.clear(),
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    metadata.cover_extension = cover_items
        .iter()
        .find(|(id, _, properties)| {
            cover_id.as_ref().is_some_and(|cover| cover == id)
                || properties
                    .split_whitespace()
                    .any(|item| item == "cover-image")
        })
        .map(|(_, href, _)| href.clone());
    metadata
}

fn inspect_fb2(path: &Path) -> Result<Metadata, ImportError> {
    let metadata_limit = MAX_XML_SIZE * 8;
    if fs::metadata(path)?.len() > metadata_limit {
        return Ok(Metadata::default());
    }
    let mut bytes = Vec::new();
    File::open(path)?
        .take(metadata_limit + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > metadata_limit {
        return Ok(Metadata::default());
    }
    let probe = String::from_utf8_lossy(&bytes[..bytes.len().min(2048)]).to_ascii_lowercase();
    if !probe.contains("<fictionbook") {
        return Err(ImportError::InvalidFormat);
    }
    let mut reader = Reader::from_reader(bytes.as_slice());
    reader.config_mut().trim_text(true);
    let mut metadata = Metadata::default();
    let mut current = String::new();
    let mut author_parts = Vec::new();
    let mut cover_id = None;
    let mut binary_id = None;
    let mut binary_content_type = None;
    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) | Ok(Event::Empty(event)) => {
                current = local_name(event.name().as_ref());
                let attrs = attributes(&event);
                if current == "image" && cover_id.is_none() {
                    cover_id = attrs
                        .get("href")
                        .map(|value| value.trim_start_matches('#').to_owned());
                } else if current == "binary" {
                    binary_id = attrs.get("id").cloned();
                    binary_content_type = attrs.get("content-type").cloned();
                }
            }
            Ok(Event::Text(text)) => {
                let value = text.decode().unwrap_or_default().trim().to_owned();
                match current.as_str() {
                    "book-title" if metadata.title.is_none() => metadata.title = Some(value),
                    "first-name" | "middle-name" | "last-name" if metadata.author.is_none() => {
                        if !value.is_empty() {
                            author_parts.push(value);
                        }
                    }
                    "genre" => metadata.genres.push(value),
                    "binary" if binary_id == cover_id => {
                        if let Ok(decoded) = STANDARD.decode(value.as_bytes()) {
                            metadata.cover_extension = binary_content_type
                                .as_deref()
                                .and_then(mime_extension)
                                .map(str::to_owned);
                            metadata.cover_bytes = Some(decoded);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(event)) => {
                if local_name(event.name().as_ref()) == "author" && !author_parts.is_empty() {
                    metadata.author = Some(author_parts.join(" "));
                    author_parts.clear();
                }
                current.clear();
            }
            Ok(Event::Eof) => break,
            Err(error) => return Err(ImportError::Xml(error)),
            _ => {}
        }
    }
    Ok(metadata)
}

fn read_zip_entry<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
    max_size: u64,
) -> Result<Vec<u8>, ImportError> {
    let entry = archive.by_name(name)?;
    if entry.size() > max_size || !safe_archive_path(Path::new(name)) {
        return Err(ImportError::TooLarge);
    }
    let mut bytes = Vec::with_capacity(usize::try_from(entry.size()).unwrap_or(0));
    entry.take(max_size + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > max_size {
        return Err(ImportError::TooLarge);
    }
    Ok(bytes)
}

fn safe_archive_path(path: &Path) -> bool {
    !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
}

fn save_cover(
    fingerprint: &str,
    bytes: Option<&[u8]>,
    extension: Option<&str>,
    cover_dir: &Path,
) -> Result<Option<String>, std::io::Error> {
    let Some(bytes) =
        bytes.filter(|value| !value.is_empty() && value.len() as u64 <= MAX_COVER_SIZE)
    else {
        return Ok(None);
    };
    let extension = image_extension(bytes).or(extension).unwrap_or("bin");
    if !["jpg", "jpeg", "png", "gif", "webp"].contains(&extension) {
        return Ok(None);
    }
    fs::create_dir_all(cover_dir)?;
    let path = cover_dir.join(format!("{fingerprint}.{extension}"));
    if !path.exists() {
        fs::write(&path, bytes)?;
    }
    Ok(Some(path.to_string_lossy().into_owned()))
}

fn image_extension(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("png")
    } else if bytes.starts_with(b"\xff\xd8\xff") {
        Some("jpg")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("gif")
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        Some("webp")
    } else {
        None
    }
}

fn mime_extension(value: &str) -> Option<&'static str> {
    match value.to_ascii_lowercase().as_str() {
        "image/jpeg" | "image/jpg" => Some("jpg"),
        "image/png" => Some("png"),
        "image/gif" => Some("gif"),
        "image/webp" => Some("webp"),
        _ => None,
    }
}

fn attribute_value(xml: &str, name: &str) -> Option<String> {
    for quote in ['"', '\''] {
        let needle = format!("{name}={quote}");
        if let Some(start) = xml.find(&needle) {
            let value_start = start + needle.len();
            if let Some(end) = xml[value_start..].find(quote) {
                return Some(xml[value_start..value_start + end].to_owned());
            }
        }
    }
    None
}

fn local_name(name: &[u8]) -> String {
    String::from_utf8_lossy(name)
        .rsplit(':')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase()
}

fn attributes(
    event: &quick_xml::events::BytesStart<'_>,
) -> std::collections::HashMap<String, String> {
    event
        .attributes()
        .filter_map(Result::ok)
        .map(|attribute| {
            (
                local_name(attribute.key.as_ref()),
                String::from_utf8_lossy(attribute.value.as_ref()).into_owned(),
            )
        })
        .collect()
}

fn bounded_metadata(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
        if normalized.is_empty() {
            None
        } else {
            Some(normalized.chars().take(MAX_METADATA_FIELD_CHARS).collect())
        }
    })
}

fn normalize_genres(values: Vec<String>) -> String {
    let mut genres = Vec::<String>::new();
    for value in values {
        for candidate in value.split([',', ';']) {
            let normalized = candidate
                .replace('_', " ")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            if normalized.is_empty()
                || normalized.chars().count() > 64
                || genres
                    .iter()
                    .any(|genre| genre.to_lowercase() == normalized.to_lowercase())
            {
                continue;
            }
            genres.push(normalized);
            if genres.len() == 12 {
                break;
            }
        }
        if genres.len() == 12 {
            break;
        }
    }
    genres.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn rejects_archive_traversal_paths() {
        assert!(!safe_archive_path(Path::new("../cover.png")));
        assert!(!safe_archive_path(Path::new("/cover.png")));
        assert!(safe_archive_path(Path::new("OPS/images/cover.png")));
    }

    #[test]
    fn bounds_and_normalizes_embedded_metadata() {
        let long = format!(
            "  {}\n ignored tail  ",
            "A".repeat(MAX_METADATA_FIELD_CHARS)
        );
        let value = bounded_metadata(Some(long)).expect("metadata");
        assert_eq!(value.chars().count(), MAX_METADATA_FIELD_CHARS);
        assert!(!value.contains('\n'));
    }

    #[test]
    fn recognizes_supported_formats_case_insensitively() {
        assert!(supported_book_path(Path::new("Book.EPUB")));
        assert!(supported_book_path(Path::new("notes.md")));
        assert!(!supported_book_path(Path::new("program.exe")));
    }

    #[test]
    fn comic_imports_use_the_larger_source_limit() {
        assert_eq!(max_book_size("pdf"), 2 * 1024 * 1024 * 1024);
        assert_eq!(max_book_size("cbz"), 4 * 1024 * 1024 * 1024);
        assert_eq!(max_book_size("cbr"), 4 * 1024 * 1024 * 1024);
    }

    #[test]
    fn extracts_epub_metadata_and_cover() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("fixture.epub");
        let file = File::create(&path).expect("epub");
        let mut archive = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        archive.start_file("mimetype", options).expect("mimetype");
        archive
            .write_all(b"application/epub+zip")
            .expect("mimetype content");
        archive
            .start_file("META-INF/container.xml", options)
            .expect("container");
        archive
            .write_all(
                br#"<?xml version="1.0"?><container><rootfiles><rootfile full-path="OPS/book.opf"/></rootfiles></container>"#,
            )
            .expect("container content");
        archive.start_file("OPS/book.opf", options).expect("opf");
        archive
            .write_all(
                br#"<?xml version="1.0"?><package><metadata><dc:title>Fixture Book</dc:title><dc:creator>Test Author</dc:creator><dc:subject>Science Fiction</dc:subject><dc:subject>Adventure</dc:subject><meta name="cover" content="cover"/></metadata><manifest><item id="cover" href="cover.png" media-type="image/png"/></manifest></package>"#,
            )
            .expect("opf content");
        archive.start_file("OPS/cover.png", options).expect("cover");
        archive
            .write_all(b"\x89PNG\r\n\x1a\nsynthetic")
            .expect("cover content");
        archive.finish().expect("finish");

        let book = inspect_book(&path, &directory.path().join("covers")).expect("inspect");
        assert_eq!(book.title, "Fixture Book");
        assert_eq!(book.author, "Test Author");
        assert_eq!(book.genres, "Science Fiction, Adventure");
        assert!(book.cover_path.is_some());
    }

    #[test]
    fn rejects_a_corrupted_epub() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("broken.epub");
        fs::write(&path, "not an archive").expect("fixture");
        assert!(inspect_book(&path, &directory.path().join("covers")).is_err());
    }

    #[test]
    fn extracts_and_normalizes_fb2_genres() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("fixture.fb2");
        fs::write(
            &path,
            r#"<?xml version="1.0" encoding="utf-8"?>
               <FictionBook><description><title-info>
                 <genre>science_fiction</genre>
                 <genre>Adventure</genre>
                 <genre>adventure</genre>
                 <book-title>Genre Fixture</book-title>
               </title-info></description><body><section><p>Text.</p></section></body>
               </FictionBook>"#,
        )
        .expect("fixture");
        let book = inspect_book(&path, &directory.path().join("covers")).expect("inspect");
        assert_eq!(book.genres, "science fiction, Adventure");
    }
}
