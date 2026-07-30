import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  loadSpecialDocument,
  type SpecialDocument,
} from "../application/fixedReader";
import {
  chooseAndImportBooks,
  chooseAndWatchFolder,
  coverUrl,
  listBooks,
  listWatchedFolders,
  removeBooks,
  scanWatchedFolders,
  setBookFavorite,
  type Book,
  type ImportSummary,
  type WatchedFolder,
} from "../application/library";
import {
  applyMetadataCandidate,
  metadataFromBook,
  removeExternalCover,
  searchMetadata,
  updateBookMetadata,
  type BookMetadataInput,
  type MetadataCandidate,
} from "../application/metadata";
import {
  chooseAndImportLanguagePackage,
  listLanguagePackages,
  removeLanguagePackage,
  type InstalledLanguagePackage,
} from "../application/languageTools";
import { loadDocument, type DocumentModel } from "../application/reader";
import { getStartupHealth, type StartupHealth } from "../application/health";
import {
  emptyStatistics,
  getStatistics,
  type StatisticsSnapshot,
} from "../application/statistics";
import {
  getSteamIntegrationStatus,
  syncSteamAchievements,
  syncSteamIfAvailable,
  type SteamIntegrationStatus,
} from "../application/steam";
import { Icon, type IconName } from "./icons";
import type { TranslationKey } from "./i18n";
import { ReaderScreen } from "./ReaderScreen";
import { SpecialReaderScreen } from "./SpecialReaderScreen";
import { AchievementsPage, StatisticsPage } from "./StatisticsPages";
import { useLocale } from "./useLocale";
import {
  normalizeBookLanguage,
  useScreenReaderSupport,
} from "./useScreenReaderSupport";

type Route = { id: string; label: TranslationKey; icon: IconName };
const routes: Route[] = [
  { id: "library", label: "library", icon: "library" },
  { id: "reading", label: "readingNow", icon: "reading" },
  { id: "collections", label: "collections", icon: "collections" },
  { id: "authors", label: "authors", icon: "authors" },
  { id: "series", label: "series", icon: "series" },
  { id: "favorites", label: "favorites", icon: "favorite" },
  { id: "achievements", label: "achievements", icon: "achievement" },
  { id: "statistics", label: "statistics", icon: "statistics" },
  { id: "settings", label: "settings", icon: "settings" },
];

