use ort::{session::Session, value::Tensor};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs::{self, File},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error;
use zip::ZipArchive;

const MANIFEST_LIMIT: u64 = 64 * 1024;
const DICTIONARY_LIMIT: u64 = 64 * 1024 * 1024;
const MODEL_LIMIT: u64 = 1024 * 1024 * 1024;
const MAX_ARCHIVE_ENTRIES: usize = 8;
const MAX_DICTIONARY_ENTRIES: usize = 250_000;
const MAX_SELECTION: usize = 4_000;

#[derive(Debug, Error)]
pub enum LanguageToolsError {
    #[error("package file operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("the package archive is invalid: {0}")]
    Archive(#[from] zip::result::ZipError),
    #[error("the package manifest or dictionary is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("the package manifest is invalid")]
    InvalidManifest,
    #[error("the package contains an unsafe path or unsupported file")]
    UnsafePackage,
    #[error("the package hash, size, or license check failed")]
    VerificationFailed,
    #[error("this package version is already installed")]
    AlreadyInstalled,
    #[error("the requested package is not installed")]
    MissingPackage,
    #[error("the selected text is invalid")]
    InvalidSelection,
    #[error("the ONNX translation provider failed: {0}")]
    Translation(String),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageFile {
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguagePackageManifest {
    pub schema_version: u32,
    pub id: String,
    pub version: String,
    pub name: String,
    pub kind: String,
    pub source_language: String,
    pub target_language: Option<String>,
    pub source_url: String,
    pub license_spdx: String,
    pub attribution: String,
    pub engine: String,
    pub engine_version: String,
    pub files: Vec<PackageFile>,
    pub input_name: Option<String>,
    pub output_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledLanguagePackage {
    pub id: String,
    pub version: String,
    pub name: String,
    pub kind: String,
    pub source_language: String,
    pub target_language: Option<String>,
    pub license_spdx: String,
    pub attribution: String,
    pub engine: String,
    pub verified: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DictionaryEntry {
    term: String,
    definitions: Vec<String>,
    #[serde(default)]
    examples: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictionaryResult {
    pub package_id: String,
    pub package_name: String,
    pub term: String,
    pub definitions: Vec<String>,
    pub examples: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationResult {
    pub package_id: String,
    pub source_language: String,
    pub target_language: String,
    pub translated_text: String,
}

pub trait TranslationProvider {
    fn translate(
        &self,
        package_dir: &Path,
        manifest: &LanguagePackageManifest,
        text: &str,
    ) -> Result<String, LanguageToolsError>;
}

pub struct OnnxTranslationProvider;

impl TranslationProvider for OnnxTranslationProvider {
    fn translate(
        &self,
        package_dir: &Path,
        manifest: &LanguagePackageManifest,
        text: &str,
    ) -> Result<String, LanguageToolsError> {
        let model = package_dir.join("model.onnx");
        let input_name = manifest
            .input_name
            .as_deref()
            .ok_or(LanguageToolsError::InvalidManifest)?;
        let output_name = manifest
            .output_name
            .as_deref()
            .ok_or(LanguageToolsError::InvalidManifest)?;

        let mut session = Session::builder()
            .and_then(|mut builder| builder.commit_from_file(model))
            .map_err(|error| LanguageToolsError::Translation(error.to_string()))?;
        let values = vec![text.to_owned()];
        let input = Tensor::from_string_array(([1_usize], &*values))
            .map_err(|error| LanguageToolsError::Translation(error.to_string()))?;
        let outputs = session
            .run(ort::inputs![input_name => input])
            .map_err(|error| LanguageToolsError::Translation(error.to_string()))?;
        let output = outputs
            .get(output_name)
            .ok_or_else(|| LanguageToolsError::Translation("missing text output".to_owned()))?;
        let (_, values) = output
            .try_extract_strings()
            .map_err(|error| LanguageToolsError::Translation(error.to_string()))?;
        values
            .into_iter()
            .next()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| LanguageToolsError::Translation("empty text output".to_owned()))
    }
}

pub struct LanguagePackageManager {
    root: PathBuf,
}

impl LanguagePackageManager {
    pub fn new(root: PathBuf) -> Result<Self, LanguageToolsError> {
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    pub fn import(
        &self,
        archive_path: &Path,
    ) -> Result<InstalledLanguagePackage, LanguageToolsError> {
        let file = File::open(archive_path)?;
        let mut archive = ZipArchive::new(file)?;
        if archive.is_empty() || archive.len() > MAX_ARCHIVE_ENTRIES {
            return Err(LanguageToolsError::UnsafePackage);
        }
        let manifest_bytes = read_zip_entry(&mut archive, "manifest.json", MANIFEST_LIMIT)?;
        let manifest: LanguagePackageManifest = serde_json::from_slice(&manifest_bytes)?;
        validate_manifest(&manifest)?;
        validate_archive_names(&mut archive, &manifest)?;
        let destination = self.package_dir(&manifest.id, &manifest.version)?;
        if destination.exists() {
            return Err(LanguageToolsError::AlreadyInstalled);
        }
        let staging = self.root.join(format!(
            ".incoming-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir(&staging)?;
        let result = (|| {
            fs::write(staging.join("manifest.json"), &manifest_bytes)?;
            for expected in &manifest.files {
                let limit = file_limit(&manifest.kind, &expected.path)?;
                let bytes = read_zip_entry(&mut archive, &expected.path, limit)?;
                verify_file(expected, &bytes)?;
                let mut output = File::create(staging.join(&expected.path))?;
                output.write_all(&bytes)?;
                output.sync_all()?;
            }
            if manifest.kind == "dictionary" {
                validate_dictionary(&fs::read(staging.join("entries.json"))?)?;
            }
            fs::create_dir_all(
                destination
                    .parent()
                    .ok_or(LanguageToolsError::UnsafePackage)?,
            )?;
            fs::rename(&staging, &destination)?;
            Ok(self.record(&manifest, true))
        })();
        if staging.exists() {
            let _ = fs::remove_dir_all(&staging);
        }
        result
    }

    pub fn list(&self) -> Result<Vec<InstalledLanguagePackage>, LanguageToolsError> {
        let mut packages = Vec::new();
        for id_entry in fs::read_dir(&self.root)? {
            let id_entry = id_entry?;
            if !id_entry.file_type()?.is_dir()
                || id_entry.file_name().to_string_lossy().starts_with('.')
            {
                continue;
            }
            for version_entry in fs::read_dir(id_entry.path())? {
                let version_entry = version_entry?;
                if !version_entry.file_type()?.is_dir() {
                    continue;
                }
                if let Ok(manifest) = read_manifest(&version_entry.path()) {
                    let verified = self
                        .verify_installed(&version_entry.path(), &manifest)
                        .is_ok();
                    packages.push(self.record(&manifest, verified));
                }
            }
        }
        packages.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(packages)
    }

    pub fn lookup(
        &self,
        text: &str,
        context: &str,
    ) -> Result<Vec<DictionaryResult>, LanguageToolsError> {
        validate_text(text, 128)?;
        validate_text(context, MAX_SELECTION)?;
        let needle = normalize_term(text);
        let mut results = Vec::new();
        for package in self.list()? {
            if package.kind != "dictionary" || !package.verified {
                continue;
            }
            let directory = self.package_dir(&package.id, &package.version)?;
            let entries: Vec<DictionaryEntry> =
                serde_json::from_slice(&fs::read(directory.join("entries.json"))?)?;
            for entry in entries {
                if normalize_term(&entry.term) == needle {
                    results.push(DictionaryResult {
                        package_id: package.id.clone(),
                        package_name: package.name.clone(),
                        term: entry.term,
                        definitions: entry.definitions,
                        examples: entry.examples,
                    });
                    if results.len() == 8 {
                        return Ok(results);
                    }
                }
            }
        }
        Ok(results)
    }

    pub fn translate(
        &self,
        package_id: &str,
        version: &str,
        text: &str,
    ) -> Result<TranslationResult, LanguageToolsError> {
        validate_text(text, MAX_SELECTION)?;
        let directory = self.package_dir(package_id, version)?;
        if !directory.is_dir() {
            return Err(LanguageToolsError::MissingPackage);
        }
        let manifest = read_manifest(&directory)?;
        self.verify_installed(&directory, &manifest)?;
        if manifest.kind != "translation" || manifest.engine != "onnxruntime-text-v1" {
            return Err(LanguageToolsError::InvalidManifest);
        }
        let target_language = manifest
            .target_language
            .clone()
            .ok_or(LanguageToolsError::InvalidManifest)?;
        let translated_text = OnnxTranslationProvider.translate(&directory, &manifest, text)?;
        Ok(TranslationResult {
            package_id: manifest.id,
            source_language: manifest.source_language,
            target_language,
            translated_text,
        })
    }

    pub fn remove(&self, package_id: &str, version: &str) -> Result<(), LanguageToolsError> {
        let directory = self.package_dir(package_id, version)?;
        if !directory.is_dir() {
            return Err(LanguageToolsError::MissingPackage);
        }
        fs::remove_dir_all(directory)?;
        Ok(())
    }

    fn package_dir(&self, id: &str, version: &str) -> Result<PathBuf, LanguageToolsError> {
        if !safe_identifier(id) || !safe_identifier(version) {
            return Err(LanguageToolsError::InvalidManifest);
        }
        Ok(self.root.join(id).join(version))
    }

    fn record(
        &self,
        manifest: &LanguagePackageManifest,
        verified: bool,
    ) -> InstalledLanguagePackage {
        InstalledLanguagePackage {
            id: manifest.id.clone(),
            version: manifest.version.clone(),
            name: manifest.name.clone(),
            kind: manifest.kind.clone(),
            source_language: manifest.source_language.clone(),
            target_language: manifest.target_language.clone(),
            license_spdx: manifest.license_spdx.clone(),
            attribution: manifest.attribution.clone(),
            engine: manifest.engine.clone(),
            verified,
        }
    }

    fn verify_installed(
        &self,
        directory: &Path,
        manifest: &LanguagePackageManifest,
    ) -> Result<(), LanguageToolsError> {
        validate_manifest(manifest)?;
        for expected in &manifest.files {
            let bytes = fs::read(directory.join(&expected.path))?;
            verify_file(expected, &bytes)?;
        }
        Ok(())
    }
}

fn validate_manifest(manifest: &LanguagePackageManifest) -> Result<(), LanguageToolsError> {
    if manifest.schema_version != 1
        || !safe_identifier(&manifest.id)
        || !safe_identifier(&manifest.version)
        || manifest.name.trim().is_empty()
        || manifest.name.len() > 120
        || !valid_language(&manifest.source_language)
        || manifest.source_url.len() > 2_048
        || !manifest.source_url.starts_with("https://")
        || manifest.attribution.trim().is_empty()
        || manifest.attribution.len() > 4_000
        || manifest.engine_version.trim().is_empty()
        || manifest.engine_version.len() > 40
        || !allowed_license(&manifest.license_spdx)
        || manifest.files.is_empty()
        || manifest.files.len() > 3
    {
        return Err(LanguageToolsError::InvalidManifest);
    }
    let names = manifest
        .files
        .iter()
        .map(|file| file.path.as_str())
        .collect::<HashSet<_>>();
    match manifest.kind.as_str() {
        "dictionary"
            if manifest.engine == "aprireader-dictionary-v1"
                && names == HashSet::from(["entries.json"])
                && manifest.target_language.is_none() => {}
        "translation"
            if manifest.engine == "onnxruntime-text-v1"
                && names == HashSet::from(["model.onnx"])
                && manifest
                    .target_language
                    .as_deref()
                    .is_some_and(valid_language)
                && manifest.input_name.as_deref().is_some_and(safe_tensor_name)
                && manifest
                    .output_name
                    .as_deref()
                    .is_some_and(safe_tensor_name) => {}
        _ => return Err(LanguageToolsError::InvalidManifest),
    }
    for file in &manifest.files {
        file_limit(&manifest.kind, &file.path)?;
        if file.sha256.len() != 64
            || !file.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
            || file.size == 0
        {
            return Err(LanguageToolsError::InvalidManifest);
        }
    }
    Ok(())
}

fn validate_archive_names(
    archive: &mut ZipArchive<File>,
    manifest: &LanguagePackageManifest,
) -> Result<(), LanguageToolsError> {
    let mut expected = manifest
        .files
        .iter()
        .map(|file| file.path.as_str())
        .collect::<HashSet<_>>();
    expected.insert("manifest.json");
    let mut actual = HashSet::new();
    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        if entry.is_dir() {
            return Err(LanguageToolsError::UnsafePackage);
        }
        let enclosed = entry
            .enclosed_name()
            .ok_or(LanguageToolsError::UnsafePackage)?;
        if enclosed
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err(LanguageToolsError::UnsafePackage);
        }
        let name = enclosed.to_string_lossy().replace('\\', "/");
        if !actual.insert(name.clone()) || !expected.contains(name.as_str()) {
            return Err(LanguageToolsError::UnsafePackage);
        }
    }
    if actual.len() != expected.len() {
        return Err(LanguageToolsError::InvalidManifest);
    }
    Ok(())
}

fn read_zip_entry(
    archive: &mut ZipArchive<File>,
    expected_name: &str,
    limit: u64,
) -> Result<Vec<u8>, LanguageToolsError> {
    let mut found = None;
    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        let enclosed = entry
            .enclosed_name()
            .ok_or(LanguageToolsError::UnsafePackage)?;
        if enclosed
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err(LanguageToolsError::UnsafePackage);
        }
        let name = enclosed.to_string_lossy().replace('\\', "/");
        if name == expected_name {
            if entry.size() > limit {
                return Err(LanguageToolsError::VerificationFailed);
            }
            let mut bytes = Vec::with_capacity(entry.size() as usize);
            entry.take(limit + 1).read_to_end(&mut bytes)?;
            if bytes.len() as u64 > limit || found.is_some() {
                return Err(LanguageToolsError::VerificationFailed);
            }
            found = Some(bytes);
        }
    }
    found.ok_or(LanguageToolsError::InvalidManifest)
}

fn verify_file(expected: &PackageFile, bytes: &[u8]) -> Result<(), LanguageToolsError> {
    let hash = format!("{:x}", Sha256::digest(bytes));
    if bytes.len() as u64 != expected.size || !hash.eq_ignore_ascii_case(&expected.sha256) {
        return Err(LanguageToolsError::VerificationFailed);
    }
    Ok(())
}

fn validate_dictionary(bytes: &[u8]) -> Result<(), LanguageToolsError> {
    let entries: Vec<DictionaryEntry> = serde_json::from_slice(bytes)?;
    if entries.is_empty() || entries.len() > MAX_DICTIONARY_ENTRIES {
        return Err(LanguageToolsError::InvalidManifest);
    }
    for entry in entries {
        if entry.term.trim().is_empty()
            || entry.term.len() > 128
            || entry.definitions.is_empty()
            || entry.definitions.len() > 16
            || entry
                .definitions
                .iter()
                .any(|value| value.trim().is_empty() || value.len() > 2_000)
            || entry.examples.len() > 8
            || entry.examples.iter().any(|value| value.len() > 2_000)
        {
            return Err(LanguageToolsError::InvalidManifest);
        }
    }
    Ok(())
}

fn read_manifest(directory: &Path) -> Result<LanguagePackageManifest, LanguageToolsError> {
    let bytes = fs::read(directory.join("manifest.json"))?;
    if bytes.len() as u64 > MANIFEST_LIMIT {
        return Err(LanguageToolsError::InvalidManifest);
    }
    Ok(serde_json::from_slice(&bytes)?)
}

fn file_limit(kind: &str, path: &str) -> Result<u64, LanguageToolsError> {
    match (kind, path) {
        ("dictionary", "entries.json") => Ok(DICTIONARY_LIMIT),
        ("translation", "model.onnx") => Ok(MODEL_LIMIT),
        _ => Err(LanguageToolsError::UnsafePackage),
    }
}

fn allowed_license(value: &str) -> bool {
    matches!(
        value,
        "MIT" | "Apache-2.0" | "BSD-2-Clause" | "BSD-3-Clause" | "ISC" | "Zlib"
    )
}

fn safe_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn valid_language(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 35
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

fn safe_tensor_name(value: &str) -> bool {
    !value.is_empty() && value.len() <= 120 && !value.chars().any(char::is_control)
}

fn validate_text(value: &str, maximum: usize) -> Result<(), LanguageToolsError> {
    if value.trim().is_empty() || value.len() > maximum || value.chars().any(|ch| ch == '\0') {
        return Err(LanguageToolsError::InvalidSelection);
    }
    Ok(())
}

fn normalize_term(value: &str) -> String {
    value
        .trim_matches(|ch: char| !ch.is_alphanumeric() && ch != '\'' && ch != '-')
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use zip::{write::SimpleFileOptions, ZipWriter};

    fn dictionary_package(hash_override: Option<&str>) -> Vec<u8> {
        let entries = br#"[{"term":"quiet","definitions":["making little noise"],"examples":["a quiet library"]}]"#;
        let hash = hash_override
            .map(str::to_owned)
            .unwrap_or_else(|| format!("{:x}", Sha256::digest(entries)));
        let manifest = serde_json::json!({
            "schemaVersion": 1,
            "id": "synthetic-en",
            "version": "1.0.0",
            "name": "Synthetic English",
            "kind": "dictionary",
            "sourceLanguage": "en",
            "targetLanguage": null,
            "sourceUrl": "https://example.invalid/synthetic",
            "licenseSpdx": "MIT",
            "attribution": "Synthetic test data",
            "engine": "aprireader-dictionary-v1",
            "engineVersion": "1",
            "files": [{"path":"entries.json","size":entries.len(),"sha256":hash}],
            "inputName": null,
            "outputName": null
        });
        let output = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(output);
        writer
            .start_file("manifest.json", SimpleFileOptions::default())
            .unwrap();
        writer.write_all(manifest.to_string().as_bytes()).unwrap();
        writer
            .start_file("entries.json", SimpleFileOptions::default())
            .unwrap();
        writer.write_all(entries).unwrap();
        writer.finish().unwrap().into_inner()
    }

    #[test]
    fn imports_and_looks_up_a_verified_dictionary() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("dictionary.apridict");
        fs::write(&archive, dictionary_package(None)).unwrap();
        let manager = LanguagePackageManager::new(temp.path().join("packages")).unwrap();
        let installed = manager.import(&archive).unwrap();
        assert!(installed.verified);
        let results = manager.lookup("Quiet", "A quiet fixture.").unwrap();
        assert_eq!(results[0].definitions, ["making little noise"]);
    }

    #[test]
    fn rejects_a_bad_hash_and_disallowed_license() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("bad.apridict");
        fs::write(&archive, dictionary_package(Some(&"0".repeat(64)))).unwrap();
        let manager = LanguagePackageManager::new(temp.path().join("packages")).unwrap();
        assert!(matches!(
            manager.import(&archive),
            Err(LanguageToolsError::VerificationFailed)
        ));
        assert!(!allowed_license("GPL-3.0-only"));
    }

    #[test]
    fn translation_manifest_requires_a_model_and_text_io() {
        let manifest = LanguagePackageManifest {
            schema_version: 1,
            id: "en-ru".into(),
            version: "1".into(),
            name: "Synthetic translator".into(),
            kind: "translation".into(),
            source_language: "en".into(),
            target_language: Some("ru".into()),
            source_url: "https://example.invalid/model".into(),
            license_spdx: "Apache-2.0".into(),
            attribution: "Synthetic".into(),
            engine: "onnxruntime-text-v1".into(),
            engine_version: "1.24".into(),
            files: vec![PackageFile {
                path: "model.onnx".into(),
                size: 1,
                sha256: "0".repeat(64),
            }],
            input_name: Some("text".into()),
            output_name: Some("translation".into()),
        };
        assert!(validate_manifest(&manifest).is_ok());
    }
}
