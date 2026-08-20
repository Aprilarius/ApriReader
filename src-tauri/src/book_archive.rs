use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use zip::ZipArchive;

const MAX_ARCHIVE_BOOKS: usize = 128;
const MAX_ARCHIVE_EXPANDED_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const MAX_ARCHIVE_ENTRY_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const MAX_SAFE_FILE_NAME_BYTES: usize = 240;

pub(crate) struct ExpandedBookInputs {
    pub paths: Vec<PathBuf>,
    pub errors: Vec<String>,
}

pub(crate) fn is_zip_path(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("zip"))
}

pub(crate) fn expand_book_inputs(paths: &[PathBuf], cache_root: &Path) -> ExpandedBookInputs {
    let mut expanded = ExpandedBookInputs {
        paths: Vec::new(),
        errors: Vec::new(),
    };
    for path in paths {
        if !is_zip_path(path) {
            expanded.paths.push(path.clone());
            continue;
        }
        match extract_archive(path, cache_root) {
            Ok(paths) => expanded.paths.extend(paths),
            Err(error) => expanded
                .errors
                .push(format!("{}: {error}", path.to_string_lossy())),
        }
    }
    expanded
}

fn extract_archive(path: &Path, cache_root: &Path) -> Result<Vec<PathBuf>, String> {
    if !path.is_file() {
        return Err("archive is unavailable".to_owned());
    }
    let digest = file_digest(path)?;
    let destination = cache_root.join(digest);
    fs::create_dir_all(&destination).map_err(|error| error.to_string())?;
    let mut archive = ZipArchive::new(File::open(path).map_err(|error| error.to_string())?)
        .map_err(|error| format!("invalid ZIP archive: {error}"))?;
    let mut books = Vec::new();
    let mut expanded_bytes = 0_u64;
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|error| format!("invalid ZIP entry: {error}"))?;
        if entry.is_dir() {
            continue;
        }
        let Some(enclosed) = entry.enclosed_name() else {
            return Err("ZIP contains an unsafe path".to_owned());
        };
        let Some(leaf) = enclosed.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if !crate::importer::supported_book_path(Path::new(leaf)) {
            continue;
        }
        if books.len() == MAX_ARCHIVE_BOOKS {
            return Err(format!("ZIP contains more than {MAX_ARCHIVE_BOOKS} books"));
        }
        let declared_size = entry.size();
        if declared_size == 0 || declared_size > MAX_ARCHIVE_ENTRY_BYTES {
            return Err("a book inside ZIP is empty or too large".to_owned());
        }
        expanded_bytes = expanded_bytes
            .checked_add(declared_size)
            .ok_or_else(|| "ZIP expanded size overflow".to_owned())?;
        if expanded_bytes > MAX_ARCHIVE_EXPANDED_BYTES {
            return Err("ZIP expands beyond the 4 GiB safety limit".to_owned());
        }
        let safe_leaf = safe_file_name(leaf);
        let entry_dir = destination.join(format!("{index:04}"));
        fs::create_dir_all(&entry_dir).map_err(|error| error.to_string())?;
        let target = entry_dir.join(safe_leaf);
        if !target.is_file()
            || fs::metadata(&target).is_ok_and(|value| value.len() != declared_size)
        {
            let temporary = entry_dir.join(".book.partial");
            let result = (|| -> Result<(), String> {
                let mut output = File::create(&temporary).map_err(|error| error.to_string())?;
                let copied =
                    std::io::copy(&mut entry.take(MAX_ARCHIVE_ENTRY_BYTES + 1), &mut output)
                        .map_err(|error| error.to_string())?;
                output.flush().map_err(|error| error.to_string())?;
                if copied != declared_size || copied > MAX_ARCHIVE_ENTRY_BYTES {
                    return Err("a ZIP entry did not match its declared safe size".to_owned());
                }
                fs::rename(&temporary, &target).map_err(|error| error.to_string())?;
                Ok(())
            })();
            if result.is_err() {
                let _ = fs::remove_file(&temporary);
            }
            result?;
        }
        books.push(target);
    }
    if books.is_empty() {
        return Err("ZIP does not contain a supported book format".to_owned());
    }
    Ok(books)
}

