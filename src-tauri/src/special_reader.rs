use rars::ArchiveReader;
use serde::Serialize;
use std::{
    cmp::Ordering,
    collections::HashMap,
    fs::{self, File},
    io::{Read, Write},
    path::{Component, Path},
};
use thiserror::Error;
use zip::ZipArchive;

const MAX_FIXED_FILE_SIZE: u64 = 512 * 1024 * 1024;
const MAX_COMIC_PAGE_SIZE: u64 = 64 * 1024 * 1024;
const MAX_COMIC_TOTAL_SIZE: u64 = 768 * 1024 * 1024;
const MAX_COMIC_PAGES: usize = 10_000;

#[derive(Debug, Error)]
pub enum SpecialReaderError {
    #[error("this format does not use the fixed-layout reader")]
    Unsupported,
    #[error("the source book is unavailable")]
    Missing,
    #[error("the document exceeds the fixed-layout safety limit")]
    TooLarge,
    #[error("the comic archive contains no supported images")]
    EmptyComic,
    #[error("the PDF signature is invalid")]
    InvalidPdf,
    #[error("encrypted or split comic archives are not supported")]
    UnsupportedArchive,
    #[error("the app-local reader cache identity is invalid")]
    InvalidCacheIdentity,
    #[error("the document is damaged or cannot be read: {0}")]
    Io(#[from] std::io::Error),
    #[error("the CBZ archive is invalid: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("the CBR archive is invalid: {0}")]
    Rar(#[from] rars::Error),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecialDocument {
    pub book_id: i64,
    pub title: String,
    pub author: String,
    pub format: String,
    pub kind: SpecialKind,
    pub source_path: Option<String>,
    pub pages: Vec<ComicPage>,
    pub progress: f64,
    pub last_page: usize,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SpecialKind {
    Pdf,
    Comic,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComicPage {
    pub index: usize,
    pub name: String,
    pub path: String,
    pub mime: String,
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_special_document(
    book_id: i64,
    title: String,
    author: String,
    format: String,
    source_path: &Path,
    fingerprint: &str,
    progress: f64,
    last_page: usize,
    cache_root: &Path,
) -> Result<SpecialDocument, SpecialReaderError> {
    if !source_path.is_file() {
        return Err(SpecialReaderError::Missing);
    }
    let size = fs::metadata(source_path)?.len();
    if size > MAX_FIXED_FILE_SIZE {
        return Err(SpecialReaderError::TooLarge);
    }
    let key = fingerprint
        .get(..24)
        .filter(|value| value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or(SpecialReaderError::InvalidCacheIdentity)?;
    let document_cache = cache_root.join(key);
    match format.as_str() {
        "PDF" => {
            validate_pdf(source_path)?;
            fs::create_dir_all(&document_cache)?;
            let target = document_cache.join("document.pdf");
            if !cache_is_current(source_path, &target, size) {
                copy_bounded_atomic(source_path, &target)?;
            }
            Ok(SpecialDocument {
                book_id,
                title,
                author,
                format,
                kind: SpecialKind::Pdf,
                source_path: Some(target.to_string_lossy().into_owned()),
                pages: Vec::new(),
                progress,
                last_page,
            })
        }
        "CBZ" | "CBR" => {
            fs::create_dir_all(&document_cache)?;
            let marker = document_cache.join(".complete");
            if !marker.is_file() || is_newer(source_path, &marker) {
                fs::remove_dir_all(&document_cache)?;
                fs::create_dir_all(&document_cache)?;
                if format == "CBZ" {
                    extract_cbz(source_path, &document_cache)?;
                } else {
                    extract_cbr(source_path, &document_cache)?;
                }
                fs::write(marker, b"ok")?;
            }
            let pages = cached_pages(&document_cache)?;
            if pages.is_empty() {
                return Err(SpecialReaderError::EmptyComic);
            }
            Ok(SpecialDocument {
                book_id,
                title,
                author,
                format,
                kind: SpecialKind::Comic,
                source_path: None,
                last_page: last_page.min(pages.len().saturating_sub(1)),
                pages,
                progress,
            })
        }
        _ => Err(SpecialReaderError::Unsupported),
    }
}

fn cache_is_current(source: &Path, target: &Path, expected_size: u64) -> bool {
    target.is_file()
        && fs::metadata(target).is_ok_and(|metadata| metadata.len() == expected_size)
        && !is_newer(source, target)
}

fn is_newer(source: &Path, cached: &Path) -> bool {
    let source_modified = fs::metadata(source).and_then(|metadata| metadata.modified());
    let cached_modified = fs::metadata(cached).and_then(|metadata| metadata.modified());
    match (source_modified, cached_modified) {
        (Ok(source), Ok(cached)) => source > cached,
        _ => true,
    }
}

fn copy_bounded_atomic(source: &Path, target: &Path) -> Result<(), SpecialReaderError> {
    let temporary = target.with_extension("part");
    let result = (|| {
        let source = File::open(source)?;
        let mut writer = LimitedWriter::new(File::create(&temporary)?, MAX_FIXED_FILE_SIZE);
        std::io::copy(&mut source.take(MAX_FIXED_FILE_SIZE + 1), &mut writer)?;
        writer.flush()?;
        if target.is_file() {
            fs::remove_file(target)?;
        }
        fs::rename(&temporary, target)?;
        Ok::<(), std::io::Error>(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map_err(Into::into)
}

fn validate_pdf(path: &Path) -> Result<(), SpecialReaderError> {
    let mut file = File::open(path)?;
    let mut header = [0_u8; 5];
    let count = file.read(&mut header)?;
    if count != header.len() || &header != b"%PDF-" {
        return Err(SpecialReaderError::InvalidPdf);
    }
    Ok(())
}

fn extract_cbz(source: &Path, target: &Path) -> Result<(), SpecialReaderError> {
    let mut archive = ZipArchive::new(File::open(source)?)?;
    let mut entries = Vec::new();
    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        let Some(path) = entry.enclosed_name() else {
            continue;
        };
        if !safe_archive_path(&path) {
            continue;
        }
        let name = path.to_string_lossy().into_owned();
        if image_extension(&name).is_some() && !entry.is_dir() {
            entries.push((name, entry.size()));
        }
    }
    validate_comic_entries(entries.iter().map(|(_, size)| *size))?;
    entries.sort_by(|left, right| natural_cmp(&left.0, &right.0));
    for (page_index, (name, _)) in entries.into_iter().enumerate() {
        let entry = archive.by_name(&name)?;
        let extension = image_extension(&name).ok_or(SpecialReaderError::EmptyComic)?;
        let output = target.join(format!("page-{page_index:05}.{extension}"));
        let mut writer = LimitedWriter::new(File::create(output)?, MAX_COMIC_PAGE_SIZE);
        std::io::copy(&mut entry.take(MAX_COMIC_PAGE_SIZE + 1), &mut writer)?;
    }
    Ok(())
}

fn extract_cbr(source: &Path, target: &Path) -> Result<(), SpecialReaderError> {
    let archive = ArchiveReader::read_path(source)?;
    let mut entries = archive
        .members()
        .filter(|member| !member.meta.is_directory)
        .filter_map(|member| {
            let name = member.meta.name_lossy();
            image_extension(&name).map(|extension| {
                (
                    member.meta.name,
                    name,
                    extension.to_owned(),
                    member.meta.unpacked_size,
                    member.meta.is_encrypted,
                    member.meta.is_split_before || member.meta.is_split_after,
                )
            })
        })
        .collect::<Vec<_>>();
    validate_comic_entries(entries.iter().map(|entry| entry.3))?;
    if entries.iter().any(|entry| entry.4 || entry.5) {
        return Err(SpecialReaderError::UnsupportedArchive);
    }
    if entries
        .iter()
        .any(|entry| !safe_archive_path(Path::new(&entry.1)))
    {
        return Err(SpecialReaderError::UnsupportedArchive);
    }
    entries.sort_by(|left, right| natural_cmp(&left.1, &right.1));
    let targets = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            (
                entry.0.clone(),
                target.join(format!("page-{index:05}.{}", entry.2)),
            )
        })
        .collect::<HashMap<_, _>>();
    archive.extract_to(None, |meta| {
        if let Some(path) = targets.get(meta.name_bytes()) {
            Ok(
                Box::new(LimitedWriter::new(File::create(path)?, MAX_COMIC_PAGE_SIZE))
                    as Box<dyn Write>,
            )
        } else {
            Ok(Box::new(std::io::sink()) as Box<dyn Write>)
        }
    })?;
    Ok(())
}

fn validate_comic_entries(sizes: impl Iterator<Item = u64>) -> Result<(), SpecialReaderError> {
    let mut count = 0_usize;
    let mut total = 0_u64;
    for size in sizes {
        count += 1;
        if count > MAX_COMIC_PAGES || size > MAX_COMIC_PAGE_SIZE {
            return Err(SpecialReaderError::TooLarge);
        }
        total = total
            .checked_add(size)
            .ok_or(SpecialReaderError::TooLarge)?;
        if total > MAX_COMIC_TOTAL_SIZE {
            return Err(SpecialReaderError::TooLarge);
        }
    }
    if count == 0 {
        return Err(SpecialReaderError::EmptyComic);
    }
    Ok(())
}

fn cached_pages(directory: &Path) -> Result<Vec<ComicPage>, SpecialReaderError> {
    let mut paths = fs::read_dir(directory)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("page-"))
        })
        .collect::<Vec<_>>();
    paths.sort();
    paths
        .into_iter()
        .enumerate()
        .map(|(index, path)| {
            let mime = validate_cached_image(&path)?;
            Ok(ComicPage {
                index,
                name: path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("page")
                    .to_owned(),
                path: path.to_string_lossy().into_owned(),
                mime: mime.to_owned(),
            })
        })
        .collect()
}

fn validate_cached_image(path: &Path) -> Result<&'static str, SpecialReaderError> {
    let mut file = File::open(path)?;
    let mut header = [0_u8; 16];
    let count = file.read(&mut header)?;
    let bytes = &header[..count];
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Ok("image/png")
    } else if bytes.starts_with(b"\xff\xd8\xff") {
        Ok("image/jpeg")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Ok("image/gif")
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        Ok("image/webp")
    } else {
        Err(SpecialReaderError::EmptyComic)
    }
}

fn image_extension(name: &str) -> Option<&'static str> {
    match Path::new(name)
        .extension()
        .and_then(|extension| extension.to_str())?
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => Some("png"),
        "jpg" | "jpeg" => Some("jpg"),
        "gif" => Some("gif"),
        "webp" => Some("webp"),
        _ => None,
    }
}

