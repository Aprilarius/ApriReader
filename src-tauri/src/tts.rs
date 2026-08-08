use crate::tts_assets::persist_cache_file;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{fs, path::Path, sync::mpsc};

pub const MAX_TTS_CHARACTERS: usize = 20_000;
pub const MAX_TTS_AUDIO_BYTES: usize = 64 * 1024 * 1024;
pub const MIN_TTS_RATE: f64 = 0.5;
pub const MAX_TTS_RATE: f64 = 2.0;
const MAX_CACHED_SEGMENTS: usize = 64;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsVoice {
    pub id: String,
    pub name: String,
    pub language: String,
    pub gender: String,
    pub is_default: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedTtsAudio {
    pub path: String,
    pub voice_id: String,
    pub character_count: usize,
}

struct SynthesizedAudio {
    bytes: Vec<u8>,
    voice_id: String,
    character_count: usize,
}

pub struct TtsService {
    sender: mpsc::Sender<Command>,
}

enum Command {
    ListVoices(mpsc::Sender<Result<Vec<TtsVoice>, String>>),
    Synthesize {
        text: String,
        voice_id: String,
        rate: f64,
        response: mpsc::Sender<Result<SynthesizedAudio, String>>,
    },
}

impl TtsService {
    pub fn start() -> Result<Self, String> {
        let (sender, receiver) = mpsc::channel();
        std::thread::Builder::new()
            .name("aprireader-tts".to_owned())
            .spawn(move || platform::worker(receiver))
            .map_err(|error| format!("failed to start the local speech worker: {error}"))?;
        Ok(Self { sender })
    }

    pub fn list_voices(&self) -> Result<Vec<TtsVoice>, String> {
        let (response, receiver) = mpsc::channel();
        self.sender
            .send(Command::ListVoices(response))
            .map_err(|_| "the local speech worker is unavailable".to_owned())?;
        receiver
            .recv()
            .map_err(|_| "the local speech worker stopped unexpectedly".to_owned())?
    }

    pub fn synthesize_to_cache(
        &self,
        text: &str,
        voice_id: &str,
        rate: f64,
        cache_dir: &Path,
    ) -> Result<PreparedTtsAudio, String> {
        validate_request(text, voice_id, rate)?;
        let (response, receiver) = mpsc::channel();
        self.sender
            .send(Command::Synthesize {
                text: text.to_owned(),
                voice_id: voice_id.to_owned(),
                rate,
                response,
            })
            .map_err(|_| "the local speech worker is unavailable".to_owned())?;
        let synthesized = receiver
            .recv()
            .map_err(|_| "the local speech worker stopped unexpectedly".to_owned())??;
        write_cached_audio(cache_dir, text, rate, synthesized)
    }
}

fn validate_request(text: &str, voice_id: &str, rate: f64) -> Result<(), String> {
    let characters = text.chars().count();
    if characters == 0 || characters > MAX_TTS_CHARACTERS {
        return Err(format!(
            "text-to-speech accepts between 1 and {MAX_TTS_CHARACTERS} characters"
        ));
    }
    if voice_id.trim().is_empty() || voice_id.chars().count() > 1024 {
        return Err("the selected speech voice is invalid".to_owned());
    }
    if !rate.is_finite() || !(MIN_TTS_RATE..=MAX_TTS_RATE).contains(&rate) {
        return Err(format!(
            "speech rate must be between {MIN_TTS_RATE} and {MAX_TTS_RATE}"
        ));
    }
    Ok(())
}

fn write_cached_audio(
    cache_dir: &Path,
    text: &str,
    rate: f64,
    synthesized: SynthesizedAudio,
) -> Result<PreparedTtsAudio, String> {
    if synthesized.bytes.len() > MAX_TTS_AUDIO_BYTES || !synthesized.bytes.starts_with(b"RIFF") {
        return Err("Windows returned an invalid or oversized speech stream".to_owned());
    }
    fs::create_dir_all(cache_dir).map_err(|error| error.to_string())?;
    let mut digest = Sha256::new();
    digest.update(synthesized.voice_id.as_bytes());
    digest.update(rate.to_le_bytes());
    digest.update(text.as_bytes());
    let name = format!("tts-{:x}.wav", digest.finalize());
    let destination = cache_dir.join(name);
    persist_cache_file(&destination, &synthesized.bytes)?;
    prune_cache(cache_dir, &destination);
    Ok(PreparedTtsAudio {
        path: destination.to_string_lossy().into_owned(),
        voice_id: synthesized.voice_id,
        character_count: synthesized.character_count,
    })
}

fn prune_cache(cache_dir: &Path, current: &Path) {
    let Ok(entries) = fs::read_dir(cache_dir) else {
        return;
    };
    let mut files = entries
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_type().is_ok_and(|kind| kind.is_file())
                && entry.file_name().to_string_lossy().starts_with("tts-")
                && entry.path().extension().is_some_and(|value| value == "wav")
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

#[cfg(windows)]
mod platform {
    use super::*;
    use std::os::windows::ffi::OsStrExt;
    use windows::{
        core::{w, HSTRING, PCWSTR, PWSTR},
        Media::SpeechSynthesis::{SpeechSynthesizer, VoiceGender, VoiceInformation},
        Storage::Streams::DataReader,
        Win32::{
            Globalization::LCIDToLocaleName,
            Media::Speech::{
                ISpObjectToken, ISpObjectTokenCategory, ISpStream, ISpVoice, SpFileStream,
                SpObjectTokenCategory, SpVoice, SPCAT_VOICES, SPFM_CREATE_ALWAYS, SPF_IS_NOT_XML,
            },
            System::{
                Com::{CoCreateInstance, CoTaskMemFree, CLSCTX_ALL},
                WinRT::{RoInitialize, RoUninitialize, RO_INIT_SINGLETHREADED},
            },
        },
    };

    const SAPI_VOICE_PREFIX: &str = "sapi:";

    pub fn worker(receiver: mpsc::Receiver<Command>) {
        let initialized = unsafe { RoInitialize(RO_INIT_SINGLETHREADED) }.is_ok();
        if !initialized {
            fail_all(receiver, "Windows speech initialization failed");
            return;
        }
        let synthesizer = SpeechSynthesizer::new().map_err(|error| error.to_string());
        for command in receiver {
            match command {
                Command::ListVoices(response) => {
                    let result = synthesizer
                        .as_ref()
                        .map_err(Clone::clone)
                        .and_then(|_| list_winrt_voices())
                        .and_then(|voices| {
                            if voices.is_empty() {
                                Err("Windows Runtime did not report any installed voices"
                                    .to_owned())
                            } else {
                                Ok(voices)
                            }
                        })
                        .or_else(|_| list_sapi_voices());
                    let _ = response.send(result);
                }
                Command::Synthesize {
                    text,
                    voice_id,
                    rate,
                    response,
                } => {
                    let result = if voice_id.starts_with(SAPI_VOICE_PREFIX) {
                        synthesize_sapi(&text, &voice_id, rate)
                    } else {
                        synthesizer
                            .as_ref()
                            .map_err(Clone::clone)
                            .and_then(|value| synthesize_winrt(value, &text, &voice_id, rate))
                    };
                    let _ = response.send(result);
                }
            }
        }
        if let Ok(value) = synthesizer {
            let _ = value.Close();
        }
        unsafe { RoUninitialize() };
    }

    fn fail_all(receiver: mpsc::Receiver<Command>, message: &str) {
        for command in receiver {
            match command {
                Command::ListVoices(response) => {
                    let _ = response.send(Err(message.to_owned()));
                }
                Command::Synthesize { response, .. } => {
                    let _ = response.send(Err(message.to_owned()));
                }
            }
        }
    }

    fn installed_voices() -> Result<Vec<VoiceInformation>, String> {
        let voices = SpeechSynthesizer::AllVoices().map_err(|error| error.to_string())?;
        let mut result =
            Vec::with_capacity(voices.Size().map_err(|error| error.to_string())? as usize);
        for index in 0..voices.Size().map_err(|error| error.to_string())? {
            result.push(voices.GetAt(index).map_err(|error| error.to_string())?);
        }
        Ok(result)
    }

    fn list_winrt_voices() -> Result<Vec<TtsVoice>, String> {
        let default_id = SpeechSynthesizer::DefaultVoice()
            .and_then(|voice| voice.Id())
            .map(|value| value.to_string())
            .unwrap_or_default();
        let mut voices = installed_voices()?
            .into_iter()
            .map(|voice| {
                let id = voice.Id().map_err(|error| error.to_string())?.to_string();
                let gender = match voice.Gender().map_err(|error| error.to_string())? {
                    VoiceGender::Female => "female",
                    VoiceGender::Male => "male",
                    _ => "unknown",
                };
                Ok(TtsVoice {
                    is_default: id == default_id,
                    id,
                    name: voice
                        .DisplayName()
                        .map_err(|error| error.to_string())?
                        .to_string(),
                    language: voice
                        .Language()
                        .map_err(|error| error.to_string())?
                        .to_string(),
                    gender: gender.to_owned(),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        voices.sort_by(|left, right| {
            right
                .is_default
                .cmp(&left.is_default)
                .then_with(|| left.language.cmp(&right.language))
                .then_with(|| left.name.cmp(&right.name))
        });
        Ok(voices)
    }

    fn synthesize_winrt(
        synthesizer: &SpeechSynthesizer,
        text: &str,
        voice_id: &str,
        rate: f64,
    ) -> Result<SynthesizedAudio, String> {
        validate_request(text, voice_id, rate)?;
        let voice = installed_voices()?
            .into_iter()
            .find(|voice| voice.Id().is_ok_and(|id| id == voice_id))
            .ok_or_else(|| "the selected Windows speech voice is no longer installed".to_owned())?;
        synthesizer
            .SetVoice(&voice)
            .map_err(|error| error.to_string())?;
        let options = synthesizer.Options().map_err(|error| error.to_string())?;
        options
            .SetSpeakingRate(rate)
            .map_err(|error| error.to_string())?;
        options
            .SetIncludeWordBoundaryMetadata(true)
            .map_err(|error| error.to_string())?;
        let stream = synthesizer
            .SynthesizeTextToStreamAsync(&HSTRING::from(text))
            .and_then(|operation| operation.get())
            .map_err(|error| error.to_string())?;
        let size = usize::try_from(stream.Size().map_err(|error| error.to_string())?)
            .map_err(|_| "Windows returned an oversized speech stream".to_owned())?;
        if size == 0 || size > MAX_TTS_AUDIO_BYTES || size > u32::MAX as usize {
            return Err("Windows returned an invalid or oversized speech stream".to_owned());
        }
        let input = stream
            .GetInputStreamAt(0)
            .map_err(|error| error.to_string())?;
        let reader = DataReader::CreateDataReader(&input).map_err(|error| error.to_string())?;
        let loaded = reader
            .LoadAsync(size as u32)
            .and_then(|operation| operation.get())
            .map_err(|error| error.to_string())?;
        if loaded as usize != size {
            return Err("Windows returned an incomplete speech stream".to_owned());
        }
        let mut bytes = vec![0; size];
        reader
            .ReadBytes(&mut bytes)
            .map_err(|error| error.to_string())?;
        let _ = reader.Close();
        let _ = stream.Close();
        Ok(SynthesizedAudio {
            bytes,
            voice_id: voice_id.to_owned(),
            character_count: text.chars().count(),
        })
    }

    fn sapi_category() -> Result<ISpObjectTokenCategory, String> {
        let category: ISpObjectTokenCategory = unsafe {
            CoCreateInstance(&SpObjectTokenCategory, None, CLSCTX_ALL)
                .map_err(|error| error.to_string())?
        };
        unsafe {
            category
                .SetId(SPCAT_VOICES, false)
                .map_err(|error| error.to_string())?;
        }
        Ok(category)
    }

    fn sapi_tokens() -> Result<Vec<ISpObjectToken>, String> {
        let category = sapi_category()?;
        let tokens = unsafe {
            category
                .EnumTokens(PCWSTR::null(), PCWSTR::null())
                .map_err(|error| error.to_string())?
        };
        let mut count = 0;
        unsafe {
            tokens
                .GetCount(&mut count)
                .map_err(|error| error.to_string())?;
        }
        (0..count)
            .map(|index| unsafe { tokens.Item(index).map_err(|error| error.to_string()) })
            .collect()
    }

    fn take_com_string(value: PWSTR) -> Result<String, String> {
        if value.is_null() {
            return Ok(String::new());
        }
        let result = unsafe { value.to_string().map_err(|error| error.to_string()) };
        unsafe { CoTaskMemFree(Some(value.as_ptr().cast())) };
        result
    }

    fn token_id(token: &ISpObjectToken) -> Result<String, String> {
        take_com_string(unsafe { token.GetId().map_err(|error| error.to_string())? })
    }

    fn token_value(token: &ISpObjectToken, name: PCWSTR) -> String {
        unsafe { token.GetStringValue(name) }
            .map_err(|error| error.to_string())
            .and_then(take_com_string)
            .unwrap_or_default()
    }

    fn token_attribute(token: &ISpObjectToken, name: PCWSTR) -> String {
        unsafe { token.OpenKey(w!("Attributes")) }
            .map_err(|error| error.to_string())
            .and_then(|attributes| {
                unsafe { attributes.GetStringValue(name) }
                    .map_err(|error| error.to_string())
                    .and_then(take_com_string)
            })
            .unwrap_or_default()
    }

    fn sapi_language(raw: &str) -> String {
        let Some(value) = raw.split(';').next() else {
            return String::new();
        };
        let Ok(lcid) = u32::from_str_radix(value.trim(), 16) else {
            return raw.to_owned();
        };
        let mut locale = [0u16; 85];
        let length = unsafe { LCIDToLocaleName(lcid, Some(&mut locale), 0) };
        if length <= 1 {
            raw.to_owned()
        } else {
            String::from_utf16_lossy(&locale[..length as usize - 1])
        }
    }

    fn list_sapi_voices() -> Result<Vec<TtsVoice>, String> {
        let default_id = sapi_category()
            .and_then(|category| {
                take_com_string(unsafe {
                    category
                        .GetDefaultTokenId()
                        .map_err(|error| error.to_string())?
                })
            })
            .unwrap_or_default();
        let mut voices = sapi_tokens()?
            .into_iter()
            .filter_map(|token| {
                let raw_id = token_id(&token).ok()?;
                let name = token_value(&token, PCWSTR::null());
                let language = sapi_language(&token_attribute(&token, w!("Language")));
                let gender = token_attribute(&token, w!("Gender")).to_ascii_lowercase();
                Some(TtsVoice {
                    is_default: raw_id == default_id,
                    id: format!("{SAPI_VOICE_PREFIX}{raw_id}"),
                    name: if name.is_empty() {
                        raw_id.clone()
                    } else {
                        name
                    },
                    language,
                    gender: match gender.as_str() {
                        "female" => "female".to_owned(),
                        "male" => "male".to_owned(),
                        _ => "unknown".to_owned(),
                    },
                })
            })
            .collect::<Vec<_>>();
        voices.sort_by(|left, right| {
            right
                .is_default
                .cmp(&left.is_default)
                .then_with(|| left.language.cmp(&right.language))
                .then_with(|| left.name.cmp(&right.name))
        });
        if voices.is_empty() {
            Err("Windows did not report any installed speech voices".to_owned())
        } else {
            Ok(voices)
        }
    }

    fn synthesize_sapi(text: &str, voice_id: &str, rate: f64) -> Result<SynthesizedAudio, String> {
        validate_request(text, voice_id, rate)?;
        let requested_id = voice_id
            .strip_prefix(SAPI_VOICE_PREFIX)
            .ok_or_else(|| "the selected Windows speech voice is invalid".to_owned())?;
        let token = sapi_tokens()?
            .into_iter()
            .find(|token| token_id(token).is_ok_and(|id| id == requested_id))
            .ok_or_else(|| "the selected Windows speech voice is no longer installed".to_owned())?;

        let mut digest = Sha256::new();
        digest.update(voice_id.as_bytes());
        digest.update(rate.to_le_bytes());
        digest.update(text.as_bytes());
        digest.update(std::process::id().to_le_bytes());
        let path = std::env::temp_dir().join(format!("aprireader-tts-{:x}.wav", digest.finalize()));
        let path_wide = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();

        let synthesis = (|| -> Result<(), String> {
            let voice: ISpVoice = unsafe {
                CoCreateInstance(&SpVoice, None, CLSCTX_ALL).map_err(|error| error.to_string())?
            };
            let stream: ISpStream = unsafe {
                CoCreateInstance(&SpFileStream, None, CLSCTX_ALL)
                    .map_err(|error| error.to_string())?
            };
            unsafe {
                stream
                    .BindToFile(
                        PCWSTR(path_wide.as_ptr()),
                        SPFM_CREATE_ALWAYS,
                        None,
                        None,
                        0,
                    )
                    .map_err(|error| format!("SAPI could not create the speech WAV: {error}"))?;
                voice
                    .SetVoice(&token)
                    .map_err(|error| format!("SAPI could not select the voice: {error}"))?;
                voice
                    .SetRate((rate.log2() * 5.0).round().clamp(-5.0, 5.0) as i32)
                    .map_err(|error| format!("SAPI could not set the speech rate: {error}"))?;
                voice
                    .SetOutput(&stream, true)
                    .map_err(|error| format!("SAPI could not select the WAV output: {error}"))?;
                let text_wide = text
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .collect::<Vec<_>>();
                voice
                    .Speak(PCWSTR(text_wide.as_ptr()), SPF_IS_NOT_XML.0 as u32, None)
                    .map_err(|error| format!("SAPI could not synthesize the text: {error}"))?;
                stream
                    .Close()
                    .map_err(|error| format!("SAPI could not finalize the speech WAV: {error}"))?;
            }
            Ok(())
        })();
        if let Err(error) = synthesis {
            let _ = fs::remove_file(&path);
            return Err(error);
        }
        let bytes = fs::read(&path).map_err(|error| error.to_string());
        let _ = fs::remove_file(&path);
        let bytes = bytes?;
        if bytes.is_empty() || bytes.len() > MAX_TTS_AUDIO_BYTES {
            return Err("Windows returned an invalid or oversized speech stream".to_owned());
        }
        Ok(SynthesizedAudio {
            bytes,
            voice_id: voice_id.to_owned(),
            character_count: text.chars().count(),
        })
    }
}

#[cfg(not(windows))]
mod platform {
    use super::*;

    pub fn worker(receiver: mpsc::Receiver<Command>) {
        for command in receiver {
            match command {
                Command::ListVoices(response) => {
                    let _ =
                        response.send(Err("local speech is available only on Windows".to_owned()));
                }
                Command::Synthesize { response, .. } => {
                    let _ =
                        response.send(Err("local speech is available only on Windows".to_owned()));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_local_speech_requests() {
        assert!(validate_request("Readable text", "voice", 1.0).is_ok());
        assert!(validate_request("", "voice", 1.0).is_err());
        assert!(validate_request(&"a".repeat(MAX_TTS_CHARACTERS + 1), "voice", 1.0).is_err());
        assert!(validate_request("text", "", 1.0).is_err());
        assert!(validate_request("text", "voice", 0.49).is_err());
        assert!(validate_request("text", "voice", 2.01).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn native_service_synthesizes_a_bounded_wave_stream() {
        let directory = tempfile::tempdir().expect("temporary cache");
        let service = TtsService::start().expect("speech service");
        let voices = service.list_voices().expect("installed voices");
        let mut errors = Vec::new();
        let prepared = voices
            .iter()
            .find_map(|voice| {
                match service.synthesize_to_cache(
                    "ApriReader local speech test.",
                    &voice.id,
                    1.0,
                    directory.path(),
                ) {
                    Ok(prepared) => Some(prepared),
                    Err(error) => {
                        errors.push(format!("{}: {error}", voice.name));
                        None
                    }
                }
            })
            .or_else(|| {
                eprintln!(
                    "native speech smoke skipped because Windows has no usable voice: {errors:?}"
                );
                None
            });
        let Some(prepared) = prepared else {
            return;
        };
        let bytes = fs::read(prepared.path).expect("cached wave");
        assert!(bytes.starts_with(b"RIFF"));
        assert!(bytes.len() <= MAX_TTS_AUDIO_BYTES);
    }
}
