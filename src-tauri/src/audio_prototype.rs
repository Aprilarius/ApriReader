use crate::audio_importer::{
    validate_audio_path, BLOCKED_DRM_EXTENSIONS, MAX_AUDIO_FILE_BYTES, NATIVE_EXTENSIONS,
    PLAYLIST_EXTENSIONS, SYSTEM_CODEC_EXTENSIONS,
};
use serde::Serialize;
use std::path::{Path, PathBuf};

pub const MIN_PLAYBACK_RATE: f64 = 0.5;
pub const MAX_PLAYBACK_RATE: f64 = 3.0;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioPrototypeCapabilities {
    pub backend: &'static str,
    pub platform_available: bool,
    pub service_ready: bool,
    pub initialization_error: Option<String>,
    pub native_extensions: &'static [&'static str],
    pub system_codec_extensions: &'static [&'static str],
    pub playlist_extensions: &'static [&'static str],
    pub blocked_drm_extensions: &'static [&'static str],
    pub min_playback_rate: f64,
    pub max_playback_rate: f64,
    pub max_file_bytes: u64,
    pub system_media_transport_controls: bool,
    pub background_playback: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioProbeResult {
    pub path: String,
    pub extension: String,
    pub file_size: u64,
    pub support_tier: String,
    pub media_source_created: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioOutputDevice {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub is_enabled: bool,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioPlaybackSnapshot {
    pub phase: String,
    pub path: Option<String>,
    pub position_seconds: f64,
    pub duration_seconds: f64,
    pub playback_rate: f64,
    pub volume: f64,
    pub can_seek: bool,
    pub can_pause: bool,
    pub last_error: Option<String>,
}

pub fn capabilities(
    service_ready: bool,
    initialization_error: Option<String>,
) -> AudioPrototypeCapabilities {
    AudioPrototypeCapabilities {
        backend: "Windows.Media.Playback.MediaPlayer",
        platform_available: cfg!(windows),
        service_ready,
        initialization_error,
        native_extensions: NATIVE_EXTENSIONS,
        system_codec_extensions: SYSTEM_CODEC_EXTENSIONS,
        playlist_extensions: PLAYLIST_EXTENSIONS,
        blocked_drm_extensions: BLOCKED_DRM_EXTENSIONS,
        min_playback_rate: MIN_PLAYBACK_RATE,
        max_playback_rate: MAX_PLAYBACK_RATE,
        max_file_bytes: MAX_AUDIO_FILE_BYTES,
        system_media_transport_controls: cfg!(windows),
        background_playback: cfg!(windows),
    }
}

#[cfg(windows)]
mod platform {
    use super::*;
    use std::sync::{mpsc, Arc, Mutex};
    use std::thread;
    use windows::{
        core::{IInspectable, HSTRING},
        Devices::Enumeration::{DeviceClass, DeviceInformation},
        Foundation::TypedEventHandler,
        Media::{
            Core::MediaSource,
            Playback::{MediaPlaybackState, MediaPlayer, MediaPlayerFailedEventArgs},
        },
        Storage::StorageFile,
        Win32::System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED},
    };

    type AudioResult<T> = Result<T, String>;

    enum Command {
        Probe(PathBuf, mpsc::Sender<AudioResult<AudioProbeResult>>),
        Load(PathBuf, mpsc::Sender<AudioResult<AudioPlaybackSnapshot>>),
        Play(mpsc::Sender<AudioResult<AudioPlaybackSnapshot>>),
        Pause(mpsc::Sender<AudioResult<AudioPlaybackSnapshot>>),
        Seek(f64, mpsc::Sender<AudioResult<AudioPlaybackSnapshot>>),
        SetRate(f64, mpsc::Sender<AudioResult<AudioPlaybackSnapshot>>),
        SetVolume(f64, mpsc::Sender<AudioResult<AudioPlaybackSnapshot>>),
        ListOutputDevices(mpsc::Sender<AudioResult<Vec<AudioOutputDevice>>>),
        SetOutputDevice(String, mpsc::Sender<AudioResult<AudioPlaybackSnapshot>>),
        Snapshot(mpsc::Sender<AudioResult<AudioPlaybackSnapshot>>),
        Stop(mpsc::Sender<AudioResult<AudioPlaybackSnapshot>>),
        Shutdown,
    }

    #[derive(Default)]
    struct EventState {
        phase: String,
        last_error: Option<String>,
    }

    pub struct AudioPrototypeService {
        sender: mpsc::Sender<Command>,
    }

    impl AudioPrototypeService {
        pub fn start() -> AudioResult<Self> {
            let (sender, receiver) = mpsc::channel();
            let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
            thread::Builder::new()
                .name("aprireader-audio".to_owned())
                .spawn(move || worker(receiver, ready_sender))
                .map_err(|error| format!("failed to start the audio service: {error}"))?;
            ready_receiver
                .recv()
                .map_err(|_| "the audio service stopped during startup".to_owned())??;
            Ok(Self { sender })
        }

        fn request<T>(
            &self,
            make_command: impl FnOnce(mpsc::Sender<AudioResult<T>>) -> Command,
        ) -> AudioResult<T> {
            let (response_sender, response_receiver) = mpsc::channel();
            self.sender
                .send(make_command(response_sender))
                .map_err(|_| "the audio service is unavailable".to_owned())?;
            response_receiver
                .recv()
                .map_err(|_| "the audio service did not return a response".to_owned())?
        }

        pub fn probe(&self, path: PathBuf) -> AudioResult<AudioProbeResult> {
            self.request(|response| Command::Probe(path, response))
        }

        pub fn load(&self, path: PathBuf) -> AudioResult<AudioPlaybackSnapshot> {
            self.request(|response| Command::Load(path, response))
        }

        pub fn play(&self) -> AudioResult<AudioPlaybackSnapshot> {
            self.request(Command::Play)
        }

        pub fn pause(&self) -> AudioResult<AudioPlaybackSnapshot> {
            self.request(Command::Pause)
        }

        pub fn seek(&self, seconds: f64) -> AudioResult<AudioPlaybackSnapshot> {
            self.request(|response| Command::Seek(seconds, response))
        }

        pub fn set_rate(&self, rate: f64) -> AudioResult<AudioPlaybackSnapshot> {
            self.request(|response| Command::SetRate(rate, response))
        }

        pub fn set_volume(&self, volume: f64) -> AudioResult<AudioPlaybackSnapshot> {
            self.request(|response| Command::SetVolume(volume, response))
        }

        pub fn list_output_devices(&self) -> AudioResult<Vec<AudioOutputDevice>> {
            self.request(Command::ListOutputDevices)
        }

        pub fn set_output_device(&self, device_id: String) -> AudioResult<AudioPlaybackSnapshot> {
            self.request(|response| Command::SetOutputDevice(device_id, response))
        }

        pub fn snapshot(&self) -> AudioResult<AudioPlaybackSnapshot> {
            self.request(Command::Snapshot)
        }

        pub fn stop(&self) -> AudioResult<AudioPlaybackSnapshot> {
            self.request(Command::Stop)
        }
    }

    impl Drop for AudioPrototypeService {
        fn drop(&mut self) {
            let _ = self.sender.send(Command::Shutdown);
        }
    }

    fn worker(receiver: mpsc::Receiver<Command>, ready: mpsc::SyncSender<AudioResult<()>>) {
        match unsafe { RoInitialize(RO_INIT_MULTITHREADED) } {
            Ok(()) => {}
            Err(error) => {
                let _ = ready.send(Err(format!("failed to initialize Windows Media: {error}")));
                return;
            }
        };
        let player = match create_player() {
            Ok(player) => player,
            Err(error) => {
                let _ = ready.send(Err(error));
                return;
            }
        };
        let event_state = Arc::new(Mutex::new(EventState {
            phase: "idle".to_owned(),
            last_error: None,
        }));
        let opened_state = Arc::clone(&event_state);
        let opened_token = player.MediaOpened(
            &TypedEventHandler::<MediaPlayer, IInspectable>::new(move |_, _| {
                if let Ok(mut state) = opened_state.lock() {
                    state.phase = "ready".to_owned();
                    state.last_error = None;
                }
                Ok(())
            }),
        );
        let failed_state = Arc::clone(&event_state);
        let failed_token = player.MediaFailed(&TypedEventHandler::<
            MediaPlayer,
            MediaPlayerFailedEventArgs,
        >::new(move |_, args| {
            if let Ok(mut state) = failed_state.lock() {
                state.phase = "failed".to_owned();
                state.last_error = args
                    .as_ref()
                    .and_then(|value| value.ErrorMessage().ok())
                    .map(|value| value.to_string())
                    .or_else(|| Some("Windows Media could not decode this file".to_owned()));
            }
            Ok(())
        }));
        let ended_state = Arc::clone(&event_state);
        let ended_token = player.MediaEnded(&TypedEventHandler::<MediaPlayer, IInspectable>::new(
            move |_, _| {
                if let Ok(mut state) = ended_state.lock() {
                    state.phase = "ended".to_owned();
                }
                Ok(())
            },
        ));
        if let Err(error) = opened_token
            .as_ref()
            .map(|_| ())
            .and(failed_token.as_ref().map(|_| ()))
            .and(ended_token.as_ref().map(|_| ()))
        {
            let _ = player.Close();
            let _ = ready.send(Err(format!("failed to register audio events: {error}")));
            return;
        }
        let opened_token = opened_token.expect("checked above");
        let failed_token = failed_token.expect("checked above");
        let ended_token = ended_token.expect("checked above");
        let _ = ready.send(Ok(()));

        let mut current_path: Option<PathBuf> = None;
        while let Ok(command) = receiver.recv() {
            match command {
                Command::Probe(path, response) => {
                    let _ = response.send(probe_file(&path));
                }
                Command::Load(path, response) => {
                    let result = load_file(&player, &event_state, &path).and_then(|canonical| {
                        current_path = Some(canonical);
                        snapshot(&player, &event_state, current_path.as_deref())
                    });
                    let _ = response.send(result);
                }
                Command::Play(response) => {
                    let result = require_source(&current_path)
                        .and_then(|_| win(player.Play()))
                        .and_then(|_| wait_for_playback_state(&player, MediaPlaybackState::Playing))
                        .and_then(|_| snapshot(&player, &event_state, current_path.as_deref()));
                    let _ = response.send(result);
                }
                Command::Pause(response) => {
                    let result = require_source(&current_path)
                        .and_then(|_| win(player.Pause()))
                        .and_then(|_| wait_for_playback_state(&player, MediaPlaybackState::Paused))
                        .and_then(|_| set_event_phase(&event_state, "paused"))
                        .and_then(|_| snapshot(&player, &event_state, current_path.as_deref()));
                    let _ = response.send(result);
                }
                Command::Seek(seconds, response) => {
                    let result = seek(&player, &current_path, seconds)
                        .and_then(|_| snapshot(&player, &event_state, current_path.as_deref()));
                    let _ = response.send(result);
                }
                Command::SetRate(rate, response) => {
                    let result = set_rate(&player, &current_path, rate)
                        .and_then(|_| snapshot(&player, &event_state, current_path.as_deref()));
                    let _ = response.send(result);
                }
                Command::SetVolume(volume, response) => {
                    let result = set_volume(&player, volume)
                        .and_then(|_| snapshot(&player, &event_state, current_path.as_deref()));
                    let _ = response.send(result);
                }
                Command::ListOutputDevices(response) => {
                    let _ = response.send(list_output_devices());
                }
                Command::SetOutputDevice(device_id, response) => {
                    let result = set_output_device(&player, &device_id)
                        .and_then(|_| snapshot(&player, &event_state, current_path.as_deref()));
                    let _ = response.send(result);
                }
                Command::Snapshot(response) => {
                    let _ = response.send(snapshot(&player, &event_state, current_path.as_deref()));
                }
                Command::Stop(response) => {
                    let result = win(player.Pause()).and_then(|_| {
                        wait_for_playback_state(&player, MediaPlaybackState::Paused)?;
                        let session = win(player.PlaybackSession())?;
                        win(session.SetPosition(windows::Foundation::TimeSpan { Duration: 0 }))?;
                        wait_for_position_reset(&session)?;
                        set_event_phase(&event_state, "ready")?;
                        snapshot(&player, &event_state, current_path.as_deref())
                    });
                    let _ = response.send(result);
                }
                Command::Shutdown => break,
            }
        }

        let _ = player.RemoveMediaOpened(opened_token);
        let _ = player.RemoveMediaFailed(failed_token);
        let _ = player.RemoveMediaEnded(ended_token);
        let _ = player.Close();
        unsafe { RoUninitialize() };
    }

    fn create_player() -> AudioResult<MediaPlayer> {
        let player = win(MediaPlayer::new())?;
        win(player.SetAutoPlay(false))?;
        win(player.SetVolume(1.0))?;
        Ok(player)
    }

    fn probe_file(path: &Path) -> AudioResult<AudioProbeResult> {
        let validated = validate_audio_path(path).map_err(|error| error.to_string())?;
        let canonical = validated.canonical;
        let file = win(StorageFile::GetFileFromPathAsync(&HSTRING::from(
            winrt_file_path(&canonical),
        )))?;
        let file = win(file.get())?;
        let _source = win(MediaSource::CreateFromStorageFile(&file))?;
        Ok(AudioProbeResult {
            path: canonical.to_string_lossy().into_owned(),
            extension: validated.extension,
            file_size: validated.file_size,
            support_tier: validated.support_tier.to_owned(),
            media_source_created: true,
        })
    }

    fn load_file(
        player: &MediaPlayer,
        event_state: &Arc<Mutex<EventState>>,
        path: &Path,
    ) -> AudioResult<PathBuf> {
        let canonical = validate_audio_path(path)
            .map_err(|error| error.to_string())?
            .canonical;
        let file = win(StorageFile::GetFileFromPathAsync(&HSTRING::from(
            winrt_file_path(&canonical),
        )))?;
        let file = win(file.get())?;
        let source = win(MediaSource::CreateFromStorageFile(&file))?;
        if let Ok(mut state) = event_state.lock() {
            state.phase = "opening".to_owned();
            state.last_error = None;
        }
        win(player.SetSource(&source))?;
        wait_for_media_opened(event_state)?;
        Ok(canonical)
    }

    fn seek(player: &MediaPlayer, current_path: &Option<PathBuf>, seconds: f64) -> AudioResult<()> {
        require_source(current_path)?;
        if !seconds.is_finite() || seconds < 0.0 {
            return Err("seek position must be a finite non-negative number".to_owned());
        }
        let ticks = (seconds * 10_000_000.0).round();
        if ticks > i64::MAX as f64 {
            return Err("seek position is too large".to_owned());
        }
        let session = win(player.PlaybackSession())?;
        if !win(session.CanSeek())? {
            return Err("the current audio source is not seekable".to_owned());
        }
        win(session.SetPosition(windows::Foundation::TimeSpan {
            Duration: ticks as i64,
        }))
    }

    fn set_rate(
        player: &MediaPlayer,
        current_path: &Option<PathBuf>,
        rate: f64,
    ) -> AudioResult<()> {
        require_source(current_path)?;
        if !rate.is_finite() || !(MIN_PLAYBACK_RATE..=MAX_PLAYBACK_RATE).contains(&rate) {
            return Err(format!(
                "playback rate must be between {MIN_PLAYBACK_RATE} and {MAX_PLAYBACK_RATE}"
            ));
        }
        let session = win(player.PlaybackSession())?;
        win(session.SetPlaybackRate(rate))
    }

    fn set_volume(player: &MediaPlayer, volume: f64) -> AudioResult<()> {
        if !volume.is_finite() || !(0.0..=1.0).contains(&volume) {
            return Err("volume must be between 0 and 1".to_owned());
        }
        win(player.SetVolume(volume))
    }

    fn list_output_devices() -> AudioResult<Vec<AudioOutputDevice>> {
        let operation = win(DeviceInformation::FindAllAsyncDeviceClass(
            DeviceClass::AudioRender,
        ))?;
        let devices = win(operation.get())?;
        let size = win(devices.Size())?.min(128);
        let mut output = Vec::with_capacity(size as usize);
        for index in 0..size {
            let device = win(devices.GetAt(index))?;
            output.push(AudioOutputDevice {
                id: win(device.Id())?.to_string(),
                name: win(device.Name())?.to_string(),
                is_default: device.IsDefault().unwrap_or(false),
                is_enabled: device.IsEnabled().unwrap_or(true),
            });
        }
        output.sort_by(|left, right| {
            right
                .is_default
                .cmp(&left.is_default)
                .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
        });
        Ok(output)
    }

    fn set_output_device(player: &MediaPlayer, device_id: &str) -> AudioResult<()> {
        if device_id.is_empty() || device_id.chars().count() > 2_048 {
            return Err("the audio output device identifier is invalid".to_owned());
        }
        let operation = win(DeviceInformation::CreateFromIdAsync(&HSTRING::from(
            device_id,
        )))?;
        let device = win(operation.get())?;
        if !device.IsEnabled().unwrap_or(true) {
            return Err("the selected audio output device is unavailable".to_owned());
        }
        win(player.SetAudioDevice(&device))
    }

    fn snapshot(
        player: &MediaPlayer,
        event_state: &Arc<Mutex<EventState>>,
        current_path: Option<&Path>,
    ) -> AudioResult<AudioPlaybackSnapshot> {
        let session = win(player.PlaybackSession())?;
        let state = win(session.PlaybackState())?;
        let event = event_state
            .lock()
            .map_err(|_| "the audio event state is unavailable".to_owned())?;
        let phase = resolve_playback_phase(state, &event.phase);
        Ok(AudioPlaybackSnapshot {
            phase: phase.to_owned(),
            path: current_path.map(|path| path.to_string_lossy().into_owned()),
            position_seconds: ticks_to_seconds(win(session.Position())?.Duration),
            duration_seconds: ticks_to_seconds(win(session.NaturalDuration())?.Duration),
            playback_rate: win(session.PlaybackRate())?,
            volume: win(player.Volume())?,
            can_seek: win(session.CanSeek())?,
            can_pause: win(session.CanPause())?,
            last_error: event.last_error.clone(),
        })
    }

    fn set_event_phase(event_state: &Arc<Mutex<EventState>>, phase: &str) -> AudioResult<()> {
        let mut event = event_state
            .lock()
            .map_err(|_| "the audio event state is unavailable".to_owned())?;
        event.phase = phase.to_owned();
        Ok(())
    }

    pub(super) fn resolve_playback_phase(state: MediaPlaybackState, event_phase: &str) -> &str {
        if event_phase == "failed" {
            return "failed";
        }
        match state {
            MediaPlaybackState::Opening => "opening",
            MediaPlaybackState::Buffering => "buffering",
            MediaPlaybackState::Playing => "playing",
            _ if event_phase == "ended" => "ended",
            MediaPlaybackState::Paused => "paused",
            _ if event_phase.is_empty() => "idle",
            _ => event_phase,
        }
    }

    fn require_source(current_path: &Option<PathBuf>) -> AudioResult<()> {
        current_path
            .as_ref()
            .map(|_| ())
            .ok_or_else(|| "load an audio file before controlling playback".to_owned())
    }

    fn ticks_to_seconds(ticks: i64) -> f64 {
        (ticks.max(0) as f64) / 10_000_000.0
    }

    fn wait_for_playback_state(
        player: &MediaPlayer,
        expected: MediaPlaybackState,
    ) -> AudioResult<()> {
        let session = win(player.PlaybackSession())?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(500);
        loop {
            if win(session.PlaybackState())? == expected {
                return Ok(());
            }
            if std::time::Instant::now() >= deadline {
                return Err("Windows Media did not apply the playback command in time".to_owned());
            }
            thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    fn wait_for_media_opened(event_state: &Arc<Mutex<EventState>>) -> AudioResult<()> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            let (phase, error) = event_state
                .lock()
                .map(|state| (state.phase.clone(), state.last_error.clone()))
                .map_err(|_| "the audio event state is unavailable".to_owned())?;
            match phase.as_str() {
                "ready" => return Ok(()),
                "failed" => {
                    return Err(error
                        .unwrap_or_else(|| "Windows Media could not decode this file".to_owned()))
                }
                _ => {}
            }
            if std::time::Instant::now() >= deadline {
                return Err("Windows Media did not open the audio source in time".to_owned());
            }
            thread::sleep(std::time::Duration::from_millis(20));
        }
    }

    fn wait_for_position_reset(
        session: &windows::Media::Playback::MediaPlaybackSession,
    ) -> AudioResult<()> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(500);
        loop {
            if win(session.Position())?.Duration <= 100_000 {
                return Ok(());
            }
            if std::time::Instant::now() >= deadline {
                return Err("Windows Media did not reset the playback position in time".to_owned());
            }
            thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    fn winrt_file_path(path: &Path) -> String {
        let path = path.to_string_lossy();
        if let Some(unc_path) = path.strip_prefix(r"\\?\UNC\") {
            format!(r"\\{unc_path}")
        } else if let Some(drive_path) = path.strip_prefix(r"\\?\") {
            drive_path.to_owned()
        } else {
            path.into_owned()
        }
    }

    fn win<T>(result: windows::core::Result<T>) -> AudioResult<T> {
        result.map_err(|error| format!("Windows Media error: {error}"))
    }
}

#[cfg(not(windows))]
mod platform {
    use super::*;

    pub struct AudioPrototypeService;

    impl AudioPrototypeService {
        pub fn start() -> Result<Self, String> {
            Err("the native audio prototype is currently available only on Windows".to_owned())
        }

        pub fn probe(&self, _path: PathBuf) -> Result<AudioProbeResult, String> {
            unavailable()
        }

        pub fn load(&self, _path: PathBuf) -> Result<AudioPlaybackSnapshot, String> {
            unavailable()
        }

        pub fn play(&self) -> Result<AudioPlaybackSnapshot, String> {
            unavailable()
        }

        pub fn pause(&self) -> Result<AudioPlaybackSnapshot, String> {
            unavailable()
        }

        pub fn seek(&self, _seconds: f64) -> Result<AudioPlaybackSnapshot, String> {
            unavailable()
        }

        pub fn set_rate(&self, _rate: f64) -> Result<AudioPlaybackSnapshot, String> {
            unavailable()
        }

        pub fn set_volume(&self, _volume: f64) -> Result<AudioPlaybackSnapshot, String> {
            unavailable()
        }

        pub fn list_output_devices(&self) -> Result<Vec<AudioOutputDevice>, String> {
            unavailable()
        }

        pub fn set_output_device(
            &self,
            _device_id: String,
        ) -> Result<AudioPlaybackSnapshot, String> {
            unavailable()
        }

        pub fn snapshot(&self) -> Result<AudioPlaybackSnapshot, String> {
            unavailable()
        }

        pub fn stop(&self) -> Result<AudioPlaybackSnapshot, String> {
            unavailable()
        }
    }

    fn unavailable<T>() -> Result<T, String> {
        Err("the native audio prototype is currently available only on Windows".to_owned())
    }
}

pub use platform::AudioPrototypeService;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn exposes_the_agreed_limits_and_drm_policy() {
        let capabilities = capabilities(cfg!(windows), None);
        assert_eq!(capabilities.max_file_bytes, 20 * 1024 * 1024 * 1024);
        assert_eq!(capabilities.min_playback_rate, 0.5);
        assert_eq!(capabilities.max_playback_rate, 3.0);
        assert!(capabilities.blocked_drm_extensions.contains(&"aax"));
        assert!(capabilities.playlist_extensions.contains(&"cue"));
    }

    #[test]
    fn validates_safe_audio_and_rejects_drm_or_playlist_inputs() {
        let directory = tempfile::tempdir().expect("temp directory");
        let wav = directory.path().join("sample.WAV");
        std::fs::File::create(&wav)
            .and_then(|mut file| file.write_all(b"RIFF"))
            .expect("audio fixture");
        let validated = validate_audio_path(&wav).expect("valid WAV path");
        assert_eq!(validated.extension, "wav");
        assert_eq!(validated.file_size, 4);
        assert_eq!(validated.support_tier, "windows-native");

        for name in ["protected.aax", "chapters.cue", "unknown.exe"] {
            let path = directory.path().join(name);
            std::fs::write(&path, b"fixture").expect("fixture");
            assert!(
                validate_audio_path(&path).is_err(),
                "{name} must be rejected"
            );
        }
    }

    #[cfg(windows)]
    #[test]
    fn media_ended_wins_over_the_paused_session_state() {
        use windows::Media::Playback::MediaPlaybackState;

        assert_eq!(
            platform::resolve_playback_phase(MediaPlaybackState::Paused, "ended"),
            "ended"
        );
        assert_eq!(
            platform::resolve_playback_phase(MediaPlaybackState::Paused, "paused"),
            "paused"
        );
        assert_eq!(
            platform::resolve_playback_phase(MediaPlaybackState::Playing, "ended"),
            "playing"
        );
    }

    #[cfg(windows)]
    #[test]
    fn native_service_opens_a_pcm_wave_source() {
        let directory = tempfile::tempdir().expect("temp directory");
        let wav = directory.path().join("silence.wav");
        write_silent_pcm_wave(&wav);

        let service = AudioPrototypeService::start().expect("native audio service");
        let probe = service
            .probe(wav.clone())
            .expect("Windows MediaSource probe");
        assert!(probe.media_source_created);
        let initial = service.load(wav).expect("load WAV source");
        assert_ne!(initial.phase, "failed", "{initial:?}");

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        loop {
            let snapshot = service.snapshot().expect("audio snapshot");
            if snapshot.phase == "ready" || snapshot.phase == "paused" {
                assert!(snapshot.duration_seconds >= 0.9, "{snapshot:?}");
                break;
            }
            assert_ne!(snapshot.phase, "failed", "{snapshot:?}");
            assert!(
                std::time::Instant::now() < deadline,
                "Windows MediaPlayer did not open the WAV source: {snapshot:?}"
            );
            std::thread::sleep(std::time::Duration::from_millis(25));
        }

        let snapshot = service.set_rate(1.25).expect("set playback rate");
        assert!((snapshot.playback_rate - 1.25).abs() < 0.001);
        let snapshot = service.set_volume(0.4).expect("set volume");
        assert!((snapshot.volume - 0.4).abs() < 0.001);
        let snapshot = service.seek(0.5).expect("seek");
        assert!(snapshot.position_seconds >= 0.45, "{snapshot:?}");
        service.play().expect("play");
        std::thread::sleep(std::time::Duration::from_millis(50));
        let snapshot = service.pause().expect("pause");
        assert_eq!(snapshot.phase, "paused", "{snapshot:?}");
        let snapshot = service.stop().expect("stop");
        assert!(snapshot.position_seconds < 0.05, "{snapshot:?}");
    }

    #[cfg(windows)]
    fn write_silent_pcm_wave(path: &Path) {
        const SAMPLE_RATE: u32 = 8_000;
        const SAMPLE_COUNT: u32 = SAMPLE_RATE;
        const DATA_BYTES: u32 = SAMPLE_COUNT * 2;
        let mut bytes = Vec::with_capacity((44 + DATA_BYTES) as usize);
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + DATA_BYTES).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
        bytes.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
        bytes.extend_from_slice(&2_u16.to_le_bytes());
        bytes.extend_from_slice(&16_u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&DATA_BYTES.to_le_bytes());
        bytes.resize((44 + DATA_BYTES) as usize, 0);
        std::fs::write(path, bytes).expect("WAV fixture");
    }
}
