use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

const GOOGLE_VOICES: &str = "https://texttospeech.googleapis.com/v1/voices";
const GOOGLE_SYNTHESIZE: &str = "https://texttospeech.googleapis.com/v1/text:synthesize";
const USER_AGENT: &str = "ApriReader/1.3.0-rc.1";
const MAX_GOOGLE_TEXT_CHARACTERS: usize = 2_000;
const MAX_GOOGLE_TEXT_BYTES: usize = 4_800;
const MAX_VOICES_RESPONSE_BYTES: u64 = 4 * 1024 * 1024;
const MAX_TTS_RESPONSE_BYTES: u64 = 48 * 1024 * 1024;
const MAX_GOOGLE_AUDIO_BYTES: usize = 32 * 1024 * 1024;
const MAX_GOOGLE_VOICES: usize = 500;
const MAX_CACHED_SEGMENTS: usize = 64;

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleTtsSettings {
    pub pitch: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleTtsStatus {
    pub configured: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleTtsVoice {
    pub id: String,
    pub name: String,
    pub language: String,
    pub category: String,
    pub gender: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedGoogleTtsAudio {
    pub path: String,
    pub voice_id: String,
    pub character_count: usize,
}

#[derive(Debug, Deserialize)]
struct VoicesResponse {
    voices: Vec<VoiceResponse>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VoiceResponse {
    language_codes: Vec<String>,
    name: String,
    #[serde(default)]
    ssml_gender: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SynthesisResponse {
    audio_content: String,
}

pub fn status() -> GoogleTtsStatus {
    GoogleTtsStatus {
        configured: credentials::read_key().is_ok_and(|value| !value.is_empty()),
    }
}

pub fn save_key(key: &str) -> Result<GoogleTtsStatus, String> {
    validate_key(key)?;
    credentials::write_key(key.trim())?;
    Ok(status())
}

pub fn delete_key() -> Result<GoogleTtsStatus, String> {
    credentials::delete_key()?;
    Ok(status())
}

pub fn list_voices(language_code: Option<&str>) -> Result<Vec<GoogleTtsVoice>, String> {
    let key = configured_key()?;
    let language = language_code
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(validate_language_code)
        .transpose()?;
    let mut request = ureq::get(GOOGLE_VOICES)
        .header("User-Agent", USER_AGENT)
        .header("x-goog-api-key", &key);
    if let Some(value) = language.as_deref() {
        request = request.query("languageCode", value);
    }
    let mut response = request.call().map_err(provider_error)?;
    let json = response
        .body_mut()
        .with_config()
        .limit(MAX_VOICES_RESPONSE_BYTES)
        .read_to_string()
        .map_err(provider_error)?;
    let parsed: VoicesResponse = serde_json::from_str(&json)
        .map_err(|_| "Google Cloud returned invalid voice data".to_owned())?;
    let mut voices = parsed
        .voices
        .into_iter()
        .filter(|voice| validate_voice_id(&voice.name).is_ok())
        .filter_map(|voice| {
            let voice_language = voice
                .language_codes
                .iter()
                .find(|value| validate_language_code(value).is_ok())?
                .chars()
                .take(35)
                .collect::<String>();
            let category = voice_category(&voice.name).to_owned();
            Some(GoogleTtsVoice {
                id: voice.name.clone(),
                name: voice.name,
                language: voice_language,
                category,
                gender: match voice.ssml_gender.as_str() {
                    "FEMALE" => "female",
                    "MALE" => "male",
                    _ => "unknown",
                }
                .to_owned(),
            })
        })
        .take(MAX_GOOGLE_VOICES)
        .collect::<Vec<_>>();
    voices.sort_by(|left, right| {
        left.language
            .cmp(&right.language)
            .then_with(|| left.name.cmp(&right.name))
    });
    Ok(voices)
}

pub fn prepare(
    text: &str,
    voice_id: &str,
    language_code: &str,
    settings: GoogleTtsSettings,
    cache_dir: &Path,
) -> Result<PreparedGoogleTtsAudio, String> {
    validate_text(text)?;
    validate_voice_id(voice_id)?;
    let language = validate_language_code(language_code)?;
    validate_settings(settings)?;
    let key = configured_key()?;
    let mut response = ureq::post(GOOGLE_SYNTHESIZE)
        .header("User-Agent", USER_AGENT)
        .header("x-goog-api-key", &key)
        .header("Content-Type", "application/json")
        .send_json(serde_json::json!({
            "input": { "text": text },
            "voice": {
                "languageCode": language,
                "name": voice_id
            },
            "audioConfig": {
                "audioEncoding": "MP3",
                "pitch": settings.pitch
            }
        }))
        .map_err(provider_error)?;
    let json = response
        .body_mut()
        .with_config()
        .limit(MAX_TTS_RESPONSE_BYTES)
        .read_to_string()
        .map_err(provider_error)?;
    let parsed: SynthesisResponse = serde_json::from_str(&json)
        .map_err(|_| "Google Cloud returned invalid speech data".to_owned())?;
    let bytes = STANDARD
        .decode(parsed.audio_content.as_bytes())
        .map_err(|_| "Google Cloud returned invalid encoded audio".to_owned())?;
    if bytes.len() < 2
        || bytes.len() > MAX_GOOGLE_AUDIO_BYTES
        || !(bytes.starts_with(b"ID3") || (bytes[0] == 0xff && bytes[1] & 0xe0 == 0xe0))
    {
        return Err("Google Cloud returned invalid or oversized MP3 audio".to_owned());
    }
    fs::create_dir_all(cache_dir).map_err(|error| error.to_string())?;
    let mut digest = Sha256::new();
    digest.update(b"google-cloud-tts-v2-expressive");
    digest.update(voice_id.as_bytes());
    digest.update(language.as_bytes());
    digest.update(settings.pitch.to_le_bytes());
    digest.update(text.as_bytes());
    let destination = cache_dir.join(format!("google-tts-{:x}.mp3", digest.finalize()));
    if !destination.is_file() {
        let temporary = destination.with_extension("mp3.tmp");
        fs::write(&temporary, &bytes).map_err(|error| error.to_string())?;
        fs::rename(&temporary, &destination).map_err(|error| error.to_string())?;
    }
    prune_cache(cache_dir, &destination);
    Ok(PreparedGoogleTtsAudio {
        path: destination.to_string_lossy().into_owned(),
        voice_id: voice_id.to_owned(),
        character_count: text.chars().count(),
    })
}

fn validate_settings(settings: GoogleTtsSettings) -> Result<(), String> {
    if !settings.pitch.is_finite() || !(-20.0..=20.0).contains(&settings.pitch) {
        return Err("Google Cloud pitch must be between -20 and 20 semitones".to_owned());
    }
    Ok(())
}

fn configured_key() -> Result<String, String> {
    credentials::read_key().and_then(|value| {
        if value.is_empty() {
            Err("Google Cloud API key is not configured".to_owned())
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
        return Err("the Google Cloud API key is invalid".to_owned());
    }
    Ok(())
}

fn validate_voice_id(voice_id: &str) -> Result<(), String> {
    if voice_id.is_empty()
        || voice_id.len() > 128
        || !voice_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err("the Google Cloud voice identifier is invalid".to_owned());
    }
    Ok(())
}

fn validate_language_code(language_code: &str) -> Result<String, String> {
    let value = language_code.trim();
    if !(2..=35).contains(&value.len())
        || !value.is_ascii()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        || value.starts_with('-')
        || value.ends_with('-')
        || value.contains("--")
    {
        return Err("the Google Cloud voice language is invalid".to_owned());
    }
    Ok(value.to_owned())
}

fn validate_text(text: &str) -> Result<(), String> {
    let count = text.chars().count();
    if count == 0 || count > MAX_GOOGLE_TEXT_CHARACTERS || text.len() > MAX_GOOGLE_TEXT_BYTES {
        return Err(format!(
            "Google Cloud speech accepts 1-{MAX_GOOGLE_TEXT_CHARACTERS} characters and at most {MAX_GOOGLE_TEXT_BYTES} UTF-8 bytes"
        ));
    }
    Ok(())
}

fn voice_category(name: &str) -> &'static str {
    let lower = name.to_ascii_lowercase();
    if lower.contains("chirp3-hd") || lower.contains("chirp-hd") {
        "Chirp HD"
    } else if lower.contains("studio") {
        "Studio"
    } else if lower.contains("neural2") {
        "Neural2"
    } else if lower.contains("wavenet") {
        "WaveNet"
    } else if lower.contains("standard") {
        "Standard"
    } else {
        "Cloud"
    }
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
                    .starts_with("google-tts-")
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
    format!("Google Cloud request failed: {error}")
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

    const TARGET: windows::core::PCWSTR = w!("ApriReader/GoogleCloudTtsApiKey");

    pub fn read_key() -> Result<String, String> {
        let mut pointer = std::ptr::null_mut();
        unsafe {
            CredReadW(TARGET, CRED_TYPE_GENERIC, None, &mut pointer)
                .map_err(|_| "Google Cloud API key is not configured".to_owned())?;
            let credential = &*pointer;
            if credential.CredentialBlob.is_null() || credential.CredentialBlobSize == 0 {
                CredFree(pointer.cast());
                return Err("the stored Google Cloud API key is empty".to_owned());
            }
            let bytes = std::slice::from_raw_parts(
                credential.CredentialBlob,
                credential.CredentialBlobSize as usize,
            );
            let value = String::from_utf8(bytes.to_vec())
                .map_err(|_| "the stored Google Cloud API key is invalid".to_owned());
            CredFree(pointer.cast());
            value
        }
    }

    pub fn write_key(key: &str) -> Result<(), String> {
        let mut target = "ApriReader/GoogleCloudTtsApiKey\0"
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
        Err("Google Cloud credentials are available only on Windows".to_owned())
    }

    pub fn write_key(_key: &str) -> Result<(), String> {
        Err("Google Cloud credentials are available only on Windows".to_owned())
    }

    pub fn delete_key() -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_google_cloud_boundaries_without_exposing_keys() {
        assert!(validate_key("AIza-valid-test-key").is_ok());
        assert!(validate_key("short").is_err());
        assert!(validate_key("secret with spaces").is_err());
        assert!(validate_voice_id("ru-RU-Wavenet-A").is_ok());
        assert!(validate_voice_id("../voice").is_err());
        assert_eq!(validate_language_code("ru-RU").unwrap(), "ru-RU");
        assert!(validate_language_code("../ru").is_err());
        assert!(validate_settings(GoogleTtsSettings { pitch: -20.0 }).is_ok());
        assert!(validate_settings(GoogleTtsSettings { pitch: 20.1 }).is_err());
    }

    #[test]
    fn enforces_the_official_google_input_byte_boundary() {
        assert!(validate_text("Readable").is_ok());
        assert!(validate_text(&"a".repeat(MAX_GOOGLE_TEXT_CHARACTERS + 1)).is_err());
        assert!(validate_text(&"😀".repeat(1_201)).is_err());
        assert_eq!(voice_category("ru-RU-Chirp3-HD-Zephyr"), "Chirp HD");
        assert_eq!(voice_category("ru-RU-Wavenet-A"), "WaveNet");
    }
}
