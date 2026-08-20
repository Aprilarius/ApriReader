use base64::{
    engine::general_purpose::{STANDARD, STANDARD_NO_PAD},
    Engine as _,
};
use quick_xml::{
    escape::unescape,
    events::{BytesRef, BytesText, Event},
    Reader,
};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{BufRead, BufReader, Read, Seek},
    path::{Component, Path},
};
use thiserror::Error;
use zip::ZipArchive;

const MAX_BOOK_SIZE: u64 = 2 * 1024 * 1024 * 1024;
const MAX_COMIC_BOOK_SIZE: u64 = 4 * 1024 * 1024 * 1024;
const MAX_XML_SIZE: u64 = 4 * 1024 * 1024;
const MAX_COVER_SIZE: u64 = 10 * 1024 * 1024;
const MAX_IN_MEMORY_FB2_SIZE: u64 = MAX_XML_SIZE * 8;
const MAX_COVER_BASE64_CHARS: usize = (MAX_COVER_SIZE as usize).div_ceil(3) * 4;
const MAX_XML_TAG_BYTES: usize = 16 * 1024;
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
    let cover_path = save_cover(&fingerprint, metadata.cover_bytes.as_deref(), cover_dir)?;
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
    let opf_path = parse_epub_container(&container)?;
    if !safe_archive_path(Path::new(&opf_path)) {
        return Err(ImportError::InvalidFormat);
    }
    let opf = read_zip_entry(&mut archive, &opf_path, MAX_XML_SIZE)?;
    let mut metadata = parse_epub_opf(&opf)?;
    if let Some(cover_href) = metadata.cover_extension.take() {
        let base = Path::new(&opf_path)
            .parent()
            .unwrap_or_else(|| Path::new(""));
        let Some(cover_href) = normalize_epub_href(&cover_href) else {
            return Ok(metadata);
        };
        let cover_archive_path = base.join(cover_href);
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

fn parse_epub_container(xml: &[u8]) -> Result<String, ImportError> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);
    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) | Ok(Event::Empty(event))
                if local_name(event.name().as_ref()) == "rootfile" =>
            {
                if let Some(path) = attributes(&event).get("full-path") {
                    if !path.is_empty() {
                        return Ok(path.clone());
                    }
                }
            }
            Ok(Event::Eof) => return Err(ImportError::InvalidFormat),
            Err(error) => return Err(ImportError::Xml(error)),
            _ => {}
        }
    }
}

fn parse_epub_opf(xml: &[u8]) -> Result<Metadata, ImportError> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);
    let mut metadata = Metadata::default();
    let mut stack = Vec::<String>::new();
    let mut cover_id = None;
    let mut cover_items = Vec::<(String, String, String)>::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) => {
                let name = local_name(event.name().as_ref());
                stack.push(name.clone());
                start_epub_metadata_element(&stack, &mut metadata);
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
            Ok(Event::Empty(event)) => {
                let name = local_name(event.name().as_ref());
                let attrs = attributes(&event);
                if name == "meta" && attrs.get("name").is_some_and(|value| value == "cover") {
                    cover_id = attrs.get("content").cloned();
                } else if name == "item" {
                    cover_items.push((
                        attrs.get("id").cloned().unwrap_or_default(),
                        attrs.get("href").cloned().unwrap_or_default(),
                        attrs.get("properties").cloned().unwrap_or_default(),
                    ));
                }
            }
            Ok(Event::Text(text)) => {
                let value = decode_xml_text(&text);
                record_epub_metadata_fragment(&stack, &value, &mut metadata);
            }
            Ok(Event::GeneralRef(reference)) => record_epub_metadata_fragment(
                &stack,
                &decode_xml_reference(&reference),
                &mut metadata,
            ),
            Ok(Event::End(_)) => {
                stack.pop();
            }
            Ok(Event::Eof) => break,
            Err(error) => return Err(ImportError::Xml(error)),
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
    Ok(metadata)
}

fn start_epub_metadata_element(stack: &[String], metadata: &mut Metadata) {
    if path_ends_with(stack, &["metadata", "title"]) && metadata.title.is_none() {
        metadata.title = Some(String::new());
    } else if path_ends_with(stack, &["metadata", "creator"]) && metadata.author.is_none() {
        metadata.author = Some(String::new());
    } else if path_ends_with(stack, &["metadata", "subject"]) {
        metadata.genres.push(String::new());
    }
}

