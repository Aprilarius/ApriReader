use serde::{Deserialize, Serialize};
use thiserror::Error;

const SEARCH_ENDPOINT: &str = "https://openlibrary.org/search.json";
const COVER_ENDPOINT: &str = "https://covers.openlibrary.org/b/id";
const USER_AGENT: &str = "ApriReader/0.1 (interactive desktop metadata lookup)";
const MAX_RESULTS: usize = 8;
const MAX_COVER_BYTES: usize = 10 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum MetadataError {
    #[error("metadata provider request failed: {0}")]
    Request(#[from] ureq::Error),
    #[error("metadata provider returned invalid data: {0}")]
    Json(#[from] serde_json::Error),
    #[error("metadata provider returned an invalid cover")]
    InvalidCover,
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
    pub genres: String,
    pub cover_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    #[serde(default)]
    docs: Vec<SearchDocument>,
}

#[derive(Debug, Deserialize)]
struct SearchDocument {
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
    language: Vec<String>,
    #[serde(default)]
    subject: Vec<String>,
    cover_i: Option<i64>,
}

pub fn search_open_library(query: &str) -> Result<Vec<MetadataCandidate>, MetadataError> {
    let mut response = ureq::get(SEARCH_ENDPOINT)
        .header("User-Agent", USER_AGENT)
        .query("q", query)
        .query(
            "fields",
            "key,title,author_name,isbn,publisher,first_publish_year,language,subject,cover_i",
        )
        .query("limit", MAX_RESULTS.to_string())
        .call()?;
    let body = response.body_mut().read_to_string()?;
    parse_search_response(&body)
}

pub fn download_cover(cover_id: i64) -> Result<(Vec<u8>, &'static str), MetadataError> {
    if cover_id <= 0 {
        return Err(MetadataError::InvalidCover);
    }
    let url = format!("{COVER_ENDPOINT}/{cover_id}-L.jpg?default=false");
    let mut response = ureq::get(&url).header("User-Agent", USER_AGENT).call()?;
    let bytes = response
        .body_mut()
        .with_config()
        .limit(MAX_COVER_BYTES as u64)
        .read_to_vec()?;
    let extension = image_extension(&bytes).ok_or(MetadataError::InvalidCover)?;
    Ok((bytes, extension))
}

pub fn parse_search_response(json: &str) -> Result<Vec<MetadataCandidate>, MetadataError> {
    let response: SearchResponse = serde_json::from_str(json)?;
    Ok(response
        .docs
        .into_iter()
        .filter(|document| !document.title.trim().is_empty() && !document.key.trim().is_empty())
        .take(MAX_RESULTS)
        .map(|document| MetadataCandidate {
            provider: "Open Library".to_owned(),
            provider_id: document.key,
            title: document.title,
            author: document.author_name.first().cloned().unwrap_or_default(),
            isbn: document.isbn.first().cloned().unwrap_or_default(),
            publisher: document.publisher.first().cloned().unwrap_or_default(),
            published_year: document
                .first_publish_year
                .map(|year| year.to_string())
                .unwrap_or_default(),
            language: document.language.first().cloned().unwrap_or_default(),
            genres: normalize_subjects(document.subject),
            cover_id: document.cover_i.filter(|id| *id > 0),
        })
        .collect())
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

fn image_extension(bytes: &[u8]) -> Option<&'static str> {
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
    fn parses_bounded_candidates_without_trusting_markup() {
        let candidates = parse_search_response(
            r#"{
                "docs": [{
                    "key": "/works/OL1W",
                    "title": "<script>Never execute me</script>",
                    "author_name": ["Author"],
                    "isbn": ["9780000000001"],
                    "publisher": ["Publisher"],
                    "first_publish_year": 2024,
                    "language": ["eng"],
                    "subject": ["Science fiction", "Adventure"],
                    "cover_i": 42
                }]
            }"#,
        )
        .expect("metadata");
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].provider_id, "/works/OL1W");
        assert_eq!(candidates[0].title, "<script>Never execute me</script>");
        assert_eq!(candidates[0].genres, "Science fiction, Adventure");
        assert_eq!(candidates[0].cover_id, Some(42));
    }

    #[test]
    fn validates_downloaded_cover_signatures() {
        assert_eq!(image_extension(b"\xff\xd8\xffpayload"), Some("jpg"));
        assert_eq!(image_extension(b"<html>not an image</html>"), None);
    }
}
