import { useMemo, useState } from "react";
import {
  applyAudiobookMetadataCandidate,
  audiobookMetadataFromRecord,
  chooseAndSetAudiobookCover,
  searchAudiobookMetadata,
  updateAudiobookMetadata,
  type AudiobookMetadataInput,
  AudiobookPartRecord,
  AudiobookRecord,
  WatchedAudioFolder,
} from "../application/audiobooks";
import { coverUrl } from "../application/library";
import type {
  MetadataCandidate,
  MetadataLanguage,
} from "../application/metadata";
import { Icon } from "./icons";
import type { Locale, TranslationKey } from "./i18n";

type Translator = (key: TranslationKey) => string;

interface AudiobooksPageProps {
  audiobooks: AudiobookRecord[];
  folders: WatchedAudioFolder[];
  selectedId: number | null;
  loading: boolean;
  busy: boolean;
  locale: Locale;
  t: Translator;
  onSelect: (id: number) => void;
  onImportFiles: () => void;
  onImportFolder: () => void;
  onWatchFolder: () => void;
  onScan: () => void;
}

export function AudiobooksPage({
  audiobooks,
  folders,
  selectedId,
  loading,
  busy,
  locale,
  t,
  onSelect,
  onImportFiles,
  onImportFolder,
  onWatchFolder,
  onScan,
}: AudiobooksPageProps) {
  const [query, setQuery] = useState("");
  const visible = useMemo(() => {
    const normalized = query.trim().toLocaleLowerCase(locale);
    if (!normalized) return audiobooks;
    return audiobooks.filter((book) =>
      `${book.title} ${book.author}`
        .toLocaleLowerCase(locale)
        .includes(normalized),
    );
  }, [audiobooks, locale, query]);
  const totalParts = audiobooks.reduce((sum, book) => sum + book.partCount, 0);
  const available = audiobooks.filter((book) => book.isAvailable).length;

  return (
    <div className="audiobooks-page">
      <section className="stat-grid" aria-label={t("audioLibraryStats")}>
        <AudioStat
          value={String(audiobooks.length)}
          label={t("audiobookCount")}
        />
        <AudioStat value={String(totalParts)} label={t("audioParts")} />
        <AudioStat value={String(available)} label={t("audioAvailable")} />
        <AudioStat
          value={String(folders.length)}
          label={t("audioWatchedFolders")}
        />
      </section>

      <div className="library-toolbar audio-toolbar">
        <div className="section-heading">
          <div>
            <h2>{t("audioLibrary")}</h2>
            <p>{t("audioLibraryHint")}</p>
          </div>
          <span>{visible.length}</span>
        </div>
        <div className="toolbar-actions">
          <button
            className="secondary-button"
            type="button"
            disabled={busy}
            onClick={onImportFolder}
          >
            <Icon name="folder" />
            {t("addAudioFolder")}
          </button>
          <button
            className="secondary-button"
            type="button"
            disabled={busy}
            onClick={onWatchFolder}
          >
            <Icon name="refresh" />
            {t("watchAudioFolder")}
          </button>
          <button
            className="primary-button"
            type="button"
            disabled={busy}
            onClick={onImportFiles}
          >
            <Icon name="plus" />
            {t("addAudiobooks")}
          </button>
        </div>
      </div>

      {audiobooks.length > 0 && (
        <label className="search audio-search">
          <span className="sr-only">{t("audioSearch")}</span>
          <Icon name="search" />
          <input
            type="search"
            placeholder={t("audioSearch")}
            value={query}
            onChange={(event) => setQuery(event.target.value)}
          />
        </label>
      )}

      {loading ? (
        <AudioEmpty title={t("audioLoading")} hint="" />
      ) : audiobooks.length === 0 ? (
        <AudioEmpty
          title={t("noAudiobooks")}
          hint={t("noAudiobooksHint")}
          action={onImportFiles}
          actionLabel={t("addAudiobooks")}
        />
      ) : visible.length === 0 ? (
        <AudioEmpty
          title={t("audioNoResults")}
          hint={t("audioNoResultsHint")}
        />
      ) : (
        <section className="audio-grid" aria-label={t("audioLibrary")}>
          {visible.map((book) => (
            <button
              key={book.id}
              type="button"
              className={`audio-card ${selectedId === book.id ? "selected" : ""}`}
              aria-pressed={selectedId === book.id}
              onClick={() => onSelect(book.id)}
            >
              <AudioCover book={book} t={t} />
              <span className="audio-card-copy">
                <strong>{book.title}</strong>
                <small>{book.author || t("unknownAuthor")}</small>
                <span className="audio-card-meta">
                  {t("audioPartCount").replace(
                    "{count}",
                    String(book.partCount),
                  )}
                  <i aria-hidden="true">•</i>
                  {formatBytes(book.totalSize, locale)}
                </span>
                {!book.isAvailable && (
                  <span className="availability-badge">
                    {t("audioIncomplete")}
                  </span>
                )}
                <span
                  className="audio-progress"
                  aria-label={t("audioProgress")}
                >
                  <i style={{ width: `${Math.round(book.progress * 100)}%` }} />
                </span>
              </span>
            </button>
          ))}
        </section>
      )}

      <section className="audio-folders" aria-labelledby="audio-folders-title">
        <div className="section-heading">
          <div>
            <h2 id="audio-folders-title">{t("audioSourceFolders")}</h2>
            <p>{t("audioSourceFoldersHint")}</p>
          </div>
          <button
            className="secondary-button"
            type="button"
            disabled={busy || folders.length === 0}
            onClick={onScan}
          >
            <Icon name="refresh" />
            {t("scanAudioFolders")}
          </button>
        </div>
        {folders.length === 0 ? (
          <p className="audio-folders-empty">{t("audioFoldersEmpty")}</p>
        ) : (
          <div className="folder-list">
            {folders.map((folder) => (
              <article key={folder.id}>
                <Icon name="folder" />
                <span>
                  <strong>{folder.path}</strong>
                  <small>
                    {folder.lastScannedAt || t("audioNeverScanned")}
                  </small>
                </span>
              </article>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}

export function AudiobookDetails({
  book,
  parts,
  loading,
  locale,
  t,
  onOpenPlayer,
  onChanged,
  onClose,
}: {
  book: AudiobookRecord | null;
  parts: AudiobookPartRecord[];
  loading: boolean;
  locale: Locale;
  t: Translator;
  onOpenPlayer: (book: AudiobookRecord) => void;
  onChanged: (book: AudiobookRecord) => void;
  onClose: () => void;
}) {
  const [mode, setMode] = useState<"details" | "edit" | "search">("details");
  const [metadata, setMetadata] = useState<AudiobookMetadataInput | null>(
    book ? audiobookMetadataFromRecord(book) : null,
  );
  const [query, setQuery] = useState(
    book ? `${book.title} ${book.author}`.trim() : "",
  );
  const [language, setLanguage] = useState<MetadataLanguage>("ru");
  const [candidates, setCandidates] = useState<MetadataCandidate[]>([]);
  const [metadataBusy, setMetadataBusy] = useState(false);
  const [metadataMessage, setMetadataMessage] = useState("");

  const runMetadata = async (
    operation: () => Promise<AudiobookRecord | null>,
  ) => {
    setMetadataBusy(true);
    setMetadataMessage("");
    try {
      const updated = await operation();
      if (updated) {
        onChanged(updated);
        setMetadata(audiobookMetadataFromRecord(updated));
        setMode("details");
        setMetadataMessage(t("metadataSaved"));
      }
    } catch (reason) {
      setMetadataMessage(
        reason instanceof Error ? reason.message : String(reason),
      );
    } finally {
      setMetadataBusy(false);
    }
  };

  return (
    <aside
      className={`details-panel audio-details ${book ? "open" : ""}`}
      aria-label={book ? undefined : t("audioDetailsEmpty")}
      aria-labelledby={book ? "audio-details-title" : undefined}
    >
      {!book ? (
        <div className="details-empty">
          <span className="empty-emblem">
            <Icon name="audio" />
          </span>
          <h2>{t("audioDetailsEmpty")}</h2>
          <p>{t("audioDetailsHint")}</p>
        </div>
      ) : (
        <div className="book-details">
          <button
            className="details-close"
            type="button"
            onClick={onClose}
            aria-label={t("closeAudioDetails")}
          >
            ×
          </button>
          <AudioCover book={book} t={t} large />
          <h2 id="audio-details-title">{book.title}</h2>
          <p className="details-author">{book.author || t("unknownAuthor")}</p>
          {metadataMessage && (
            <p className="metadata-status" role="status">
              {metadataMessage}
            </p>
          )}
          {mode === "edit" && metadata ? (
            <form
              className="metadata-form-grid audio-metadata-form"
              onSubmit={(event) => {
                event.preventDefault();
                void runMetadata(() =>
                  updateAudiobookMetadata(book.id, metadata),
                );
              }}
            >
              <AudioMetadataFields
                value={metadata}
                onChange={setMetadata}
                t={t}
              />
              <div className="dialog-actions">
                <button
                  className="primary-button"
                  type="submit"
                  disabled={metadataBusy}
                >
                  {t("save")}
                </button>
                <button
                  className="secondary-button"
                  type="button"
                  disabled={metadataBusy}
                  onClick={() =>
                    void runMetadata(() => chooseAndSetAudiobookCover(book.id))
                  }
                >
                  {t("changeCover")}
                </button>
                <button
                  className="secondary-button"
                  type="button"
                  onClick={() => setMode("details")}
                >
                  {t("cancel")}
                </button>
              </div>
            </form>
          ) : mode === "search" ? (
            <section className="audio-metadata-search">
              <label>
                {t("searchQuery")}
                <input
                  value={query}
                  onChange={(event) => setQuery(event.target.value)}
                />
              </label>
              <label>
                {t("metadataLanguage")}
                <select
                  value={language}
                  onChange={(event) =>
                    setLanguage(event.target.value as MetadataLanguage)
                  }
                >
                  <option value="ru">{t("metadataLanguageRussian")}</option>
                  <option value="en">{t("metadataLanguageEnglish")}</option>
                </select>
              </label>
              <div className="dialog-actions">
                <button
                  className="primary-button"
                  type="button"
                  disabled={metadataBusy}
                  onClick={() => {
                    setMetadataBusy(true);
                    setMetadataMessage("");
                    void searchAudiobookMetadata(book.id, query, language)
                      .then(setCandidates)
                      .catch((reason: unknown) =>
                        setMetadataMessage(
                          reason instanceof Error
                            ? reason.message
                            : String(reason),
                        ),
                      )
                      .finally(() => setMetadataBusy(false));
                  }}
                >
                  {t("searchMetadataAction")}
                </button>
                <button
                  className="secondary-button"
                  type="button"
                  onClick={() => setMode("details")}
                >
                  {t("cancel")}
                </button>
              </div>
              <div className="audio-metadata-candidates">
                {candidates.map((candidate) => (
                  <article
                    key={`${candidate.provider}:${candidate.providerId}`}
                  >
                    <strong>{candidate.title}</strong>
                    <small>
                      {candidate.author || t("unknownAuthor")} ·{" "}
                      {candidate.provider}
                    </small>
                    <button
                      className="secondary-button"
                      type="button"
                      disabled={metadataBusy}
                      onClick={() =>
                        void runMetadata(() =>
                          applyAudiobookMetadataCandidate(book.id, candidate),
                        )
                      }
                    >
                      {t("applyCandidate")}
                    </button>
                  </article>
                ))}
              </div>
            </section>
          ) : (
            <>
              {!book.isAvailable && (
                <p className="availability-warning">
                  {t("audioUnavailableHint")}
                </p>
              )}
              <dl className="audio-facts">
                <div>
                  <dt>{t("audioParts")}</dt>
                  <dd>{book.partCount}</dd>
                </div>
                <div>
                  <dt>{t("audioBookSize")}</dt>
                  <dd>{formatBytes(book.totalSize, locale)}</dd>
                </div>
              </dl>
              {(book.narrator ||
                book.series ||
                book.genres ||
                book.publishedYear) && (
                <dl className="audio-facts audio-metadata-facts">
                  {book.narrator && (
                    <div>
                      <dt>{t("audioNarrator")}</dt>
                      <dd>{book.narrator}</dd>
                    </div>
                  )}
                  {book.series && (
                    <div>
                      <dt>{t("seriesField")}</dt>
                      <dd>{book.series}</dd>
                    </div>
                  )}
                  {book.genres && (
                    <div>
                      <dt>{t("genresField")}</dt>
                      <dd>{book.genres}</dd>
                    </div>
                  )}
                  {book.publishedYear && (
                    <div>
                      <dt>{t("publishedYear")}</dt>
                      <dd>{book.publishedYear}</dd>
                    </div>
                  )}
                </dl>
              )}
              {book.description && (
                <p className="audio-description">{book.description}</p>
              )}
              <div className="metadata-actions">
                <button
                  className="secondary-button"
                  type="button"
                  onClick={() => setMode("edit")}
                >
                  {t("editMetadata")}
                </button>
                <button
                  className="secondary-button"
                  type="button"
                  onClick={() => setMode("search")}
                >
                  {t("findMetadata")}
                </button>
              </div>
              <button
                className="primary-button details-read"
                type="button"
                disabled={!book.isAvailable}
                onClick={() => onOpenPlayer(book)}
              >
                <Icon name="audio" />
                {t("openAudioPlayer")}
              </button>
              <section
                className="audio-parts"
                aria-labelledby="audio-parts-title"
              >
                <h3 id="audio-parts-title">{t("audioParts")}</h3>
                {loading ? (
                  <p>{t("audioPartsLoading")}</p>
                ) : (
                  <ol>
                    {parts.map((part) => (
                      <li
                        key={part.id}
                        className={part.isAvailable ? "" : "unavailable"}
                      >
                        <span>{part.ordinal + 1}</span>
                        <span>
                          <strong>{part.title}</strong>
                          <small>
                            {part.format} · {formatBytes(part.fileSize, locale)}
                          </small>
                        </span>
                        {!part.isAvailable && <em>{t("partUnavailable")}</em>}
                      </li>
                    ))}
                  </ol>
                )}
              </section>
            </>
          )}
        </div>
      )}
    </aside>
  );
}

function AudioMetadataFields({
  value,
  onChange,
  t,
}: {
  value: AudiobookMetadataInput;
  onChange: (value: AudiobookMetadataInput) => void;
  t: Translator;
}) {
  const field = (
    key: keyof AudiobookMetadataInput,
    label: TranslationKey,
    limit: number,
  ) => (
    <label>
      {t(label)}
      <input
        value={value[key]}
        maxLength={limit}
        onChange={(event) => onChange({ ...value, [key]: event.target.value })}
      />
    </label>
  );
  return (
    <>
      {field("title", "titleField", 512)}
      {field("author", "authorField", 512)}
      {field("narrator", "audioNarrator", 512)}
      {field("series", "seriesField", 512)}
      {field("genres", "genresField", 1024)}
      {field("language", "languageField", 64)}
      {field("publishedYear", "publishedYear", 32)}
      <label>
        {t("descriptionField")}
        <textarea
          value={value.description}
          maxLength={16384}
          onChange={(event) =>
            onChange({ ...value, description: event.target.value })
          }
        />
      </label>
    </>
  );
}

function AudioCover({
  book,
  t,
  large = false,
}: {
  book: AudiobookRecord;
  t: Translator;
  large?: boolean;
}) {
  return book.coverPath ? (
    <img
      className={`audio-cover ${large ? "large" : ""}`}
      src={coverUrl(book.coverPath)}
      alt=""
    />
  ) : (
    <span
      className={`audio-cover audio-cover-fallback ${large ? "large" : ""}`}
    >
      <Icon name="audio" />
      <strong>{book.title}</strong>
      <small>{book.author || t("unknownAuthor")}</small>
    </span>
  );
}

function AudioStat({ value, label }: { value: string; label: string }) {
  return (
    <article className="stat-card">
      <span className="stat-icon">
        <Icon name="audio" />
      </span>
      <div>
        <strong>{value}</strong>
        <small>{label}</small>
      </div>
    </article>
  );
}

function AudioEmpty({
  title,
  hint,
  action,
  actionLabel,
}: {
  title: string;
  hint: string;
  action?: () => void;
  actionLabel?: string;
}) {
  return (
    <section className="empty-state">
      <span className="empty-emblem">
        <Icon name="audio" />
      </span>
      <h2>{title}</h2>
      {hint && <p>{hint}</p>}
      {action && actionLabel && (
        <button className="primary-button" type="button" onClick={action}>
          {actionLabel}
        </button>
      )}
    </section>
  );
}

function formatBytes(bytes: number, locale: Locale) {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KiB", "MiB", "GiB", "TiB"];
  let value = bytes / 1024;
  let unit = units[0]!;
  for (let index = 1; index < units.length && value >= 1024; index += 1) {
    value /= 1024;
    unit = units[index]!;
  }
  return `${new Intl.NumberFormat(locale, { maximumFractionDigits: value < 10 ? 1 : 0 }).format(value)} ${unit}`;
}
