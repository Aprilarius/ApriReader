use quick_xml::{
    escape::unescape,
    events::{BytesRef, Event},
    Reader,
};
use serde::Serialize;
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{BufReader, Read},
    path::{Component, Path},
};
use thiserror::Error;
use zip::ZipArchive;

const MAX_TEXT_FILE_SIZE: u64 = 32 * 1024 * 1024;
const MAX_ARCHIVE_ENTRY_SIZE: u64 = 8 * 1024 * 1024;
const MAX_DOCUMENT_TEXT: usize = 64 * 1024 * 1024;
const MAX_RENDER_SECTION_TEXT: usize = 160 * 1024;

#[derive(Debug, Error)]
pub enum ReaderError {
    #[error("this format is not available in the reflow reader")]
    Unsupported,
    #[error("the source book is unavailable")]
    Missing,
    #[error("the document is too large for safe text rendering")]
    TooLarge,
    #[error("the document is damaged or cannot be read: {0}")]
    Io(#[from] std::io::Error),
    #[error("the archive is invalid: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("the document markup is invalid: {0}")]
    Xml(#[from] quick_xml::Error),
    #[error("the document does not contain readable text")]
    Empty,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentModel {
    pub book_id: i64,
    pub title: String,
    pub author: String,
    pub format: String,
    pub sections: Vec<DocumentSection>,
    pub progress: f64,
    pub last_section: usize,
    pub section_progress: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSection {
    pub id: String,
    pub title: String,
    pub blocks: Vec<DocumentBlock>,
    /// Archive-relative path this section was produced from, so a link written
    /// as `notes.xhtml#id45` can be resolved back to a section. Empty for
    /// single-file formats.
    #[serde(default)]
    pub source: String,
    /// Every `id` attribute seen in the source file. Fragment-only links and
    /// books that collect all notes in one file resolve through this.
    #[serde(default)]
    pub anchors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentBlock {
    pub kind: BlockKind,
    pub text: String,
    /// Ranges of `text` that were hyperlinks in the source markup.
    ///
    /// Deliberately stored beside the text rather than by turning the text
    /// into a tree of inline nodes: annotations and speech highlighting both
    /// address this block by offsets into `text`, and any change to the string
    /// would silently move every stored highlight in the user's library.
    #[serde(default)]
    pub links: Vec<InlineLink>,
}

/// `start` and `end` are offsets in UTF-16 code units, matching the indices the
/// WebView uses for `String.prototype.slice`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InlineLink {
    pub start: usize,
    pub end: usize,
    pub href: String,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BlockKind {
    Heading,
    Paragraph,
    Quote,
    ListItem,
    Code,
    Divider,
}

pub fn read_document(path: &Path) -> Result<Vec<DocumentSection>, ReaderError> {
    if !path.is_file() {
        return Err(ReaderError::Missing);
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let sections = match extension.as_str() {
        "txt" => read_txt(path)?,
        "md" | "markdown" => read_markdown(path)?,
        "html" | "htm" => read_html_file(path)?,
        "epub" => read_epub(path)?,
        "fb2" => read_fb2(path)?,
        "docx" => read_docx(path)?,
        _ => return Err(ReaderError::Unsupported),
    };
    let text_size = sections
        .iter()
        .flat_map(|section| &section.blocks)
        .map(|block| block.text.len())
        .sum::<usize>();
    if text_size == 0 {
        return Err(ReaderError::Empty);
    }
    if text_size > MAX_DOCUMENT_TEXT {
        return Err(ReaderError::TooLarge);
    }
    Ok(split_sections_for_rendering(sections))
}

/// Keeps a malformed or unusually-exported book from putting an entire novel
/// into one WebView layout pass. In particular, some FB2 producers emit one
/// multi-megabyte `<p>` element. The reader renders one section at a time, so
/// bounded sections keep navigation and page measurement responsive on phones.
fn split_sections_for_rendering(sections: Vec<DocumentSection>) -> Vec<DocumentSection> {
    let mut result = Vec::new();
    for section in sections {
        let mut chunks = Vec::<Vec<DocumentBlock>>::new();
        let mut current = Vec::new();
        let mut current_text_len: usize = 0;
        for block in section.blocks {
            for fragment in split_block_for_rendering(block) {
                let fragment_len = fragment.text.len();
                if !current.is_empty()
                    && current_text_len.saturating_add(fragment_len) > MAX_RENDER_SECTION_TEXT
                {
                    chunks.push(current);
                    current = Vec::new();
                    current_text_len = 0;
                }
                current_text_len = current_text_len.saturating_add(fragment_len);
                current.push(fragment);
            }
        }
        if !current.is_empty() {
            chunks.push(current);
        }
        let multiple_chunks = chunks.len() > 1;
        for (index, blocks) in chunks.into_iter().enumerate() {
            result.push(DocumentSection {
                // Splitting is an internal rendering concern; every part still
                // comes from the same source file, and link resolution has to
                // keep finding it.
                source: section.source.clone(),
                anchors: if index == 0 {
                    section.anchors.clone()
                } else {
                    Vec::new()
                },
                id: if index == 0 {
                    section.id.clone()
                } else {
                    format!("{}-part-{}", section.id, index + 1)
                },
                title: if multiple_chunks {
                    format!("{} — {}", section.title, index + 1)
                } else {
                    section.title.clone()
                },
                blocks,
            });
        }
    }
    result
}

fn split_block_for_rendering(block: DocumentBlock) -> Vec<DocumentBlock> {
    if block.text.len() <= MAX_RENDER_SECTION_TEXT {
        return vec![block];
    }
    let mut fragments = Vec::new();
    let mut start = 0;
    while start < block.text.len() {
        let mut end = (start + MAX_RENDER_SECTION_TEXT).min(block.text.len());
        while end > start && !block.text.is_char_boundary(end) {
            end -= 1;
        }
        if end < block.text.len() {
            if let Some(offset) = block.text[start..end].rfind(char::is_whitespace) {
                let candidate = start + offset;
                if candidate > start + MAX_RENDER_SECTION_TEXT / 2 {
                    end = candidate;
                }
            }
        }
        if end <= start {
            break;
        }
        let text = block.text[start..end].trim().to_owned();
        if !text.is_empty() {
            fragments.push(DocumentBlock {
                // Offsets are relative to the original string, so they cannot
                // survive the split. Only blocks above 160 KB reach this path,
                // and dropping the links there is safer than shifting them onto
                // unrelated words.
                links: Vec::new(),
                kind: block.kind,
                text,
            });
        }
        start = end;
        while start < block.text.len()
            && block.text[start..]
                .chars()
                .next()
                .is_some_and(char::is_whitespace)
        {
            start += block.text[start..].chars().next().unwrap().len_utf8();
        }
    }
    if fragments.is_empty() {
        vec![block]
    } else {
        fragments
    }
}

fn read_docx(path: &Path) -> Result<Vec<DocumentSection>, ReaderError> {
    let mut archive = ZipArchive::new(File::open(path)?)?;
    let document_xml = read_zip_entry(
        &mut archive,
        "word/document.xml",
        MAX_ARCHIVE_ENTRY_SIZE * 4,
    )?;
    let mut reader = Reader::from_reader(document_xml.as_slice());
    reader.config_mut().trim_text(false);
    let mut sections = Vec::new();
    let mut section = DocumentSection {
        source: String::new(),
        anchors: Vec::new(),
        id: "section-1".to_owned(),
        title: file_title(path),
        blocks: Vec::new(),
    };
    let mut in_paragraph = false;
    let mut in_text = false;
    let mut paragraph = String::new();
    let mut paragraph_style = String::new();
    let mut is_list = false;
    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) => match local_name(event.name().as_ref()).as_str() {
                "p" => {
                    in_paragraph = true;
                    paragraph.clear();
                    paragraph_style.clear();
                    is_list = false;
                }
                "t" if in_paragraph => in_text = true,
                "pstyle" if in_paragraph => {
                    paragraph_style = xml_attributes(&event)
                        .get("val")
                        .cloned()
                        .unwrap_or_default()
                        .to_ascii_lowercase();
                }
                "numpr" if in_paragraph => is_list = true,
                "tab" if in_paragraph => paragraph.push(' '),
                "br" if in_paragraph => paragraph.push('\n'),
                _ => {}
            },
            Ok(Event::Empty(event)) => match local_name(event.name().as_ref()).as_str() {
                "pstyle" if in_paragraph => {
                    paragraph_style = xml_attributes(&event)
                        .get("val")
                        .cloned()
                        .unwrap_or_default()
                        .to_ascii_lowercase();
                }
                "numpr" if in_paragraph => is_list = true,
                "tab" if in_paragraph => paragraph.push(' '),
                "br" if in_paragraph => paragraph.push('\n'),
                _ => {}
            },
            Ok(Event::Text(text)) if in_text => {
                paragraph.push_str(&text.decode().unwrap_or_default());
            }
            Ok(Event::GeneralRef(reference)) if in_text => {
                paragraph.push_str(&decode_xml_reference(&reference));
            }
            Ok(Event::End(event)) => match local_name(event.name().as_ref()).as_str() {
                "t" => in_text = false,
                "p" => {
                    in_paragraph = false;
                    let text = normalize_space(&paragraph);
                    if text.is_empty() {
                        continue;
                    }
                    if is_heading_one(&paragraph_style) {
                        if !section.blocks.is_empty() {
                            sections.push(section);
                            section = DocumentSection {
                                source: String::new(),
                                anchors: Vec::new(),
                                id: format!("section-{}", sections.len() + 1),
                                title: text,
                                blocks: Vec::new(),
                            };
                        } else {
                            section.title = text;
                        }
                    } else {
                        let kind = if paragraph_style.contains("heading")
                            || paragraph_style.contains("title")
                        {
                            BlockKind::Heading
                        } else if paragraph_style.contains("quote") {
                            BlockKind::Quote
                        } else if is_list || paragraph_style.contains("list") {
                            BlockKind::ListItem
                        } else {
                            BlockKind::Paragraph
                        };
                        section.blocks.push(DocumentBlock {
                            links: Vec::new(),
                            kind,
                            text,
                        });
                    }
                }
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(error) => return Err(error.into()),
            _ => {}
        }
    }
    if !section.blocks.is_empty() {
        sections.push(section);
    }
    Ok(sections)
}

fn is_heading_one(style: &str) -> bool {
    matches!(
        style,
        "heading1" | "heading 1" | "заголовок1" | "заголовок 1"
    )
}

fn read_text_bytes(path: &Path) -> Result<Vec<u8>, ReaderError> {
    if fs::metadata(path)?.len() > MAX_TEXT_FILE_SIZE {
        return Err(ReaderError::TooLarge);
    }
    let mut bytes = Vec::new();
    File::open(path)?
        .take(MAX_TEXT_FILE_SIZE + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_TEXT_FILE_SIZE {
        return Err(ReaderError::TooLarge);
    }
    Ok(bytes)
}

fn read_txt(path: &Path) -> Result<Vec<DocumentSection>, ReaderError> {
    let text = String::from_utf8_lossy(&read_text_bytes(path)?).replace("\r\n", "\n");
    let blocks = paragraphs(&text)
        .into_iter()
        .map(|text| DocumentBlock {
            links: Vec::new(),
            kind: BlockKind::Paragraph,
            text,
        })
        .collect();
    Ok(vec![DocumentSection {
        source: String::new(),
        anchors: Vec::new(),
        id: "text".to_owned(),
        title: file_title(path),
        blocks,
    }])
}

fn read_markdown(path: &Path) -> Result<Vec<DocumentSection>, ReaderError> {
    let text = String::from_utf8_lossy(&read_text_bytes(path)?).replace("\r\n", "\n");
    let mut sections = Vec::new();
    let mut current = DocumentSection {
        source: String::new(),
        anchors: Vec::new(),
        id: "section-1".to_owned(),
        title: file_title(path),
        blocks: Vec::new(),
    };
    let mut paragraph = Vec::new();
    let mut in_code = false;
    let mut code = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            flush_paragraph(&mut current.blocks, &mut paragraph);
            if in_code {
                current.blocks.push(DocumentBlock {
                    links: Vec::new(),
                    kind: BlockKind::Code,
                    text: code.join("\n"),
                });
                code.clear();
            }
            in_code = !in_code;
        } else if in_code {
            code.push(line.to_owned());
        } else if let Some(heading) = trimmed.strip_prefix("# ") {
            flush_paragraph(&mut current.blocks, &mut paragraph);
            if !current.blocks.is_empty() {
                sections.push(current);
                current = DocumentSection {
                    source: String::new(),
                    anchors: Vec::new(),
                    id: format!("section-{}", sections.len() + 1),
                    title: clean_inline_markdown(heading),
                    blocks: Vec::new(),
                };
            } else {
                current.title = clean_inline_markdown(heading);
            }
        } else if let Some(heading) = trimmed
            .strip_prefix("## ")
            .or_else(|| trimmed.strip_prefix("### "))
        {
            flush_paragraph(&mut current.blocks, &mut paragraph);
            current.blocks.push(DocumentBlock {
                links: Vec::new(),
                kind: BlockKind::Heading,
                text: clean_inline_markdown(heading),
            });
        } else if let Some(quote) = trimmed.strip_prefix('>') {
            flush_paragraph(&mut current.blocks, &mut paragraph);
            current.blocks.push(DocumentBlock {
                links: Vec::new(),
                kind: BlockKind::Quote,
                text: clean_inline_markdown(quote.trim()),
            });
        } else if let Some(item) = list_item(trimmed) {
            flush_paragraph(&mut current.blocks, &mut paragraph);
            current.blocks.push(DocumentBlock {
                links: Vec::new(),
                kind: BlockKind::ListItem,
                text: clean_inline_markdown(item),
            });
        } else if trimmed == "---" || trimmed == "***" {
            flush_paragraph(&mut current.blocks, &mut paragraph);
            current.blocks.push(DocumentBlock {
                links: Vec::new(),
                kind: BlockKind::Divider,
                text: String::new(),
            });
        } else if trimmed.is_empty() {
            flush_paragraph(&mut current.blocks, &mut paragraph);
        } else {
            paragraph.push(trimmed.to_owned());
        }
    }
    flush_paragraph(&mut current.blocks, &mut paragraph);
    if !code.is_empty() {
        current.blocks.push(DocumentBlock {
            links: Vec::new(),
            kind: BlockKind::Code,
            text: code.join("\n"),
        });
    }
    if !current.blocks.is_empty() {
        sections.push(current);
    }
    Ok(sections)
}

fn read_html_file(path: &Path) -> Result<Vec<DocumentSection>, ReaderError> {
    let bytes = read_text_bytes(path)?;
    Ok(vec![html_section(&bytes, "html", &file_title(path), "")])
}

fn read_epub(path: &Path) -> Result<Vec<DocumentSection>, ReaderError> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    let container = read_zip_entry(
        &mut archive,
        "META-INF/container.xml",
        MAX_ARCHIVE_ENTRY_SIZE,
    )?;
    let opf_path = xml_attribute(&container, "rootfile", "full-path")
        .and_then(|path| normalize_epub_href(&path))
        .ok_or(ReaderError::Empty)?;
    if !safe_archive_path(Path::new(&opf_path)) {
        return Err(ReaderError::Empty);
    }
    let opf = read_zip_entry(&mut archive, &opf_path, MAX_ARCHIVE_ENTRY_SIZE)?;
    let (manifest, spine) = epub_package(&opf)?;
    let base = Path::new(&opf_path)
        .parent()
        .unwrap_or_else(|| Path::new(""));
    let mut sections = Vec::new();
    for idref in spine {
        let Some(href) = manifest.get(&idref) else {
            continue;
        };
        let Some(href) = normalize_epub_href(href) else {
            continue;
        };
        let entry_path = base.join(href);
        if !safe_archive_path(&entry_path) {
            continue;
        }
        let entry_name = entry_path.to_string_lossy().replace('\\', "/");
        let bytes = read_zip_entry(&mut archive, &entry_name, MAX_ARCHIVE_ENTRY_SIZE)?;
        let fallback = format!("Section {}", sections.len() + 1);
        sections.push(html_section(
            &bytes,
            &format!("section-{}", sections.len() + 1),
            &fallback,
            &entry_name,
        ));
    }
    Ok(sections
        .into_iter()
        .filter(|section| !section.blocks.is_empty())
        .collect())
}

fn read_fb2(path: &Path) -> Result<Vec<DocumentSection>, ReaderError> {
    let file = File::open(path)?;
    let mut reader = Reader::from_reader(BufReader::new(file));
    reader.config_mut().trim_text(false);
    let mut event_buffer = Vec::with_capacity(16 * 1024);
    let mut sections = Vec::new();
    let mut current = DocumentSection {
        source: String::new(),
        anchors: Vec::new(),
        id: "section-1".to_owned(),
        title: file_title(path),
        blocks: Vec::new(),
    };
    let mut stack = Vec::<String>::new();
    let mut title_parts = Vec::new();
    let mut text_buffer = None::<String>;
    loop {
        match reader.read_event_into(&mut event_buffer)? {
            Event::Start(event) => {
                let name = local_name(event.name().as_ref());
                if name == "section" && !current.blocks.is_empty() {
                    sections.push(current);
                    current = DocumentSection {
                        source: String::new(),
                        anchors: Vec::new(),
                        id: format!("section-{}", sections.len() + 1),
                        title: format!("Section {}", sections.len() + 1),
                        blocks: Vec::new(),
                    };
                }
                if name == "p" || name == "subtitle" {
                    text_buffer = Some(String::new());
                }
                stack.push(name);
            }
            Event::Text(text) => {
                if let Some(buffer) = text_buffer.as_mut() {
                    buffer.push_str(&text.decode().unwrap_or_default());
                    if buffer.len() > MAX_DOCUMENT_TEXT {
                        return Err(ReaderError::TooLarge);
                    }
                }
            }
            Event::GeneralRef(reference) => {
                if let Some(buffer) = text_buffer.as_mut() {
                    buffer.push_str(&decode_xml_reference(&reference));
                    if buffer.len() > MAX_DOCUMENT_TEXT {
                        return Err(ReaderError::TooLarge);
                    }
                }
            }
            Event::CData(text) => {
                if let Some(buffer) = text_buffer.as_mut() {
                    buffer.push_str(&text.decode().unwrap_or_default());
                    if buffer.len() > MAX_DOCUMENT_TEXT {
                        return Err(ReaderError::TooLarge);
                    }
                }
            }
            Event::End(event) => {
                let name = local_name(event.name().as_ref());
                if name == "p" || name == "subtitle" {
                    let value = normalize_space(text_buffer.take().as_deref().unwrap_or_default());
                    if !value.is_empty() {
                        if name == "p" && stack.iter().any(|item| item == "title") {
                            title_parts.push(value);
                        } else {
                            current.blocks.push(DocumentBlock {
                                links: Vec::new(),
                                kind: if name == "subtitle" {
                                    BlockKind::Heading
                                } else if stack.iter().any(|item| item == "cite") {
                                    BlockKind::Quote
                                } else {
                                    BlockKind::Paragraph
                                },
                                text: value,
                            });
                        }
                    }
                }
                if name == "title" && !title_parts.is_empty() {
                    current.title = title_parts.join(" ");
                    title_parts.clear();
                }
                stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
        event_buffer.clear();
    }
    if !current.blocks.is_empty() {
        sections.push(current);
    }
    Ok(sections)
}

/// `hr` also starts with `h`, so heading tags have to be matched exactly.
fn is_heading_tag(tag: &str) -> bool {
    matches!(tag, "h1" | "h2" | "h3" | "h4" | "h5" | "h6")
}

/// Reports whether an opening tag carries a class that marks a chapter opener,
/// e.g. `class="title6"` or `class="subtitle"`.
fn has_title_class(tag_source: &str) -> bool {
    let lowered = tag_source.to_ascii_lowercase();
    let Some(start) = lowered.find("class=") else {
        return false;
    };
    let rest = &lowered[start + "class=".len()..];
    let quote = rest.chars().next().filter(|c| *c == '"' || *c == '\'');
    let value = match quote {
        Some(mark) => rest[1..].split(mark).next().unwrap_or_default(),
        None => rest
            .split(|c: char| c.is_whitespace() || c == '>')
            .next()
            .unwrap_or_default(),
    };
    // A plain `contains` would also match words like "untitled", so the class
    // name has to begin with, or be suffixed onto, "title"/"subtitle".
    value.split_whitespace().any(|name| {
        ["title", "subtitle"].iter().any(|marker| {
            name.starts_with(marker)
                || name.ends_with(&format!("-{marker}"))
                || name.ends_with(&format!("_{marker}"))
        })
    })
}

/// Reads one attribute out of the text between `<` and `>`.
fn tag_attribute(tag_source: &str, name: &str) -> Option<String> {
    let lowered = tag_source.to_ascii_lowercase();
    let mut search = 0;
    loop {
        let found = lowered[search..].find(name)? + search;
        let before_ok = found == 0
            || lowered[..found]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_whitespace());
        let rest = &tag_source[found + name.len()..];
        let trimmed = rest.trim_start();
        if before_ok && trimmed.starts_with('=') {
            let value = trimmed[1..].trim_start();
            let quote = value.chars().next().filter(|c| *c == '"' || *c == '\'');
            return Some(match quote {
                Some(mark) => value[1..].split(mark).next().unwrap_or_default().to_owned(),
                None => value
                    .split(|c: char| c.is_whitespace() || c == '>' || c == '/')
                    .next()
                    .unwrap_or_default()
                    .to_owned(),
            });
        }
        search = found + name.len();
        if search >= lowered.len() {
            return None;
        }
    }
}

/// A hyperlink captured while scanning a block: its text, how much raw text
/// preceded it in the buffer, and its destination.
type CapturedLink = (String, usize, String);

/// Rounds an index down to the nearest character boundary of `text`.
fn floor_char_boundary(text: &str, index: usize) -> usize {
    let mut index = index.min(text.len());
    while index > 0 && !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

/// Maps each captured hyperlink onto a range of the finished block text.
///
/// The link text is matched inside the already normalised string instead of
/// being tracked through normalisation, which keeps `text` byte-for-byte what
/// it was before links existed.
///
/// Matching starts from where the anchor opened rather than from the first
/// occurrence: a paragraph can mention `[1]` as ordinary text before the real
/// reference, and anchoring on that would make the wrong characters clickable.
/// Whitespace collapsing shifts the estimate slightly, so the search begins a
/// couple of bytes early and falls back to a forward scan.
fn resolve_inline_links(text: &str, raw_links: &[CapturedLink]) -> Vec<InlineLink> {
    let mut links = Vec::new();
    let mut cursor = 0;
    for (raw, raw_prefix_len, href) in raw_links {
        let needle = decode_entities(&normalize_space(raw));
        if needle.is_empty() || cursor >= text.len() {
            continue;
        }
        let hint = floor_char_boundary(text, (*raw_prefix_len).saturating_sub(2)).max(cursor);
        let found = text[hint..]
            .find(needle.as_str())
            .map(|offset| hint + offset)
            .or_else(|| text[cursor..].find(needle.as_str()).map(|o| cursor + o));
        let Some(byte_start) = found else {
            continue;
        };
        let start = text[..byte_start].encode_utf16().count();
        links.push(InlineLink {
            start,
            end: start + needle.encode_utf16().count(),
            href: href.clone(),
        });
        cursor = byte_start + needle.len();
    }
    links
}

fn html_section(bytes: &[u8], id: &str, fallback_title: &str, source: &str) -> DocumentSection {
    let markup = String::from_utf8_lossy(bytes);
    let mut blocks = Vec::new();
    let mut buffer = String::new();
    let mut title = None;
    let mut hidden_depth = 0_u32;
    let mut div_depth = 0_u32;
    let mut title_div: Option<u32> = None;
    let mut paragraph_is_title = false;
    let mut anchors: Vec<String> = Vec::new();
    let mut open_link: Option<(usize, String)> = None;
    let mut buffer_links: Vec<CapturedLink> = Vec::new();
    let mut index = 0;
    while index < markup.len() {
        let rest = &markup[index..];
        if rest.starts_with('<') {
            if let Some(end) = rest.find('>') {
                let tag_source = &rest[1..end];
                let closing = tag_source.trim_start().starts_with('/');
                let tag = tag_source
                    .trim_start_matches(|character: char| {
                        character == '/' || character == '!' || character == '?'
                    })
                    .split(|character: char| character.is_whitespace() || character == '/')
                    .next()
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                if hidden_depth == 0 && !closing {
                    if let Some(value) = tag_attribute(tag_source, "id") {
                        if !value.is_empty() {
                            anchors.push(value);
                        }
                    }
                }
                if hidden_depth == 0 && tag == "a" {
                    if closing {
                        if let Some((offset, href)) = open_link.take() {
                            if buffer.len() > offset {
                                let prefix_len = normalize_space(&buffer[..offset]).len();
                                buffer_links.push((buffer[offset..].to_owned(), prefix_len, href));
                            }
                        }
                    } else if !tag_source.trim_end().ends_with('/') {
                        open_link = tag_attribute(tag_source, "href")
                            .filter(|href| !href.is_empty())
                            .map(|href| (buffer.len(), href));
                    }
                }
                if ["script", "style", "iframe", "object", "svg"].contains(&tag.as_str()) {
                    if closing {
                        hidden_depth = hidden_depth.saturating_sub(1);
                    } else if !tag_source.trim_end().ends_with('/') {
                        hidden_depth = hidden_depth.saturating_add(1);
                    }
                } else if hidden_depth == 0
                    && [
                        "p",
                        "div",
                        "li",
                        "blockquote",
                        "h1",
                        "h2",
                        "h3",
                        "h4",
                        "h5",
                        "h6",
                        "br",
                        "hr",
                    ]
                    .contains(&tag.as_str())
                {
                    if !buffer.trim().is_empty() {
                        let text = decode_entities(&normalize_space(&buffer));
                        let kind = if tag == "li" {
                            BlockKind::ListItem
                        } else if tag == "blockquote" {
                            BlockKind::Quote
                        } else if is_heading_tag(&tag) || title_div.is_some() || paragraph_is_title
                        {
                            BlockKind::Heading
                        } else {
                            BlockKind::Paragraph
                        };
                        if kind_matches_heading(kind) && title.is_none() {
                            title = Some(text.clone());
                        }
                        let links = resolve_inline_links(&text, &buffer_links);
                        blocks.push(DocumentBlock { links, kind, text });
                    }
                    buffer.clear();
                    buffer_links.clear();
                    open_link = None;
                    if tag == "hr" {
                        blocks.push(DocumentBlock {
                            links: Vec::new(),
                            kind: BlockKind::Divider,
                            text: String::new(),
                        });
                    }
                    /*
                     * Converted books - FB2 exports above all - mark chapter
                     * openers with a styled wrapper such as
                     * <div class="title"><p>51. Tantalus</p></div> and never
                     * emit a heading element. Tracking that wrapper is what
                     * keeps those chapters from falling back to "Section N".
                     */
                    let self_closing = tag_source.trim_end().ends_with('/');
                    if tag == "div" {
                        if closing {
                            if title_div == Some(div_depth) {
                                title_div = None;
                            }
                            div_depth = div_depth.saturating_sub(1);
                        } else if !self_closing {
                            div_depth = div_depth.saturating_add(1);
                            if title_div.is_none() && has_title_class(tag_source) {
                                title_div = Some(div_depth);
                            }
                        }
                    } else if tag == "p" {
                        paragraph_is_title =
                            !closing && !self_closing && has_title_class(tag_source);
                    }
                }
                index += end + 1;
            } else {
                break;
            }
        } else {
            let next = rest.find('<').unwrap_or(rest.len());
            if hidden_depth == 0 {
                buffer.push_str(&rest[..next]);
                buffer.push(' ');
            }
            index += next;
        }
    }
    if !buffer.trim().is_empty() {
        if let Some((offset, href)) = open_link.take() {
            if buffer.len() > offset {
                let prefix_len = normalize_space(&buffer[..offset]).len();
                buffer_links.push((buffer[offset..].to_owned(), prefix_len, href));
            }
        }
        let text = decode_entities(&normalize_space(&buffer));
        let links = resolve_inline_links(&text, &buffer_links);
        blocks.push(DocumentBlock {
            links,
            kind: BlockKind::Paragraph,
            text,
        });
    }
    DocumentSection {
        source: source.to_owned(),
        anchors,
        id: id.to_owned(),
        title: title.unwrap_or_else(|| fallback_title.to_owned()),
        blocks: blocks
            .into_iter()
            .filter(|block| !block.text.is_empty() || matches!(block.kind, BlockKind::Divider))
            .collect(),
    }
}

fn epub_package(xml: &[u8]) -> Result<(HashMap<String, String>, Vec<String>), ReaderError> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    let mut manifest = HashMap::new();
    let mut spine = Vec::new();
    loop {
        match reader.read_event()? {
            Event::Start(event) | Event::Empty(event) => {
                let name = local_name(event.name().as_ref());
                let attributes = xml_attributes(&event);
                if name == "item" {
                    if let (Some(id), Some(href)) = (attributes.get("id"), attributes.get("href")) {
                        manifest.insert(id.clone(), href.clone());
                    }
                } else if name == "itemref" {
                    if let Some(idref) = attributes.get("idref") {
                        spine.push(idref.clone());
                    }
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok((manifest, spine))
}

fn xml_attribute(xml: &[u8], element: &str, attribute: &str) -> Option<String> {
    let mut reader = Reader::from_reader(xml);
    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) | Ok(Event::Empty(event))
                if local_name(event.name().as_ref()) == element =>
            {
                return xml_attributes(&event).get(attribute).cloned();
            }
            Ok(Event::Eof) | Err(_) => return None,
            _ => {}
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
            let high = epub_hex_value(*bytes.get(index + 1)?)?;
            let low = epub_hex_value(*bytes.get(index + 2)?)?;
            decoded.push(high.checked_mul(16)?.checked_add(low)?);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    let decoded = String::from_utf8(decoded).ok()?;
    (!decoded.contains('\0')).then_some(decoded)
}

fn epub_hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn xml_attributes(event: &quick_xml::events::BytesStart<'_>) -> HashMap<String, String> {
    event
        .attributes()
        .with_checks(false)
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

fn read_zip_entry<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
    limit: u64,
) -> Result<Vec<u8>, ReaderError> {
    let entry = archive.by_name(name)?;
    if entry.size() > limit {
        return Err(ReaderError::TooLarge);
    }
    let mut bytes = Vec::with_capacity(entry.size() as usize);
    entry.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(ReaderError::TooLarge);
    }
    Ok(bytes)
}

fn safe_archive_path(path: &Path) -> bool {
    !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
}

fn paragraphs(text: &str) -> Vec<String> {
    text.split("\n\n")
        .map(normalize_space)
        .filter(|paragraph| !paragraph.is_empty())
        .collect()
}

fn flush_paragraph(blocks: &mut Vec<DocumentBlock>, paragraph: &mut Vec<String>) {
    if !paragraph.is_empty() {
        blocks.push(DocumentBlock {
            links: Vec::new(),
            kind: BlockKind::Paragraph,
            text: clean_inline_markdown(&paragraph.join(" ")),
        });
        paragraph.clear();
    }
}

fn list_item(line: &str) -> Option<&str> {
    line.strip_prefix("- ")
        .or_else(|| line.strip_prefix("* "))
        .or_else(|| {
            let (number, content) = line.split_once(". ")?;
            number
                .chars()
                .all(|character| character.is_ascii_digit())
                .then_some(content)
        })
}

fn clean_inline_markdown(value: &str) -> String {
    normalize_space(
        &value
            .replace("**", "")
            .replace("__", "")
            .replace(['`', '*', '_'], ""),
    )
}

fn normalize_space(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn decode_entities(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(start) = rest.find('&') {
        output.push_str(&rest[..start]);
        rest = &rest[start..];
        let Some(end) = rest.find(';').filter(|end| *end <= 32) else {
            output.push('&');
            rest = &rest[1..];
            continue;
        };
        let entity = &rest[1..end];
        if let Some(decoded) = decode_html_entity(entity) {
            output.push(decoded);
            rest = &rest[end + 1..];
        } else {
            output.push_str(&rest[..=end]);
            rest = &rest[end + 1..];
        }
    }
    output.push_str(rest);
    output
}

fn decode_html_entity(entity: &str) -> Option<char> {
    match entity {
        "amp" => Some('&'),
        "apos" | "#39" => Some('\''),
        "gt" => Some('>'),
        "lt" => Some('<'),
        "nbsp" => Some(' '),
        "quot" => Some('"'),
        "mdash" => Some('—'),
        "ndash" => Some('–'),
        "hellip" => Some('…'),
        "laquo" => Some('«'),
        "raquo" => Some('»'),
        value if value.starts_with("#x") || value.starts_with("#X") => {
            u32::from_str_radix(&value[2..], 16)
                .ok()
                .and_then(char::from_u32)
        }
        value if value.starts_with('#') => value[1..].parse::<u32>().ok().and_then(char::from_u32),
        _ => None,
    }
}

fn file_title(path: &Path) -> String {
    path.file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("Untitled")
        .to_owned()
}

fn local_name(name: &[u8]) -> String {
    String::from_utf8_lossy(name)
        .rsplit(':')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase()
}

fn kind_matches_heading(kind: BlockKind) -> bool {
    matches!(kind, BlockKind::Heading)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn splits_an_oversized_paragraph_into_renderable_sections() {
        let sections = split_sections_for_rendering(vec![DocumentSection {
            source: String::new(),
            anchors: Vec::new(),
            id: "chapter".to_owned(),
            title: "Chapter".to_owned(),
            blocks: vec![DocumentBlock {
                links: Vec::new(),
                kind: BlockKind::Paragraph,
                text: "word ".repeat((MAX_RENDER_SECTION_TEXT / 5) + 200),
            }],
        }]);

        assert!(sections.len() > 1);
        assert!(sections.iter().all(|section| section
            .blocks
            .iter()
            .map(|block| block.text.len())
            .sum::<usize>()
            <= MAX_RENDER_SECTION_TEXT));
        assert_eq!(sections[0].id, "chapter");
        assert_eq!(sections[1].id, "chapter-part-2");
    }

    #[test]
    fn markdown_becomes_safe_structured_blocks() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("book.md");
        fs::write(
            &path,
            "# Opening\n\nFirst **paragraph**.\n\n> A quote\n\n- One",
        )
        .expect("fixture");
        let sections = read_document(&path).expect("document");
        assert_eq!(sections[0].title, "Opening");
        assert_eq!(sections[0].blocks.len(), 3);
        assert_eq!(sections[0].blocks[0].text, "First paragraph.");
    }

    #[test]
    fn html_drops_script_and_iframe_content() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("book.html");
        fs::write(
            &path,
            "<h1>Safe title</h1><script>steal()</script><p>Visible &#8212; text &hellip; &amp;lt;</p><iframe src='https://example.com'>hidden</iframe>",
        )
        .expect("fixture");
        let sections = read_document(&path).expect("document");
        let text = sections[0]
            .blocks
            .iter()
            .map(|block| block.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(text.contains("Visible — text … &lt;"));
        assert!(!text.contains("steal"));
        assert!(!text.contains("hidden"));
        assert_eq!(sections[0].title, "Safe title");
    }

    #[test]
    fn epub_uses_spine_order_and_never_exposes_markup() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("book.epub");
        let file = File::create(&path).expect("epub");
        let mut archive = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        archive
            .start_file("META-INF/container.xml", options)
            .expect("container");
        archive
            .write_all(br#"<container><rootfile full-path="OPS/book.opf"/></container>"#)
            .expect("container content");
        archive.start_file("OPS/book.opf", options).expect("opf");
        archive
            .write_all(br#"<package><manifest><item id="two" href="two.xhtml"/><item id="one" href="chapter%20one.xhtml#start"/></manifest><spine><itemref idref="one"/><itemref idref="two"/></spine></package>"#)
            .expect("opf content");
        archive
            .start_file("OPS/chapter one.xhtml", options)
            .expect("one");
        archive
            .write_all(b"<h1>One</h1><p>First &amp; foremost</p>")
            .expect("one content");
        archive.start_file("OPS/two.xhtml", options).expect("two");
        archive
            .write_all(b"<h1>Two</h1><p>Second</p>")
            .expect("two content");
        archive.finish().expect("archive");

        let sections = read_document(&path).expect("document");
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].title, "One");
        assert!(sections[0]
            .blocks
            .iter()
            .any(|block| block.text == "First & foremost"));
        assert_eq!(sections[1].title, "Two");
    }

    #[test]
    fn fb2_respects_declared_legacy_encoding() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("book.fb2");
        let mut bytes =
            br#"<?xml version="1.0" encoding="windows-1251"?><FictionBook><body><section><title><p>"#
                .to_vec();
        bytes.extend_from_slice(&[0xcf, 0xf0, 0xe8, 0xe2, 0xe5, 0xf2]);
        bytes.extend_from_slice(br#"</p></title><p>Text</p></section></body></FictionBook>"#);
        fs::write(&path, bytes).expect("fixture");
        let sections = read_document(&path).expect("document");
        assert_eq!(sections[0].title, "Привет");
    }

    #[test]
    fn fb2_keeps_inline_markup_and_entities_in_one_paragraph() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("inline.fb2");
        fs::write(
            &path,
            r#"<FictionBook><body><section><title><p>Rock &amp; Roll</p></title>
               <p>Before <strong>bold &amp; clear</strong> after.</p>
               </section></body></FictionBook>"#,
        )
        .expect("fixture");
        let sections = read_document(&path).expect("document");
        assert_eq!(sections[0].title, "Rock & Roll");
        assert_eq!(sections[0].blocks.len(), 1);
        assert_eq!(sections[0].blocks[0].text, "Before bold & clear after.");
    }

    #[test]
    fn fb2_streams_past_large_embedded_images() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("illustrated.fb2");
        let mut file = File::create(&path).expect("fixture");
        file.write_all(
            br#"<?xml version="1.0" encoding="utf-8"?><FictionBook><body><section><title><p>Compact book</p></title><p>Readable before image.</p></section></body><binary id="cover">"#,
        )
        .expect("prefix");
        let chunk = vec![b'A'; 1024 * 1024];
        for _ in 0..33 {
            file.write_all(&chunk).expect("embedded image");
        }
        file.write_all(br#"</binary></FictionBook>"#)
            .expect("suffix");
        drop(file);

        let sections = read_document(&path).expect("document");
        assert_eq!(sections[0].title, "Compact book");
        assert_eq!(sections[0].blocks[0].text, "Readable before image.");
    }

    #[test]
    fn docx_becomes_safe_semantic_sections() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("book.docx");
        let file = File::create(&path).expect("DOCX");
        let mut archive = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        archive
            .start_file("word/document.xml", options)
            .expect("document");
        archive
            .write_all(
                br#"<w:document xmlns:w="urn:test"><w:body>
                <w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>Opening</w:t></w:r></w:p>
                <w:p><w:r><w:t>Safe &amp; sound</w:t></w:r><w:hyperlink r:id="external"><w:r><w:t> only</w:t></w:r></w:hyperlink></w:p>
                <w:p><w:pPr><w:numPr/></w:pPr><w:r><w:t>List item</w:t></w:r></w:p>
                </w:body></w:document>"#,
            )
            .expect("document XML");
        archive.finish().expect("archive");
        let sections = read_document(&path).expect("document");
        assert_eq!(sections[0].title, "Opening");
        assert_eq!(sections[0].blocks[0].text, "Safe & sound only");
        assert!(matches!(sections[0].blocks[1].kind, BlockKind::ListItem));
    }

    #[test]
    fn rejects_archive_traversal() {
        assert!(!safe_archive_path(Path::new("../chapter.xhtml")));
        assert!(safe_archive_path(Path::new("OPS/chapter.xhtml")));
    }

    #[test]
    fn names_a_chapter_from_a_styled_title_wrapper() {
        // The shape emitted by FB2-to-EPUB converters: no heading element at
        // all, only a styled wrapper around an ordinary paragraph.
        let markup = br#"<html><body class="z"><span><span id="id67"><div class="title6">
<p class="p"><strong>51. Tantalus</strong></p>
</div><p class="empty-line"/><p class="p1">Tantalus ruled in Paphlagonia.</p></span></span></body></html>"#;
        let section = html_section(markup, "section-1", "Section 1", "");
        assert_eq!(section.title, "51. Tantalus");
        assert!(matches!(section.blocks[0].kind, BlockKind::Heading));
        assert_eq!(section.blocks[1].text, "Tantalus ruled in Paphlagonia.");
        assert!(matches!(section.blocks[1].kind, BlockKind::Paragraph));
    }

    #[test]
    fn keeps_a_footnote_marker_as_a_link_range() {
        let markup = br#"<html><body><p class="p1">He called it<a href="ch2-85.xhtml#id45" class="a">[85]</a> a myth.</p></body></html>"#;
        let section = html_section(markup, "section-1", "Section 1", "OPS/ch1-15.xhtml");
        let block = &section.blocks[0];
        assert_eq!(block.text, "He called it [85] a myth.");
        assert_eq!(block.links.len(), 1);
        let link = &block.links[0];
        assert_eq!(link.href, "ch2-85.xhtml#id45");
        // The range has to select exactly the marker and nothing around it.
        let selected: Vec<u16> = block
            .text
            .encode_utf16()
            .skip(link.start)
            .take(link.end - link.start)
            .collect();
        assert_eq!(String::from_utf16(&selected).unwrap(), "[85]");
        assert_eq!(section.source, "OPS/ch1-15.xhtml");
    }

    #[test]
    fn measures_link_offsets_in_utf16_units() {
        // Cyrillic text ahead of the marker: byte offsets and UTF-16 offsets
        // diverge here, and the WebView slices by the latter.
        let markup =
            "<html><body><p>\u{41f}\u{440}\u{43e}\u{43c}\u{435}\u{442}\u{435}\u{439}<a href=\"#n1\">[1]</a></p></body></html>";
        let section = html_section(markup.as_bytes(), "section-1", "Section 1", "");
        let block = &section.blocks[0];
        let link = &block.links[0];
        let prefix: Vec<u16> = block.text.encode_utf16().take(link.start).collect();
        assert_eq!(
            String::from_utf16(&prefix).unwrap(),
            "\u{41f}\u{440}\u{43e}\u{43c}\u{435}\u{442}\u{435}\u{439} "
        );
        assert_eq!(link.href, "#n1");
    }

    #[test]
    fn anchors_a_marker_that_also_appears_as_plain_text() {
        // The first "[1]" is ordinary text; only the second one is a link.
        let markup = br##"<html><body><p>Note [1] is cited as<a href="#n1">[1]</a> below.</p></body></html>"##;
        let section = html_section(markup, "section-1", "Section 1", "");
        let block = &section.blocks[0];
        let link = &block.links[0];
        let before: Vec<u16> = block.text.encode_utf16().take(link.start).collect();
        assert_eq!(
            String::from_utf16(&before).unwrap(),
            "Note [1] is cited as "
        );
    }

    #[test]
    fn keeps_repeated_markers_in_document_order() {
        let markup = br##"<html><body><p>A<a href="#a">[1]</a> and B<a href="#b">[1]</a></p></body></html>"##;
        let section = html_section(markup, "section-1", "Section 1", "");
        let links = &section.blocks[0].links;
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].href, "#a");
        assert_eq!(links[1].href, "#b");
        assert!(links[1].start > links[0].start);
    }