fn record_epub_metadata_fragment(stack: &[String], value: &str, metadata: &mut Metadata) {
    if path_ends_with(stack, &["metadata", "title"]) {
        if let Some(title) = metadata.title.as_mut() {
            title.push_str(value);
        }
    } else if path_ends_with(stack, &["metadata", "creator"]) {
        if let Some(author) = metadata.author.as_mut() {
            author.push_str(value);
        }
    } else if path_ends_with(stack, &["metadata", "subject"]) {
        if let Some(genre) = metadata.genres.last_mut() {
            genre.push_str(value);
        }
    }
}

fn normalize_epub_href(value: &str) -> Option<String> {
    let value = value.split(['#', '?']).next()?.trim();
    if value.is_empty() || value.contains("://") {
        return None;
    }
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = *bytes.get(index + 1)?;
            let low = *bytes.get(index + 2)?;
            decoded.push(
                hex_value(high)?
                    .checked_mul(16)?
                    .checked_add(hex_value(low)?)?,
            );
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    let decoded = String::from_utf8(decoded).ok()?;
    (!decoded.contains('\0')).then_some(decoded)
}

fn hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn inspect_fb2(path: &Path) -> Result<Metadata, ImportError> {
    let file_size = fs::metadata(path)?.len();
    if !has_fb2_root(path)? {
        return Err(ImportError::InvalidFormat);
    }
    if file_size > MAX_IN_MEMORY_FB2_SIZE {
        return inspect_large_fb2(path);
    }
    let bytes = fs::read(path)?;
    let mut reader = Reader::from_reader(bytes.as_slice());
    reader.config_mut().trim_text(false);
    let mut metadata = Metadata::default();
    let mut stack = Vec::<String>::new();
    let mut author_parts = Vec::new();
    let mut cover_id = None;
    let mut binary_id = None;
    let mut binary_content_type = None;
    let mut binary_payload = Vec::<u8>::new();
    let mut binary_payload_too_large = false;
    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) => {
                let current = local_name(event.name().as_ref());
                let attrs = attributes(&event);
                stack.push(current.clone());
                start_fb2_metadata_element(&stack, &mut metadata, &mut author_parts);
                if current == "image"
                    && cover_id.is_none()
                    && path_ends_with(&stack, &["title-info", "coverpage", "image"])
                {
                    cover_id = attrs
                        .get("href")
                        .map(|value| value.trim_start_matches('#').to_owned());
                } else if current == "binary" {
                    binary_id = attrs.get("id").cloned();
                    binary_content_type = attrs.get("content-type").cloned();
                    binary_payload.clear();
                    binary_payload_too_large = false;
                }
            }
            Ok(Event::Empty(event)) => {
                let current = local_name(event.name().as_ref());
                let attrs = attributes(&event);
                if current == "image"
                    && cover_id.is_none()
                    && path_ends_with(&stack, &["title-info", "coverpage"])
                {
                    cover_id = attrs
                        .get("href")
                        .map(|value| value.trim_start_matches('#').to_owned());
                }
            }
            Ok(Event::Text(text)) => {
                if is_fb2_metadata_text_path(&stack) {
                    let value = decode_xml_text(&text);
                    record_fb2_metadata_fragment(&stack, &value, &mut metadata, &mut author_parts);
                }
                if binary_id.is_some() && binary_id == cover_id {
                    append_base64_chunk(
                        text.as_ref(),
                        &mut binary_payload,
                        &mut binary_payload_too_large,
                    );
                }
            }
            Ok(Event::GeneralRef(reference)) => record_fb2_metadata_fragment(
                &stack,
                &decode_xml_reference(&reference),
                &mut metadata,
                &mut author_parts,
            ),
            Ok(Event::CData(text)) if binary_id.is_some() && binary_id == cover_id => {
                append_base64_chunk(
                    text.as_ref(),
                    &mut binary_payload,
                    &mut binary_payload_too_large,
                );
            }
            Ok(Event::End(event)) => {
                let current = local_name(event.name().as_ref());
                if current == "author"
                    && path_ends_with(&stack, &["title-info", "author"])
                    && metadata.author.is_none()
                    && !author_parts.is_empty()
                {
                    metadata.author = Some(join_author_parts(&author_parts));
                    author_parts.clear();
                }
                if current == "binary" && binary_id.is_some() && binary_id == cover_id {
                    apply_fb2_cover(
                        &mut metadata,
                        &binary_payload,
                        binary_payload_too_large,
                        binary_content_type.as_deref(),
                    );
                }
                if current == "binary" {
                    binary_id = None;
                    binary_content_type = None;
                    binary_payload.clear();
                    binary_payload_too_large = false;
                }
                stack.pop();
            }
            Ok(Event::Eof) => break,
            Err(error) => return Err(ImportError::Xml(error)),
            _ => {}
        }
    }
    Ok(metadata)
}