fn safe_archive_path(path: &Path) -> bool {
    !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
}

fn natural_cmp(left: &str, right: &str) -> Ordering {
    let mut left = left.chars().peekable();
    let mut right = right.chars().peekable();
    loop {
        match (left.peek(), right.peek()) {
            (Some(a), Some(b)) if a.is_ascii_digit() && b.is_ascii_digit() => {
                let left_number = take_number(&mut left);
                let right_number = take_number(&mut right);
                match left_number.cmp(&right_number) {
                    Ordering::Equal => {}
                    ordering => return ordering,
                }
            }
            (Some(_), Some(_)) => {
                let a = left.next().unwrap_or_default().to_ascii_lowercase();
                let b = right.next().unwrap_or_default().to_ascii_lowercase();
                match a.cmp(&b) {
                    Ordering::Equal => {}
                    ordering => return ordering,
                }
            }
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
        }
    }
}

fn take_number(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> u128 {
    let mut value = 0_u128;
    while chars.peek().is_some_and(char::is_ascii_digit) {
        value = value.saturating_mul(10).saturating_add(
            chars
                .next()
                .unwrap_or_default()
                .to_digit(10)
                .unwrap_or(0)
                .into(),
        );
    }
    value
}

struct LimitedWriter<W> {
    inner: W,
    remaining: u64,
}

impl<W> LimitedWriter<W> {
    fn new(inner: W, limit: u64) -> Self {
        Self {
            inner,
            remaining: limit,
        }
    }
}

impl<W: Write> Write for LimitedWriter<W> {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        if buffer.len() as u64 > self.remaining {
            return Err(std::io::Error::other("comic page exceeds safety limit"));
        }
        let written = self.inner.write(buffer)?;
        self.remaining = self.remaining.saturating_sub(written as u64);
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG: &[u8] = &[137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82];

    #[test]
    fn cbz_pages_use_natural_order_and_safe_cache_names() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source = directory.path().join("comic.cbz");
        let file = File::create(&source).expect("CBZ");
        let mut archive = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        for name in ["page10.png", "page2.png", "page1.png"] {
            archive.start_file(name, options).expect("page");
            archive.write_all(PNG).expect("PNG");
        }
        archive.finish().expect("archive");
        let target = directory.path().join("cache");
        fs::create_dir(&target).expect("cache");
        extract_cbz(&source, &target).expect("extract");
        let pages = cached_pages(&target).expect("pages");
        assert_eq!(pages.len(), 3);
        assert!(pages[0].path.ends_with("page-00000.png"));
    }

    #[test]
    fn cbr_reads_rar3_family_without_external_tools() {
        use rars::{
            rar15_40::{write_stored_archive, StoredEntry, WriterOptions},
            version::ArchiveVersion,
            FeatureSet,
        };
        let directory = tempfile::tempdir().expect("temporary directory");
        let source = directory.path().join("comic.cbr");
        let entries = [StoredEntry {
            name: b"001.png",
            data: PNG,
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
        fs::write(&source, bytes).expect("CBR");
        let target = directory.path().join("cache");
        fs::create_dir(&target).expect("cache");
        extract_cbr(&source, &target).expect("extract");
        assert_eq!(cached_pages(&target).expect("pages").len(), 1);
    }

    #[test]
    fn cbr_reads_rar5_family_without_external_tools() {
        use rars::{
            rar50::{Rar50Writer, StoredEntry, WriterOptions},
            version::ArchiveVersion,
            FeatureSet,
        };
        let directory = tempfile::tempdir().expect("temporary directory");
        let source = directory.path().join("comic-rar5.cbr");
        let entries = [StoredEntry {
            name: b"unicode-page-001.png",
            data: PNG,
            mtime: None,
            attributes: 0x20,
            host_os: 3,
        }];
        let bytes = Rar50Writer::new(WriterOptions::new(
            ArchiveVersion::Rar50,
            FeatureSet::store_only(),
        ))
        .stored_entries(&entries)
        .finish()
        .expect("RAR5");
        fs::write(&source, bytes).expect("CBR");
        let target = directory.path().join("cache");
        fs::create_dir(&target).expect("cache");
        extract_cbr(&source, &target).expect("extract");
        assert_eq!(cached_pages(&target).expect("pages").len(), 1);
    }

    #[test]
    fn rejects_unsafe_paths_and_non_images() {
        assert!(!safe_archive_path(Path::new("../page.png")));
        assert!(safe_archive_path(Path::new("chapter/page.png")));
        assert_eq!(image_extension("page.svg"), None);
    }

    #[test]
    fn rejects_a_file_without_a_pdf_signature() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source = directory.path().join("renamed.pdf");
        fs::write(&source, b"not a PDF").expect("fixture");
        assert!(matches!(
            validate_pdf(&source),
            Err(SpecialReaderError::InvalidPdf)
        ));
    }

    #[test]
    fn rejects_an_invalid_cache_identity_without_touching_the_cache_root() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source = directory.path().join("book.pdf");
        let cache = directory.path().join("readers");
        fs::create_dir(&cache).expect("cache root");
        fs::write(cache.join("keep.txt"), b"keep").expect("sentinel");
        fs::write(&source, b"%PDF-1.4\n%%EOF").expect("PDF");

        let result = prepare_special_document(
            1,
            "Book".to_owned(),
            String::new(),
            "PDF".to_owned(),
            &source,
            "../../unsafe-cache-identity",
            0.0,
            0,
            &cache,
        );

        assert!(matches!(
            result,
            Err(SpecialReaderError::InvalidCacheIdentity)
        ));
        assert_eq!(fs::read(cache.join("keep.txt")).expect("sentinel"), b"keep");
    }
}
