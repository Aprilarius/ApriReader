import { useCallback, useEffect, useRef, useState } from "react";
import {
  createAudiobookBookmark,
  deleteAudiobookBookmark,
  getAudioSnapshot,
  listAudioOutputDevices,
  listAudiobookBookmarks,
  listAudiobookChapters,
  loadAudioFile,
  pauseAudio,
  playAudio,
  saveAudiobookPosition,
  seekAudio,
  setAudioRate,
  setAudioOutputDevice,
  setAudioVolume,
  type AudiobookBookmarkRecord,
  type AudiobookChapterRecord,
  type AudioPlaybackSnapshot,
  type AudioOutputDevice,
} from "../application/audioPlayer";
import {
  listenForAudioCloseRequest,
  resolveAudioClose,
  type AudioCloseDecision,
} from "../application/audioLifecycle";
import type {
  AudiobookPartRecord,
  AudiobookRecord,
} from "../application/audiobooks";
import { coverUrl } from "../application/library";
import {
  endAudiobookSession,
  recordAudiobookActivity,
  startAudiobookSession,
} from "../application/statistics";
import { Icon } from "./icons";
import type { TranslationKey } from "./i18n";
import { readLocalValue, writeLocalValue } from "./localStorage";

type Translator = (key: TranslationKey) => string;

const audioRateKey = "aprireader.audio.rate";
const audioVolumeKey = "aprireader.audio.volume";
const audioOutputDeviceKey = "aprireader.audio.outputDevice";
const playbackRates = [0.5, 0.75, 1, 1.25, 1.5, 1.75, 2, 2.5, 3];
type SleepTimerMode = "off" | "end-part" | "15" | "30" | "45" | "60";

const emptySnapshot: AudioPlaybackSnapshot = {
  phase: "idle",
  path: null,
  positionSeconds: 0,
  durationSeconds: 0,
  playbackRate: 1,
  volume: 1,
  canSeek: false,
  canPause: false,
  lastError: null,
};