fn inspect_large_fb2(path: &Path) -> Result<Metadata, ImportError> {
    let source = File::open(path)?.take(MAX_XML_SIZE + 1);
    let mut reader = Reader::from_reader(BufReader::new(source));
    reader.config_mut().trim_text(false);
    let mut buffer = Vec::new();
    let mut metadata = Metadata::default();
    let mut stack = Vec::<String>::new();
    let mut author_parts = Vec::new();
    let mut cover_id = None;
    let mut description_closed = false;
    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(event)) => {
                let current = local_name(event.name().as_ref());
                let attrs = attributes(&event);
                stack.push(current.clone());
                start_fb2_metadata_element(&stack, &mut metadata, &mut author_parts);
                if current == "image"
                    && cover_id.is_none()
                    && path_ends_with(&stack, &["title-info", "coverpage", "image"])
                {
                    cover_id = attrs
                        .get("href")
                        .map(|value| value.trim_start_matches('#').to_owned());
                }
            }
            Ok(Event::Empty(event)) => {
                let current = local_name(event.name().as_ref());
                let attrs = attributes(&event);
                if current == "image"
                    && cover_id.is_none()
                    && path_ends_with(&stack, &["title-info", "coverpage"])
                {
                    cover_id = attrs
                        .get("href")
                        .map(|value| value.trim_start_matches('#').to_owned());
                }
            }
            Ok(Event::Text(text)) if is_fb2_metadata_text_path(&stack) => {
                record_fb2_metadata_fragment(
                    &stack,
                    &decode_xml_text(&text),
                    &mut metadata,
                    &mut author_parts,
                )
            }
            Ok(Event::GeneralRef(reference)) => record_fb2_metadata_fragment(
                &stack,
                &decode_xml_reference(&reference),
                &mut metadata,
                &mut author_parts,
            ),
            Ok(Event::End(event)) => {
                let current = local_name(event.name().as_ref());
                if current == "author"
                    && path_ends_with(&stack, &["title-info", "author"])
                    && metadata.author.is_none()
                    && !author_parts.is_empty()
                {
                    metadata.author = Some(join_author_parts(&author_parts));
                    author_parts.clear();
                }
                if current == "description" {
                    description_closed = true;
                }
                stack.pop();
                if description_closed {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(error) => return Err(ImportError::Xml(error)),
            _ => {}
        }
        buffer.clear();
    }
    if !description_closed {
        return Ok(Metadata::default());
    }
    if let Some(cover_id) = cover_id {
        if let Some(cover) = extract_large_fb2_cover(path, &cover_id)? {
            metadata.cover_extension = image_extension(&cover).map(str::to_owned);
            metadata.cover_bytes = Some(cover);
        }
    }
    Ok(metadata)
}

fn has_fb2_root(path: &Path) -> Result<bool, ImportError> {
    let source = File::open(path)?.take(MAX_XML_TAG_BYTES as u64 * 4);
    let mut reader = Reader::from_reader(BufReader::new(source));
    let mut buffer = Vec::new();
    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(event)) | Ok(Event::Empty(event)) => {
                return Ok(local_name(event.name().as_ref()) == "fictionbook");
            }
            Ok(Event::Eof) => return Ok(false),
            Err(error) => return Err(ImportError::Xml(error)),
            _ => {}
        }
        buffer.clear();
    }
}

