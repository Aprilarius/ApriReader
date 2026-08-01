use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Seek},
    path::Path,
};
use thiserror::Error;

const MAX_FONT_BYTES: u64 = 24 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum FontImportError {
    #[error("font file operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("the selected font format is not supported")]
    UnsupportedFormat,
    #[error("the selected font is empty or larger than 24 MB")]
    InvalidSize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedReaderFont {
    pub name: String,
    pub family: String,
    pub path: String,
}

pub fn import_reader_font(
    source: &Path,
    destination_dir: &Path,
) -> Result<ImportedReaderFont, FontImportError> {
    let metadata = fs::metadata(source)?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_FONT_BYTES {
        return Err(FontImportError::InvalidSize);
    }
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or(FontImportError::UnsupportedFormat)?;
    if !matches!(extension.as_str(), "ttf" | "otf" | "woff" | "woff2") {
        return Err(FontImportError::UnsupportedFormat);
    }

    let mut file = fs::File::open(source)?;
    let mut signature = [0_u8; 4];
    file.read_exact(&mut signature)?;
    let valid_signature = match extension.as_str() {
        "ttf" => matches!(&signature, b"\0\x01\0\0" | b"true"),
        "otf" => &signature == b"OTTO",
        "woff" => &signature == b"wOFF",
        "woff2" => &signature == b"wOF2",
        _ => false,
    };
    if !valid_signature {
        return Err(FontImportError::UnsupportedFormat);
    }

    file.rewind()?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(MAX_FONT_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_FONT_BYTES {
        return Err(FontImportError::InvalidSize);
    }
    let digest = format!("{:x}", Sha256::digest(&bytes));
    fs::create_dir_all(destination_dir)?;
    let target = destination_dir.join(format!("{digest}.{extension}"));
    if !target.is_file() {
        let temporary = destination_dir.join(format!("{digest}.part"));
        fs::write(&temporary, &bytes)?;
        fs::rename(temporary, &target)?;
    }
    let name = source
        .file_stem()
        .and_then(|value| value.to_str())
        .map(|value| value.chars().take(80).collect::<String>())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "Imported font".to_owned());
    Ok(ImportedReaderFont {
        name,
        family: format!("ApriReaderImported_{}", &digest[..16]),
        path: target.to_string_lossy().into_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_a_bounded_font_to_an_app_managed_name() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source = directory.path().join("Quiet Reading.ttf");
        fs::write(&source, b"\0\x01\0\0synthetic-font-fixture").expect("fixture");
        let imported =
            import_reader_font(&source, &directory.path().join("fonts")).expect("font import");
        assert_eq!(imported.name, "Quiet Reading");
        assert!(Path::new(&imported.path).is_file());
        assert!(imported.family.starts_with("ApriReaderImported_"));
        assert_eq!(
            fs::read(&source).expect("source remains unchanged"),
            b"\0\x01\0\0synthetic-font-fixture"
        );
    }

    #[test]
    fn rejects_an_extension_with_a_mismatched_signature() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let source = directory.path().join("not-a-font.otf");
        fs::write(&source, b"\0\x01\0\0wrong-kind").expect("fixture");
        assert!(matches!(
            import_reader_font(&source, &directory.path().join("fonts")),
            Err(FontImportError::UnsupportedFormat)
        ));
    }
}
