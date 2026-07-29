use quick_xml::{events::Event, Reader};
use serde::Serialize;
use std::{
    collections::HashMap,
    fs::{self, File},
    io::Read,
    path::{Component, Path},
};
use thiserror::Error;
use zip::ZipArchive;

const MAX_TEXT_FILE_SIZE: u64 = 32 * 1024 * 1024;
const MAX_ARCHIVE_ENTRY_SIZE: u64 = 8 * 1024 * 1024;
const MAX_DOCUMENT_TEXT: usize = 64 * 1024 * 1024;

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
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentBlock {
    pub kind: BlockKind,
    pub text: String,
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
    Ok(sections)
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
                        section.blocks.push(DocumentBlock { kind, text });
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
    Ok(fs::read(path)?)
}

fn read_txt(path: &Path) -> Result<Vec<DocumentSection>, ReaderError> {
    let text = String::from_utf8_lossy(&read_text_bytes(path)?).replace("\r\n", "\n");
    let blocks = paragraphs(&text)
        .into_iter()
        .map(|text| DocumentBlock {
            kind: BlockKind::Paragraph,
            text,
        })
        .collect();
    Ok(vec![DocumentSection {
        id: "text".to_owned(),
        title: file_title(path),
        blocks,
    }])
}

fn read_markdown(path: &Path) -> Result<Vec<DocumentSection>, ReaderError> {
    let text = String::from_utf8_lossy(&read_text_bytes(path)?).replace("\r\n", "\n");
    let mut sections = Vec::new();
    let mut current = DocumentSection {
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
                kind: BlockKind::Heading,
                text: clean_inline_markdown(heading),
            });
        } else if let Some(quote) = trimmed.strip_prefix('>') {
            flush_paragraph(&mut current.blocks, &mut paragraph);
            current.blocks.push(DocumentBlock {
                kind: BlockKind::Quote,
                text: clean_inline_markdown(quote.trim()),
            });
        } else if let Some(item) = list_item(trimmed) {
            flush_paragraph(&mut current.blocks, &mut paragraph);
            current.blocks.push(DocumentBlock {
                kind: BlockKind::ListItem,
                text: clean_inline_markdown(item),
            });
        } else if trimmed == "---" || trimmed == "***" {
            flush_paragraph(&mut current.blocks, &mut paragraph);
            current.blocks.push(DocumentBlock {
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
    Ok(vec![html_section(&bytes, "html", &file_title(path))])
}

fn read_epub(path: &Path) -> Result<Vec<DocumentSection>, ReaderError> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    let container = read_zip_entry(
        &mut archive,
        "META-INF/container.xml",
        MAX_ARCHIVE_ENTRY_SIZE,
    )?;
    let opf_path = xml_attribute(&container, "rootfile", "full-path").ok_or(ReaderError::Empty)?;
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
        let href = href.split('#').next().unwrap_or_default();
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
        ));
    }
    Ok(sections
        .into_iter()
        .filter(|section| !section.blocks.is_empty())
        .collect())
}

