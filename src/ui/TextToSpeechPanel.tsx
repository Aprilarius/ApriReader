import { useEffect, useMemo, useRef, useState } from "react";
import {
  getAudioSnapshot,
  loadAudioFile,
  pauseAudio,
  playAudio,
  setAudioRate,
  stopAudio,
  type AudioPlaybackSnapshot,
} from "../application/audioPlayer";
import {
  deleteAzureTtsKey,
  getAzureTtsStatus,
  listAzureTtsRegions,
  listAzureTtsVoices,
  prepareAzureTtsSection,
  saveAzureTtsKey,
  type AzureTtsRegion,
  type AzureTtsVoice,
  type PreparedAzureTtsAudio,
} from "../application/azureTts";
import {
  deleteCloudTtsKey,
  getCloudTtsStatus,
  listCloudTtsVoices,
  prepareCloudTtsSection,
  saveCloudTtsKey,
  type CloudTtsTiming,
  type CloudTtsVoice,
  type PreparedCloudTtsAudio,
} from "../application/cloudTts";
import {
  deleteGoogleTtsKey,
  getGoogleTtsStatus,
  listGoogleTtsVoices,
  prepareGoogleTtsSection,
  saveGoogleTtsKey,
  type GoogleTtsVoice,
  type PreparedGoogleTtsAudio,
} from "../application/googleTts";
import type { DocumentSection } from "../application/reader";
import {
  appendTtsExportPart,
  beginTtsExport,
  cancelTtsExport,
  chooseTtsExportPath,
  clearTtsCache,
  finishTtsExport,
  getTtsCacheSummary,
  maxTtsExportParts,
  type TtsCacheSummary,
} from "../application/ttsAssets";
import {
  listTtsVoices,
  prepareTtsSection,
  type PreparedTtsAudio,
  type TtsVoice,
} from "../application/tts";
import {
  buildTtsQueue,
  highlightAtPlaybackPosition,
  type TtsHighlightRange,
  type TtsQueueChunk,
  type TtsReadingScope,
} from "../application/ttsQueue";
import {
  applyPronunciationDictionary,
  createTtsPreferenceId,
  maxTtsPronunciationRules,
  maxTtsVoicePresets,
  normalizePronunciationRule,
  normalizeVoicePreset,
  parseTtsPreferences,
  remapCloudTtsTimings,
  ttsPreferencesStorageKey,
  type TtsPreferences,
  type TtsProvider,
} from "../application/ttsPreferences";
import type { TranslationKey } from "./i18n";
import { readLocalValue, writeLocalValue } from "./localStorage";

type Translator = (key: TranslationKey) => string;

const voiceKey = "aprireader.tts.voice";
const cloudVoiceKey = "aprireader.tts.elevenlabs.voice";
const providerKey = "aprireader.tts.provider";
const cloudConsentKey = "aprireader.tts.elevenlabs.consent.v1";
const googleVoiceKey = "aprireader.tts.google.voice";
const googleConsentKey = "aprireader.tts.google.consent.v1";
const azureVoiceKey = "aprireader.tts.azure.voice";
const azureRegionKey = "aprireader.tts.azure.region";
const azureConsentKey = "aprireader.tts.azure.consent.v1";
const rateKey = "aprireader.tts.rate";
const scopeKey = "aprireader.tts.scope";
const expressiveKey = "aprireader.tts.expressive.v1";
const rates = [0.5, 0.75, 1, 1.25, 1.5, 1.75, 2] as const;
type PreparedPlaybackAudio =
  | PreparedTtsAudio
  | PreparedCloudTtsAudio
  | PreparedGoogleTtsAudio
  | PreparedAzureTtsAudio;

function readRate() {
  const value = Number(readLocalValue(rateKey));
  return rates.includes(value as (typeof rates)[number]) ? value : 1;
}

function readScope(): TtsReadingScope {
  return readLocalValue(scopeKey) === "book" ? "book" : "section";
}

function readProvider(): TtsProvider {
  const value = readLocalValue(providerKey);
  return value === "elevenlabs" || value === "google" || value === "azure"
    ? value
    : "local";
}

type ExpressiveSettings = {
  stability: number;
  similarityBoost: number;
  style: number;
  speakerBoost: boolean;
  googlePitch: number;
  azurePitch: number;
};

const defaultExpressiveSettings: ExpressiveSettings = {
  stability: 0.5,
  similarityBoost: 0.75,
  style: 0,
  speakerBoost: true,
  googlePitch: 0,
  azurePitch: 0,
};

function readExpressiveSettings(): ExpressiveSettings {
  try {
    const raw = readLocalValue(expressiveKey);
    if (!raw || raw.length > 2_048) return defaultExpressiveSettings;
    const value = JSON.parse(raw) as Partial<ExpressiveSettings>;
    return {
      stability: validRange(value.stability, 0, 1, 0.5),
      similarityBoost: validRange(value.similarityBoost, 0, 1, 0.75),
      style: validRange(value.style, 0, 1, 0),
      speakerBoost:
        typeof value.speakerBoost === "boolean" ? value.speakerBoost : true,
      googlePitch: validRange(value.googlePitch, -20, 20, 0),
      azurePitch: Math.round(validRange(value.azurePitch, -50, 50, 0)),
    };
  } catch {
    return defaultExpressiveSettings;
  }
}

function validRange(
  value: unknown,
  minimum: number,
  maximum: number,
  fallback: number,
) {
  return typeof value === "number" &&
    Number.isFinite(value) &&
    value >= minimum &&
    value <= maximum
    ? value
    : fallback;
}

function formatBytes(value: number) {
  if (value < 1_024) return `${value} B`;
  if (value < 1_024 ** 2) return `${(value / 1_024).toFixed(1)} KiB`;
  if (value < 1_024 ** 3) return `${(value / 1_024 ** 2).toFixed(1)} MiB`;
  return `${(value / 1_024 ** 3).toFixed(2)} GiB`;
}

