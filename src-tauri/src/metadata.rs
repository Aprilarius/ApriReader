use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

const OPEN_LIBRARY_SEARCH_ENDPOINT: &str = "https://openlibrary.org/search.json";
const OPEN_LIBRARY_COVER_ENDPOINT: &str = "https://covers.openlibrary.org/b/id";
const INVENTAIRE_SEARCH_ENDPOINT: &str = "https://inventaire.io/api/search";
const INVENTAIRE_IMAGE_ENDPOINT: &str = "https://inventaire.io";
const USER_AGENT: &str = "ApriReader/1.3.0-rc.2 (interactive desktop metadata lookup)";
const MAX_RESULTS_PER_PROVIDER: usize = 8;
const MAX_COMBINED_RESULTS: usize = 12;
const MAX_SEARCH_RESPONSE_BYTES: u64 = 2 * 1024 * 1024;
pub const MAX_COVER_BYTES: usize = 10 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum MetadataError {
    #[error("metadata provider request failed: {0}")]
    Request(#[from] ureq::Error),
    #[error("metadata provider returned invalid data: {0}")]
    Json(#[from] serde_json::Error),
    #[error("metadata provider returned an invalid cover")]
    InvalidCover,
    #[error("the selected metadata language is invalid")]
    InvalidLanguage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataLanguage {
    Russian,
    English,
}

impl MetadataLanguage {
    pub fn parse(value: &str) -> Result<Self, MetadataError> {
        match value {
            "ru" => Ok(Self::Russian),
            "en" => Ok(Self::English),
            _ => Err(MetadataError::InvalidLanguage),
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::Russian => "ru",
            Self::English => "en",
        }
    }

    fn open_library_code(self) -> &'static str {
        match self {
            Self::Russian => "rus",
            Self::English => "eng",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MetadataCandidate {
    pub provider: String,
    pub provider_id: String,
    pub title: String,
    pub author: String,
    pub isbn: String,
    pub publisher: String,
    pub published_year: String,
    pub language: String,
    #[serde(default)]
    pub series: String,
    pub genres: String,
    pub cover_id: Option<i64>,
    #[serde(default)]
    pub cover_path: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Deserialize)]
struct OpenLibrarySearchResponse {
    #[serde(default)]
    docs: Vec<OpenLibraryDocument>,
}

#[derive(Debug, Deserialize, Default)]
struct OpenLibraryEditions {
    #[serde(default)]
    docs: Vec<OpenLibraryEdition>,
}

#[derive(Debug, Deserialize, Default)]
struct OpenLibraryEdition {
    #[serde(default)]
    key: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    author_name: Vec<String>,
    #[serde(default)]
    isbn: Vec<String>,
    #[serde(default)]
    publisher: Vec<String>,
    #[serde(default)]
    publish_date: Vec<String>,
    #[serde(default)]
    language: Vec<String>,
    #[serde(default)]
    subject: Vec<String>,
    cover_i: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct OpenLibraryDocument {
    #[serde(default)]
    key: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    author_name: Vec<String>,
    #[serde(default)]
    isbn: Vec<String>,
    #[serde(default)]
    publisher: Vec<String>,
    first_publish_year: Option<i64>,
    #[serde(default)]
    subject: Vec<String>,
    cover_i: Option<i64>,
    #[serde(default)]
    editions: OpenLibraryEditions,
}

#[derive(Debug, Deserialize)]
struct InventaireSearchResponse {
    #[serde(default)]
    results: Vec<InventaireResult>,
}

#[derive(Debug, Deserialize)]
struct InventaireResult {
    #[serde(default)]
    uri: String,
    #[serde(default)]
    label: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    image: String,
}

pub fn search_metadata(
    query: &str,
    language: MetadataLanguage,
) -> Result<Vec<MetadataCandidate>, MetadataError> {
    let open_library = search_open_library(query, language);
    let inventaire = search_inventaire(query, language);
    match (open_library, inventaire) {
        (Ok(open_library), Ok(inventaire)) => Ok(merge_candidates(open_library, inventaire)),
        (Ok(candidates), Err(_)) | (Err(_), Ok(candidates)) => Ok(candidates),
        (Err(error), Err(_)) => Err(error),
    }
}

pub fn search_open_library(
    query: &str,
    language: MetadataLanguage,
) -> Result<Vec<MetadataCandidate>, MetadataError> {
    let qualified_query = format!("{query} language:{}", language.open_library_code());
    let mut response = ureq::get(OPEN_LIBRARY_SEARCH_ENDPOINT)
        .header("User-Agent", USER_AGENT)
        .query("q", qualified_query)
        .query("lang", language.code())
        .query(
            "fields",
            "key,title,author_name,isbn,publisher,first_publish_year,language,subject,cover_i,editions,editions.key,editions.title,editions.author_name,editions.isbn,editions.publisher,editions.publish_date,editions.language,editions.subject,editions.cover_i",
        )
        .query("limit", MAX_RESULTS_PER_PROVIDER.to_string())
        .call()?;
    let body = response
        .body_mut()
        .with_config()
        .limit(MAX_SEARCH_RESPONSE_BYTES)
        .read_to_string()?;
    parse_open_library_response(&body, language)
}

pub fn search_inventaire(
    query: &str,
    language: MetadataLanguage,
) -> Result<Vec<MetadataCandidate>, MetadataError> {
    let mut response = ureq::get(INVENTAIRE_SEARCH_ENDPOINT)
        .header("User-Agent", USER_AGENT)
        .query("search", query)
        .query("lang", language.code())
        .query("types", "works")
        .query("limit", MAX_RESULTS_PER_PROVIDER.to_string())
        .call()?;
    let body = response
        .body_mut()
        .with_config()
        .limit(MAX_SEARCH_RESPONSE_BYTES)
        .read_to_string()?;
    parse_inventaire_response(&body, language)
}

pub fn download_open_library_cover(
    cover_id: i64,
) -> Result<(Vec<u8>, &'static str), MetadataError> {
    if cover_id <= 0 {
        return Err(MetadataError::InvalidCover);
    }
    let url = format!("{OPEN_LIBRARY_COVER_ENDPOINT}/{cover_id}-L.jpg?default=false");
    let mut response = ureq::get(&url).header("User-Agent", USER_AGENT).call()?;
    let bytes = response
        .body_mut()
        .with_config()
        .limit(MAX_COVER_BYTES as u64)
        .read_to_vec()?;
    let extension = image_extension(&bytes).ok_or(MetadataError::InvalidCover)?;
    Ok((bytes, extension))
}

pub fn download_inventaire_cover(
    cover_path: &str,
) -> Result<(Vec<u8>, &'static str), MetadataError> {
    if !valid_inventaire_cover_path(cover_path) {
        return Err(MetadataError::InvalidCover);
    }
    let url = format!("{INVENTAIRE_IMAGE_ENDPOINT}{cover_path}");
    let mut response = ureq::get(&url).header("User-Agent", USER_AGENT).call()?;
    let bytes = response
        .body_mut()
        .with_config()
        .limit(MAX_COVER_BYTES as u64)
        .read_to_vec()?;
    let extension = image_extension(&bytes).ok_or(MetadataError::InvalidCover)?;
    Ok((bytes, extension))
}

pub fn parse_open_library_response(
    json: &str,
    language: MetadataLanguage,
) -> Result<Vec<MetadataCandidate>, MetadataError> {
    let response: OpenLibrarySearchResponse = serde_json::from_str(json)?;
    Ok(response
        .docs
        .into_iter()
        .filter_map(|document| open_library_candidate(document, language))
        .take(MAX_RESULTS_PER_PROVIDER)
        .collect())
}

fn open_library_candidate(
    document: OpenLibraryDocument,
    language: MetadataLanguage,
) -> Option<MetadataCandidate> {
    let edition = document.editions.docs.into_iter().find(|edition| {
        edition
            .language
            .iter()
            .any(|value| value == language.open_library_code())
    });
    let key = edition
        .as_ref()
        .map(|value| value.key.trim())
        .filter(|value| value.starts_with("/books/"))
        .unwrap_or_else(|| document.key.trim());
    let title = edition
        .as_ref()
        .map(|value| value.title.as_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(&document.title);
    if title.trim().is_empty()
        || key.chars().count() > 128
        || !(key.starts_with("/works/") || key.starts_with("/books/"))
    {
        return None;
    }
    let author = edition
        .as_ref()
        .and_then(|value| value.author_name.first())
        .or_else(|| document.author_name.first())
        .map(String::as_str)
        .unwrap_or_default();
    let isbn = edition
        .as_ref()
        .and_then(|value| value.isbn.first())
        .or_else(|| document.isbn.first())
        .map(String::as_str)
        .unwrap_or_default();
    let publisher = edition
        .as_ref()
        .and_then(|value| value.publisher.first())
        .or_else(|| document.publisher.first())
        .map(String::as_str)
        .unwrap_or_default();
    let published_year = edition
        .as_ref()
        .and_then(|value| value.publish_date.first())
        .and_then(|value| extract_year(value))
        .or(document.first_publish_year)
        .map(|value| value.to_string())
        .unwrap_or_default();
    let subjects = edition
        .as_ref()
        .filter(|value| !value.subject.is_empty())
        .map(|value| value.subject.clone())
        .unwrap_or(document.subject);
    let cover_id = edition
        .as_ref()
        .and_then(|value| value.cover_i)
        .or(document.cover_i)
        .filter(|id| *id > 0);

    Some(MetadataCandidate {
        provider: "Open Library".to_owned(),
        provider_id: key.to_owned(),
        title: bounded_provider_text(title, 512),
        author: bounded_provider_text(author, 512),
        isbn: bounded_provider_text(isbn, 64),
        publisher: bounded_provider_text(publisher, 512),
        published_year,
        language: language.open_library_code().to_owned(),
        series: String::new(),
        genres: normalize_subjects(subjects),
        cover_id,
        cover_path: String::new(),
        description: String::new(),
    })
}

pub fn parse_inventaire_response(
    json: &str,
    language: MetadataLanguage,
) -> Result<Vec<MetadataCandidate>, MetadataError> {
    let response: InventaireSearchResponse = serde_json::from_str(json)?;
    Ok(response
        .results
        .into_iter()
        .filter(|entity| valid_inventaire_uri(&entity.uri) && !entity.label.trim().is_empty())
        .take(MAX_RESULTS_PER_PROVIDER)
        .map(|entity| MetadataCandidate {
            provider: "Inventaire".to_owned(),
            provider_id: entity.uri,
            title: bounded_provider_text(&entity.label, 512),
            author: String::new(),
            isbn: String::new(),
            publisher: String::new(),
            published_year: String::new(),
            language: language.open_library_code().to_owned(),
            series: String::new(),
            genres: String::new(),
            cover_id: None,
            cover_path: entity.image.trim().chars().take(128).collect(),
            description: bounded_provider_text(&entity.description, 16_384),
        })
        .collect())
}

fn merge_candidates(
    open_library: Vec<MetadataCandidate>,
    secondary: Vec<MetadataCandidate>,
) -> Vec<MetadataCandidate> {
    let mut combined = Vec::new();
    let mut keys = HashSet::new();
    for candidate in secondary.into_iter().chain(open_library) {
        let candidate_keys = candidate_identity_keys(&candidate);
        if candidate_keys.iter().any(|key| keys.contains(key)) {
            continue;
        }
        keys.extend(candidate_keys);
        combined.push(candidate);
        if combined.len() == MAX_COMBINED_RESULTS {
            break;
        }
    }
    combined
}

pub fn valid_inventaire_uri(value: &str) -> bool {
    value
        .strip_prefix("wd:Q")
        .is_some_and(|id| !id.is_empty() && id.bytes().all(|byte| byte.is_ascii_digit()))
        || value
            .strip_prefix("inv:")
            .is_some_and(|id| id.len() == 32 && id.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

pub fn valid_inventaire_cover_path(value: &str) -> bool {
    value.strip_prefix("/img/entities/").is_some_and(|digest| {
        digest.len() == 40
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    })
}

fn candidate_identity_keys(candidate: &MetadataCandidate) -> Vec<String> {
    let isbn_keys = candidate
        .isbn
        .split([',', ';'])
        .filter_map(|value| {
            let normalized = value
                .chars()
                .filter(|character| character.is_ascii_digit() || matches!(character, 'x' | 'X'))
                .collect::<String>()
                .to_lowercase();
            matches!(normalized.len(), 10 | 13).then(|| format!("isbn:{normalized}"))
        })
        .collect::<Vec<_>>();
    if !isbn_keys.is_empty() {
        return isbn_keys;
    }
    vec![format!(
        "text:{}:{}",
        normalized_identity_text(&candidate.title),
        normalized_identity_text(&candidate.author)
    )]
}

fn normalized_identity_text(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn extract_year(value: &str) -> Option<i64> {
    value
        .split(|character: char| !character.is_ascii_digit())
        .find(|part| part.len() == 4)
        .and_then(|part| part.parse::<i64>().ok())
}

fn bounded_provider_text(value: &str, max_chars: usize) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_chars)
        .collect()
}

fn normalize_subjects(subjects: Vec<String>) -> String {
    let mut genres = Vec::<String>::new();
    for subject in subjects {
        let normalized = subject.split_whitespace().collect::<Vec<_>>().join(" ");
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
    genres.join(", ")
}

pub fn image_extension(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\xff\xd8\xff") {
        Some("jpg")
    } else if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("png")
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        Some("webp")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_matching_open_library_edition_language() {
        let candidates = parse_open_library_response(
            r#"{
                "docs": [{
                    "key": "/works/OL1W",
                    "title": "English title",
                    "author_name": ["English Author"],
                    "subject": ["Science fiction", "Adventure"],
                    "cover_i": 21,
                    "editions": {"docs": [{
                        "key": "/books/OL2M",
                        "title": "Русское название",
                        "author_name": ["Русский Автор"],
                        "isbn": ["9780000000001"],
                        "publisher": ["Издательство"],
                        "publish_date": ["2024"],
                        "language": ["rus"],
                        "cover_i": 42
                    }]}
                }]
            }"#,
            MetadataLanguage::Russian,
        )
        .expect("metadata");
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].provider_id, "/books/OL2M");
        assert_eq!(candidates[0].title, "Русское название");
        assert_eq!(candidates[0].language, "rus");
        assert_eq!(candidates[0].genres, "Science fiction, Adventure");
        assert_eq!(candidates[0].cover_id, Some(42));
    }

    #[test]
    fn parses_bounded_inventaire_results() {
        let candidates = parse_inventaire_response(
            r#"{"results":[{"uri":"wd:Q74287","label":"Хоббит, или Туда и обратно","description":"повесть Джона Р. Р. Толкина (1937)","image":"/img/entities/9842e78c21762ef84b852c4771c5afdd3f9b0578"}]}"#,
            MetadataLanguage::Russian,
        )
        .expect("metadata");
        assert_eq!(candidates[0].provider, "Inventaire");
        assert_eq!(candidates[0].provider_id, "wd:Q74287");
        assert_eq!(
            candidates[0].description,
            "повесть Джона Р. Р. Толкина (1937)"
        );
        assert_eq!(candidates[0].language, "rus");
        assert_eq!(candidates[0].cover_id, None);
        assert!(valid_inventaire_cover_path(&candidates[0].cover_path));
    }

    #[test]
    fn merges_duplicate_provider_results_by_isbn() {
        let inventaire = MetadataCandidate {
            provider: "Inventaire".to_owned(),
            provider_id: "wd:Q1".to_owned(),
            title: "Книга".to_owned(),
            author: "Автор".to_owned(),
            isbn: "978-5-000-00000-1".to_owned(),
            publisher: String::new(),
            published_year: String::new(),
            language: "rus".to_owned(),
            series: String::new(),
            genres: String::new(),
            cover_id: None,
            cover_path: String::new(),
            description: String::new(),
        };
        let mut open_library = inventaire.clone();
        open_library.provider = "Open Library".to_owned();
        open_library.provider_id = "/books/OL1M".to_owned();
        open_library.isbn = "9785000000001".to_owned();
        let merged = merge_candidates(vec![open_library], vec![inventaire]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].provider, "Inventaire");
    }

    #[test]
    fn validates_downloaded_cover_signatures() {
        assert_eq!(image_extension(b"\xff\xd8\xffpayload"), Some("jpg"));
        assert_eq!(image_extension(b"<html>not an image</html>"), None);
    }
}