fn start_fb2_metadata_element(
    stack: &[String],
    metadata: &mut Metadata,
    author_parts: &mut Vec<String>,
) {
    if path_ends_with(stack, &["title-info", "book-title"]) && metadata.title.is_none() {
        metadata.title = Some(String::new());
    } else if (path_ends_with(stack, &["title-info", "author", "first-name"])
        || path_ends_with(stack, &["title-info", "author", "middle-name"])
        || path_ends_with(stack, &["title-info", "author", "last-name"]))
        && metadata.author.is_none()
    {
        author_parts.push(String::new());
    } else if path_ends_with(stack, &["title-info", "genre"]) {
        metadata.genres.push(String::new());
    }
}

fn record_fb2_metadata_fragment(
    stack: &[String],
    value: &str,
    metadata: &mut Metadata,
    author_parts: &mut [String],
) {
    if path_ends_with(stack, &["title-info", "book-title"]) {
        if let Some(title) = metadata.title.as_mut() {
            title.push_str(value);
        }
    } else if (path_ends_with(stack, &["title-info", "author", "first-name"])
        || path_ends_with(stack, &["title-info", "author", "middle-name"])
        || path_ends_with(stack, &["title-info", "author", "last-name"]))
        && metadata.author.is_none()
    {
        if let Some(part) = author_parts.last_mut() {
            part.push_str(value);
        }
    } else if path_ends_with(stack, &["title-info", "genre"]) {
        if let Some(genre) = metadata.genres.last_mut() {
            genre.push_str(value);
        }
    }
}

fn is_fb2_metadata_text_path(stack: &[String]) -> bool {
    path_ends_with(stack, &["title-info", "book-title"])
        || path_ends_with(stack, &["title-info", "author", "first-name"])
        || path_ends_with(stack, &["title-info", "author", "middle-name"])
        || path_ends_with(stack, &["title-info", "author", "last-name"])
        || path_ends_with(stack, &["title-info", "genre"])
}

fn join_author_parts(parts: &[String]) -> String {
    parts
        .iter()
        .filter(|part| !part.trim().is_empty())
        .map(|part| part.trim())
        .collect::<Vec<_>>()
        .join(" ")
}

fn path_ends_with(stack: &[String], suffix: &[&str]) -> bool {
    stack.len() >= suffix.len()
        && stack[stack.len() - suffix.len()..]
            .iter()
            .map(String::as_str)
            .eq(suffix.iter().copied())
}

fn append_base64_chunk(chunk: &[u8], target: &mut Vec<u8>, too_large: &mut bool) {
    if *too_large {
        return;
    }
    for byte in chunk
        .iter()
        .copied()
        .filter(|byte| !byte.is_ascii_whitespace())
    {
        if target.len() == MAX_COVER_BASE64_CHARS {
            target.clear();
            *too_large = true;
            return;
        }
        target.push(byte);
    }
}

fn apply_fb2_cover(
    metadata: &mut Metadata,
    payload: &[u8],
    too_large: bool,
    content_type: Option<&str>,
) {
    if too_large {
        return;
    }
    if let Some(decoded) = decode_fb2_base64(payload) {
        if decoded.len() as u64 <= MAX_COVER_SIZE {
            metadata.cover_extension = image_extension(&decoded)
                .or_else(|| content_type.and_then(mime_extension))
                .map(str::to_owned);
            metadata.cover_bytes = Some(decoded);
        }
    }
}

fn decode_fb2_base64(payload: &[u8]) -> Option<Vec<u8>> {
    STANDARD
        .decode(payload)
        .or_else(|_| STANDARD_NO_PAD.decode(payload))
        .ok()
}

fn extract_large_fb2_cover(path: &Path, cover_id: &str) -> Result<Option<Vec<u8>>, ImportError> {
    let mut reader = BufReader::new(File::open(path)?);
    let mut tag = Vec::new();
    while read_next_xml_tag(&mut reader, &mut tag)? {
        let mut tag_reader = Reader::from_reader(tag.as_slice());
        let Ok(Event::Start(event)) = tag_reader.read_event() else {
            continue;
        };
        if local_name(event.name().as_ref()) != "binary" {
            continue;
        }
        let attrs = attributes(&event);
        if attrs.get("id").map(String::as_str) != Some(cover_id) {
            continue;
        }
        let mut payload = Vec::new();
        let mut too_large = false;
        if !read_base64_until_tag(&mut reader, &mut tag, &mut payload, &mut too_large)? {
            return Ok(None);
        }
        if xml_end_tag_local_name(&tag).as_deref() != Some("binary") {
            return Ok(None);
        }
        if too_large {
            return Ok(None);
        }
        let Some(decoded) = decode_fb2_base64(&payload) else {
            return Ok(None);
        };
        return Ok((decoded.len() as u64 <= MAX_COVER_SIZE).then_some(decoded));
    }
    Ok(None)
}