fn read_fb2(path: &Path) -> Result<Vec<DocumentSection>, ReaderError> {
    let bytes = read_text_bytes(path)?;
    let mut reader = Reader::from_reader(bytes.as_slice());
    reader.config_mut().trim_text(true);
    let mut sections = Vec::new();
    let mut current = DocumentSection {
        id: "section-1".to_owned(),
        title: file_title(path),
        blocks: Vec::new(),
    };
    let mut stack = Vec::<String>::new();
    let mut title_parts = Vec::new();
    loop {
        match reader.read_event()? {
            Event::Start(event) => {
                let name = local_name(event.name().as_ref());
                if name == "section" && !current.blocks.is_empty() {
                    sections.push(current);
                    current = DocumentSection {
                        id: format!("section-{}", sections.len() + 1),
                        title: format!("Section {}", sections.len() + 1),
                        blocks: Vec::new(),
                    };
                }
                stack.push(name);
            }
            Event::Text(text) => {
                let value = normalize_space(&text.decode().unwrap_or_default());
                if value.is_empty() {
                    continue;
                }
                let current_name = stack.last().map(String::as_str).unwrap_or_default();
                let inside_title = stack.iter().any(|name| name == "title");
                if inside_title && current_name == "p" {
                    title_parts.push(value);
                } else if current_name == "subtitle" {
                    current.blocks.push(DocumentBlock {
                        kind: BlockKind::Heading,
                        text: value,
                    });
                } else if current_name == "p" {
                    current.blocks.push(DocumentBlock {
                        kind: if stack.iter().any(|name| name == "cite") {
                            BlockKind::Quote
                        } else {
                            BlockKind::Paragraph
                        },
                        text: value,
                    });
                }
            }
            Event::End(event) => {
                let name = local_name(event.name().as_ref());
                if name == "title" && !title_parts.is_empty() {
                    current.title = title_parts.join(" ");
                    title_parts.clear();
                }
                stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if !current.blocks.is_empty() {
        sections.push(current);
    }
    Ok(sections)
}

fn html_section(bytes: &[u8], id: &str, fallback_title: &str) -> DocumentSection {
    let source = String::from_utf8_lossy(bytes);
    let mut blocks = Vec::new();
    let mut buffer = String::new();
    let mut title = None;
    let mut hidden_depth = 0_u32;
    let mut index = 0;
    while index < source.len() {
        let rest = &source[index..];
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
                if ["script", "style", "iframe", "object", "svg"].contains(&tag.as_str()) {
                    if closing {
                        hidden_depth = hidden_depth.saturating_sub(1);
                    } else if !tag_source.trim_end().ends_with('/') {
                        hidden_depth = hidden_depth.saturating_add(1);
                    }
                } else if hidden_depth == 0
                    && ["p", "div", "li", "blockquote", "h1", "h2", "h3", "br", "hr"]
                        .contains(&tag.as_str())
                {
                    if !buffer.trim().is_empty() {
                        let text = decode_entities(&normalize_space(&buffer));
                        let kind = if tag == "li" {
                            BlockKind::ListItem
                        } else if tag == "blockquote" {
                            BlockKind::Quote
                        } else if tag.starts_with('h') {
                            BlockKind::Heading
                        } else {
                            BlockKind::Paragraph
                        };
                        if kind_matches_heading(kind) && title.is_none() {
                            title = Some(text.clone());
                        }
                        blocks.push(DocumentBlock { kind, text });
                    }
                    buffer.clear();
                    if tag == "hr" {
                        blocks.push(DocumentBlock {
                            kind: BlockKind::Divider,
                            text: String::new(),
                        });
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
        blocks.push(DocumentBlock {
            kind: BlockKind::Paragraph,
            text: decode_entities(&normalize_space(&buffer)),
        });
    }
    DocumentSection {
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

fn xml_attributes(event: &quick_xml::events::BytesStart<'_>) -> HashMap<String, String> {
    event
        .attributes()
        .with_checks(false)
        .filter_map(Result::ok)
        .map(|attribute| {
            (
                local_name(attribute.key.as_ref()),
                String::from_utf8_lossy(attribute.value.as_ref()).into_owned(),
            )
        })
        .collect()
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
    value
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
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
            "<h1>Safe title</h1><script>steal()</script><p>Visible text</p><iframe src='https://example.com'>hidden</iframe>",
        )
        .expect("fixture");
        let sections = read_document(&path).expect("document");
        let text = sections[0]
            .blocks
            .iter()
            .map(|block| block.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(text.contains("Visible text"));
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
            .write_all(br#"<package><manifest><item id="two" href="two.xhtml"/><item id="one" href="one.xhtml"/></manifest><spine><itemref idref="one"/><itemref idref="two"/></spine></package>"#)
            .expect("opf content");
        archive.start_file("OPS/one.xhtml", options).expect("one");
        archive
            .write_all(b"<h1>One</h1><p>First</p>")
            .expect("one content");
        archive.start_file("OPS/two.xhtml", options).expect("two");
        archive
            .write_all(b"<h1>Two</h1><p>Second</p>")
            .expect("two content");
        archive.finish().expect("archive");

        let sections = read_document(&path).expect("document");
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].title, "One");
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
                <w:p><w:r><w:t>Safe text</w:t></w:r><w:hyperlink r:id="external"><w:r><w:t> only</w:t></w:r></w:hyperlink></w:p>
                <w:p><w:pPr><w:numPr/></w:pPr><w:r><w:t>List item</w:t></w:r></w:p>
                </w:body></w:document>"#,
            )
            .expect("document XML");
        archive.finish().expect("archive");
        let sections = read_document(&path).expect("document");
        assert_eq!(sections[0].title, "Opening");
        assert_eq!(sections[0].blocks[0].text, "Safe text only");
        assert!(matches!(sections[0].blocks[1].kind, BlockKind::ListItem));
    }

    #[test]
    fn rejects_archive_traversal() {
        assert!(!safe_archive_path(Path::new("../chapter.xhtml")));
        assert!(safe_archive_path(Path::new("OPS/chapter.xhtml")));
    }
}