export function App() {
  const { locale, t, toggleLocale } = useLocale();
  const { screenReaderSupport, setScreenReaderSupport } =
    useScreenReaderSupport();
  const [route, setRoute] = useState("library");
  const [books, setBooks] = useState<Book[]>([]);
  const [folders, setFolders] = useState<WatchedFolder[]>([]);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [query, setQuery] = useState("");
  const [format, setFormat] = useState("ALL");
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const [document, setDocument] = useState<DocumentModel | null>(null);
  const [specialDocument, setSpecialDocument] =
    useState<SpecialDocument | null>(null);
  const [readerLanguage, setReaderLanguage] = useState<string>();
  const [readerLoading, setReaderLoading] = useState(false);
  const [statistics, setStatistics] =
    useState<StatisticsSnapshot>(emptyStatistics);
  const [startupHealth, setStartupHealth] = useState<StartupHealth | null>(
    null,
  );
  const mainRef = useRef<HTMLElement>(null);
  const previousRoute = useRef(route);
  const current = routes.find((item) => item.id === route) ?? routes[0]!;
  const selected = books.find((book) => book.id === selectedId) ?? null;

  const refresh = useCallback(async () => {
    try {
      const [nextBooks, nextFolders, nextStatistics] = await Promise.all([
        listBooks(),
        listWatchedFolders(),
        getStatistics(),
      ]);
      setBooks(nextBooks);
      setFolders(nextFolders);
      setStatistics(nextStatistics);
      setSelectedId((currentId) =>
        nextBooks.some((book) => book.id === currentId) ? currentId : null,
      );
      setError("");
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
    void syncSteamIfAvailable().catch(() => undefined);
    void getStartupHealth()
      .then(setStartupHealth)
      .catch(() => undefined);
  }, [refresh]);

  useEffect(() => {
    const preventBrowserMenu = (event: Event) => event.preventDefault();
    window.document.addEventListener("contextmenu", preventBrowserMenu);
    return () =>
      window.document.removeEventListener("contextmenu", preventBrowserMenu);
  }, []);

  useEffect(() => {
    if (previousRoute.current !== route) {
      mainRef.current?.focus();
      previousRoute.current = route;
    }
  }, [route]);

  const formats = useMemo(
    () => [...new Set(books.map((book) => book.format))].sort(),
    [books],
  );
  const visibleBooks = useMemo(() => {
    const normalized = query.trim().toLocaleLowerCase(locale);
    return books.filter((book) => {
      const matchesFormat = format === "ALL" || book.format === format;
      const matchesQuery =
        !normalized ||
        `${book.title} ${book.author} ${book.genres} ${book.format}`
          .toLocaleLowerCase(locale)
          .includes(normalized);
      return matchesFormat && matchesQuery;
    });
  }, [books, format, locale, query]);
  const readingNowBooks = useMemo(
    () =>
      books
        .filter((book) => book.lastOpenedAt !== null && book.progress < 0.995)
        .sort(
          (left, right) =>
            (right.lastOpenedAt ?? 0) - (left.lastOpenedAt ?? 0) ||
            left.title.localeCompare(right.title, locale),
        ),
    [books, locale],
  );
  const favoriteBooks = useMemo(
    () =>
      books
        .filter((book) => book.isFavorite)
        .sort((left, right) => left.title.localeCompare(right.title, locale)),
    [books, locale],
  );

  const runImport = async (operation: () => Promise<ImportSummary | null>) => {
    setBusy(true);
    setMessage("");
    setError("");
    try {
      const summary = await operation();
      if (summary) {
        setMessage(summaryMessage(t("importDone"), summary));
        if (summary.errors.length > 0) setError(summary.errors.join("\n"));
        await refresh();
      }
    } catch (reason) {
      setError(
        `${t("importError")}: ${reason instanceof Error ? reason.message : String(reason)}`,
      );
    } finally {
      setBusy(false);
    }
  };

  const toggleFavorite = async (book: Book) => {
    setError("");
    try {
      const updated = await setBookFavorite(book.id, !book.isFavorite);
      setBooks((items) =>
        items.map((item) => (item.id === updated.id ? updated : item)),
      );
      if (route === "favorites" && !updated.isFavorite) {
        setSelectedId((currentId) =>
          currentId === updated.id ? null : currentId,
        );
      }
      return updated;
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
      throw reason;
    }
  };

  const confirmAndRemoveBooks = async (targets: Book[]): Promise<boolean> => {
    const uniqueTargets = [
      ...new Map(targets.map((book) => [book.id, book])).values(),
    ];
    if (uniqueTargets.length === 0) return false;
    const confirmation =
      uniqueTargets.length === 1
        ? t("removeBookConfirm").replace("{title}", uniqueTargets[0]!.title)
        : t("removeBooksConfirm").replace(
            "{count}",
            String(uniqueTargets.length),
          );
    if (!window.confirm(confirmation)) return false;

    setBusy(true);
    setMessage("");
    setError("");
    try {
      const removed = await removeBooks(uniqueTargets.map((book) => book.id));
      setSelectedId((currentId) =>
        uniqueTargets.some((book) => book.id === currentId) ? null : currentId,
      );
      await refresh();
      setMessage(
        removed === 1
          ? t("bookRemoved")
          : t("booksRemoved").replace("{count}", String(removed)),
      );
      return true;
    } catch (reason) {
      setError(
        `${t("removeBooksError")}: ${reason instanceof Error ? reason.message : String(reason)}`,
      );
      return false;
    } finally {
      setBusy(false);
    }
  };

  const openBook = async (book: Book) => {
    setReaderLoading(true);
    setError("");
    try {
      setReaderLanguage(normalizeBookLanguage(book.language));
      if (isSpecialFormat(book.format)) {
        setSpecialDocument(await loadSpecialDocument(book.id));
      } else {
        setDocument(await loadDocument(book.id));
      }
      setSelectedId(null);
    } catch (reason) {
      setError(
        `${t("readerError")}: ${reason instanceof Error ? reason.message : String(reason)}`,
      );
    } finally {
      setReaderLoading(false);
    }
  };

  if (document) {
    return (
      <ReaderScreen
        document={document}
        t={t}
        language={readerLanguage}
        screenReaderSupport={screenReaderSupport}
        onClose={() => {
          setDocument(null);
          void refresh();
        }}
        onProgress={(progress) => {
          setBooks((items) =>
            items.map((book) =>
              book.id === document.bookId ? { ...book, progress } : book,
            ),
          );
        }}
      />
    );
  }

  if (specialDocument) {
    return (
      <SpecialReaderScreen
        document={specialDocument}
        t={t}
        language={readerLanguage}
        screenReaderSupport={screenReaderSupport}
        onClose={() => {
          setSpecialDocument(null);
          void refresh();
        }}
        onProgress={(progress) => {
          setBooks((items) =>
            items.map((book) =>
              book.id === specialDocument.bookId ? { ...book, progress } : book,
            ),
          );
        }}
      />
    );
  }

  return (
    <div className="app-shell">
      <a className="skip-link" href="#main-content">
        {t("skipToContent")}
      </a>
      <aside className="sidebar" aria-label={t("appName")}>
        <div className="brand">
          <span className="brand-mark">
            <Icon name="reading" />
          </span>
          <span className="brand-name">{t("appName")}</span>
        </div>
        <nav className="nav-list" aria-label={t("library")}>
          {routes.map((item) => (
            <button
              className={`nav-item ${route === item.id ? "active" : ""}`}
              type="button"
              key={item.id}
              aria-label={t(item.label)}
              aria-current={route === item.id ? "page" : undefined}
              title={t(item.label)}
              onClick={() => setRoute(item.id)}
            >
              <Icon name={item.icon} />
              <span className="nav-label">{t(item.label)}</span>
            </button>
          ))}
        </nav>
        <div className="sidebar-foot">
          <p>{t("stageLabel")}</p>
          <button
            className="language-button"
            type="button"
            onClick={toggleLocale}
            aria-label={t("switchLanguage")}
          >
            <span aria-hidden="true">{locale === "ru" ? "RU" : "EN"}</span>
            <span className="nav-label">
              {locale === "ru" ? "Русский" : "English"}
            </span>
          </button>
        </div>
      </aside>

      <main
        ref={mainRef}
        id="main-content"
        className="main-content"
        tabIndex={-1}
        aria-labelledby="page-title"
      >
        <section className="page">
          <header className="page-header">
            <div>
              <p className="eyebrow">{t("personalLibrary")}</p>
              <h1 id="page-title">
                {route === "library" ? t("greeting") : t(current.label)}
              </h1>
            </div>
            <label className="search">
              <span className="sr-only">{t("search")}</span>
              <Icon name="search" />
              <input
                type="search"
                placeholder={t("search")}
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                disabled={route !== "library"}
              />
            </label>
          </header>

          {message && (
            <p className="notice" role="status">
              {message}
            </p>
          )}
          {startupHealth?.recoveredFromBackup && (
            <p className="recovery-notice" role="status">
              {t("recoveryNotice")}
            </p>
          )}
          {startupHealth?.previousExitUnclean &&
            !startupHealth.recoveredFromBackup && (
              <p className="recovery-notice" role="status">
                {t("uncleanExitNotice")}
              </p>
            )}
          {error && (
            <pre className="error-message" role="alert">
              {error}
            </pre>
          )}

          {route === "library" ? (
            <LibraryPage
              books={books}
              visibleBooks={visibleBooks}
              formats={formats}
              selectedId={selectedId}
              activeFormat={format}
              loading={loading}
              busy={busy}
              statistics={statistics}
              t={t}
              onFormat={setFormat}
              onSelect={setSelectedId}
              onOpen={(book) => void openBook(book)}
              onFavorite={(book) => void toggleFavorite(book)}
              onRemove={confirmAndRemoveBooks}
              onImport={() => void runImport(chooseAndImportBooks)}
              onWatch={() => void runImport(chooseAndWatchFolder)}
            />
          ) : route === "reading" ? (
            <ReadingNowPage
              books={readingNowBooks}
              selectedId={selectedId}
              locale={locale}
              loading={loading}
              t={t}
              onSelect={setSelectedId}
              onOpen={(book) => void openBook(book)}
              onFavorite={(book) => void toggleFavorite(book)}
            />
          ) : route === "collections" ? (
            <CollectionsPage
              folders={folders}
              formats={formats}
              busy={busy}
              t={t}
              onWatch={() => void runImport(chooseAndWatchFolder)}
              onScan={() => void runImport(async () => scanWatchedFolders())}
            />
          ) : route === "settings" ? (
            <SettingsPage
              t={t}
              screenReaderSupport={screenReaderSupport}
              onScreenReaderSupportChange={setScreenReaderSupport}
            />
          ) : route === "statistics" ? (
            <StatisticsPage t={t} onChanged={setStatistics} />
          ) : route === "authors" ? (
            <AuthorsPage
              books={books}
              selectedId={selectedId}
              locale={locale}
              loading={loading}
              t={t}
              onSelect={setSelectedId}
              onOpen={(book) => void openBook(book)}
              onFavorite={(book) => void toggleFavorite(book)}
              onBrowse={() => setRoute("library")}
            />
          ) : route === "series" ? (
            <SeriesPage
              books={books}
              selectedId={selectedId}
              locale={locale}
              loading={loading}
              t={t}
              onSelect={setSelectedId}
              onOpen={(book) => void openBook(book)}
              onFavorite={(book) => void toggleFavorite(book)}
              onBrowse={() => setRoute("library")}
            />
          ) : route === "achievements" ? (
            <AchievementsPage t={t} />
          ) : route === "favorites" ? (
            <FavoritesPage
              books={favoriteBooks}
              selectedId={selectedId}
              loading={loading}
              t={t}
              onSelect={setSelectedId}
              onOpen={(book) => void openBook(book)}
              onFavorite={(book) => void toggleFavorite(book)}
              onBrowse={() => setRoute("library")}
            />
          ) : (
            <EmptyState title={t(current.label)} hint={t("notAvailable")} />
          )}
        </section>
      </main>

      <BookDetails
        key={selected?.id ?? "empty-details"}
        book={selected}
        t={t}
        busy={readerLoading}
        onRead={(book) => void openBook(book)}
        onFavorite={toggleFavorite}
        onRemove={(book) => confirmAndRemoveBooks([book])}
        onUpdated={(book) =>
          setBooks((items) =>
            items.map((item) => (item.id === book.id ? book : item)),
          )
        }
        onClose={() => setSelectedId(null)}
      />
      {selected && (
        <button
          className="drawer-scrim"
          type="button"
          aria-label={t("detailsEmpty")}
          onClick={() => setSelectedId(null)}
        />
      )}
    </div>
  );
}

export function LanguagePackagesPage({ t }: { t: Translator }) {
  const [packages, setPackages] = useState<InstalledLanguagePackage[]>([]);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");

  const refresh = () =>
    listLanguagePackages()
      .then(setPackages)
      .catch((reason) => setMessage(String(reason)));

  useEffect(() => {
    void refresh();
  }, []);

  const importPackage = async () => {
    setBusy(true);
    try {
      const installed = await chooseAndImportLanguagePackage();
      if (installed) {
        setMessage(t("languagePackageImported"));
        await refresh();
      }
    } catch (reason) {
      setMessage(String(reason));
    } finally {
      setBusy(false);
    }
  };

  const removePackage = async (item: InstalledLanguagePackage) => {
    if (!window.confirm(t("removeLanguagePackageConfirm"))) return;
    setBusy(true);
    try {
      await removeLanguagePackage(item.id, item.version);
      setMessage(t("languagePackageRemoved"));
      await refresh();
    } catch (reason) {
      setMessage(String(reason));
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="language-packages-page">
      <div className="language-packages-intro">
        <div>
          <p className="eyebrow">{t("offlineLanguageTools")}</p>
          <h2>{t("languagePackages")}</h2>
          <p>{t("languagePackagesHint")}</p>
        </div>
        <button
          type="button"
          disabled={busy}
          onClick={() => void importPackage()}
        >
          {t("importLanguagePackage")}
        </button>
      </div>
      {message && (
        <p className="inline-status" role="status">
          {message}
        </p>
      )}
      {packages.length === 0 ? (
        <div className="language-package-empty">
          <h3>{t("noLanguagePackages")}</h3>
          <p>{t("noLanguagePackagesHint")}</p>
        </div>
      ) : (
        <div className="language-package-list">
          {packages.map((item) => (
            <article key={`${item.id}-${item.version}`}>
              <div>
                <span className="language-package-kind">
                  {t(
                    item.kind === "dictionary"
                      ? "dictionaryPackage"
                      : "translationPackage",
                  )}
                </span>
                <h3>{item.name}</h3>
                <p>
                  {item.sourceLanguage}
                  {item.targetLanguage ? ` → ${item.targetLanguage}` : ""}
                  {" · "}
                  {item.licenseSpdx}
                  {" · "}v{item.version}
                </p>
                <small>{item.attribution}</small>
              </div>
              <div className="language-package-actions">
                <span className={item.verified ? "verified" : "invalid"}>
                  {t(item.verified ? "packageVerified" : "packageInvalid")}
                </span>
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => void removePackage(item)}
                >
                  {t("removeLanguagePackage")}
                </button>
              </div>
            </article>
          ))}
        </div>
      )}
    </section>
  );
}

export function SettingsPage({
  t,
  screenReaderSupport,
  onScreenReaderSupportChange,
}: {
  t: Translator;
  screenReaderSupport: boolean;
  onScreenReaderSupportChange: (enabled: boolean) => void;
}) {
  return (
    <div className="settings-page">
      <section className="accessibility-settings settings-section">
        <div className="section-heading">
          <div>
            <p className="eyebrow">{t("accessibility")}</p>
            <h2>{t("screenReaderSupport")}</h2>
          </div>
          <span
            className={`integration-badge ${screenReaderSupport ? "available" : ""}`}
          >
            {screenReaderSupport ? t("enabled") : t("disabled")}
          </span>
        </div>
        <p className="settings-hint">{t("screenReaderSupportHint")}</p>
        <label className="settings-toggle">
          <input
            type="checkbox"
            checked={screenReaderSupport}
            onChange={(event) =>
              onScreenReaderSupportChange(event.target.checked)
            }
          />
          <span>
            <strong>{t("screenReaderAnnouncements")}</strong>
            <small>{t("screenReaderEssentialHint")}</small>
          </span>
        </label>
      </section>
      <SteamIntegrationPanel t={t} />
      <LanguagePackagesPage t={t} />
    </div>
  );
}

export function SteamIntegrationPanel({ t }: { t: Translator }) {
  const [status, setStatus] = useState<SteamIntegrationStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");

  const refreshSteam = useCallback(
    () =>
      getSteamIntegrationStatus()
        .then(setStatus)
        .catch((reason) => setMessage(String(reason))),
    [],
  );

  useEffect(() => {
    void refreshSteam();
  }, [refreshSteam]);

  const synchronize = async () => {
    setBusy(true);
    setMessage("");
    try {
      const result = await syncSteamAchievements();
      setMessage(
        result.synchronized > 0
          ? t("steamSyncComplete")
          : t("steamNothingToSync"),
      );
      await refreshSteam();
    } catch (reason) {
      setMessage(String(reason));
      await refreshSteam();
    } finally {
      setBusy(false);
    }
  };

  if (!status) {
    return (
      <section className="steam-settings settings-section">
        <p>{message || t("loading")}</p>
      </section>
    );
  }

  return (
    <section className="steam-settings settings-section">
      <div className="section-heading">
        <div>
          <p className="eyebrow">{t("steamIntegration")}</p>
          <h2>{t("steamAchievements")}</h2>
        </div>
        <span
          className={`integration-badge ${status.providerAvailable ? "available" : ""}`}
        >
          {status.providerAvailable
            ? t("steamConnected")
            : t("steamUnavailable")}
        </span>
      </div>
      <p className="settings-hint">
        {status.buildProfile === "steam"
          ? t("steamBuildHint")
          : t("githubBuildHint")}
      </p>
      <div className="integration-stats">
        <div>
          <span>{t("steamBuildProfile")}</span>
          <strong>
            {status.buildProfile === "steam"
              ? t("steamProfile")
              : t("githubProfile")}
          </strong>
        </div>
        <div>
          <span>{t("steamPending")}</span>
          <strong>{status.pendingUnlocks}</strong>
        </div>
        <div>
          <span>{t("steamSynchronized")}</span>
          <strong>{status.syncedUnlocks}</strong>
        </div>
        <div>
          <span>{t("steamOverlay")}</span>
          <strong>
            {status.overlayEnabled === null
              ? t("notChecked")
              : status.overlayEnabled
                ? t("enabled")
                : t("disabled")}
          </strong>
        </div>
      </div>
      {(message || status.lastError) && (
        <p
          className={status.lastError ? "error-message" : "notice"}
          role="status"
        >
          {message || status.lastError}
        </p>
      )}
      <button
        className="secondary-button"
        type="button"
        disabled={
          busy || !status.providerAvailable || status.pendingUnlocks === 0
        }
        onClick={() => void synchronize()}
      >
        {busy ? t("steamSyncing") : t("steamSyncNow")}
      </button>
    </section>
  );
}

type Translator = ReturnType<typeof useLocale>["t"];

function LibraryPage({
  books,
  visibleBooks,
  formats,
  selectedId,
  activeFormat,
  loading,
  busy,
  statistics,
  t,
  onFormat,
  onSelect,
  onOpen,
  onFavorite,
  onRemove,
  onImport,
  onWatch,
}: {
  books: Book[];
  visibleBooks: Book[];
  formats: string[];
  selectedId: number | null;
  activeFormat: string;
  loading: boolean;
  busy: boolean;
  statistics: StatisticsSnapshot;
  t: Translator;
  onFormat: (format: string) => void;
  onSelect: (id: number) => void;
  onOpen: (book: Book) => void;
  onFavorite: (book: Book) => void;
  onRemove: (books: Book[]) => Promise<boolean>;
  onImport: () => void;
  onWatch: () => void;
}) {
  const [renderLimit, setRenderLimit] = useState(120);
  const [selectionMode, setSelectionMode] = useState(false);
  const [selectedBookIds, setSelectedBookIds] = useState<Set<number>>(
    () => new Set(),
  );
  const renderedBooks = visibleBooks.slice(0, renderLimit);
  useEffect(() => setRenderLimit(120), [visibleBooks]);
  useEffect(() => {
    setSelectedBookIds((current) => {
      const availableIds = new Set(books.map((book) => book.id));
      return new Set([...current].filter((id) => availableIds.has(id)));
    });
  }, [books]);

  const cancelSelection = () => {
    setSelectionMode(false);
    setSelectedBookIds(new Set());
  };
  const toggleSelection = (bookId: number) => {
    setSelectedBookIds((current) => {
      const next = new Set(current);
      if (next.has(bookId)) next.delete(bookId);
      else next.add(bookId);
      return next;
    });
  };
  const removeSelection = async () => {
    const selectedBooks = books.filter((book) => selectedBookIds.has(book.id));
    if (await onRemove(selectedBooks)) cancelSelection();
  };

  return (
    <>
      <section className="stat-grid" aria-label={t("statistics")}>
        <Stat icon="library" value={String(books.length)} label={t("books")} />
        <Stat
          icon="achievement"
          value={String(books.filter((book) => book.progress >= 0.995).length)}
          label={t("completed")}
        />
        <Stat
          icon="statistics"
          value={String(statistics.currentStreak)}
          label={t("streak")}
        />
        <Stat
          icon="reading"
          value={`${Math.floor(statistics.todayActiveSeconds / 60)} / ${statistics.dailyGoalMinutes}`}
          label={t("dailyGoal")}
        />
      </section>
      <div className="library-toolbar">
        <div className="section-heading">
          <h2>{t("library")}</h2>
          <span>{visibleBooks.length}</span>
        </div>
        <div className="toolbar-actions">
          {selectionMode ? (
            <>
              <span className="selection-count" role="status">
                {t("selectedBooks").replace(
                  "{count}",
                  String(selectedBookIds.size),
                )}
              </span>
              <button
                className="secondary-button"
                type="button"
                disabled={busy}
                onClick={() =>
                  setSelectedBookIds(
                    new Set(visibleBooks.map((book) => book.id)),
                  )
                }
              >
                {t("selectAllVisible")}
              </button>
              <button
                className="danger-button"
                type="button"
                disabled={busy || selectedBookIds.size === 0}
                onClick={() => void removeSelection()}
              >
                {t("removeFromLibrary")}
              </button>
              <button
                className="secondary-button"
                type="button"
                disabled={busy}
                onClick={cancelSelection}
              >
                {t("cancelSelection")}
              </button>
            </>
          ) : (
            <>
              <button
                className="secondary-button"
                type="button"
                disabled={busy || books.length === 0}
                onClick={() => setSelectionMode(true)}
              >
                {t("selectBooks")}
              </button>
              <button
                className="secondary-button"
                type="button"
                disabled={busy}
                onClick={onWatch}
              >
                <Icon name="folder" />
                {t("watchFolder")}
              </button>
              <button
                className="primary-button"
                type="button"
                disabled={busy}
                onClick={onImport}
              >
                <Icon name="plus" />
                {t("addBooks")}
              </button>
            </>
          )}
        </div>
      </div>
      {formats.length > 0 && (
        <div className="format-filters" aria-label={t("collections")}>
          {["ALL", ...formats].map((item) => (
            <button
              type="button"
              className={activeFormat === item ? "active" : ""}
              key={item}
              aria-pressed={activeFormat === item}
              onClick={() => onFormat(item)}
            >
              {item === "ALL" ? t("allBooks") : item}
            </button>
          ))}
        </div>
      )}
      {loading ? (
        <EmptyState title={t("loading")} hint="" />
      ) : books.length === 0 ? (
        <EmptyState
          title={t("noBooks")}
          hint={t("noBooksHint")}
          action={onImport}
          actionLabel={t("addBooks")}
        />
      ) : visibleBooks.length === 0 ? (
        <EmptyState title={t("noBooks")} hint={t("search")} />
      ) : (
        <section className="book-grid" aria-label={t("library")}>
          {renderedBooks.map((book) => (
            <BookCard
              key={book.id}
              book={book}
              selected={
                selectionMode
                  ? selectedBookIds.has(book.id)
                  : book.id === selectedId
              }
              selectionMode={selectionMode}
              selectionLabel={t("selectionLabel").replace(
                "{title}",
                book.title,
              )}
              unknownAuthor={t("unknownAuthor")}
              unavailable={t("unavailable")}
              addFavoriteLabel={t("addToFavorites")}
              removeFavoriteLabel={t("removeFromFavorites")}
              onSelect={onSelect}
              onOpen={onOpen}
              onFavorite={onFavorite}
              onToggleSelection={toggleSelection}
            />
          ))}
          {renderedBooks.length < visibleBooks.length && (
            <div className="library-load-more">
              <p role="status">
                {t("showingBooks")
                  .replace("{shown}", String(renderedBooks.length))
                  .replace("{total}", String(visibleBooks.length))}
              </p>
              <button
                className="secondary-button"
                type="button"
                onClick={() => setRenderLimit((limit) => limit + 120)}
              >
                {t("showMoreBooks")}
              </button>
            </div>
          )}
        </section>
      )}
    </>
  );
}

function ReadingNowPage({
  books,
  selectedId,
  locale,
  loading,
  t,
  onSelect,
  onOpen,
  onFavorite,
}: {
  books: Book[];
  selectedId: number | null;
  locale: string;
  loading: boolean;
  t: Translator;
  onSelect: (id: number) => void;
  onOpen: (book: Book) => void;
  onFavorite: (book: Book) => void;
}) {
  if (loading) {
    return <EmptyState title={t("loading")} hint="" />;
  }

  if (books.length === 0) {
    return (
      <EmptyState
        title={t("readingNowEmpty")}
        hint={t("readingNowEmptyHint")}
      />
    );
  }

  return (
    <>
      <section className="reading-now-intro">
        <div>
          <p className="eyebrow">{t("personalLibrary")}</p>
          <h2>{t("continueReading")}</h2>
          <p>{t("readingNowHint")}</p>
        </div>
        <div className="reading-now-count" aria-label={t("booksInProgress")}>
          <strong>{books.length}</strong>
          <span>{t("booksInProgress")}</span>
        </div>
      </section>
      <section
        className="book-grid reading-now-grid"
        aria-label={t("readingNow")}
      >
        {books.map((book) => (
          <BookCard
            key={book.id}
            book={book}
            selected={book.id === selectedId}
            unknownAuthor={t("unknownAuthor")}
            unavailable={t("unavailable")}
            lastOpenedLabel={`${t("lastRead")}: ${formatLastOpened(book.lastOpenedAt, locale, t)}`}
            actionLabel={t("continueReading")}
            addFavoriteLabel={t("addToFavorites")}
            removeFavoriteLabel={t("removeFromFavorites")}
            onSelect={onSelect}
            onOpen={onOpen}
            onFavorite={onFavorite}
          />
        ))}
      </section>
    </>
  );
}

function FavoritesPage({
  books,
  selectedId,
  loading,
  t,
  onSelect,
  onOpen,
  onFavorite,
  onBrowse,
}: {
  books: Book[];
  selectedId: number | null;
  loading: boolean;
  t: Translator;
  onSelect: (id: number) => void;
  onOpen: (book: Book) => void;
  onFavorite: (book: Book) => void;
  onBrowse: () => void;
}) {
  if (loading) {
    return <EmptyState title={t("loading")} hint="" />;
  }

  if (books.length === 0) {
    return (
      <EmptyState
        title={t("favoritesEmpty")}
        hint={t("favoritesEmptyHint")}
        action={onBrowse}
        actionLabel={t("browseLibrary")}
      />
    );
  }

  return (
    <>
      <section className="reading-now-intro favorites-intro">
        <div>
          <p className="eyebrow">{t("personalLibrary")}</p>
          <h2>{t("favoriteBooks")}</h2>
          <p>{t("favoritesHint")}</p>
        </div>
        <div className="reading-now-count" aria-label={t("favoriteBooks")}>
          <strong>{books.length}</strong>
          <span>{t("favoriteBooks")}</span>
        </div>
      </section>
      <section className="book-grid" aria-label={t("favorites")}>
        {books.map((book) => (
          <BookCard
            key={book.id}
            book={book}
            selected={book.id === selectedId}
            unknownAuthor={t("unknownAuthor")}
            unavailable={t("unavailable")}
            addFavoriteLabel={t("addToFavorites")}
            removeFavoriteLabel={t("removeFromFavorites")}
            onSelect={onSelect}
            onOpen={onOpen}
            onFavorite={onFavorite}
          />
        ))}
      </section>
    </>
  );
}

type AuthorGroup = {
  key: string;
  name: string;
  books: Book[];
};

function AuthorsPage({
  books,
  selectedId,
  locale,
  loading,
  t,
  onSelect,
  onOpen,
  onFavorite,
  onBrowse,
}: {
  books: Book[];
  selectedId: number | null;
  locale: string;
  loading: boolean;
  t: Translator;
  onSelect: (id: number) => void;
  onOpen: (book: Book) => void;
  onFavorite: (book: Book) => void;
  onBrowse: () => void;
}) {
  const [selectedAuthor, setSelectedAuthor] = useState<string | null>(null);
  const groups = useMemo(
    () => groupBooksByAuthor(books, locale, t("unknownAuthor")),
    [books, locale, t],
  );
  const selectedGroup =
    selectedAuthor === null
      ? null
      : (groups.find((group) => group.key === selectedAuthor) ?? null);

  useEffect(() => {
    if (
      selectedAuthor !== null &&
      !groups.some((group) => group.key === selectedAuthor)
    ) {
      setSelectedAuthor(null);
    }
  }, [groups, selectedAuthor]);

  if (loading) {
    return <EmptyState title={t("loading")} hint="" />;
  }

  if (books.length === 0) {
    return (
      <EmptyState
        title={t("authorsEmpty")}
        hint={t("authorsEmptyHint")}
        action={onBrowse}
        actionLabel={t("browseLibrary")}
      />
    );
  }

  if (selectedGroup) {
    return (
      <>
        <div className="author-detail-header">
          <button
            className="secondary-button"
            type="button"
            onClick={() => setSelectedAuthor(null)}
          >
            <span aria-hidden="true">←</span>
            {t("allAuthors")}
          </button>
          <div>
            <p className="eyebrow">{t("booksByAuthor")}</p>
            <h2>{selectedGroup.name}</h2>
            <p>{formatBookCount(selectedGroup.books.length, locale, t)}</p>
          </div>
        </div>
        <section className="book-grid" aria-label={selectedGroup.name}>
          {selectedGroup.books.map((book) => (
            <BookCard
              key={book.id}
              book={book}
              selected={book.id === selectedId}
              unknownAuthor={t("unknownAuthor")}
              unavailable={t("unavailable")}
              addFavoriteLabel={t("addToFavorites")}
              removeFavoriteLabel={t("removeFromFavorites")}
              onSelect={onSelect}
              onOpen={onOpen}
              onFavorite={onFavorite}
            />
          ))}
        </section>
      </>
    );
  }

  return (
    <>
      <section className="reading-now-intro authors-intro">
        <div>
          <p className="eyebrow">{t("personalLibrary")}</p>
          <h2>{t("libraryAuthors")}</h2>
          <p>{t("authorsHint")}</p>
        </div>
        <div className="reading-now-count" aria-label={t("libraryAuthors")}>
          <strong>{groups.length}</strong>
          <span>{t("libraryAuthors")}</span>
        </div>
      </section>
      <section className="author-grid" aria-label={t("authors")}>
        {groups.map((group) => (
          <button
            className="author-card"
            type="button"
            key={group.key}
            aria-label={`${group.name}, ${formatBookCount(group.books.length, locale, t)}`}
            onClick={() => setSelectedAuthor(group.key)}
          >
            <span className="author-monogram" aria-hidden="true">
              {authorInitials(group.name)}
            </span>
            <span className="author-card-copy">
              <strong>{group.name}</strong>
              <small>{formatBookCount(group.books.length, locale, t)}</small>
            </span>
            <span className="author-card-arrow" aria-hidden="true">
              →
            </span>
          </button>
        ))}
      </section>
    </>
  );
}

function groupBooksByAuthor(
  books: Book[],
  locale: string,
  unknownAuthor: string,
): AuthorGroup[] {
  const groups = new Map<string, AuthorGroup>();
  for (const book of books) {
    const author = book.author.trim();
    const key = author.toLocaleLowerCase(locale);
    const current = groups.get(key);
    if (current) {
      current.books.push(book);
    } else {
      groups.set(key, {
        key,
        name: author || unknownAuthor,
        books: [book],
      });
    }
  }

  return [...groups.values()]
    .map((group) => ({
      ...group,
      books: [...group.books].sort((left, right) =>
        left.title.localeCompare(right.title, locale),
      ),
    }))
    .sort((left, right) => {
      if (left.key === "") return 1;
      if (right.key === "") return -1;
      return left.name.localeCompare(right.name, locale);
    });
}

function authorInitials(name: string) {
  const initials = name
    .split(/\s+/u)
    .filter(Boolean)
    .slice(0, 2)
    .map((part) => [...part][0])
    .join("")
    .toLocaleUpperCase();
  return initials || "?";
}

function formatBookCount(count: number, locale: string, t: Translator) {
  const category = new Intl.PluralRules(locale).select(count);
  const key = {
    one: "bookCountOne",
    few: "bookCountFew",
    many: "bookCountMany",
    other: "bookCountOther",
    zero: "bookCountOther",
    two: "bookCountOther",
  }[category] as TranslationKey;
  return t(key).replace("{count}", String(count));
}

type SeriesGroup = {
  key: string;
  name: string;
  books: Book[];
};

function SeriesPage({
  books,
  selectedId,
  locale,
  loading,
  t,
  onSelect,
  onOpen,
  onFavorite,
  onBrowse,
}: {
  books: Book[];
  selectedId: number | null;
  locale: string;
  loading: boolean;
  t: Translator;
  onSelect: (id: number) => void;
  onOpen: (book: Book) => void;
  onFavorite: (book: Book) => void;
  onBrowse: () => void;
}) {
  const [selectedSeries, setSelectedSeries] = useState<string | null>(null);
  const groups = useMemo(
    () => groupBooksBySeries(books, locale, t("noSeries")),
    [books, locale, t],
  );
  const selectedGroup =
    selectedSeries === null
      ? null
      : (groups.find((group) => group.key === selectedSeries) ?? null);

  useEffect(() => {
    if (
      selectedSeries !== null &&
      !groups.some((group) => group.key === selectedSeries)
    ) {
      setSelectedSeries(null);
    }
  }, [groups, selectedSeries]);

  if (loading) {
    return <EmptyState title={t("loading")} hint="" />;
  }

  if (books.length === 0) {
    return (
      <EmptyState
        title={t("seriesEmpty")}
        hint={t("seriesEmptyHint")}
        action={onBrowse}
        actionLabel={t("browseLibrary")}
      />
    );
  }

  if (selectedGroup) {
    return (
      <>
        <div className="author-detail-header series-detail-header">
          <button
            className="secondary-button"
            type="button"
            onClick={() => setSelectedSeries(null)}
          >
            <span aria-hidden="true">←</span>
            {t("allSeries")}
          </button>
          <div>
            <p className="eyebrow">{t("booksInSeries")}</p>
            <h2>{selectedGroup.name}</h2>
            <p>
              {formatBookCount(selectedGroup.books.length, locale, t)}
              {" · "}
              {t("seriesTitleOrder")}
            </p>
          </div>
        </div>
        <section className="book-grid" aria-label={selectedGroup.name}>
          {selectedGroup.books.map((book) => (
            <BookCard
              key={book.id}
              book={book}
              selected={book.id === selectedId}
              unknownAuthor={t("unknownAuthor")}
              unavailable={t("unavailable")}
              addFavoriteLabel={t("addToFavorites")}
              removeFavoriteLabel={t("removeFromFavorites")}
              onSelect={onSelect}
              onOpen={onOpen}
              onFavorite={onFavorite}
            />
          ))}
        </section>
      </>
    );
  }

  return (
    <>
      <section className="reading-now-intro series-intro">
        <div>
          <p className="eyebrow">{t("personalLibrary")}</p>
          <h2>{t("librarySeries")}</h2>
          <p>{t("seriesHint")}</p>
        </div>
        <div className="reading-now-count" aria-label={t("librarySeries")}>
          <strong>{groups.length}</strong>
          <span>{t("librarySeries")}</span>
        </div>
      </section>
      <section className="author-grid series-grid" aria-label={t("series")}>
        {groups.map((group) => (
          <button
            className="author-card series-card"
            type="button"
            key={group.key}
            aria-label={`${group.name}, ${formatBookCount(group.books.length, locale, t)}`}
            onClick={() => setSelectedSeries(group.key)}
          >
            <span className="author-monogram series-mark" aria-hidden="true">
              <Icon name="series" />
            </span>
            <span className="author-card-copy">
              <strong>{group.name}</strong>
              <small>{formatBookCount(group.books.length, locale, t)}</small>
            </span>
            <span className="author-card-arrow" aria-hidden="true">
              →
            </span>
          </button>
        ))}
      </section>
    </>
  );
}

function groupBooksBySeries(
  books: Book[],
  locale: string,
  noSeries: string,
): SeriesGroup[] {
  const groups = new Map<string, SeriesGroup>();
  const collator = new Intl.Collator(locale, {
    numeric: true,
    sensitivity: "base",
  });

  for (const book of books) {
    const series = book.series.trim().replace(/\s+/gu, " ");
    const key = series.toLocaleLowerCase(locale);
    const current = groups.get(key);
    if (current) {
      current.books.push(book);
    } else {
      groups.set(key, {
        key,
        name: series || noSeries,
        books: [book],
      });
    }
  }

  return [...groups.values()]
    .map((group) => ({
      ...group,
      books: [...group.books].sort(
        (left, right) =>
          collator.compare(left.title, right.title) || left.id - right.id,
      ),
    }))
    .sort((left, right) => {
      if (left.key === "") return 1;
      if (right.key === "") return -1;
      return collator.compare(left.name, right.name);
    });
}

function CollectionsPage({
  folders,
  formats,
  busy,
  t,
  onWatch,
  onScan,
}: {
  folders: WatchedFolder[];
  formats: string[];
  busy: boolean;
  t: Translator;
  onWatch: () => void;
  onScan: () => void;
}) {
  return (
    <>
      <div className="library-toolbar">
        <div className="section-heading">
          <h2>{t("watchedFolders")}</h2>
          <span>{folders.length}</span>
        </div>
        <div className="toolbar-actions">
          <button
            className="secondary-button"
            type="button"
            disabled={busy || folders.length === 0}
            onClick={onScan}
          >
            <Icon name="refresh" />
            {t("scanFolders")}
          </button>
          <button
            className="primary-button"
            type="button"
            disabled={busy}
            onClick={onWatch}
          >
            <Icon name="folder" />
            {t("watchFolder")}
          </button>
        </div>
      </div>
      {folders.length === 0 ? (
        <EmptyState
          title={t("watchedFolders")}
          hint={t("folderEmpty")}
          action={onWatch}
          actionLabel={t("watchFolder")}
        />
      ) : (
        <div className="folder-list">
          {folders.map((folder) => (
            <article key={folder.id}>
              <Icon name="folder" />
              <div>
                <strong>{folder.path}</strong>
                <small>{folder.lastScannedAt ?? "—"}</small>
              </div>
            </article>
          ))}
        </div>
      )}
      {formats.length > 0 && (
        <section className="virtual-collections">
          <h2>{t("collections")}</h2>
          <div>
            {formats.map((item) => (
              <span key={item}>{item}</span>
            ))}
          </div>
        </section>
      )}
    </>
  );
}

function BookCard({
  book,
  selected,
  unknownAuthor,
  unavailable,
  lastOpenedLabel,
  actionLabel,
  addFavoriteLabel,
  removeFavoriteLabel,
  selectionMode = false,
  selectionLabel,
  onSelect,
  onOpen,
  onFavorite,
  onToggleSelection,
}: {
  book: Book;
  selected: boolean;
  unknownAuthor: string;
  unavailable: string;
  lastOpenedLabel?: string;
  actionLabel?: string;
  addFavoriteLabel: string;
  removeFavoriteLabel: string;
  selectionMode?: boolean;
  selectionLabel?: string;
  onSelect: (id: number) => void;
  onOpen: (book: Book) => void;
  onFavorite: (book: Book) => void;
  onToggleSelection?: (id: number) => void;
}) {
  return (
    <article
      className={`book-card ${selected ? "selected" : ""} ${selectionMode ? "selection-mode" : ""} ${book.isAvailable ? "" : "unavailable"}`}
    >
      {selectionMode ? (
        <button
          className={`selection-toggle ${selected ? "active" : ""}`}
          type="button"
          aria-label={selectionLabel}
          aria-pressed={selected}
          onClick={() => onToggleSelection?.(book.id)}
        >
          <span aria-hidden="true">{selected ? "✓" : ""}</span>
        </button>
      ) : (
        <button
          className={`favorite-toggle ${book.isFavorite ? "active" : ""}`}
          type="button"
          aria-label={book.isFavorite ? removeFavoriteLabel : addFavoriteLabel}
          aria-pressed={book.isFavorite}
          onClick={() => onFavorite(book)}
        >
          <Icon name="favorite" />
        </button>
      )}
      <button
        type="button"
        className="book-select"
        aria-label={`${book.title} — ${book.author || unknownAuthor}, ${book.format}, ${Math.round(book.progress * 100)}%`}
        onClick={() =>
          selectionMode ? onToggleSelection?.(book.id) : onSelect(book.id)
        }
        onDoubleClick={() => {
          if (!selectionMode) onOpen(book);
        }}
      >
        <span className="cover-wrap">
          {book.coverPath ? (
            <img
              className="book-cover"
              src={coverUrl(book.coverPath)}
              alt=""
              loading="lazy"
            />
          ) : (
            <span className="book-cover fallback-cover">
              <Icon name="reading" />
              <strong>{book.title}</strong>
              <small>{book.author || unknownAuthor}</small>
            </span>
          )}
          {!book.isAvailable && (
            <span className="availability-badge">{unavailable}</span>
          )}
        </span>
        <span className="book-title">{book.title}</span>
        <span className="book-author">{book.author || unknownAuthor}</span>
        <span className="book-format">{book.format}</span>
        <span
          className="book-reading-progress"
          role="progressbar"
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={Math.round(book.progress * 100)}
        >
          <span>
            <i style={{ width: `${book.progress * 100}%` }} />
          </span>
          <span>{Math.round(book.progress * 100)}%</span>
        </span>
        {lastOpenedLabel && (
          <span className="book-last-opened">{lastOpenedLabel}</span>
        )}
      </button>
      {actionLabel && (
        <button
          className="continue-book-button"
          type="button"
          disabled={!book.isAvailable}
          aria-label={`${actionLabel} — ${book.title}`}
          onClick={() => onOpen(book)}
        >
          <Icon name="reading" />
          {book.isAvailable ? actionLabel : unavailable}
        </button>
      )}
    </article>
  );
}

function formatLastOpened(
  timestamp: number | null,
  locale: string,
  t: Translator,
) {
  if (timestamp === null) return t("recently");
  const openedAt = new Date(timestamp * 1_000);
  if (Number.isNaN(openedAt.getTime())) return t("recently");

  const elapsedSeconds = Math.max(
    0,
    Math.floor((Date.now() - openedAt.getTime()) / 1_000),
  );
  const relative = new Intl.RelativeTimeFormat(locale, { numeric: "auto" });
  if (elapsedSeconds < 60) return t("recently");
  if (elapsedSeconds < 3_600) {
    return relative.format(
      -Math.max(1, Math.floor(elapsedSeconds / 60)),
      "minute",
    );
  }
  if (elapsedSeconds < 86_400) {
    return relative.format(
      -Math.max(1, Math.floor(elapsedSeconds / 3_600)),
      "hour",
    );
  }
  if (elapsedSeconds < 7 * 86_400) {
    return relative.format(
      -Math.max(1, Math.floor(elapsedSeconds / 86_400)),
      "day",
    );
  }
  return new Intl.DateTimeFormat(locale, {
    day: "numeric",
    month: "long",
    year:
      openedAt.getFullYear() === new Date().getFullYear()
        ? undefined
        : "numeric",
  }).format(openedAt);
}

export function BookDetails({
  book,
  t,
  busy,
  onRead,
  onFavorite,
  onRemove,
  onUpdated,
  onClose,
}: {
  book: Book | null;
  t: Translator;
  busy: boolean;
  onRead: (book: Book) => void;
  onFavorite: (book: Book) => Promise<Book>;
  onRemove: (book: Book) => Promise<boolean>;
  onUpdated: (book: Book) => void;
  onClose: () => void;
}) {
  const [mode, setMode] = useState<"view" | "edit" | "search">("view");
  const [metadata, setMetadata] = useState<BookMetadataInput | null>(
    book ? metadataFromBook(book) : null,
  );
  const [metadataQuery, setMetadataQuery] = useState(
    book ? book.isbn || `${book.title} ${book.author}`.trim() : "",
  );
  const [candidates, setCandidates] = useState<MetadataCandidate[]>([]);
  const [metadataSearched, setMetadataSearched] = useState(false);
  const [actionBusy, setActionBusy] = useState(false);
  const [status, setStatus] = useState("");
  const [actionError, setActionError] = useState("");

  const runAction = async (action: () => Promise<Book>, message: string) => {
    setActionBusy(true);
    setActionError("");
    try {
      const updated = await action();
      onUpdated(updated);
      setMetadata(metadataFromBook(updated));
      setMode("view");
      setStatus(message);
    } catch (reason) {
      setActionError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setActionBusy(false);
    }
  };

  return (
    <aside
      className={`details-panel ${book ? "open" : ""}`}
      aria-label={t("detailsEmpty")}
    >
      {!book ? (
        <div className="details-empty">
          <span className="empty-emblem">
            <Icon name="book" />
          </span>
          <h2>{t("detailsEmpty")}</h2>
          <p>{t("detailsHint")}</p>
        </div>
      ) : (
        <div className="book-details">
          <button
            className="details-close"
            type="button"
            onClick={onClose}
            aria-label="Close"
          >
            ×
          </button>
          {book.coverPath ? (
            <img
              className="details-cover"
              src={coverUrl(book.coverPath)}
              alt=""
            />
          ) : (
            <div className="details-cover fallback-cover">
              <Icon name="reading" />
              <strong>{book.title}</strong>
              <small>{book.author || t("unknownAuthor")}</small>
            </div>
          )}
          {mode === "edit" && metadata ? (
            <MetadataEditor
              value={metadata}
              t={t}
              busy={actionBusy}
              error={actionError}
              onChange={setMetadata}
              onCancel={() => setMode("view")}
              onSave={() =>
                void runAction(
                  () => updateBookMetadata(book.id, metadata),
                  t("metadataSaved"),
                )
              }
            />
          ) : mode === "search" ? (
            <MetadataSearch
              query={metadataQuery}
              candidates={candidates}
              searched={metadataSearched}
              t={t}
              busy={actionBusy}
              error={actionError}
              onQuery={setMetadataQuery}
              onCancel={() => setMode("view")}
              onSearch={() => {
                setActionBusy(true);
                setActionError("");
                void searchMetadata(book.id, metadataQuery)
                  .then(setCandidates)
                  .catch((reason: unknown) =>
                    setActionError(
                      reason instanceof Error ? reason.message : String(reason),
                    ),
                  )
                  .finally(() => {
                    setActionBusy(false);
                    setMetadataSearched(true);
                  });
              }}
              onApply={(candidate) =>
                void runAction(
                  () => applyMetadataCandidate(book.id, candidate),
                  t("metadataApplied"),
                )
              }
            />
          ) : (
            <>
              <h2>{book.title}</h2>
              {book.subtitle && (
                <p className="details-subtitle">{book.subtitle}</p>
              )}
              <p className="details-author">
                {book.author || t("unknownAuthor")}
              </p>
              <button
                type="button"
                className={`secondary-button details-favorite ${book.isFavorite ? "active" : ""}`}
                disabled={actionBusy}
                aria-pressed={book.isFavorite}
                onClick={() =>
                  void runAction(
                    () => onFavorite(book),
                    book.isFavorite ? t("favoriteRemoved") : t("favoriteAdded"),
                  )
                }
              >
                <Icon name="favorite" />
                {book.isFavorite
                  ? t("removeFromFavorites")
                  : t("addToFavorites")}
              </button>
              {status && (
                <p className="metadata-status" role="status">
                  {status}
                </p>
              )}
              {actionError && (
                <p className="error-message" role="alert">
                  {actionError}
                </p>
              )}
              {!book.isAvailable && (
                <p className="availability-warning">{t("unavailable")}</p>
              )}
              <dl className="details-meta">
                <div>
                  <dt>{t("fileFormat")}</dt>
                  <dd>{book.format}</dd>
                </div>
                <div>
                  <dt>{t("fileSize")}</dt>
                  <dd>{formatBytes(book.fileSize)}</dd>
                </div>
                {book.isbn && (
                  <div>
                    <dt>{t("isbn")}</dt>
                    <dd>{book.isbn}</dd>
                  </div>
                )}
                {book.publishedYear && (
                  <div>
                    <dt>{t("publishedYear")}</dt>
                    <dd>{book.publishedYear}</dd>
                  </div>
                )}
                {book.publisher && (
                  <div className="wide">
                    <dt>{t("publisher")}</dt>
                    <dd>{book.publisher}</dd>
                  </div>
                )}
                {book.genres && (
                  <div className="wide">
                    <dt>{t("genresField")}</dt>
                    <dd>{book.genres}</dd>
                  </div>
                )}
                <div className="wide">
                  <dt>{t("metadataSource")}</dt>
                  <dd>{metadataSourceLabel(book.metadataSource, t)}</dd>
                </div>
                <div>
                  <dt>{t("added")}</dt>
                  <dd>{book.addedAt}</dd>
                </div>
                <div className="wide">
                  <dt>{t("sourceFile")}</dt>
                  <dd title={book.sourcePath}>{book.sourcePath}</dd>
                </div>
              </dl>
              {book.description && (
                <p className="details-description">{book.description}</p>
              )}
              <div className="metadata-actions">
                <button
                  type="button"
                  className="secondary-button"
                  onClick={() => {
                    setMetadata(metadataFromBook(book));
                    setMode("edit");
                    setActionError("");
                  }}
                >
                  {t("editMetadata")}
                </button>
                <button
                  type="button"
                  className="secondary-button"
                  onClick={() => {
                    setMode("search");
                    setCandidates([]);
                    setMetadataSearched(false);
                    setActionError("");
                  }}
                >
                  {t("findMetadata")}
                </button>
              </div>
              {book.coverSource === "open_library" && (
                <div className="external-cover-policy">
                  <p>{t("externalCoverPolicy")}</p>
                  <button
                    type="button"
                    className="text-button danger"
                    disabled={actionBusy}
                    onClick={() =>
                      void runAction(
                        () => removeExternalCover(book.id),
                        t("metadataSaved"),
                      )
                    }
                  >
                    {t("removeExternalCover")}
                  </button>
                </div>
              )}
              {isReaderFormat(book.format) && (
                <button
                  className="primary-button details-read"
                  type="button"
                  disabled={!book.isAvailable || busy}
                  onClick={() => onRead(book)}
                >
                  <Icon name="reading" />
                  {book.progress > 0 ? t("continueReading") : t("readBook")}
                </button>
              )}
              <div className="remove-book-policy">
                <p>{t("removeBookPolicy")}</p>
                <button
                  className="danger-button"
                  type="button"
                  disabled={actionBusy || busy}
                  onClick={() => {
                    setActionBusy(true);
                    void onRemove(book).finally(() => setActionBusy(false));
                  }}
                >
                  {t("removeFromLibrary")}
                </button>
              </div>
            </>
          )}
        </div>
      )}
    </aside>
  );
}

function MetadataEditor({
  value,
  t,
  busy,
  error,
  onChange,
  onCancel,
  onSave,
}: {
  value: BookMetadataInput;
  t: Translator;
  busy: boolean;
  error: string;
  onChange: (value: BookMetadataInput) => void;
  onCancel: () => void;
  onSave: () => void;
}) {
  const field = (
    key: keyof BookMetadataInput,
    label: TranslationKey,
    multiline = false,
  ) => (
    <label className={multiline ? "wide" : ""}>
      <span>{t(label)}</span>
      {multiline ? (
        <textarea
          value={value[key]}
          maxLength={16_384}
          onChange={(event) =>
            onChange({ ...value, [key]: event.target.value })
          }
        />
      ) : (
        <input
          value={value[key]}
          required={key === "title"}
          maxLength={key === "isbn" ? 64 : key === "genres" ? 1_024 : 512}
          onChange={(event) =>
            onChange({ ...value, [key]: event.target.value })
          }
        />
      )}
    </label>
  );
  return (
    <form
      className="metadata-editor"
      onSubmit={(event) => {
        event.preventDefault();
        onSave();
      }}
    >
      <h2>{t("editMetadata")}</h2>
      <div className="metadata-form-grid">
        {field("title", "titleField")}
        {field("author", "authorField")}
        {field("subtitle", "subtitleField")}
        {field("isbn", "isbn")}
        {field("publisher", "publisher")}
        {field("publishedYear", "publishedYear")}
        {field("language", "languageField")}
        {field("series", "seriesField")}
        {field("genres", "genresField")}
        {field("description", "descriptionField", true)}
      </div>
      {error && (
        <p className="error-message" role="alert">
          {error}
        </p>
      )}
      <div className="dialog-actions">
        <button type="button" onClick={onCancel}>
          {t("cancel")}
        </button>
        <button className="primary-button" type="submit" disabled={busy}>
          {t("saveMetadata")}
        </button>
      </div>
    </form>
  );
}

function MetadataSearch({
  query,
  candidates,
  searched,
  t,
  busy,
  error,
  onQuery,
  onCancel,
  onSearch,
  onApply,
}: {
  query: string;
  candidates: MetadataCandidate[];
  searched: boolean;
  t: Translator;
  busy: boolean;
  error: string;
  onQuery: (query: string) => void;
  onCancel: () => void;
  onSearch: () => void;
  onApply: (candidate: MetadataCandidate) => void;
}) {
  return (
    <section className="metadata-search-panel">
      <h2>{t("findMetadata")}</h2>
      <p>{t("metadataSearchHint")}</p>
      <form
        onSubmit={(event) => {
          event.preventDefault();
          onSearch();
        }}
      >
        <label>
          <span>{t("searchQuery")}</span>
          <input
            value={query}
            maxLength={256}
            required
            onChange={(event) => onQuery(event.target.value)}
          />
        </label>
        <button className="primary-button" type="submit" disabled={busy}>
          {t("searchMetadataAction")}
        </button>
      </form>
      {error && (
        <p className="error-message" role="alert">
          {error}
        </p>
      )}
      {searched && !busy && candidates.length === 0 && (
        <p className="metadata-empty">{t("noMetadataResults")}</p>
      )}
      <div className="metadata-candidates">
        {candidates.map((candidate) => (
          <article key={candidate.providerId}>
            <small>{candidate.provider}</small>
            <h3>{candidate.title}</h3>
            <p>{candidate.author || t("unknownAuthor")}</p>
            <dl>
              {candidate.publishedYear && (
                <div>
                  <dt>{t("publishedYear")}</dt>
                  <dd>{candidate.publishedYear}</dd>
                </div>
              )}
              {candidate.publisher && (
                <div>
                  <dt>{t("publisher")}</dt>
                  <dd>{candidate.publisher}</dd>
                </div>
              )}
              {candidate.isbn && (
                <div>
                  <dt>{t("isbn")}</dt>
                  <dd>{candidate.isbn}</dd>
                </div>
              )}
              {candidate.genres && (
                <div>
                  <dt>{t("genresField")}</dt>
                  <dd>{candidate.genres}</dd>
                </div>
              )}
            </dl>
            {candidate.coverId && (
              <span className="cover-available">{t("coverAvailable")}</span>
            )}
            <button
              type="button"
              className="secondary-button"
              disabled={busy}
              onClick={() => onApply(candidate)}
            >
              {t("applyCandidate")}
            </button>
          </article>
        ))}
      </div>
      <button type="button" className="text-button" onClick={onCancel}>
        {t("cancel")}
      </button>
    </section>
  );
}

function metadataSourceLabel(source: string, t: Translator) {
  if (source === "manual") return t("manualMetadata");
  if (source === "open_library") return t("openLibrary");
  return t("embeddedMetadata");
}

function Stat({
  icon,
  value,
  label,
}: {
  icon: IconName;
  value: string;
  label: string;
}) {
  return (
    <article className="stat-card">
      <span className="stat-icon">
        <Icon name={icon} />
      </span>
      <span>
        <strong>{value}</strong>
        <small>{label}</small>
      </span>
    </article>
  );
}

function EmptyState({
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
        <Icon name="reading" />
      </span>
      <h2>{title}</h2>
      {hint && <p>{hint}</p>}
      {action && actionLabel && (
        <button type="button" className="primary-button" onClick={action}>
          <Icon name="plus" />
          {actionLabel}
        </button>
      )}
    </section>
  );
}

function summaryMessage(template: string, summary: ImportSummary) {
  return template
    .replace("{imported}", String(summary.imported))
    .replace("{duplicates}", String(summary.duplicates))
    .replace("{failed}", String(summary.failed));
}

function formatBytes(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function isReflowFormat(format: string) {
  return [
    "TXT",
    "HTML",
    "HTM",
    "MD",
    "MARKDOWN",
    "EPUB",
    "FB2",
    "DOCX",
  ].includes(format);
}

function isSpecialFormat(format: string) {
  return ["PDF", "CBZ", "CBR"].includes(format);
}

function isReaderFormat(format: string) {
  return isReflowFormat(format) || isSpecialFormat(format);
}