fn xml_end_tag_local_name(tag: &[u8]) -> Option<String> {
    let name = tag
        .strip_prefix(b"</")?
        .split(|byte| byte.is_ascii_whitespace() || *byte == b'>')
        .next()?;
    (!name.is_empty()).then(|| local_name(name))
}

fn read_next_xml_tag<R: BufRead>(reader: &mut R, tag: &mut Vec<u8>) -> std::io::Result<bool> {
    tag.clear();
    loop {
        let buffer = reader.fill_buf()?;
        if buffer.is_empty() {
            return Ok(false);
        }
        if let Some(position) = buffer.iter().position(|byte| *byte == b'<') {
            reader.consume(position + 1);
            tag.push(b'<');
            break;
        }
        let consumed = buffer.len();
        reader.consume(consumed);
    }
    let mut quote = None;
    loop {
        let buffer = reader.fill_buf()?;
        if buffer.is_empty() {
            return Ok(false);
        }
        let mut completed = None;
        for (position, byte) in buffer.iter().copied().enumerate() {
            if tag.len() + position + 1 > MAX_XML_TAG_BYTES {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "XML tag exceeds the safety limit",
                ));
            }
            match (quote, byte) {
                (Some(active), current) if active == current => quote = None,
                (None, b'\'' | b'"') => quote = Some(byte),
                (None, b'>') => {
                    completed = Some(position);
                    break;
                }
                _ => {}
            }
        }
        if let Some(position) = completed {
            tag.extend_from_slice(&buffer[..=position]);
            reader.consume(position + 1);
            return Ok(true);
        }
        if tag.len() + buffer.len() > MAX_XML_TAG_BYTES {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "XML tag exceeds the safety limit",
            ));
        }
        let consumed = buffer.len();
        tag.extend_from_slice(buffer);
        reader.consume(consumed);
    }
}

fn read_base64_until_tag<R: BufRead>(
    reader: &mut R,
    tag: &mut Vec<u8>,
    payload: &mut Vec<u8>,
    too_large: &mut bool,
) -> std::io::Result<bool> {
    loop {
        let buffer = reader.fill_buf()?;
        if buffer.is_empty() {
            return Ok(false);
        }
        if let Some(position) = buffer.iter().position(|byte| *byte == b'<') {
            append_base64_chunk(&buffer[..position], payload, too_large);
            reader.consume(position);
            return read_next_xml_tag(reader, tag);
        }
        append_base64_chunk(buffer, payload, too_large);
        let consumed = buffer.len();
        reader.consume(consumed);
    }
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
    cover_dir: &Path,
) -> Result<Option<String>, std::io::Error> {
    let Some(bytes) =
        bytes.filter(|value| !value.is_empty() && value.len() as u64 <= MAX_COVER_SIZE)
    else {
        return Ok(None);
    };
    let Some(extension) = image_extension(bytes) else {
        return Ok(None);
    };
    fs::create_dir_all(cover_dir)?;
    let path = cover_dir.join(format!("{fingerprint}.{extension}"));
    let already_current = fs::read(&path).is_ok_and(|current| current == bytes);
    if !already_current {
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
    match value
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "image/jpeg" | "image/jpg" => Some("jpg"),
        "image/png" => Some("png"),
        "image/gif" => Some("gif"),
        "image/webp" => Some("webp"),
        _ => None,
    }
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
            let raw = String::from_utf8_lossy(attribute.value.as_ref());
            let value = unescape(&raw)
                .map(|value| value.into_owned())
                .unwrap_or_else(|_| raw.into_owned());
            (local_name(attribute.key.as_ref()), value)
        })
        .collect()
}

fn decode_xml_text(text: &BytesText<'_>) -> String {
    let Ok(decoded) = text.decode() else {
        return String::new();
    };
    unescape(&decoded)
        .map(|value| value.into_owned())
        .unwrap_or_else(|_| decoded.into_owned())
}