fn file_digest(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|error| error.to_string())?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut total = 0_u64;
    loop {
        let count = file.read(&mut buffer).map_err(|error| error.to_string())?;
        if count == 0 {
            break;
        }
        total = total
            .checked_add(count as u64)
            .ok_or_else(|| "ZIP size overflow".to_owned())?;
        if total > MAX_ARCHIVE_EXPANDED_BYTES {
            return Err("ZIP is larger than the 4 GiB safety limit".to_owned());
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn safe_file_name(value: &str) -> String {
    let name = value
        .chars()
        .map(|character| {
            if character.is_control()
                || matches!(
                    character,
                    '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
                )
            {
                '_'
            } else {
                character
            }
        })
        .collect::<String>();
    let path = Path::new(&name);
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(&name);
    // Keep the format suffix even when a ZIP entry has a very long title: the
    // imported copy is opened later based on its extension.
    let suffix = if extension.is_empty() {
        String::new()
    } else {
        format!(".{extension}")
    };
    let maximum_stem = MAX_SAFE_FILE_NAME_BYTES.saturating_sub(suffix.len());
    let mut name = format!("{}{}", truncate_utf8(stem, maximum_stem.max(1)), suffix);
    if name.is_empty() || name == "." || name == ".." {
        name = "book.bin".to_owned();
    }
    name
}

fn truncate_utf8(value: &str, maximum_bytes: usize) -> &str {
    if value.len() <= maximum_bytes {
        return value;
    }
    let mut end = maximum_bytes;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    &value[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_supported_books_from_nested_zip_folders() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("books.zip");
        let file = File::create(&source).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("novels/Book One.fb2", options).unwrap();
        zip.write_all(b"<FictionBook><body><section><p>One</p></section></body></FictionBook>")
            .unwrap();
        zip.start_file("notes/readme.exe", options).unwrap();
        zip.write_all(b"ignored").unwrap();
        zip.start_file("Book Two.txt", options).unwrap();
        zip.write_all(b"Two").unwrap();
        zip.finish().unwrap();

        let expanded = expand_book_inputs(&[source], &directory.path().join("cache"));
        assert!(expanded.errors.is_empty());
        assert_eq!(expanded.paths.len(), 2);
        assert!(expanded
            .paths
            .iter()
            .any(|path| path.extension().is_some_and(|value| value == "fb2")));
        assert!(expanded
            .paths
            .iter()
            .any(|path| path.extension().is_some_and(|value| value == "txt")));
    }

    #[test]
    fn reports_zip_without_supported_books() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("empty.zip");
        let file = File::create(&source).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file("image.jpg", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"image").unwrap();
        zip.finish().unwrap();
        let expanded = expand_book_inputs(&[source], &directory.path().join("cache"));
        assert!(expanded.paths.is_empty());
        assert_eq!(expanded.errors.len(), 1);
    }

    #[test]
    fn extracted_reflow_books_can_be_opened_and_keep_their_extension() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("books.zip");
        let file = File::create(&source).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        let long_name = format!("nested/{}.fb2", "Book ".repeat(80));
        zip.start_file(long_name, options).unwrap();
        zip.write_all(
            b"<FictionBook><body><section><p>ZIP FB2 text.</p></section></body></FictionBook>",
        )
        .unwrap();
        zip.start_file("nested/notes.txt", options).unwrap();
        zip.write_all(b"ZIP TXT text.").unwrap();
        zip.finish().unwrap();

        let expanded = expand_book_inputs(&[source], &directory.path().join("cache"));
        assert!(expanded.errors.is_empty());
        assert_eq!(expanded.paths.len(), 2);
        for path in expanded.paths {
            assert!(path.is_file());
            let sections = crate::reader::read_document(&path).expect("read extracted book");
            assert!(!sections.is_empty());
        }
    }

    #[test]
    fn safe_file_name_fits_filesystem_limits_without_losing_the_format() {
        let name = safe_file_name(&format!("{}.epub", "Ж".repeat(200)));
        assert!(name.len() <= MAX_SAFE_FILE_NAME_BYTES);
        assert!(name.ends_with(".epub"));
        assert!(name.is_char_boundary(name.len()));
    }
}