export function TextToSpeechPanel({
  title,
  sections,
  currentSectionIndex,
  language,
  t,
  onNavigate,
  onHighlight,
}: {
  title: string;
  sections: DocumentSection[];
  currentSectionIndex: number;
  language?: string;
  t: Translator;
  onNavigate: (sectionIndex: number) => void;
  onHighlight: (range: TtsHighlightRange | null) => void;
}) {
  const [voices, setVoices] = useState<TtsVoice[]>([]);
  const [voiceId, setVoiceId] = useState(readLocalValue(voiceKey) ?? "");
  const [provider, setProvider] = useState<TtsProvider>(readProvider);
  const [cloudConfigured, setCloudConfigured] = useState(false);
  const [cloudVoices, setCloudVoices] = useState<CloudTtsVoice[]>([]);
  const [cloudVoiceId, setCloudVoiceId] = useState(
    readLocalValue(cloudVoiceKey) ?? "",
  );
  const [apiKey, setApiKey] = useState("");
  const [cloudConsent, setCloudConsent] = useState(
    readLocalValue(cloudConsentKey) === "accepted",
  );
  const [showCloudConsent, setShowCloudConsent] = useState(false);
  const [googleConfigured, setGoogleConfigured] = useState(false);
  const [googleVoices, setGoogleVoices] = useState<GoogleTtsVoice[]>([]);
  const [googleVoiceId, setGoogleVoiceId] = useState(
    readLocalValue(googleVoiceKey) ?? "",
  );
  const [googleApiKey, setGoogleApiKey] = useState("");
  const [googleConsent, setGoogleConsent] = useState(
    readLocalValue(googleConsentKey) === "accepted",
  );
  const [showGoogleConsent, setShowGoogleConsent] = useState(false);
  const [azureConfigured, setAzureConfigured] = useState(false);
  const [azureRegions, setAzureRegions] = useState<AzureTtsRegion[]>([]);
  const [azureRegion, setAzureRegion] = useState(
    readLocalValue(azureRegionKey) ?? "westeurope",
  );
  const [azureVoices, setAzureVoices] = useState<AzureTtsVoice[]>([]);
  const [azureVoiceId, setAzureVoiceId] = useState(
    readLocalValue(azureVoiceKey) ?? "",
  );
  const [azureApiKey, setAzureApiKey] = useState("");
  const [azureConsent, setAzureConsent] = useState(
    readLocalValue(azureConsentKey) === "accepted",
  );
  const [showAzureConsent, setShowAzureConsent] = useState(false);
  const [rate, setRate] = useState(readRate);
  const [scope, setScope] = useState<TtsReadingScope>(readScope);
  const [expressive, setExpressive] = useState(readExpressiveSettings);
  const [cacheSummary, setCacheSummary] = useState<TtsCacheSummary | null>(
    null,
  );
  const [showExportConfirm, setShowExportConfirm] = useState(false);
  const [exportProgress, setExportProgress] = useState({
    current: 0,
    total: 0,
  });
  const [preferences, setPreferences] = useState(() =>
    parseTtsPreferences(readLocalValue(ttsPreferencesStorageKey)),
  );
  const [selectedPresetId, setSelectedPresetId] = useState("");
  const [presetName, setPresetName] = useState("");
  const [pronunciationId, setPronunciationId] = useState("");
  const [pronunciationSource, setPronunciationSource] = useState("");
  const [pronunciationReplacement, setPronunciationReplacement] = useState("");
  const [snapshot, setSnapshot] = useState<AudioPlaybackSnapshot | null>(null);
  const [chunkPosition, setChunkPosition] = useState({ index: 0, total: 0 });
  const [busy, setBusy] = useState(true);
  const [message, setMessage] = useState("");
  const ownsPlayback = useRef(false);
  const advancing = useRef(false);
  const generation = useRef(0);
  const queue = useRef<TtsQueueChunk[]>([]);
  const queueIndex = useRef(0);
  const expectedSection = useRef(currentSectionIndex);
  const prepared = useRef(new Map<string, Promise<PreparedPlaybackAudio>>());
  const activeTimings = useRef<CloudTtsTiming[]>([]);
  const advanceRef = useRef<() => Promise<void>>(() => Promise.resolve());
  const stopRef = useRef<() => Promise<void>>(() => Promise.resolve());
  const exportCancelled = useRef(false);
  const exportSession = useRef("");
  const pendingConsentAction = useRef<"play" | "export">("play");
  const currentSection = sections[currentSectionIndex];
  const googleLanguageFilter =
    language && /^[A-Za-z]{2,3}(?:-[A-Za-z0-9]{2,8})*$/u.test(language)
      ? language
      : undefined;
  const characterCount = useMemo(
    () =>
      currentSection
        ? Array.from(
            [
              currentSection.title,
              ...currentSection.blocks.map((block) => block.text),
            ].join("\n\n"),
          ).length
        : 0,
    [currentSection],
  );
  const selectedVoiceId =
    provider === "local"
      ? voiceId
      : provider === "elevenlabs"
        ? cloudVoiceId
        : provider === "google"
          ? googleVoiceId
          : azureVoiceId;
  const selectedGoogleVoice = googleVoices.find(
    (voice) => voice.id === googleVoiceId,
  );
  const selectedAzureVoice = azureVoices.find(
    (voice) => voice.id === azureVoiceId,
  );

  const persistPreferences = (next: TtsPreferences) => {
    setPreferences(next);
    writeLocalValue(ttsPreferencesStorageKey, JSON.stringify(next));
  };

  const persistExpressive = (next: ExpressiveSettings) => {
    resetSession();
    setExpressive(next);
    writeLocalValue(expressiveKey, JSON.stringify(next));
  };

  useEffect(() => {
    let disposed = false;
    void listTtsVoices()
      .then((items) => {
        if (disposed) return;
        setVoices(items);
        setVoiceId((current) => {
          if (items.some((voice) => voice.id === current)) return current;
          const languagePrefix = language?.split("-")[0]?.toLowerCase();
          return (
            (
              items.find(
                (voice) =>
                  languagePrefix &&
                  voice.language.toLowerCase().startsWith(languagePrefix),
              ) ??
              items.find((voice) => voice.isDefault) ??
              items[0]
            )?.id ?? ""
          );
        });
      })
      .catch((reason: unknown) =>
        setMessage(reason instanceof Error ? reason.message : String(reason)),
      )
      .finally(() => {
        if (!disposed) setBusy(false);
      });
    return () => {
      disposed = true;
      void stopRef.current();
    };
  }, [language]);

  useEffect(() => {
    let disposed = false;
    void getCloudTtsStatus()
      .then((status) => {
        if (!disposed) setCloudConfigured(status.configured);
      })
      .catch(() => undefined);
    return () => {
      disposed = true;
    };
  }, []);

  useEffect(() => {
    void getTtsCacheSummary()
      .then(setCacheSummary)
      .catch(() => undefined);
  }, []);

  useEffect(() => {
    let disposed = false;
    void getGoogleTtsStatus()
      .then((status) => {
        if (!disposed) setGoogleConfigured(status.configured);
      })
      .catch(() => undefined);
    return () => {
      disposed = true;
    };
  }, []);

  useEffect(() => {
    let disposed = false;
    void Promise.all([getAzureTtsStatus(), listAzureTtsRegions()])
      .then(([status, regions]) => {
        if (disposed) return;
        setAzureConfigured(status.configured);
        setAzureRegions(regions);
        setAzureRegion((current) =>
          regions.some((region) => region.id === current)
            ? current
            : (regions.find((region) => region.id === "westeurope")?.id ??
              regions[0]?.id ??
              ""),
        );
      })
      .catch(() => undefined);
    return () => {
      disposed = true;
    };
  }, []);

  useEffect(() => {
    if (provider !== "elevenlabs" || !cloudConfigured) return;
    let disposed = false;
    setBusy(true);
    setMessage("");
    void listCloudTtsVoices()
      .then((items) => {
        if (disposed) return;
        setCloudVoices(items);
        setCloudVoiceId((current) =>
          items.some((voice) => voice.id === current)
            ? current
            : (items[0]?.id ?? ""),
        );
      })
      .catch((reason: unknown) => {
        if (!disposed)
          setMessage(reason instanceof Error ? reason.message : String(reason));
      })
      .finally(() => {
        if (!disposed) setBusy(false);
      });
    return () => {
      disposed = true;
    };
  }, [cloudConfigured, provider]);

  useEffect(() => {
    if (provider !== "google" || !googleConfigured) return;
    let disposed = false;
    setBusy(true);
    setMessage("");
    void listGoogleTtsVoices(googleLanguageFilter)
      .then((items) => {
        if (disposed) return;
        setGoogleVoices(items);
        setGoogleVoiceId((current) =>
          items.some((voice) => voice.id === current)
            ? current
            : (items[0]?.id ?? ""),
        );
      })
      .catch((reason: unknown) => {
        if (!disposed)
          setMessage(reason instanceof Error ? reason.message : String(reason));
      })
      .finally(() => {
        if (!disposed) setBusy(false);
      });
    return () => {
      disposed = true;
    };
  }, [googleConfigured, googleLanguageFilter, provider]);

  useEffect(() => {
    if (provider !== "azure" || !azureConfigured || !azureRegion) return;
    let disposed = false;
    setBusy(true);
    setMessage("");
    void listAzureTtsVoices(azureRegion, googleLanguageFilter)
      .then((items) => {
        if (disposed) return;
        setAzureVoices(items);
        setAzureVoiceId((current) =>
          items.some((voice) => voice.id === current)
            ? current
            : (items[0]?.id ?? ""),
        );
      })
      .catch((reason: unknown) => {
        if (!disposed)
          setMessage(reason instanceof Error ? reason.message : String(reason));
      })
      .finally(() => {
        if (!disposed) setBusy(false);
      });
    return () => {
      disposed = true;
    };
  }, [azureConfigured, azureRegion, googleLanguageFilter, provider]);

  const preparedChunk = (chunk: TtsQueueChunk) => {
    const pronounced = applyPronunciationDictionary(
      chunk.text,
      preferences.dictionaryEnabled ? preferences.pronunciations : [],
    );
    const key = `${provider}\n${selectedVoiceId}\n${selectedGoogleVoice?.language ?? selectedAzureVoice?.language ?? ""}\n${azureRegion}\n${rate}\n${JSON.stringify(expressive)}\n${chunk.id}\n${pronounced.text}`;
    const existing = prepared.current.get(key);
    if (existing) return existing;
    const request: Promise<PreparedPlaybackAudio> =
      provider === "elevenlabs"
        ? prepareCloudTtsSection(pronounced.text, selectedVoiceId, {
            stability: expressive.stability,
            similarityBoost: expressive.similarityBoost,
            style: expressive.style,
            speakerBoost: expressive.speakerBoost,
          }).then((audio) => ({
            ...audio,
            timings: remapCloudTtsTimings(
              audio.timings,
              pronounced.sourceOffsets,
            ),
          }))
        : provider === "google"
          ? prepareGoogleTtsSection(
              pronounced.text,
              selectedVoiceId,
              selectedGoogleVoice?.language ?? "",
              { pitch: expressive.googlePitch },
            )
          : provider === "azure"
            ? prepareAzureTtsSection(
                pronounced.text,
                selectedVoiceId,
                selectedAzureVoice?.language ?? "",
                azureRegion,
                { pitchPercent: expressive.azurePitch },
              )
            : prepareTtsSection(pronounced.text, selectedVoiceId, rate);
    prepared.current.set(key, request);
    return request;
  };

  const playChunk = async (index: number, sessionGeneration: number) => {
    const chunk = queue.current[index];
    if (!chunk || sessionGeneration !== generation.current) return;
    setBusy(true);
    setMessage("");
    expectedSection.current = chunk.sectionIndex;
    if (chunk.sectionIndex !== currentSectionIndex) {
      onNavigate(chunk.sectionIndex);
    }
    try {
      const audio = await preparedChunk(chunk);
      if (sessionGeneration !== generation.current) return;
      await loadAudioFile(audio.path);
      activeTimings.current = "timings" in audio ? audio.timings : [];
      await setAudioRate(provider === "local" ? 1 : rate);
      const nextSnapshot = await playAudio();
      if (sessionGeneration !== generation.current) {
        await stopAudio().catch(() => undefined);
        return;
      }
      queueIndex.current = index;
      ownsPlayback.current = true;
      setChunkPosition({ index: index + 1, total: queue.current.length });
      setSnapshot(nextSnapshot);
      onHighlight(
        highlightAtPlaybackPosition(chunk, 0, 0, activeTimings.current),
      );
      const next = queue.current[index + 1];
      if (next) void preparedChunk(next).catch(() => undefined);
    } catch (reason) {
      ownsPlayback.current = false;
      onHighlight(null);
      setMessage(
        reason instanceof Error &&
          reason.message === "TTS_PRONUNCIATION_EXPANSION_LIMIT"
          ? t("ttsPronunciationExpansionLimit")
          : reason instanceof Error
            ? reason.message
            : String(reason),
      );
    } finally {
      if (sessionGeneration === generation.current) setBusy(false);
    }
  };

  const stop = async () => {
    generation.current += 1;
    ownsPlayback.current = false;
    advancing.current = false;
    queue.current = [];
    prepared.current.clear();
    activeTimings.current = [];
    setSnapshot(null);
    setChunkPosition({ index: 0, total: 0 });
    onHighlight(null);
    try {
      await stopAudio();
    } catch (reason) {
      setMessage(reason instanceof Error ? reason.message : String(reason));
    }
  };
  stopRef.current = stop;

  const advance = async () => {
    if (advancing.current || !ownsPlayback.current) return;
    advancing.current = true;
    const nextIndex = queueIndex.current + 1;
    if (nextIndex >= queue.current.length) {
      await stop();
      setMessage(t("ttsFinished"));
      return;
    }
    prepared.current.forEach((_value, key) => {
      if (!key.includes(queue.current[nextIndex]!.id))
        prepared.current.delete(key);
    });
    await playChunk(nextIndex, generation.current);
    advancing.current = false;
  };
  advanceRef.current = advance;

  useEffect(() => {
    const timer = window.setInterval(() => {
      if (!ownsPlayback.current) return;
      void getAudioSnapshot()
        .then((value) => {
          setSnapshot(value);
          const chunk = queue.current[queueIndex.current];
          if (chunk && value.phase === "playing") {
            onHighlight(
              highlightAtPlaybackPosition(
                chunk,
                value.positionSeconds,
                value.durationSeconds,
                activeTimings.current,
              ),
            );
          }
          if (value.phase === "ended") void advanceRef.current();
          if (value.phase === "failed") {
            ownsPlayback.current = false;
            onHighlight(null);
            setMessage(value.lastError ?? t("ttsPlaybackFailed"));
          }
        })
        .catch((reason: unknown) => {
          setMessage(reason instanceof Error ? reason.message : String(reason));
        });
    }, 250);
    return () => window.clearInterval(timer);
  }, [onHighlight, t]);

  useEffect(() => {
    if (!ownsPlayback.current) {
      expectedSection.current = currentSectionIndex;
      return;
    }
    if (currentSectionIndex !== expectedSection.current) void stopRef.current();
  }, [currentSectionIndex]);

  const start = async (confirmedProvider?: TtsProvider) => {
    if (!selectedVoiceId) return;
    if (
      provider === "elevenlabs" &&
      !cloudConsent &&
      confirmedProvider !== "elevenlabs"
    ) {
      pendingConsentAction.current = "play";
      setShowCloudConsent(true);
      return;
    }
    if (
      provider === "google" &&
      !googleConsent &&
      confirmedProvider !== "google"
    ) {
      pendingConsentAction.current = "play";
      setShowGoogleConsent(true);
      return;
    }
    if (
      provider === "azure" &&
      !azureConsent &&
      confirmedProvider !== "azure"
    ) {
      pendingConsentAction.current = "play";
      setShowAzureConsent(true);
      return;
    }
    if (ownsPlayback.current && snapshot?.phase === "paused") {
      setSnapshot(await playAudio());
      return;
    }
    await stopAudio().catch(() => undefined);
    generation.current += 1;
    const sessionGeneration = generation.current;
    try {
      queue.current = buildTtsQueue(sections, currentSectionIndex, scope);
    } catch (reason) {
      setMessage(
        reason instanceof Error && reason.message === "TTS_QUEUE_LIMIT"
          ? t("ttsTooManyFragments")
          : reason instanceof Error
            ? reason.message
            : String(reason),
      );
      return;
    }
    if (queue.current.length === 0) {
      setMessage(t("ttsNoReadableText"));
      return;
    }
    prepared.current.clear();
    queueIndex.current = 0;
    await playChunk(0, sessionGeneration);
  };

  const pause = async () => {
    try {
      setSnapshot(await pauseAudio());
    } catch (reason) {
      setMessage(reason instanceof Error ? reason.message : String(reason));
    }
  };

  const resetSession = () => {
    if (ownsPlayback.current || queue.current.length > 0)
      void stopRef.current();
  };

  const configureCloud = async () => {
    setBusy(true);
    setMessage("");
    try {
      const status = await saveCloudTtsKey(apiKey);
      setApiKey("");
      setCloudConfigured(status.configured);
      setProvider("elevenlabs");
      writeLocalValue(providerKey, "elevenlabs");
    } catch (reason) {
      setMessage(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusy(false);
    }
  };

  const removeCloud = async () => {
    resetSession();
    setBusy(true);
    setMessage("");
    try {
      await deleteCloudTtsKey();
      setCloudConfigured(false);
      setCloudVoices([]);
      setCloudVoiceId("");
      setCloudConsent(false);
      writeLocalValue(cloudConsentKey, "");
      setProvider("local");
      writeLocalValue(providerKey, "local");
    } catch (reason) {
      setMessage(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusy(false);
    }
  };

  const configureGoogle = async () => {
    setBusy(true);
    setMessage("");
    try {
      const status = await saveGoogleTtsKey(googleApiKey);
      setGoogleApiKey("");
      setGoogleConfigured(status.configured);
      setProvider("google");
      writeLocalValue(providerKey, "google");
    } catch (reason) {
      setMessage(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusy(false);
    }
  };

  const removeGoogle = async () => {
    resetSession();
    setBusy(true);
    setMessage("");
    try {
      await deleteGoogleTtsKey();
      setGoogleConfigured(false);
      setGoogleVoices([]);
      setGoogleVoiceId("");
      setGoogleConsent(false);
      writeLocalValue(googleConsentKey, "");
      setProvider("local");
      writeLocalValue(providerKey, "local");
    } catch (reason) {
      setMessage(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusy(false);
    }
  };

  const configureAzure = async () => {
    setBusy(true);
    setMessage("");
    try {
      const status = await saveAzureTtsKey(azureApiKey);
      setAzureApiKey("");
      setAzureConfigured(status.configured);
      setProvider("azure");
      writeLocalValue(providerKey, "azure");
    } catch (reason) {
      setMessage(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusy(false);
    }
  };

  const removeAzure = async () => {
    resetSession();
    setBusy(true);
    setMessage("");
    try {
      await deleteAzureTtsKey();
      setAzureConfigured(false);
      setAzureVoices([]);
      setAzureVoiceId("");
      setAzureConsent(false);
      writeLocalValue(azureConsentKey, "");
      setProvider("local");
      writeLocalValue(providerKey, "local");
    } catch (reason) {
      setMessage(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusy(false);
    }
  };

  const chooseProvider = (next: TtsProvider) => {
    resetSession();
    setShowCloudConsent(false);
    setShowGoogleConsent(false);
    setShowAzureConsent(false);
    setProvider(next);
    setMessage("");
    writeLocalValue(providerKey, next);
  };

  const applyPreset = () => {
    const preset = preferences.presets.find(
      (value) => value.id === selectedPresetId,
    );
    if (!preset) return;
    resetSession();
    setProvider(preset.provider);
    setRate(preset.rate);
    writeLocalValue(providerKey, preset.provider);
    writeLocalValue(rateKey, String(preset.rate));
    if (preset.provider === "local") {
      setVoiceId(preset.voiceId);
      writeLocalValue(voiceKey, preset.voiceId);
    } else if (preset.provider === "elevenlabs") {
      setCloudVoiceId(preset.voiceId);
      writeLocalValue(cloudVoiceKey, preset.voiceId);
      persistExpressive({
        ...expressive,
        stability: preset.stability ?? 0.5,
        similarityBoost: preset.similarityBoost ?? 0.75,
        style: preset.style ?? 0,
        speakerBoost: preset.speakerBoost ?? true,
      });
    } else {
      if (preset.provider === "google") {
        setGoogleVoiceId(preset.voiceId);
        writeLocalValue(googleVoiceKey, preset.voiceId);
        persistExpressive({ ...expressive, googlePitch: preset.pitch ?? 0 });
      } else {
        setAzureVoiceId(preset.voiceId);
        writeLocalValue(azureVoiceKey, preset.voiceId);
        if (preset.region) {
          setAzureRegion(preset.region);
          writeLocalValue(azureRegionKey, preset.region);
        }
        persistExpressive({ ...expressive, azurePitch: preset.pitch ?? 0 });
      }
    }
    setMessage(t("ttsPresetApplied"));
  };

  const savePreset = () => {
    const existing = preferences.presets.find(
      (value) => value.id === selectedPresetId,
    );
    if (!existing && preferences.presets.length >= maxTtsVoicePresets) {
      setMessage(t("ttsPresetLimit"));
      return;
    }
    if (
      preferences.presets.some(
        (value) =>
          value.id !== selectedPresetId &&
          value.name.toLocaleLowerCase() ===
            presetName.trim().toLocaleLowerCase(),
      )
    ) {
      setMessage(t("ttsPresetDuplicate"));
      return;
    }
    const preset = normalizeVoicePreset({
      id: existing?.id ?? createTtsPreferenceId(),
      name: presetName,
      provider,
      voiceId: selectedVoiceId,
      rate,
      region: provider === "azure" ? azureRegion : undefined,
      stability: provider === "elevenlabs" ? expressive.stability : undefined,
      similarityBoost:
        provider === "elevenlabs" ? expressive.similarityBoost : undefined,
      style: provider === "elevenlabs" ? expressive.style : undefined,
      speakerBoost:
        provider === "elevenlabs" ? expressive.speakerBoost : undefined,
      pitch:
        provider === "google"
          ? expressive.googlePitch
          : provider === "azure"
            ? expressive.azurePitch
            : undefined,
    });
    if (!preset) {
      setMessage(t("ttsPresetInvalid"));
      return;
    }
    const presets = existing
      ? preferences.presets.map((value) =>
          value.id === existing.id ? preset : value,
        )
      : [...preferences.presets, preset];
    persistPreferences({ ...preferences, presets });
    setSelectedPresetId(preset.id);
    setPresetName(preset.name);
    setMessage(t(existing ? "ttsPresetUpdated" : "ttsPresetSaved"));
  };

  const deletePreset = () => {
    if (!selectedPresetId) return;
    persistPreferences({
      ...preferences,
      presets: preferences.presets.filter(
        (value) => value.id !== selectedPresetId,
      ),
    });
    setSelectedPresetId("");
    setPresetName("");
    setMessage(t("ttsPresetDeleted"));
  };

  const savePronunciation = () => {
    const existing = preferences.pronunciations.find(
      (value) => value.id === pronunciationId,
    );
    if (
      !existing &&
      preferences.pronunciations.length >= maxTtsPronunciationRules
    ) {
      setMessage(t("ttsPronunciationLimit"));
      return;
    }
    if (
      preferences.pronunciations.some(
        (value) =>
          value.id !== pronunciationId &&
          value.source.toLocaleLowerCase() ===
            pronunciationSource.trim().toLocaleLowerCase(),
      )
    ) {
      setMessage(t("ttsPronunciationDuplicate"));
      return;
    }
    const rule = normalizePronunciationRule({
      id: existing?.id ?? createTtsPreferenceId(),
      source: pronunciationSource,
      replacement: pronunciationReplacement,
    });
    if (!rule) {
      setMessage(t("ttsPronunciationInvalid"));
      return;
    }
    resetSession();
    persistPreferences({
      ...preferences,
      pronunciations: existing
        ? preferences.pronunciations.map((value) =>
            value.id === existing.id ? rule : value,
          )
        : [...preferences.pronunciations, rule],
    });
    setPronunciationId("");
    setPronunciationSource("");
    setPronunciationReplacement("");
    setMessage(
      t(existing ? "ttsPronunciationUpdated" : "ttsPronunciationSaved"),
    );
  };

  const editPronunciation = (id: string) => {
    const rule = preferences.pronunciations.find((value) => value.id === id);
    if (!rule) return;
    setPronunciationId(rule.id);
    setPronunciationSource(rule.source);
    setPronunciationReplacement(rule.replacement);
  };

  const deletePronunciation = (id: string) => {
    resetSession();
    persistPreferences({
      ...preferences,
      pronunciations: preferences.pronunciations.filter(
        (value) => value.id !== id,
      ),
    });
    if (pronunciationId === id) {
      setPronunciationId("");
      setPronunciationSource("");
      setPronunciationReplacement("");
    }
    setMessage(t("ttsPronunciationDeleted"));
  };

  const refreshCache = async () => {
    setCacheSummary(await getTtsCacheSummary());
  };

  const removeCachedSpeech = async (selectedProvider?: TtsProvider) => {
    await stopRef.current();
    setBusy(true);
    setMessage("");
    try {
      setCacheSummary(await clearTtsCache(selectedProvider));
      setMessage(t("ttsCacheCleared"));
    } catch (reason) {
      setMessage(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusy(false);
    }
  };

  const requestExport = () => {
    if (!selectedVoiceId) return;
    pendingConsentAction.current = "export";
    if (provider === "elevenlabs" && !cloudConsent) {
      setShowCloudConsent(true);
      return;
    }
    if (provider === "google" && !googleConsent) {
      setShowGoogleConsent(true);
      return;
    }
    if (provider === "azure" && !azureConsent) {
      setShowAzureConsent(true);
      return;
    }
    setShowExportConfirm(true);
  };

  const exportNarration = async () => {
    setShowExportConfirm(false);
    let chunks: TtsQueueChunk[];
    try {
      chunks = buildTtsQueue(sections, currentSectionIndex, scope);
    } catch (reason) {
      setMessage(
        reason instanceof Error && reason.message === "TTS_QUEUE_LIMIT"
          ? t("ttsTooManyFragments")
          : reason instanceof Error
            ? reason.message
            : String(reason),
      );
      return;
    }
    if (chunks.length === 0) {
      setMessage(t("ttsNoReadableText"));
      return;
    }
    if (chunks.length > maxTtsExportParts) {
      setMessage(
        t("ttsExportTooManyParts").replace(
          "{limit}",
          String(maxTtsExportParts),
        ),
      );
      return;
    }
    const playlistPath = await chooseTtsExportPath(title);
    if (!playlistPath) return;
    await stopRef.current();
    setBusy(true);
    setMessage("");
    setExportProgress({ current: 0, total: chunks.length });
    exportCancelled.current = false;
    prepared.current.clear();
    try {
      const started = await beginTtsExport(playlistPath, chunks.length);
      exportSession.current = started.sessionId;
      for (let index = 0; index < chunks.length; index += 1) {
        if (exportCancelled.current) throw new Error("TTS_EXPORT_CANCELLED");
        const chunk = chunks[index]!;
        const audio = await preparedChunk(chunk);
        if (exportCancelled.current) throw new Error("TTS_EXPORT_CANCELLED");
        const sectionTitle = sections[chunk.sectionIndex]?.title.trim();
        await appendTtsExportPart(started.sessionId, {
          sourcePath: audio.path,
          title: sectionTitle
            ? `${sectionTitle} · ${index + 1}`
            : `${t("ttsExportPart")} ${index + 1}`,
        });
        setExportProgress({ current: index + 1, total: chunks.length });
        prepared.current.clear();
      }
      const result = await finishTtsExport(started.sessionId);
      exportSession.current = "";
      setMessage(
        t("ttsExportFinished")
          .replace("{parts}", String(result.parts))
          .replace("{path}", result.playlistPath),
      );
      await refreshCache();
    } catch (reason) {
      const sessionId = exportSession.current;
      exportSession.current = "";
      if (sessionId) await cancelTtsExport(sessionId).catch(() => undefined);
      setMessage(
        reason instanceof Error && reason.message === "TTS_EXPORT_CANCELLED"
          ? t("ttsExportCancelled")
          : reason instanceof Error
            ? reason.message
            : String(reason),
      );
    } finally {
      prepared.current.clear();
      setExportProgress({ current: 0, total: 0 });
      setBusy(false);
    }
  };

  return (
    <div className="tts-panel">
      <p className="privacy-note">
        {provider === "local"
          ? t("ttsLocalPrivacy")
          : provider === "elevenlabs"
            ? t("ttsCloudPrivacy")
            : provider === "google"
              ? t("ttsGooglePrivacy")
              : t("ttsAzurePrivacy")}
      </p>
      <fieldset className="tts-scope tts-provider">
        <legend>{t("ttsProvider")}</legend>
        <label>
          <input
            type="radio"
            name="tts-provider"
            checked={provider === "local"}
            onChange={() => chooseProvider("local")}
          />
          {t("ttsProviderWindows")}
        </label>
        <label>
          <input
            type="radio"
            name="tts-provider"
            checked={provider === "elevenlabs"}
            onChange={() => chooseProvider("elevenlabs")}
          />
          {t("ttsProviderElevenLabs")}
        </label>
        <label>
          <input
            type="radio"
            name="tts-provider"
            checked={provider === "google"}
            onChange={() => chooseProvider("google")}
          />
          {t("ttsProviderGoogle")}
        </label>
        <label>
          <input
            type="radio"
            name="tts-provider"
            checked={provider === "azure"}
            onChange={() => chooseProvider("azure")}
          />
          {t("ttsProviderAzure")}
        </label>
      </fieldset>
      <details className="tts-preferences">
        <summary>{t("ttsVoicePresets")}</summary>
        <div className="tts-preferences-body">
          <label>
            {t("ttsSavedPreset")}
            <select
              value={selectedPresetId}
              onChange={(event) => {
                const id = event.target.value;
                const preset = preferences.presets.find(
                  (value) => value.id === id,
                );
                setSelectedPresetId(id);
                setPresetName(preset?.name ?? "");
              }}
            >
              <option value="">{t("ttsNewPreset")}</option>
              {preferences.presets.map((preset) => (
                <option key={preset.id} value={preset.id}>
                  {preset.name}
                </option>
              ))}
            </select>
          </label>
          <label>
            {t("ttsPresetName")}
            <input
              type="text"
              value={presetName}
              maxLength={40}
              onChange={(event) => setPresetName(event.target.value)}
            />
          </label>
          <p className="tts-preference-hint">{t("ttsPresetHint")}</p>
          <div className="dialog-actions tts-compact-actions">
            <button
              type="button"
              className="secondary-button"
              disabled={!selectedPresetId}
              onClick={applyPreset}
            >
              {t("ttsApplyPreset")}
            </button>
            <button
              type="button"
              className="secondary-button"
              disabled={!selectedVoiceId || presetName.trim().length === 0}
              onClick={savePreset}
            >
              {selectedPresetId ? t("ttsUpdatePreset") : t("ttsSavePreset")}
            </button>
            <button
              type="button"
              className="text-button danger"
              disabled={!selectedPresetId}
              onClick={deletePreset}
            >
              {t("deleteAnnotation")}
            </button>
          </div>
        </div>
      </details>
      {provider === "elevenlabs" && !cloudConfigured && (
        <div className="tts-cloud-credentials">
          <p>{t("ttsCloudKeyHint")}</p>
          <label>
            {t("ttsCloudApiKey")}
            <input
              type="password"
              value={apiKey}
              maxLength={512}
              autoComplete="off"
              spellCheck={false}
              onChange={(event) => setApiKey(event.target.value)}
            />
          </label>
          <button
            type="button"
            className="secondary-button"
            disabled={busy || apiKey.trim().length < 8}
            onClick={() => void configureCloud()}
          >
            {t("ttsCloudSaveKey")}
          </button>
        </div>
      )}
      {provider === "elevenlabs" && cloudConfigured && (
        <div className="tts-cloud-status">
          <span>{t("ttsCloudKeyStored")}</span>
          <button
            type="button"
            className="text-button danger"
            disabled={busy}
            onClick={() => void removeCloud()}
          >
            {t("ttsCloudDeleteKey")}
          </button>
        </div>
      )}
      {provider === "google" && !googleConfigured && (
        <div className="tts-cloud-credentials">
          <p>{t("ttsGoogleKeyHint")}</p>
          <label>
            {t("ttsGoogleApiKey")}
            <input
              type="password"
              value={googleApiKey}
              maxLength={512}
              autoComplete="off"
              spellCheck={false}
              onChange={(event) => setGoogleApiKey(event.target.value)}
            />
          </label>
          <button
            type="button"
            className="secondary-button"
            disabled={busy || googleApiKey.trim().length < 8}
            onClick={() => void configureGoogle()}
          >
            {t("ttsCloudSaveKey")}
          </button>
        </div>
      )}
      {provider === "google" && googleConfigured && (
        <div className="tts-cloud-status">
          <span>{t("ttsGoogleKeyStored")}</span>
          <button
            type="button"
            className="text-button danger"
            disabled={busy}
            onClick={() => void removeGoogle()}
          >
            {t("ttsCloudDeleteKey")}
          </button>
        </div>
      )}
      {provider === "azure" && (
        <label>
          {t("ttsAzureRegion")}
          <select
            value={azureRegion}
            disabled={busy || azureRegions.length === 0}
            onChange={(event) => {
              resetSession();
              setAzureRegion(event.target.value);
              writeLocalValue(azureRegionKey, event.target.value);
            }}
          >
            {azureRegions.map((region) => (
              <option key={region.id} value={region.id}>
                {region.name} · {region.id}
              </option>
            ))}
          </select>
        </label>
      )}
      {provider === "azure" && !azureConfigured && (
        <div className="tts-cloud-credentials">
          <p>{t("ttsAzureKeyHint")}</p>
          <label>
            {t("ttsAzureApiKey")}
            <input
              type="password"
              value={azureApiKey}
              maxLength={512}
              autoComplete="off"
              spellCheck={false}
              onChange={(event) => setAzureApiKey(event.target.value)}
            />
          </label>
          <button
            type="button"
            className="secondary-button"
            disabled={busy || !azureRegion || azureApiKey.trim().length < 8}
            onClick={() => void configureAzure()}
          >
            {t("ttsCloudSaveKey")}
          </button>
        </div>
      )}
      {provider === "azure" && azureConfigured && (
        <div className="tts-cloud-status">
          <span>{t("ttsAzureKeyStored")}</span>
          <button
            type="button"
            className="text-button danger"
            disabled={busy}
            onClick={() => void removeAzure()}
          >
            {t("ttsCloudDeleteKey")}
          </button>
        </div>
      )}
      {provider === "local" ? (
        <label>
          {t("ttsVoice")}
          <select
            value={voiceId}
            disabled={busy || voices.length === 0}
            onChange={(event) => {
              resetSession();
              setVoiceId(event.target.value);
              writeLocalValue(voiceKey, event.target.value);
            }}
          >
            {voices.map((voice) => (
              <option key={voice.id} value={voice.id}>
                {voice.name} · {voice.language}
                {voice.isDefault ? ` (${t("ttsDefaultVoice")})` : ""}
              </option>
            ))}
          </select>
        </label>
      ) : provider === "elevenlabs" ? (
        <label>
          {t("ttsCloudVoice")}
          <select
            value={cloudVoiceId}
            disabled={busy || !cloudConfigured || cloudVoices.length === 0}
            onChange={(event) => {
              resetSession();
              setCloudVoiceId(event.target.value);
              writeLocalValue(cloudVoiceKey, event.target.value);
            }}
          >
            {cloudVoices.map((voice) => (
              <option key={voice.id} value={voice.id}>
                {voice.name} · {voice.language}
              </option>
            ))}
          </select>
        </label>
      ) : provider === "google" ? (
        <label>
          {t("ttsGoogleVoice")}
          <select
            value={googleVoiceId}
            disabled={busy || !googleConfigured || googleVoices.length === 0}
            onChange={(event) => {
              resetSession();
              setGoogleVoiceId(event.target.value);
              writeLocalValue(googleVoiceKey, event.target.value);
            }}
          >
            {googleVoices.map((voice) => (
              <option key={voice.id} value={voice.id}>
                {voice.name} · {voice.language} · {voice.category}
              </option>
            ))}
          </select>
        </label>
      ) : (
        <label>
          {t("ttsAzureVoice")}
          <select
            value={azureVoiceId}
            disabled={busy || !azureConfigured || azureVoices.length === 0}
            onChange={(event) => {
              resetSession();
              setAzureVoiceId(event.target.value);
              writeLocalValue(azureVoiceKey, event.target.value);
            }}
          >
            {azureVoices.map((voice) => (
              <option key={voice.id} value={voice.id}>
                {voice.name} · {voice.language} · {voice.category}
              </option>
            ))}
          </select>
        </label>
      )}
      {provider !== "local" && (
        <details className="tts-preferences">
          <summary>{t("ttsExpressiveness")}</summary>
          <div className="tts-preferences-body tts-expressive-controls">
            {provider === "elevenlabs" && (
              <>
                <label>
                  {t("ttsStability")} · {Math.round(expressive.stability * 100)}
                  %
                  <input
                    type="range"
                    min="0"
                    max="1"
                    step="0.05"
                    value={expressive.stability}
                    disabled={busy}
                    onChange={(event) =>
                      persistExpressive({
                        ...expressive,
                        stability: Number(event.target.value),
                      })
                    }
                  />
                </label>
                <label>
                  {t("ttsSimilarity")} ·{" "}
                  {Math.round(expressive.similarityBoost * 100)}%
                  <input
                    type="range"
                    min="0"
                    max="1"
                    step="0.05"
                    value={expressive.similarityBoost}
                    disabled={busy}
                    onChange={(event) =>
                      persistExpressive({
                        ...expressive,
                        similarityBoost: Number(event.target.value),
                      })
                    }
                  />
                </label>
                <label>
                  {t("ttsStyle")} · {Math.round(expressive.style * 100)}%
                  <input
                    type="range"
                    min="0"
                    max="1"
                    step="0.05"
                    value={expressive.style}
                    disabled={busy}
                    onChange={(event) =>
                      persistExpressive({
                        ...expressive,
                        style: Number(event.target.value),
                      })
                    }
                  />
                </label>
                <label className="tts-dictionary-toggle">
                  <input
                    type="checkbox"
                    checked={expressive.speakerBoost}
                    disabled={busy}
                    onChange={(event) =>
                      persistExpressive({
                        ...expressive,
                        speakerBoost: event.target.checked,
                      })
                    }
                  />
                  {t("ttsSpeakerBoost")}
                </label>
              </>
            )}
            {provider === "google" && (
              <label>
                {t("ttsPitchSemitones")} ·{" "}
                {expressive.googlePitch > 0 ? "+" : ""}
                {expressive.googlePitch}
                <input
                  type="range"
                  min="-20"
                  max="20"
                  step="1"
                  value={expressive.googlePitch}
                  disabled={busy}
                  onChange={(event) =>
                    persistExpressive({
                      ...expressive,
                      googlePitch: Number(event.target.value),
                    })
                  }
                />
              </label>
            )}
            {provider === "azure" && (
              <label>
                {t("ttsPitchPercent")} · {expressive.azurePitch > 0 ? "+" : ""}
                {expressive.azurePitch}%
                <input
                  type="range"
                  min="-50"
                  max="50"
                  step="5"
                  value={expressive.azurePitch}
                  disabled={busy}
                  onChange={(event) =>
                    persistExpressive({
                      ...expressive,
                      azurePitch: Number(event.target.value),
                    })
                  }
                />
              </label>
            )}
            <p className="tts-preference-hint">{t("ttsExpressivenessHint")}</p>
          </div>
        </details>
      )}
      <label>
        {t("ttsSpeechRate")}
        <select
          value={rate}
          disabled={busy}
          onChange={(event) => {
            resetSession();
            const next = Number(event.target.value);
            setRate(next);
            writeLocalValue(rateKey, String(next));
          }}
        >
          {rates.map((value) => (
            <option key={value} value={value}>
              {value}×
            </option>
          ))}
        </select>
      </label>
      <details className="tts-preferences">
        <summary>
          {t("ttsPronunciationDictionary").replace(
            "{count}",
            String(preferences.pronunciations.length),
          )}
        </summary>
        <div className="tts-preferences-body">
          <label className="tts-dictionary-toggle">
            <input
              type="checkbox"
              checked={preferences.dictionaryEnabled}
              onChange={(event) => {
                resetSession();
                persistPreferences({
                  ...preferences,
                  dictionaryEnabled: event.target.checked,
                });
              }}
            />
            {t("ttsPronunciationEnabled")}
          </label>
          <p className="tts-preference-hint">{t("ttsPronunciationHint")}</p>
          <div className="tts-pronunciation-form">
            <label>
              {t("ttsPronunciationSource")}
              <input
                type="text"
                value={pronunciationSource}
                maxLength={80}
                onChange={(event) => setPronunciationSource(event.target.value)}
              />
            </label>
            <label>
              {t("ttsPronunciationReplacement")}
              <input
                type="text"
                value={pronunciationReplacement}
                maxLength={160}
                onChange={(event) =>
                  setPronunciationReplacement(event.target.value)
                }
              />
            </label>
            <div className="dialog-actions tts-compact-actions">
              <button
                type="button"
                className="secondary-button"
                disabled={
                  pronunciationSource.trim().length === 0 ||
                  pronunciationReplacement.trim().length === 0
                }
                onClick={savePronunciation}
              >
                {pronunciationId
                  ? t("ttsPronunciationUpdate")
                  : t("ttsPronunciationAdd")}
              </button>
              {pronunciationId && (
                <button
                  type="button"
                  className="text-button"
                  onClick={() => {
                    setPronunciationId("");
                    setPronunciationSource("");
                    setPronunciationReplacement("");
                  }}
                >
                  {t("cancel")}
                </button>
              )}
            </div>
          </div>
          {preferences.pronunciations.length > 0 && (
            <ul className="tts-pronunciation-list">
              {preferences.pronunciations.map((rule) => (
                <li key={rule.id}>
                  <span>
                    <strong>{rule.source}</strong>
                    <span aria-hidden="true"> → </span>
                    {rule.replacement}
                  </span>
                  <span className="tts-rule-actions">
                    <button
                      type="button"
                      className="text-button"
                      onClick={() => editPronunciation(rule.id)}
                    >
                      {t("editMetadata")}
                    </button>
                    <button
                      type="button"
                      className="text-button danger"
                      onClick={() => deletePronunciation(rule.id)}
                    >
                      {t("deleteAnnotation")}
                    </button>
                  </span>
                </li>
              ))}
            </ul>
          )}
        </div>
      </details>
      <details className="tts-preferences">
        <summary>{t("ttsCacheManagement")}</summary>
        <div className="tts-preferences-body">
          <p className="tts-preference-hint">
            {cacheSummary
              ? t("ttsCacheSummary")
                  .replace("{files}", String(cacheSummary.totalFiles))
                  .replace("{size}", formatBytes(cacheSummary.totalBytes))
              : t("ttsCacheLoading")}
          </p>
          {cacheSummary && (
            <ul className="tts-cache-list">
              {cacheSummary.providers.map((value) => (
                <li key={value.provider}>
                  <span>
                    {t(
                      value.provider === "local"
                        ? "ttsCacheProviderLocal"
                        : value.provider === "elevenlabs"
                          ? "ttsProviderElevenLabs"
                          : value.provider === "google"
                            ? "ttsProviderGoogle"
                            : "ttsProviderAzure",
                    )}
                  </span>
                  <span>
                    {value.files} · {formatBytes(value.bytes)}
                  </span>
                </li>
              ))}
            </ul>
          )}
          <div className="dialog-actions tts-compact-actions">
            <button
              type="button"
              className="secondary-button"
              disabled={busy || !cacheSummary || cacheSummary.totalFiles === 0}
              onClick={() => void removeCachedSpeech(provider)}
            >
              {t("ttsClearProviderCache")}
            </button>
            <button
              type="button"
              className="text-button danger"
              disabled={busy || !cacheSummary || cacheSummary.totalFiles === 0}
              onClick={() => void removeCachedSpeech()}
            >
              {t("ttsClearAllCache")}
            </button>
          </div>
        </div>
      </details>
      <fieldset className="tts-scope">
        <legend>{t("ttsReadingScope")}</legend>
        <label>
          <input
            type="radio"
            name="tts-scope"
            value="section"
            checked={scope === "section"}
            onChange={() => {
              resetSession();
              setScope("section");
              writeLocalValue(scopeKey, "section");
            }}
          />
          {t("ttsCurrentSection")}
        </label>
        <label>
          <input
            type="radio"
            name="tts-scope"
            value="book"
            checked={scope === "book"}
            onChange={() => {
              resetSession();
              setScope("book");
              writeLocalValue(scopeKey, "book");
            }}
          />
          {t("ttsWholeBook")}
        </label>
      </fieldset>
      <p className="tts-section-size">
        {t("ttsSectionCharacters").replace("{count}", String(characterCount))}
      </p>
      {chunkPosition.total > 0 && (
        <p className="tts-progress" role="status">
          {t("ttsFragmentProgress")
            .replace("{current}", String(chunkPosition.index))
            .replace("{total}", String(chunkPosition.total))}
        </p>
      )}
      {exportProgress.total > 0 && (
        <p className="tts-progress" role="status">
          {t("ttsExportProgress")
            .replace("{current}", String(exportProgress.current))
            .replace("{total}", String(exportProgress.total))}
        </p>
      )}
      <div className="dialog-actions tts-actions">
        {snapshot?.phase === "playing" ? (
          <button
            type="button"
            className="primary-button"
            onClick={() => void pause()}
          >
            {t("audioPause")}
          </button>
        ) : (
          <button
            type="button"
            className="primary-button"
            disabled={
              busy ||
              !selectedVoiceId ||
              (provider === "elevenlabs" && !cloudConfigured) ||
              (provider === "google" &&
                (!googleConfigured || !selectedGoogleVoice)) ||
              (provider === "azure" &&
                (!azureConfigured || !azureRegion || !selectedAzureVoice))
            }
            onClick={() => void start()}
          >
            {snapshot?.phase === "paused"
              ? t("audioPlay")
              : scope === "book"
                ? t("ttsReadBook")
                : t("ttsReadSection")}
          </button>
        )}
        <button
          type="button"
          className="secondary-button"
          disabled={!ownsPlayback.current && queue.current.length === 0}
          onClick={() => void stop()}
        >
          {t("ttsStop")}
        </button>
        {exportProgress.total > 0 ? (
          <button
            type="button"
            className="secondary-button danger"
            onClick={() => {
              exportCancelled.current = true;
              setMessage(t("ttsExportCancelling"));
            }}
          >
            {t("ttsCancelExport")}
          </button>
        ) : (
          <button
            type="button"
            className="secondary-button"
            disabled={
              busy ||
              !selectedVoiceId ||
              (provider === "elevenlabs" && !cloudConfigured) ||
              (provider === "google" &&
                (!googleConfigured || !selectedGoogleVoice)) ||
              (provider === "azure" &&
                (!azureConfigured || !azureRegion || !selectedAzureVoice))
            }
            onClick={requestExport}
          >
            {t("ttsExportNarration")}
          </button>
        )}
      </div>
      {showExportConfirm && (
        <div
          className="tts-cloud-consent"
          role="group"
          aria-label={t("ttsExportConfirmTitle")}
        >
          <strong>{t("ttsExportConfirmTitle")}</strong>
          <p>
            {provider === "local"
              ? t("ttsExportConfirmLocal")
              : t("ttsExportConfirmCloud")}
          </p>
          <div className="dialog-actions">
            <button
              type="button"
              className="secondary-button"
              onClick={() => setShowExportConfirm(false)}
            >
              {t("cancel")}
            </button>
            <button
              type="button"
              className="primary-button"
              onClick={() => void exportNarration()}
            >
              {t("ttsBeginExport")}
            </button>
          </div>
        </div>
      )}
      {showCloudConsent && (
        <div
          className="tts-cloud-consent"
          role="group"
          aria-label={t("ttsCloudConsentTitle")}
        >
          <strong>{t("ttsCloudConsentTitle")}</strong>
          <p>{t("ttsCloudConsentBody")}</p>
          <div className="dialog-actions">
            <button
              type="button"
              className="secondary-button"
              onClick={() => setShowCloudConsent(false)}
            >
              {t("cancel")}
            </button>
            <button
              type="button"
              className="primary-button"
              onClick={() => {
                setCloudConsent(true);
                setShowCloudConsent(false);
                writeLocalValue(cloudConsentKey, "accepted");
                if (pendingConsentAction.current === "export")
                  setShowExportConfirm(true);
                else void start("elevenlabs");
              }}
            >
              {t("ttsCloudConsentContinue")}
            </button>
          </div>
        </div>
      )}
      {showGoogleConsent && (
        <div
          className="tts-cloud-consent"
          role="group"
          aria-label={t("ttsGoogleConsentTitle")}
        >
          <strong>{t("ttsGoogleConsentTitle")}</strong>
          <p>{t("ttsGoogleConsentBody")}</p>
          <div className="dialog-actions">
            <button
              type="button"
              className="secondary-button"
              onClick={() => setShowGoogleConsent(false)}
            >
              {t("cancel")}
            </button>
            <button
              type="button"
              className="primary-button"
              onClick={() => {
                setGoogleConsent(true);
                setShowGoogleConsent(false);
                writeLocalValue(googleConsentKey, "accepted");
                if (pendingConsentAction.current === "export")
                  setShowExportConfirm(true);
                else void start("google");
              }}
            >
              {t("ttsCloudConsentContinue")}
            </button>
          </div>
        </div>
      )}
      {showAzureConsent && (
        <div
          className="tts-cloud-consent"
          role="group"
          aria-label={t("ttsAzureConsentTitle")}
        >
          <strong>{t("ttsAzureConsentTitle")}</strong>
          <p>{t("ttsAzureConsentBody")}</p>
          <div className="dialog-actions">
            <button
              type="button"
              className="secondary-button"
              onClick={() => setShowAzureConsent(false)}
            >
              {t("cancel")}
            </button>
            <button
              type="button"
              className="primary-button"
              onClick={() => {
                setAzureConsent(true);
                setShowAzureConsent(false);
                writeLocalValue(azureConsentKey, "accepted");
                if (pendingConsentAction.current === "export")
                  setShowExportConfirm(true);
                else void start("azure");
              }}
            >
              {t("ttsCloudConsentContinue")}
            </button>
          </div>
        </div>
      )}
      {busy && <p role="status">{t("ttsPreparing")}</p>}
      {message && (
        <p className="availability-warning" role="alert">
          {message}
        </p>
      )}
      {provider === "local" && voices.length === 0 && !busy && !message && (
        <p className="availability-warning">{t("ttsNoVoices")}</p>
      )}
      {provider === "elevenlabs" &&
        cloudConfigured &&
        cloudVoices.length === 0 &&
        !busy &&
        !message && (
          <p className="availability-warning">{t("ttsCloudNoVoices")}</p>
        )}
      {provider === "google" &&
        googleConfigured &&
        googleVoices.length === 0 &&
        !busy &&
        !message && (
          <p className="availability-warning">{t("ttsGoogleNoVoices")}</p>
        )}
      {provider === "azure" &&
        azureConfigured &&
        azureVoices.length === 0 &&
        !busy &&
        !message && (
          <p className="availability-warning">{t("ttsAzureNoVoices")}</p>
        )}
    </div>
  );
}