fn decode_xml_reference(reference: &BytesRef<'_>) -> String {
    if let Ok(Some(character)) = reference.resolve_char_ref() {
        return character.to_string();
    }
    let name = reference.decode().unwrap_or_default();
    match name.as_ref() {
        "amp" => "&".to_owned(),
        "apos" => "'".to_owned(),
        "gt" => ">".to_owned(),
        "lt" => "<".to_owned(),
        "quot" => "\"".to_owned(),
        _ => format!("&{name};"),
    }
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
                br#"<?xml version="1.0"?><container><!-- full-path="wrong.opf" --><rootfiles><rootfile full-path = "OPS/book.opf"/></rootfiles></container>"#,
            )
            .expect("container content");
        archive.start_file("OPS/book.opf", options).expect("opf");
        archive
            .write_all(
                br#"<?xml version="1.0"?><package><metadata><dc:title>Fixture &amp; Book</dc:title><dc:creator>Test &amp; Author</dc:creator><dc:subject>Science Fiction</dc:subject><dc:subject>Adventure</dc:subject><meta name="cover" content="cover"/></metadata><manifest><item id="cover" href="cover%20art.png#image" media-type="image/png"/></manifest></package>"#,
            )
            .expect("opf content");
        archive
            .start_file("OPS/cover art.png", options)
            .expect("cover");
        archive
            .write_all(b"\x89PNG\r\n\x1a\nsynthetic")
            .expect("cover content");
        archive.finish().expect("finish");

        let book = inspect_book(&path, &directory.path().join("covers")).expect("inspect");
        assert_eq!(book.title, "Fixture & Book");
        assert_eq!(book.author, "Test & Author");
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

    #[test]
    fn accepts_prefixed_fb2_root_and_decodes_metadata_entities() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("prefixed.fb2");
        fs::write(
            &path,
            r#"<?xml version="1.0" encoding="utf-8"?>
               <fb:FictionBook xmlns:fb="urn:fb2"><fb:description><fb:title-info>
                 <fb:author><fb:first-name>Tom &amp;</fb:first-name><fb:last-name>Jerry</fb:last-name></fb:author>
                 <fb:book-title>Cats &amp; Mice</fb:book-title>
               </fb:title-info></fb:description><fb:body><fb:section><fb:p>Text.</fb:p></fb:section></fb:body>
               </fb:FictionBook>"#,
        )
        .expect("fixture");
        let book = inspect_book(&path, &directory.path().join("covers")).expect("inspect");
        assert_eq!(book.title, "Cats & Mice");
        assert_eq!(book.author, "Tom & Jerry");
    }

    #[test]
    fn extracts_fb2_cover_with_base64_whitespace() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("wrapped-cover.fb2");
        let cover = b"\x89PNG\r\n\x1a\nsynthetic-cover";
        let encoded = STANDARD.encode(cover);
        let wrapped = encoded
            .as_bytes()
            .chunks(5)
            .map(|chunk| String::from_utf8_lossy(chunk))
            .collect::<Vec<_>>()
            .join(" \r\n\t");
        fs::write(
            &path,
            format!(
                r##"<?xml version="1.0" encoding="utf-8"?>
                <FictionBook xmlns:l="http://www.w3.org/1999/xlink">
                  <description><title-info>
                    <book-title>Wrapped cover</book-title>
                    <coverpage><image l:href="#cover.png"/></coverpage>
                  </title-info></description>
                  <body><section><p>Text.</p></section></body>
                  <binary id="cover.png" content-type="image/png">{wrapped}</binary>
                </FictionBook>"##
            ),
        )
        .expect("fixture");
        let covers = directory.path().join("covers");
        let book = inspect_book(&path, &covers).expect("inspect");
        let cover_path = book.cover_path.expect("embedded cover");
        assert_eq!(fs::read(cover_path).expect("saved cover"), cover);
    }

    #[test]
    fn ignores_body_image_when_fb2_has_no_coverpage() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("body-image.fb2");
        let encoded = STANDARD.encode(b"\xff\xd8\xffsynthetic-body-image");
        fs::write(
            &path,
            format!(
                r##"<?xml version="1.0" encoding="utf-8"?>
                <FictionBook xmlns:l="http://www.w3.org/1999/xlink">
                  <description><title-info><book-title>No cover</book-title></title-info></description>
                  <body><section><image l:href="#illustration.jpg"/></section></body>
                  <binary id="illustration.jpg" content-type="image/jpeg">{encoded}</binary>
                </FictionBook>"##
            ),
        )
        .expect("fixture");
        let book = inspect_book(&path, &directory.path().join("covers")).expect("inspect");
        assert!(book.cover_path.is_none());
    }

    #[test]
    fn rejects_embedded_cover_when_mime_claim_does_not_match_image_bytes() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("fake-cover.fb2");
        let encoded = STANDARD.encode(b"plain text posing as an image");
        fs::write(
            &path,
            format!(
                r##"<FictionBook xmlns:l="urn:xlink"><description><title-info>
                <book-title>Safe cover</book-title><coverpage><image l:href="#cover.jpg"/></coverpage>
                </title-info></description><body/><binary id="cover.jpg" content-type="image/jpeg">{encoded}</binary></FictionBook>"##
            ),
        )
        .expect("fixture");
        let book = inspect_book(&path, &directory.path().join("covers")).expect("inspect");
        assert!(book.cover_path.is_none());
    }

    #[test]
    fn reads_streamed_xml_tag_with_angle_bracket_inside_attribute() {
        let input = br#"ignored<binary id="cover>art.jpg" content-type="image/jpeg">payload"#;
        let mut reader = BufReader::new(input.as_slice());
        let mut tag = Vec::new();
        assert!(read_next_xml_tag(&mut reader, &mut tag).expect("tag"));
        assert_eq!(
            tag,
            br#"<binary id="cover>art.jpg" content-type="image/jpeg">"#
        );
    }

    #[test]
    fn ignores_document_info_author_when_title_info_has_no_author() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("document-author.fb2");
        fs::write(
            &path,
            r#"<?xml version="1.0" encoding="utf-8"?>
               <FictionBook><description>
                 <title-info><book-title>Anonymous work</book-title></title-info>
                 <document-info><author><first-name>File</first-name><last-name>Creator</last-name></author></document-info>
               </description><body><section><p>Text.</p></section></body></FictionBook>"#,
        )
        .expect("fixture");
        let book = inspect_book(&path, &directory.path().join("covers")).expect("inspect");
        assert_eq!(book.title, "Anonymous work");
        assert!(book.author.is_empty());
    }

    #[test]
    fn streams_metadata_beyond_the_legacy_fb2_size_boundary() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("large.fb2");
        let cover = b"\xff\xd8\xffsynthetic-large-cover";
        let encoded = STANDARD.encode(cover);
        let wrapped = encoded
            .as_bytes()
            .chunks(6)
            .map(|chunk| String::from_utf8_lossy(chunk))
            .collect::<Vec<_>>()
            .join(" \n");
        let mut file = File::create(&path).expect("fixture");
        file.write_all(
            br##"<?xml version="1.0" encoding="utf-8"?><FictionBook xmlns:l="http://www.w3.org/1999/xlink"><description><title-info><genre>fiction</genre><author><first-name>Large</first-name><last-name>Author</last-name></author><book-title>Large metadata fixture</book-title><coverpage><image l:href="#cover.jpg"/></coverpage></title-info></description><body><section><p>"##,
        )
        .expect("prefix");
        let chunk = [b'x'; 64 * 1024];
        while file.metadata().expect("metadata").len() <= MAX_IN_MEMORY_FB2_SIZE {
            file.write_all(&chunk).expect("body");
        }
        file.write_all(
            format!(
                r#"</p></section></body><binary id="cover.jpg" content-type="image/jpeg">{wrapped}</binary></FictionBook>"#
            )
            .as_bytes(),
        )
            .expect("suffix");
        drop(file);
        let covers = directory.path().join("covers");
        let book = inspect_book(&path, &covers).expect("inspect");
        assert_eq!(book.title, "Large metadata fixture");
        assert_eq!(book.author, "Large Author");
        assert_eq!(book.genres, "fiction");
        assert_eq!(
            fs::read(book.cover_path.expect("embedded cover")).expect("saved cover"),
            cover
        );
    }
}