    #[test]
    fn collects_anchor_targets_for_link_resolution() {
        let markup = br#"<html><body><span id="id45"><p>Note body.</p></span></body></html>"#;
        let section = html_section(markup, "section-1", "Section 1", "OPS/ch2-85.xhtml");
        assert!(section.anchors.iter().any(|value| value == "id45"));
    }

    #[test]
    fn reads_attributes_without_confusing_similar_names() {
        assert_eq!(
            tag_attribute(r#"a href="target.xhtml#id1" class="a""#, "href").as_deref(),
            Some("target.xhtml#id1")
        );
        // "data-href" must not be mistaken for "href".
        assert_eq!(tag_attribute(r#"a data-href="x""#, "href"), None);
        assert_eq!(
            tag_attribute(r#"span id='id45'"#, "id").as_deref(),
            Some("id45")
        );
    }

    #[test]
    fn names_a_chapter_from_a_deep_heading() {
        let markup = br"<html><body><h4>Appendix</h4><p>Body.</p></body></html>";
        let section = html_section(markup, "section-1", "Section 1", "");
        assert_eq!(section.title, "Appendix");
    }

    #[test]
    fn does_not_mistake_a_horizontal_rule_for_a_heading() {
        let markup = br"<html><body><p>Plain body text</p><hr/><p>More.</p></body></html>";
        let section = html_section(markup, "section-1", "Section 1", "");
        assert_eq!(section.title, "Section 1");
        assert!(matches!(section.blocks[0].kind, BlockKind::Paragraph));
    }

    #[test]
    fn keeps_ordinary_classes_out_of_the_title() {
        let markup =
            br#"<html><body><div class="chapter"><p class="p1">Body only.</p></div></body></html>"#;
        let section = html_section(markup, "section-1", "Section 1", "");
        assert_eq!(section.title, "Section 1");
        assert!(!has_title_class(r#"div class="untitled-note""#));
        assert!(has_title_class(r#"div class="title6""#));
        assert!(has_title_class(r#"p class='subtitle'"#));
    }
}