export function AudiobookPlayer({
  book,
  parts,
  t,
  onProgress,
  onClose,
}: {
  book: AudiobookRecord;
  parts: AudiobookPartRecord[];
  t: Translator;
  onProgress: (book: AudiobookRecord) => void;
  onClose: () => void;
}) {
  const initialPart = boundedInitialPart(book, parts);
  const [partIndex, setPartIndex] = useState(initialPart);
  const [snapshot, setSnapshot] = useState(emptySnapshot);
  const [seekDraft, setSeekDraft] = useState<number | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [rate, setRate] = useState(() => readNumber(audioRateKey, 1, 0.5, 3));
  const [volume, setVolume] = useState(() =>
    readNumber(audioVolumeKey, 1, 0, 1),
  );
  const [bookmarks, setBookmarks] = useState<AudiobookBookmarkRecord[]>([]);
  const [chapters, setChapters] = useState<AudiobookChapterRecord[]>([]);
  const [bookmarkNote, setBookmarkNote] = useState("");
  const [sleepMode, setSleepMode] = useState<SleepTimerMode>("off");
  const [sleepDeadline, setSleepDeadline] = useState<number | null>(null);
  const [sleepRemaining, setSleepRemaining] = useState(0);
  const [closePrompt, setClosePrompt] = useState(false);
  const [rememberCloseChoice, setRememberCloseChoice] = useState(false);
  const [outputDevices, setOutputDevices] = useState<AudioOutputDevice[]>([]);
  const [outputDeviceId, setOutputDeviceId] = useState(
    () => readLocalValue(audioOutputDeviceKey) ?? "",
  );
  const snapshotRef = useRef(snapshot);
  const partIndexRef = useRef(partIndex);
  const rateRef = useRef(rate);
  const volumeRef = useRef(volume);
  const requestRef = useRef(0);
  const pollBusyRef = useRef(false);
  const endedHandledRef = useRef(false);
  const lastSavedRef = useRef({ partIndex: -1, position: -10 });
  const seekTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const advanceRef = useRef<() => void>(() => undefined);
  const sleepModeRef = useRef<SleepTimerMode>("off");
  const listeningTokenRef = useRef<string | null>(null);
  const currentPart = parts[partIndex] ?? null;
  const playing = snapshot.phase === "playing";
  const displayedPosition = seekDraft ?? snapshot.positionSeconds;

  const applySnapshot = useCallback((next: AudioPlaybackSnapshot) => {
    snapshotRef.current = next;
    setSnapshot(next);
    if (next.lastError) setError(next.lastError);
  }, []);

  const persist = useCallback(
    async (index: number, value: AudioPlaybackSnapshot, force = false) => {
      if (value.durationSeconds <= 0 || index < 0 || index >= parts.length)
        return;
      const previous = lastSavedRef.current;
      if (
        !force &&
        previous.partIndex === index &&
        Math.abs(previous.position - value.positionSeconds) < 5
      ) {
        return;
      }
      lastSavedRef.current = {
        partIndex: index,
        position: value.positionSeconds,
      };
      try {
        const updated = await saveAudiobookPosition(
          book.id,
          index,
          Math.min(value.positionSeconds, value.durationSeconds),
          value.durationSeconds,
        );
        onProgress(updated);
      } catch (reason) {
        setError(reason instanceof Error ? reason.message : String(reason));
      }
    },
    [book.id, onProgress, parts.length],
  );

  const loadPart = useCallback(
    async (index: number, autoplay: boolean, resumePosition = 0) => {
      const part = parts[index];
      if (!part || !part.isAvailable) return;
      const request = ++requestRef.current;
      setLoading(true);
      setBusy(true);
      setError("");
      setSeekDraft(null);
      endedHandledRef.current = false;
      try {
        let next = await loadAudioFile(part.sourcePath);
        if (request !== requestRef.current) return;
        next = await setAudioVolume(volumeRef.current);
        if (request !== requestRef.current) return;
        next = await setAudioRate(rateRef.current);
        if (request !== requestRef.current) return;
        if (resumePosition > 0 && next.canSeek) {
          next = await seekAudio(
            Math.min(resumePosition, next.durationSeconds),
          );
          if (request !== requestRef.current) return;
        }
        if (autoplay) {
          next = await playAudio();
          if (request !== requestRef.current) return;
        }
        setPartIndex(index);
        partIndexRef.current = index;
        applySnapshot(next);
        void persist(index, next, true);
      } catch (reason) {
        if (request === requestRef.current) {
          setError(reason instanceof Error ? reason.message : String(reason));
        }
      } finally {
        if (request === requestRef.current) {
          setLoading(false);
          setBusy(false);
        }
      }
    },
    [applySnapshot, parts, persist],
  );

  const moveToPart = useCallback(
    (direction: -1 | 1, autoplay = playing) => {
      const next = nextAvailablePart(parts, partIndexRef.current, direction);
      if (next !== null) {
        void persist(partIndexRef.current, snapshotRef.current, true);
        void loadPart(next, autoplay);
      }
    },
    [loadPart, parts, persist, playing],
  );
  advanceRef.current = () => moveToPart(1, true);

  useEffect(() => {
    if (initialPart >= 0)
      void loadPart(initialPart, false, book.lastPositionSeconds);
    else {
      setLoading(false);
      setError(t("audioNoPlayableParts"));
    }
    return () => {
      requestRef.current += 1;
      if (seekTimerRef.current) clearTimeout(seekTimerRef.current);
      void persist(partIndexRef.current, snapshotRef.current, true);
      void pauseAudio().catch(() => undefined);
    };
  }, [book.lastPositionSeconds, initialPart, loadPart, persist, t]);

  useEffect(() => {
    let disposed = false;
    void Promise.all([
      listAudiobookBookmarks(book.id),
      listAudiobookChapters(book.id),
    ])
      .then(([nextBookmarks, nextChapters]) => {
        if (!disposed) {
          setBookmarks(nextBookmarks);
          setChapters(nextChapters);
        }
      })
      .catch((reason: unknown) => {
        if (!disposed)
          setError(reason instanceof Error ? reason.message : String(reason));
      });
    return () => {
      disposed = true;
    };
  }, [book.id]);

  useEffect(() => {
    let disposed = false;
    void listAudioOutputDevices()
      .then(async (devices) => {
        if (disposed) return;
        setOutputDevices(devices);
        const preferred = outputDeviceId;
        const preferredAvailable = devices.some(
          (device) => device.id === preferred && device.isEnabled,
        );
        const target = preferredAvailable
          ? preferred
          : devices.find((device) => device.isDefault && device.isEnabled)?.id;
        if (target) {
          const next = await setAudioOutputDevice(target);
          if (!disposed) applySnapshot(next);
        }
        if (preferred && !preferredAvailable) {
          writeLocalValue(audioOutputDeviceKey, "");
          setOutputDeviceId("");
        }
      })
      .catch(() => undefined);
    return () => {
      disposed = true;
    };
  }, [applySnapshot, outputDeviceId]);

  useEffect(() => {
    let disposed = false;
    void startAudiobookSession(book.id, book.progress)
      .then((token) => {
        if (disposed) {
          if (token) void endAudiobookSession(token).catch(() => undefined);
        } else {
          listeningTokenRef.current = token;
        }
      })
      .catch(() => undefined);
    const timer = window.setInterval(() => {
      const token = listeningTokenRef.current;
      if (!token) return;
      const value = snapshotRef.current;
      const index = partIndexRef.current;
      const partProgress =
        value.durationSeconds > 0
          ? value.positionSeconds / value.durationSeconds
          : 0;
      const progress = Math.max(
        0,
        Math.min(1, (index + partProgress) / Math.max(1, parts.length)),
      );
      void recordAudiobookActivity(
        token,
        value.phase === "playing",
        progress,
      ).catch(() => undefined);
    }, 5000);
    return () => {
      disposed = true;
      window.clearInterval(timer);
      const token = listeningTokenRef.current;
      listeningTokenRef.current = null;
      if (token) void endAudiobookSession(token).catch(() => undefined);
    };
  }, [book.id, book.progress, parts.length]);

  useEffect(() => {
    const timer = window.setInterval(() => {
      if (loading || pollBusyRef.current) return;
      pollBusyRef.current = true;
      void getAudioSnapshot()
        .then((next) => {
          applySnapshot(next);
          void persist(partIndexRef.current, next);
          if (next.phase === "ended" && !endedHandledRef.current) {
            endedHandledRef.current = true;
            if (sleepModeRef.current === "end-part") {
              sleepModeRef.current = "off";
              setSleepMode("off");
              setSleepRemaining(0);
            } else {
              advanceRef.current();
            }
          }
        })
        .catch((reason: unknown) =>
          setError(reason instanceof Error ? reason.message : String(reason)),
        )
        .finally(() => {
          pollBusyRef.current = false;
        });
    }, 750);
    return () => window.clearInterval(timer);
  }, [applySnapshot, loading, persist]);

  const runControl = async (
    operation: () => Promise<AudioPlaybackSnapshot>,
    persistAfter = false,
  ) => {
    setBusy(true);
    setError("");
    try {
      const next = await operation();
      applySnapshot(next);
      if (persistAfter) await persist(partIndexRef.current, next, true);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusy(false);
    }
  };

  const commitSeek = (seconds: number) => {
    const bounded = Math.max(0, Math.min(seconds, snapshot.durationSeconds));
    setSeekDraft(bounded);
    if (seekTimerRef.current) clearTimeout(seekTimerRef.current);
    seekTimerRef.current = setTimeout(() => {
      void runControl(() => seekAudio(bounded), true).finally(() =>
        setSeekDraft(null),
      );
    }, 100);
  };

  const chooseSleepTimer = (mode: SleepTimerMode) => {
    sleepModeRef.current = mode;
    setSleepMode(mode);
    if (mode === "off" || mode === "end-part") {
      setSleepDeadline(null);
      setSleepRemaining(0);
      return;
    }
    const seconds = Number(mode) * 60;
    setSleepDeadline(Date.now() + seconds * 1000);
    setSleepRemaining(seconds);
  };

  useEffect(() => {
    if (sleepDeadline === null) return;
    const timer = window.setInterval(() => {
      const remaining = Math.max(
        0,
        Math.ceil((sleepDeadline - Date.now()) / 1000),
      );
      setSleepRemaining(remaining);
      if (remaining === 0) {
        window.clearInterval(timer);
        sleepModeRef.current = "off";
        setSleepMode("off");
        setSleepDeadline(null);
        void pauseAudio()
          .then((next) => {
            applySnapshot(next);
            return persist(partIndexRef.current, next, true);
          })
          .catch((reason: unknown) =>
            setError(reason instanceof Error ? reason.message : String(reason)),
          );
      }
    }, 1000);
    return () => window.clearInterval(timer);
  }, [applySnapshot, persist, sleepDeadline]);

  const jumpToLocation = (index: number, seconds: number) => {
    if (index === partIndexRef.current) {
      commitSeek(seconds);
      return;
    }
    void persist(partIndexRef.current, snapshotRef.current, true);
    void loadPart(index, playing, seconds);
  };

  const addBookmark = async () => {
    setBusy(true);
    setError("");
    try {
      const created = await createAudiobookBookmark(
        book.id,
        partIndexRef.current,
        snapshotRef.current.positionSeconds,
        bookmarkNote.trim(),
      );
      setBookmarks((items) =>
        [...items, created].sort(
          (left, right) =>
            left.partIndex - right.partIndex ||
            left.positionSeconds - right.positionSeconds,
        ),
      );
      setBookmarkNote("");
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusy(false);
    }
  };

  const removeBookmark = async (bookmarkId: number) => {
    setBusy(true);
    setError("");
    try {
      await deleteAudiobookBookmark(bookmarkId);
      setBookmarks((items) => items.filter((item) => item.id !== bookmarkId));
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusy(false);
    }
  };

  const handleCloseDecision = async (decision: AudioCloseDecision) => {
    setBusy(true);
    setError("");
    try {
      await persist(partIndexRef.current, snapshotRef.current, true);
      await resolveAudioClose(decision, rememberCloseChoice);
      setClosePrompt(false);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listenForAudioCloseRequest(() => setClosePrompt(true)).then(
      (stopListening) => {
        if (disposed) stopListening();
        else unlisten = stopListening;
      },
    );
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  const closePlayer = async () => {
    if (snapshotRef.current.path && snapshotRef.current.canPause) {
      await runControl(pauseAudio, true);
    } else {
      await persist(partIndexRef.current, snapshotRef.current, true);
    }
    onClose();
  };

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      if (target?.matches("input, select, textarea, button")) return;
      if (event.code === "Space") {
        event.preventDefault();
        void runControl(playing ? pauseAudio : playAudio, true);
      } else if (event.key === "ArrowLeft") {
        event.preventDefault();
        commitSeek(snapshotRef.current.positionSeconds - 15);
      } else if (event.key === "ArrowRight") {
        event.preventDefault();
        commitSeek(snapshotRef.current.positionSeconds + 15);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  });

  return (
    <main className="audio-player-screen">
      <header className="audio-player-header">
        <button
          className="secondary-button audio-player-back"
          type="button"
          disabled={busy}
          onClick={() => void closePlayer()}
        >
          {t("backToAudiobooks")}
        </button>
        <div className="audio-player-status" aria-live="polite">
          {loading ? t("audioPlayerLoading") : phaseLabel(snapshot.phase, t)}
        </div>
      </header>

      {error && (
        <p className="error-message audio-player-error" role="alert">
          {t("audioPlaybackError")}: {error}
        </p>
      )}

      <section
        className="audio-player-layout"
        aria-labelledby="audio-player-title"
      >
        <div className="audio-player-now">
          {book.coverPath ? (
            <img
              className="audio-player-cover"
              src={coverUrl(book.coverPath)}
              alt=""
            />
          ) : (
            <div className="audio-player-cover audio-cover-fallback">
              <Icon name="audio" />
              <strong>{book.title}</strong>
              <small>{book.author || t("unknownAuthor")}</small>
            </div>
          )}
          <p className="eyebrow">{t("audiobooks")}</p>
          <h1 id="audio-player-title">{book.title}</h1>
          <p className="audio-player-author">
            {book.author || t("unknownAuthor")}
          </p>
          <p className="audio-current-part">
            {t("audioCurrentPart")
              .replace("{current}", String(partIndex + 1))
              .replace("{total}", String(parts.length))}
            {currentPart ? ` · ${currentPart.title}` : ""}
          </p>
        </div>

        <div className="audio-player-controls">
          <div className="audio-timeline">
            <input
              type="range"
              min="0"
              max={Math.max(1, snapshot.durationSeconds)}
              step="1"
              value={Math.min(
                displayedPosition,
                Math.max(1, snapshot.durationSeconds),
              )}
              disabled={loading || !snapshot.canSeek}
              aria-label={t("audioSeek")}
              onChange={(event) => commitSeek(Number(event.target.value))}
            />
            <div>
              <span>{formatTime(displayedPosition)}</span>
              <span>{formatTime(snapshot.durationSeconds)}</span>
            </div>
          </div>

          <div
            className="audio-transport"
            aria-label={t("audioPlaybackControls")}
          >
            <button
              type="button"
              disabled={loading || busy}
              aria-label={t("audioPreviousPart")}
              onClick={() => {
                if (snapshot.positionSeconds > 5) commitSeek(0);
                else moveToPart(-1);
              }}
            >
              <Icon name="previous" />
            </button>
            <button
              type="button"
              disabled={loading || busy || !snapshot.canSeek}
              aria-label={t("audioRewind")}
              aria-keyshortcuts="ArrowLeft"
              onClick={() => commitSeek(snapshot.positionSeconds - 15)}
            >
              <Icon name="rewind" />
              <span>15</span>
            </button>
            <button
              className="audio-play-button"
              type="button"
              disabled={loading || busy || !currentPart}
              aria-label={playing ? t("audioPause") : t("audioPlay")}
              aria-keyshortcuts="Space"
              onClick={() =>
                void runControl(playing ? pauseAudio : playAudio, true)
              }
            >
              <Icon name={playing ? "pause" : "play"} />
            </button>
            <button
              type="button"
              disabled={loading || busy || !snapshot.canSeek}
              aria-label={t("audioForward")}
              aria-keyshortcuts="ArrowRight"
              onClick={() => commitSeek(snapshot.positionSeconds + 15)}
            >
              <Icon name="forward" />
              <span>15</span>
            </button>
            <button
              type="button"
              disabled={
                loading ||
                busy ||
                nextAvailablePart(parts, partIndex, 1) === null
              }
              aria-label={t("audioNextPart")}
              onClick={() => moveToPart(1)}
            >
              <Icon name="next" />
            </button>
          </div>

          <div className="audio-adjustments">
            <label>
              <span>{t("audioSpeed")}</span>
              <select
                value={rate}
                disabled={loading || busy}
                onChange={(event) => {
                  const value = Number(event.target.value);
                  setRate(value);
                  rateRef.current = value;
                  writeLocalValue(audioRateKey, String(value));
                  void runControl(() => setAudioRate(value));
                }}
              >
                {playbackRates.map((value) => (
                  <option key={value} value={value}>
                    {value}×
                  </option>
                ))}
              </select>
            </label>
            <label>
              <span>{t("audioSleepTimer")}</span>
              <select
                value={sleepMode}
                onChange={(event) =>
                  chooseSleepTimer(event.target.value as SleepTimerMode)
                }
              >
                <option value="off">{t("audioSleepOff")}</option>
                <option value="end-part">{t("audioSleepEndPart")}</option>
                <option value="15">
                  {t("audioSleepMinutes").replace("{count}", "15")}
                </option>
                <option value="30">
                  {t("audioSleepMinutes").replace("{count}", "30")}
                </option>
                <option value="45">
                  {t("audioSleepMinutes").replace("{count}", "45")}
                </option>
                <option value="60">
                  {t("audioSleepMinutes").replace("{count}", "60")}
                </option>
              </select>
              {sleepRemaining > 0 && (
                <small className="audio-sleep-status" role="status">
                  {t("audioSleepRemaining").replace(
                    "{time}",
                    formatTime(sleepRemaining),
                  )}
                </small>
              )}
            </label>
            <label>
              <span>{t("audioOutputDevice")}</span>
              <select
                value={outputDeviceId}
                disabled={outputDevices.length === 0 || busy}
                onChange={(event) => {
                  const value = event.target.value;
                  setOutputDeviceId(value);
                  writeLocalValue(audioOutputDeviceKey, value);
                }}
              >
                <option value="">{t("audioSystemDefaultDevice")}</option>
                {outputDevices.map((device) => (
                  <option
                    key={device.id}
                    value={device.id}
                    disabled={!device.isEnabled}
                  >
                    {device.name}
                    {device.isDefault ? ` (${t("audioDefaultDevice")})` : ""}
                  </option>
                ))}
              </select>
            </label>
            <label className="audio-volume-control">
              <Icon name="volume" />
              <span className="sr-only">{t("audioVolume")}</span>
              <input
                type="range"
                min="0"
                max="1"
                step="0.05"
                value={volume}
                disabled={loading}
                aria-label={t("audioVolume")}
                onChange={(event) => {
                  const value = Number(event.target.value);
                  setVolume(value);
                  volumeRef.current = value;
                  writeLocalValue(audioVolumeKey, String(value));
                  void runControl(() => setAudioVolume(value));
                }}
              />
              <output>{Math.round(volume * 100)}%</output>
            </label>
          </div>
        </div>

        <aside className="audio-queue" aria-labelledby="audio-queue-title">
          <section className="audio-location-section audio-parts-section">
            <h2 id="audio-queue-title">{t("audioQueue")}</h2>
            <ol>
              {parts.map((part, index) => (
                <li key={part.id}>
                  <button
                    type="button"
                    className={index === partIndex ? "active" : ""}
                    aria-current={index === partIndex ? "true" : undefined}
                    disabled={!part.isAvailable || loading || busy}
                    onClick={() => {
                      void persist(
                        partIndexRef.current,
                        snapshotRef.current,
                        true,
                      );
                      void loadPart(
                        index,
                        playing,
                        index === book.lastPartIndex
                          ? book.lastPositionSeconds
                          : 0,
                      );
                    }}
                  >
                    <span>{index + 1}</span>
                    <span>
                      <strong>{part.title}</strong>
                      <small>
                        {part.format}
                        {part.durationSeconds
                          ? ` · ${formatTime(part.durationSeconds)}`
                          : ""}
                      </small>
                    </span>
                  </button>
                </li>
              ))}
            </ol>
          </section>

          <section className="audio-location-section">
            <h2>{t("audioChapters")}</h2>
            {chapters.length ? (
              <ol>
                {chapters.map((chapter) => (
                  <li key={chapter.id}>
                    <button
                      type="button"
                      disabled={loading || busy}
                      onClick={() =>
                        jumpToLocation(chapter.partIndex, chapter.startSeconds)
                      }
                    >
                      <span>{chapter.ordinal + 1}</span>
                      <span>
                        <strong>{chapter.title}</strong>
                        <small>
                          {t("audioPartShort").replace(
                            "{count}",
                            String(chapter.partIndex + 1),
                          )}{" "}
                          · {formatTime(chapter.startSeconds)}
                        </small>
                      </span>
                    </button>
                  </li>
                ))}
              </ol>
            ) : (
              <p className="audio-location-empty">{t("audioNoChapters")}</p>
            )}
          </section>

          <section className="audio-location-section">
            <h2>{t("audioBookmarks")}</h2>
            <div className="audio-bookmark-form">
              <input
                value={bookmarkNote}
                maxLength={512}
                placeholder={t("audioBookmarkNote")}
                aria-label={t("audioBookmarkNote")}
                onChange={(event) => setBookmarkNote(event.target.value)}
              />
              <button
                className="secondary-button"
                type="button"
                disabled={loading || busy || !currentPart}
                onClick={() => void addBookmark()}
              >
                {t("audioAddBookmark")}
              </button>
            </div>
            {bookmarks.length ? (
              <ol>
                {bookmarks.map((bookmark) => (
                  <li className="audio-bookmark-row" key={bookmark.id}>
                    <button
                      type="button"
                      disabled={loading || busy}
                      onClick={() =>
                        jumpToLocation(
                          bookmark.partIndex,
                          bookmark.positionSeconds,
                        )
                      }
                    >
                      <span>{bookmark.partIndex + 1}</span>
                      <span>
                        <strong>
                          {bookmark.note || t("audioBookmarkDefault")}
                        </strong>
                        <small>{formatTime(bookmark.positionSeconds)}</small>
                      </span>
                    </button>
                    <button
                      className="audio-bookmark-delete"
                      type="button"
                      disabled={busy}
                      aria-label={t("audioDeleteBookmark")}
                      onClick={() => void removeBookmark(bookmark.id)}
                    >
                      ×
                    </button>
                  </li>
                ))}
              </ol>
            ) : (
              <p className="audio-location-empty">{t("audioNoBookmarks")}</p>
            )}
          </section>
        </aside>
      </section>

      {closePrompt && (
        <div className="dialog-backdrop" role="presentation">
          <section
            className="confirmation-dialog audio-close-dialog"
            role="dialog"
            aria-modal="true"
            aria-labelledby="audio-close-title"
          >
            <h2 id="audio-close-title">{t("audioCloseTitle")}</h2>
            <p>{t("audioCloseHint")}</p>
            <label className="settings-toggle">
              <input
                type="checkbox"
                checked={rememberCloseChoice}
                onChange={(event) =>
                  setRememberCloseChoice(event.target.checked)
                }
              />
              <span>
                <strong>{t("audioRememberCloseChoice")}</strong>
              </span>
            </label>
            <div className="dialog-actions">
              <button
                className="primary-button"
                type="button"
                disabled={busy}
                onClick={() => void handleCloseDecision("tray")}
              >
                {t("audioContinueInTray")}
              </button>
              <button
                className="danger-button"
                type="button"
                disabled={busy}
                onClick={() => void handleCloseDecision("exit")}
              >
                {t("audioExitCompletely")}
              </button>
              <button
                className="secondary-button"
                type="button"
                disabled={busy}
                onClick={() => setClosePrompt(false)}
              >
                {t("cancel")}
              </button>
            </div>
          </section>
        </div>
      )}
    </main>
  );
}

function boundedInitialPart(
  book: AudiobookRecord,
  parts: AudiobookPartRecord[],
) {
  const preferred = Math.max(0, Math.min(book.lastPartIndex, parts.length - 1));
  if (parts[preferred]?.isAvailable) return preferred;
  return parts.findIndex((part) => part.isAvailable);
}

function nextAvailablePart(
  parts: AudiobookPartRecord[],
  current: number,
  direction: -1 | 1,
) {
  for (
    let index = current + direction;
    index >= 0 && index < parts.length;
    index += direction
  ) {
    if (parts[index]?.isAvailable) return index;
  }
  return null;
}

function readNumber(
  key: string,
  fallback: number,
  minimum: number,
  maximum: number,
) {
  const stored = readLocalValue(key);
  if (stored === null) return fallback;
  const value = Number(stored);
  return Number.isFinite(value) && value >= minimum && value <= maximum
    ? value
    : fallback;
}

function formatTime(seconds: number) {
  if (!Number.isFinite(seconds) || seconds <= 0) return "0:00";
  const total = Math.floor(seconds);
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const remaining = total % 60;
  return hours > 0
    ? `${hours}:${String(minutes).padStart(2, "0")}:${String(remaining).padStart(2, "0")}`
    : `${minutes}:${String(remaining).padStart(2, "0")}`;
}

function phaseLabel(phase: AudioPlaybackSnapshot["phase"], t: Translator) {
  if (phase === "playing") return t("audioPlaying");
  if (phase === "buffering" || phase === "opening")
    return t("audioPlayerLoading");
  if (phase === "ended") return t("audioEnded");
  if (phase === "failed") return t("audioPlaybackFailed");
  return t("audioPaused");
}
