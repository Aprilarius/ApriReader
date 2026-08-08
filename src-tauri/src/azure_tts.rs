use crate::tts_assets::persist_cache_file;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

const USER_AGENT: &str = "ApriReader/1.3.0-rc.2";
const MAX_AZURE_TEXT_CHARACTERS: usize = 2_000;
const MAX_AZURE_SSML_BYTES: usize = 16 * 1024;
const MAX_VOICES_RESPONSE_BYTES: u64 = 4 * 1024 * 1024;
const MAX_AZURE_AUDIO_BYTES: u64 = 32 * 1024 * 1024;
const MAX_AZURE_VOICES: usize = 500;
const MAX_CACHED_SEGMENTS: usize = 64;
const OUTPUT_FORMAT: &str = "audio-24khz-48kbitrate-mono-mp3";

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AzureTtsSettings {
    pub pitch_percent: i16,
}

const REGIONS: [(&str, &str); 33] = [
    ("australiaeast", "Australia East"),
    ("brazilsouth", "Brazil South"),
    ("canadacentral", "Canada Central"),
    ("canadaeast", "Canada East"),
    ("centralindia", "Central India"),
    ("centralus", "Central US"),
    ("eastasia", "East Asia"),
    ("eastus", "East US"),
    ("eastus2", "East US 2"),
    ("francecentral", "France Central"),
    ("germanywestcentral", "Germany West Central"),
    ("italynorth", "Italy North"),
    ("japaneast", "Japan East"),
    ("japanwest", "Japan West"),
    ("koreacentral", "Korea Central"),
    ("northcentralus", "North Central US"),
    ("northeurope", "North Europe"),
    ("norwayeast", "Norway East"),
    ("qatarcentral", "Qatar Central"),
    ("southafricanorth", "South Africa North"),
    ("southcentralus", "South Central US"),
    ("southeastasia", "Southeast Asia"),
    ("swedencentral", "Sweden Central"),
    ("switzerlandnorth", "Switzerland North"),
    ("switzerlandwest", "Switzerland West"),
    ("uaenorth", "UAE North"),
    ("uksouth", "UK South"),
    ("ukwest", "UK West"),
    ("westcentralus", "West Central US"),
    ("westeurope", "West Europe"),
    ("westus", "West US"),
    ("westus2", "West US 2"),
    ("westus3", "West US 3"),
];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AzureTtsStatus {
    pub configured: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AzureTtsRegion {
    pub id: &'static str,
    pub name: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AzureTtsVoice {
    pub id: String,
    pub name: String,
    pub language: String,
    pub category: String,
    pub gender: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedAzureTtsAudio {
    pub path: String,
    pub voice_id: String,
    pub character_count: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct VoiceResponse {
    short_name: String,
    display_name: String,
    #[serde(default)]
    local_name: String,
    locale: String,
    #[serde(default)]
    gender: String,
    #[serde(default)]
    voice_type: String,
    #[serde(default)]
    status: String,
}

pub fn status() -> AzureTtsStatus {
    AzureTtsStatus {
        configured: credentials::read_key().is_ok_and(|value| !value.is_empty()),
    }
}

pub fn regions() -> Vec<AzureTtsRegion> {
    REGIONS
        .iter()
        .map(|(id, name)| AzureTtsRegion { id, name })
        .collect()
}

pub fn save_key(key: &str) -> Result<AzureTtsStatus, String> {
    validate_key(key)?;
    credentials::write_key(key.trim())?;
    Ok(status())
}

pub fn delete_key() -> Result<AzureTtsStatus, String> {
    credentials::delete_key()?;
    Ok(status())
}

pub fn list_voices(region: &str, language: Option<&str>) -> Result<Vec<AzureTtsVoice>, String> {
    let region = validate_region(region)?;
    let language = language.and_then(normalize_language_prefix);
    let key = configured_key()?;
    let url = format!("https://{region}.tts.speech.microsoft.com/cognitiveservices/voices/list");
    let mut response = ureq::get(&url)
        .header("User-Agent", USER_AGENT)
        .header("Ocp-Apim-Subscription-Key", &key)
        .call()
        .map_err(provider_error)?;
    let json = response
        .body_mut()
        .with_config()
        .limit(MAX_VOICES_RESPONSE_BYTES)
        .read_to_string()
        .map_err(provider_error)?;
    let parsed: Vec<VoiceResponse> = serde_json::from_str(&json)
        .map_err(|_| "Azure Speech returned invalid voice data".to_owned())?;
    let mut voices = parsed
        .into_iter()
        .filter(|voice| validate_voice_id(&voice.short_name).is_ok())
        .filter(|voice| validate_language(&voice.locale).is_ok())
        .filter(|voice| {
            language
                .as_ref()
                .is_none_or(|prefix| voice.locale.to_ascii_lowercase().starts_with(prefix))
        })
        .take(MAX_AZURE_VOICES)
        .map(|voice| {
            let display = if voice.local_name.trim().is_empty() {
                voice.display_name
            } else {
                voice.local_name
            };
            let preview = if voice.status.eq_ignore_ascii_case("preview") {
                " Preview"
            } else {
                ""
            };
            AzureTtsVoice {
                id: voice.short_name,
                name: display.chars().take(128).collect(),
                language: voice.locale,
                category: format!("{}{}", voice.voice_type, preview)
                    .trim()
                    .chars()
                    .take(64)
                    .collect(),
                gender: match voice.gender.as_str() {
                    "Female" => "female",
                    "Male" => "male",
                    _ => "unknown",
                }
                .to_owned(),
            }
        })
        .collect::<Vec<_>>();
    voices.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(voices)
}

pub fn prepare(
    text: &str,
    voice_id: &str,
    language: &str,
    region: &str,
    settings: AzureTtsSettings,
    cache_dir: &Path,
) -> Result<PreparedAzureTtsAudio, String> {
    validate_text(text)?;
    validate_voice_id(voice_id)?;
    let language = validate_language(language)?;
    let region = validate_region(region)?;
    validate_settings(settings)?;
    let key = configured_key()?;
    let ssml = build_ssml(text, voice_id, &language, settings)?;
    let url = format!("https://{region}.tts.speech.microsoft.com/cognitiveservices/v1");
    let mut response = ureq::post(&url)
        .header("User-Agent", USER_AGENT)
        .header("Ocp-Apim-Subscription-Key", &key)
        .header("Content-Type", "application/ssml+xml")
        .header("X-Microsoft-OutputFormat", OUTPUT_FORMAT)
        .send(ssml.as_bytes())
        .map_err(provider_error)?;
    let bytes = response
        .body_mut()
        .with_config()
        .limit(MAX_AZURE_AUDIO_BYTES)
        .read_to_vec()
        .map_err(provider_error)?;
    if bytes.len() < 2
        || !(bytes.starts_with(b"ID3") || (bytes[0] == 0xff && bytes[1] & 0xe0 == 0xe0))
    {
        return Err("Azure Speech returned invalid MP3 audio".to_owned());
    }
    fs::create_dir_all(cache_dir).map_err(|error| error.to_string())?;
    let mut digest = Sha256::new();
    digest.update(b"azure-speech-tts-v2-expressive");
    digest.update(region.as_bytes());
    digest.update(voice_id.as_bytes());
    digest.update(settings.pitch_percent.to_le_bytes());
    digest.update(text.as_bytes());
    let destination = cache_dir.join(format!("azure-tts-{:x}.mp3", digest.finalize()));
    persist_cache_file(&destination, &bytes)?;
    prune_cache(cache_dir, &destination);
    Ok(PreparedAzureTtsAudio {
        path: destination.to_string_lossy().into_owned(),
        voice_id: voice_id.to_owned(),
        character_count: text.chars().count(),
    })
}

fn validate_settings(settings: AzureTtsSettings) -> Result<(), String> {
    if !(-50..=50).contains(&settings.pitch_percent) {
        return Err("Azure Speech pitch must be between -50% and +50%".to_owned());
    }
    Ok(())
}

fn configured_key() -> Result<String, String> {
    credentials::read_key().and_then(|value| {
        if value.is_empty() {
            Err("Azure Speech key is not configured".to_owned())
        } else {
            Ok(value)
        }
    })
}

fn validate_key(key: &str) -> Result<(), String> {
    let value = key.trim();
    if !(8..=512).contains(&value.len())
        || !value.is_ascii()
        || value
            .bytes()
            .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
    {
        return Err("the Azure Speech key is invalid".to_owned());
    }
    Ok(())
}

fn validate_region(region: &str) -> Result<&str, String> {
    REGIONS
        .iter()
        .find_map(|(id, _)| (*id == region).then_some(*id))
        .ok_or_else(|| "the Azure Speech region is not supported".to_owned())
}

fn validate_voice_id(voice_id: &str) -> Result<(), String> {
    if voice_id.is_empty()
        || voice_id.len() > 128
        || !voice_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err("the Azure Speech voice identifier is invalid".to_owned());
    }
    Ok(())
}

fn validate_language(language: &str) -> Result<String, String> {
    let value = language.trim();
    if !(2..=35).contains(&value.len())
        || !value.is_ascii()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        || value.starts_with('-')
        || value.ends_with('-')
        || value.contains("--")
    {
        return Err("the Azure Speech voice language is invalid".to_owned());
    }
    Ok(value.to_owned())
}

fn normalize_language_prefix(language: &str) -> Option<String> {
    let value = validate_language(language).ok()?;
    Some(value.to_ascii_lowercase())
}

fn validate_text(text: &str) -> Result<(), String> {
    let count = text.chars().count();
    if count == 0 || count > MAX_AZURE_TEXT_CHARACTERS {
        return Err(format!(
            "Azure Speech accepts between 1 and {MAX_AZURE_TEXT_CHARACTERS} characters"
        ));
    }
    Ok(())
}

fn build_ssml(
    text: &str,
    voice_id: &str,
    language: &str,
    settings: AzureTtsSettings,
) -> Result<String, String> {
    let escaped = text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;");
    let ssml = format!(
        "<speak version=\"1.0\" xml:lang=\"{language}\"><voice name=\"{voice_id}\"><prosody pitch=\"{:+}%\">{escaped}</prosody></voice></speak>",
        settings.pitch_percent
    );
    if ssml.len() > MAX_AZURE_SSML_BYTES {
        return Err("Azure Speech SSML request is too large".to_owned());
    }
    Ok(ssml)
}

fn prune_cache(cache_dir: &Path, current: &Path) {
    let Ok(entries) = fs::read_dir(cache_dir) else {
        return;
    };
    let mut files = entries
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_type().is_ok_and(|kind| kind.is_file())
                && entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("azure-tts-")
                && entry.path().extension().is_some_and(|value| value == "mp3")
        })
        .map(|entry| {
            let modified = entry
                .metadata()
                .and_then(|metadata| metadata.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            (modified, entry.path())
        })
        .collect::<Vec<_>>();
    files.sort_by_key(|(modified, _)| *modified);
    let remove_count = files.len().saturating_sub(MAX_CACHED_SEGMENTS);
    for (_, path) in files.into_iter().take(remove_count) {
        if path != current {
            let _ = fs::remove_file(path);
        }
    }
}

fn provider_error(error: ureq::Error) -> String {
    format!("Azure Speech request failed: {error}")
}

#[cfg(windows)]
mod credentials {
    use windows::{
        core::{w, PWSTR},
        Win32::Security::Credentials::{
            CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE,
            CRED_TYPE_GENERIC,
        },
    };

    const TARGET: windows::core::PCWSTR = w!("ApriReader/AzureSpeechApiKey");

    pub fn read_key() -> Result<String, String> {
        let mut pointer = std::ptr::null_mut();
        unsafe {
            CredReadW(TARGET, CRED_TYPE_GENERIC, None, &mut pointer)
                .map_err(|_| "Azure Speech key is not configured".to_owned())?;
            let credential = &*pointer;
            if credential.CredentialBlob.is_null() || credential.CredentialBlobSize == 0 {
                CredFree(pointer.cast());
                return Err("the stored Azure Speech key is empty".to_owned());
            }
            let bytes = std::slice::from_raw_parts(
                credential.CredentialBlob,
                credential.CredentialBlobSize as usize,
            );
            let value = String::from_utf8(bytes.to_vec())
                .map_err(|_| "the stored Azure Speech key is invalid".to_owned());
            CredFree(pointer.cast());
            value
        }
    }

    pub fn write_key(key: &str) -> Result<(), String> {
        let mut target = "ApriReader/AzureSpeechApiKey\0"
            .encode_utf16()
            .collect::<Vec<_>>();
        let mut user = "ApriReader\0".encode_utf16().collect::<Vec<_>>();
        let mut bytes = key.as_bytes().to_vec();
        let credential = CREDENTIALW {
            Type: CRED_TYPE_GENERIC,
            TargetName: PWSTR(target.as_mut_ptr()),
            CredentialBlobSize: bytes.len() as u32,
            CredentialBlob: bytes.as_mut_ptr(),
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            UserName: PWSTR(user.as_mut_ptr()),
            ..Default::default()
        };
        unsafe { CredWriteW(&credential, 0).map_err(|error| error.to_string()) }
    }

    pub fn delete_key() -> Result<(), String> {
        unsafe { CredDeleteW(TARGET, CRED_TYPE_GENERIC, None).map_err(|error| error.to_string()) }
    }
}

#[cfg(not(windows))]
mod credentials {
    pub fn read_key() -> Result<String, String> {
        Err("Azure Speech credentials are available only on Windows".to_owned())
    }
    pub fn write_key(_key: &str) -> Result<(), String> {
        Err("Azure Speech credentials are available only on Windows".to_owned())
    }
    pub fn delete_key() -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_regions_keys_and_voice_identifiers() {
        assert_eq!(regions().len(), 33);
        assert_eq!(validate_region("westeurope").unwrap(), "westeurope");
        assert!(validate_region("evil.example.com").is_err());
        assert!(validate_key("azure-valid-test-key").is_ok());
        assert!(validate_key("bad key").is_err());
        assert!(validate_voice_id("ru-RU-SvetlanaNeural").is_ok());
        assert!(validate_voice_id("../voice").is_err());
    }

    #[test]
    fn escapes_book_text_before_building_bounded_ssml() {
        let ssml = build_ssml(
            "A < B & C's",
            "en-US-AvaNeural",
            "en-US",
            AzureTtsSettings { pitch_percent: 10 },
        )
        .unwrap();
        assert!(ssml.contains("A &lt; B &amp; C&apos;s"));
        assert!(ssml.contains("pitch=\"+10%\""));
        assert!(!ssml.contains("A < B"));
        assert!(validate_text(&"a".repeat(MAX_AZURE_TEXT_CHARACTERS + 1)).is_err());
        assert!(validate_settings(AzureTtsSettings { pitch_percent: 51 }).is_err());
    }
}
