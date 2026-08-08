use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

const ELEVENLABS_VOICES: &str = "https://api.elevenlabs.io/v2/voices";
const ELEVENLABS_TTS_ROOT: &str = "https://api.elevenlabs.io/v1/text-to-speech";
const USER_AGENT: &str = "ApriReader/1.3.0-rc.1";
const MAX_CLOUD_TEXT_CHARACTERS: usize = 2_000;
const MAX_VOICES_RESPONSE_BYTES: u64 = 4 * 1024 * 1024;
const MAX_TTS_RESPONSE_BYTES: u64 = 48 * 1024 * 1024;
const MAX_CLOUD_AUDIO_BYTES: usize = 32 * 1024 * 1024;
const MAX_CACHED_SEGMENTS: usize = 64;

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudTtsSettings {
    pub stability: f64,
    pub similarity_boost: f64,
    pub style: f64,
    pub speaker_boost: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudTtsStatus {
    pub configured: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudTtsVoice {
    pub id: String,
    pub name: String,
    pub language: String,
    pub category: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudTtsTiming {
    pub start_offset: usize,
    pub end_offset: usize,
    pub start_seconds: f64,
    pub end_seconds: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedCloudTtsAudio {
    pub path: String,
    pub voice_id: String,
    pub character_count: usize,
    pub timings: Vec<CloudTtsTiming>,
}

#[derive(Debug, Deserialize)]
struct VoiceSearchResponse {
    voices: Vec<VoiceResponse>,
}

#[derive(Debug, Deserialize)]
struct VoiceResponse {
    voice_id: String,
    name: String,
    #[serde(default)]
    category: String,
    #[serde(default)]
    labels: std::collections::HashMap<String, String>,
    #[serde(default)]
    verified_languages: Vec<VerifiedLanguage>,
}

#[derive(Debug, Deserialize)]
struct VerifiedLanguage {
    #[serde(default)]
    locale: String,
    #[serde(default)]
    language: String,
}

#[derive(Debug, Deserialize)]
struct TtsResponse {
    audio_base64: String,
    alignment: Option<AlignmentResponse>,
}

#[derive(Debug, Deserialize)]
struct AlignmentResponse {
    characters: Vec<String>,
    character_start_times_seconds: Vec<f64>,
    character_end_times_seconds: Vec<f64>,
}

pub fn status() -> CloudTtsStatus {
    CloudTtsStatus {
        configured: credentials::read_key().is_ok_and(|value| !value.is_empty()),
    }
}

pub fn save_key(key: &str) -> Result<CloudTtsStatus, String> {
    validate_key(key)?;
    credentials::write_key(key.trim())?;
    Ok(status())
}

pub fn delete_key() -> Result<CloudTtsStatus, String> {
    credentials::delete_key()?;
    Ok(status())
}

pub fn list_voices() -> Result<Vec<CloudTtsVoice>, String> {
    let key = configured_key()?;
    let mut response = ureq::get(ELEVENLABS_VOICES)
        .query("page_size", "100")
        .query("sort", "name")
        .query("sort_direction", "asc")
        .header("User-Agent", USER_AGENT)
        .header("xi-api-key", &key)
        .call()
        .map_err(provider_error)?;
    let json = response
        .body_mut()
        .with_config()
        .limit(MAX_VOICES_RESPONSE_BYTES)
        .read_to_string()
        .map_err(provider_error)?;
    let parsed: VoiceSearchResponse = serde_json::from_str(&json)
        .map_err(|_| "ElevenLabs returned invalid voice data".to_owned())?;
    let mut voices = parsed
        .voices
        .into_iter()
        .take(100)
        .filter(|voice| validate_voice_id(&voice.voice_id).is_ok())
        .map(|voice| {
            let language = voice
                .verified_languages
                .iter()
                .find_map(|entry| {
                    if !entry.locale.trim().is_empty() {
                        Some(entry.locale.trim().to_owned())
                    } else if !entry.language.trim().is_empty() {
                        Some(entry.language.trim().to_owned())
                    } else {
                        None
                    }
                })
                .or_else(|| voice.labels.get("language").cloned())
                .unwrap_or_else(|| "multilingual".to_owned());
            CloudTtsVoice {
                id: voice.voice_id,
                name: if voice.name.trim().is_empty() {
                    "Unnamed voice".to_owned()
                } else {
                    voice.name.chars().take(256).collect()
                },
                language: language.chars().take(64).collect(),
                category: voice.category.chars().take(64).collect(),
            }
        })
        .collect::<Vec<_>>();
    voices.sort_by_key(|voice| voice.name.to_lowercase());
    Ok(voices)
}

pub fn prepare(
    text: &str,
    voice_id: &str,
    settings: CloudTtsSettings,
    cache_dir: &Path,
) -> Result<PreparedCloudTtsAudio, String> {
    validate_text(text)?;
    validate_voice_id(voice_id)?;
    validate_settings(settings)?;
    let key = configured_key()?;
    let url = format!("{ELEVENLABS_TTS_ROOT}/{voice_id}/with-timestamps");
    let mut response = ureq::post(&url)
        .query("output_format", "mp3_44100_128")
        .header("User-Agent", USER_AGENT)
        .header("xi-api-key", &key)
        .header("Content-Type", "application/json")
        .send_json(serde_json::json!({
            "text": text,
            "model_id": "eleven_multilingual_v2",
            "voice_settings": {
                "stability": settings.stability,
                "similarity_boost": settings.similarity_boost,
                "style": settings.style,
                "use_speaker_boost": settings.speaker_boost
            }
        }))
        .map_err(provider_error)?;
    let json = response
        .body_mut()
        .with_config()
        .limit(MAX_TTS_RESPONSE_BYTES)
        .read_to_string()
        .map_err(provider_error)?;
    let parsed: TtsResponse = serde_json::from_str(&json)
        .map_err(|_| "ElevenLabs returned invalid speech data".to_owned())?;
    let bytes = STANDARD
        .decode(parsed.audio_base64.as_bytes())
        .map_err(|_| "ElevenLabs returned invalid encoded audio".to_owned())?;
    if bytes.len() < 2
        || bytes.len() > MAX_CLOUD_AUDIO_BYTES
        || !(bytes.starts_with(b"ID3") || (bytes[0] == 0xff && bytes[1] & 0xe0 == 0xe0))
    {
        return Err("ElevenLabs returned invalid or oversized MP3 audio".to_owned());
    }
    let timings = parse_alignment(text, parsed.alignment)?;
    fs::create_dir_all(cache_dir).map_err(|error| error.to_string())?;
    let mut digest = Sha256::new();
    digest.update(b"elevenlabs-v2-expressive");
    digest.update(voice_id.as_bytes());
    digest.update(settings.stability.to_le_bytes());
    digest.update(settings.similarity_boost.to_le_bytes());
    digest.update(settings.style.to_le_bytes());
    digest.update([u8::from(settings.speaker_boost)]);
    digest.update(text.as_bytes());
    let destination = cache_dir.join(format!("cloud-tts-{:x}.mp3", digest.finalize()));
    if !destination.is_file() {
        let temporary = destination.with_extension("mp3.tmp");
        fs::write(&temporary, &bytes).map_err(|error| error.to_string())?;
        fs::rename(&temporary, &destination).map_err(|error| error.to_string())?;
    }
    prune_cache(cache_dir, &destination);
    Ok(PreparedCloudTtsAudio {
        path: destination.to_string_lossy().into_owned(),
        voice_id: voice_id.to_owned(),
        character_count: text.chars().count(),
        timings,
    })
}

fn validate_settings(settings: CloudTtsSettings) -> Result<(), String> {
    if !settings.stability.is_finite()
        || !settings.similarity_boost.is_finite()
        || !settings.style.is_finite()
        || !(0.0..=1.0).contains(&settings.stability)
        || !(0.0..=1.0).contains(&settings.similarity_boost)
        || !(0.0..=1.0).contains(&settings.style)
    {
        return Err("ElevenLabs expressive controls must be between 0 and 1".to_owned());
    }
    Ok(())
}

fn configured_key() -> Result<String, String> {
    credentials::read_key().and_then(|value| {
        if value.is_empty() {
            Err("ElevenLabs API key is not configured".to_owned())
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
        return Err("the ElevenLabs API key is invalid".to_owned());
    }
    Ok(())
}

fn validate_voice_id(voice_id: &str) -> Result<(), String> {
    if voice_id.is_empty()
        || voice_id.len() > 128
        || !voice_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err("the ElevenLabs voice identifier is invalid".to_owned());
    }
    Ok(())
}

fn validate_text(text: &str) -> Result<(), String> {
    let count = text.chars().count();
    if count == 0 || count > MAX_CLOUD_TEXT_CHARACTERS {
        return Err(format!(
            "cloud speech accepts between 1 and {MAX_CLOUD_TEXT_CHARACTERS} characters"
        ));
    }
    Ok(())
}

fn parse_alignment(
    text: &str,
    alignment: Option<AlignmentResponse>,
) -> Result<Vec<CloudTtsTiming>, String> {
    let alignment = alignment.ok_or_else(|| "ElevenLabs returned no speech timing".to_owned())?;
    let count = alignment.characters.len();
    if count == 0
        || count != alignment.character_start_times_seconds.len()
        || count != alignment.character_end_times_seconds.len()
        || alignment.characters.concat() != text
    {
        return Err("ElevenLabs returned inconsistent speech timing".to_owned());
    }
    let mut offset = 0usize;
    let mut last_start = 0.0;
    let mut result = Vec::with_capacity(count);
    for index in 0..count {
        let value = &alignment.characters[index];
        let start = alignment.character_start_times_seconds[index];
        let end = alignment.character_end_times_seconds[index];
        if !start.is_finite()
            || !end.is_finite()
            || start < 0.0
            || end < start
            || start < last_start
        {
            return Err("ElevenLabs returned invalid speech timing".to_owned());
        }
        let end_offset = offset + value.encode_utf16().count();
        result.push(CloudTtsTiming {
            start_offset: offset,
            end_offset,
            start_seconds: start,
            end_seconds: end,
        });
        offset = end_offset;
        last_start = start;
    }
    Ok(result)
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
                    .starts_with("cloud-tts-")
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
    format!("ElevenLabs request failed: {error}")
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

    const TARGET: windows::core::PCWSTR = w!("ApriReader/ElevenLabsApiKey");

    pub fn read_key() -> Result<String, String> {
        let mut pointer = std::ptr::null_mut();
        unsafe {
            CredReadW(TARGET, CRED_TYPE_GENERIC, None, &mut pointer)
                .map_err(|_| "ElevenLabs API key is not configured".to_owned())?;
            let credential = &*pointer;
            if credential.CredentialBlob.is_null() || credential.CredentialBlobSize == 0 {
                CredFree(pointer.cast());
                return Err("the stored ElevenLabs API key is empty".to_owned());
            }
            let bytes = std::slice::from_raw_parts(
                credential.CredentialBlob,
                credential.CredentialBlobSize as usize,
            );
            let value = String::from_utf8(bytes.to_vec())
                .map_err(|_| "the stored ElevenLabs API key is invalid".to_owned());
            CredFree(pointer.cast());
            value
        }
    }

    pub fn write_key(key: &str) -> Result<(), String> {
        let mut target = "ApriReader/ElevenLabsApiKey\0"
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
        Err("ElevenLabs credentials are available only on Windows".to_owned())
    }

    pub fn write_key(_key: &str) -> Result<(), String> {
        Err("ElevenLabs credentials are available only on Windows".to_owned())
    }

    pub fn delete_key() -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_cloud_boundaries_without_exposing_keys() {
        assert!(validate_key("sk_valid_key").is_ok());
        assert!(validate_key("short").is_err());
        assert!(validate_key("secret with spaces").is_err());
        assert!(validate_voice_id("21m00Tcm4TlvDq8ikWAM").is_ok());
        assert!(validate_voice_id("../voice").is_err());
        assert!(validate_text("Readable").is_ok());
        assert!(validate_text(&"a".repeat(MAX_CLOUD_TEXT_CHARACTERS + 1)).is_err());
        assert!(validate_settings(CloudTtsSettings {
            stability: 0.5,
            similarity_boost: 0.75,
            style: 0.0,
            speaker_boost: true,
        })
        .is_ok());
        assert!(validate_settings(CloudTtsSettings {
            stability: 1.1,
            similarity_boost: 0.75,
            style: 0.0,
            speaker_boost: true,
        })
        .is_err());
    }

    #[test]
    fn converts_exact_character_alignment_to_utf16_offsets() {
        let timings = parse_alignment(
            "Я😀",
            Some(AlignmentResponse {
                characters: vec!["Я".to_owned(), "😀".to_owned()],
                character_start_times_seconds: vec![0.0, 0.2],
                character_end_times_seconds: vec![0.2, 0.6],
            }),
        )
        .expect("valid alignment");
        assert_eq!(timings[0].start_offset, 0);
        assert_eq!(timings[0].end_offset, 1);
        assert_eq!(timings[1].start_offset, 1);
        assert_eq!(timings[1].end_offset, 3);
    }
}
