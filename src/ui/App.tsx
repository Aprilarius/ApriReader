import {
  Component,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import type { CSSProperties, ReactNode } from "react";
import {
  chooseAndImportAudiobookFolder,
  chooseAndImportAudiobooks,
  chooseAndWatchAudioFolder,
  listAudiobookParts,
  listAudiobooks,
  listWatchedAudioFolders,
  scanWatchedAudioFolders,
  type AudioImportSummary,
  type AudiobookPartRecord,
  type AudiobookRecord,
  type WatchedAudioFolder,
} from "../application/audiobooks";
import {
  readAudioCloseBehavior,
  setAudioCloseBehavior,
  syncAudioCloseBehavior,
  type AudioCloseBehavior,
} from "../application/audioLifecycle";
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
  removeWatchedFolder,
  scanWatchedFolders,
  setBookFavorite,
  type Book,
  type ImportSummary,
  type WatchedFolder,
} from "../application/library";
import {
  listenForLaunchFiles,
  openLaunchPath,
  takeLaunchPaths,
} from "../application/launchBooks";
import {
  applyMetadataCandidate,
  chooseAndSetLocalCover,
  metadataFromBook,
  removeExternalCover,
  searchMetadata,
  updateBookMetadata,
  type BookMetadataInput,
  type MetadataCandidate,
  type MetadataLanguage,
} from "../application/metadata";
import { loadDocument, type DocumentModel } from "../application/reader";
import { getStartupHealth, type StartupHealth } from "../application/health";
import {
  conservativePlatformCapabilities,
  getPlatformCapabilities,
} from "../application/platform";
import {
  emptyStatistics,
  getStatistics,
  type StatisticsSnapshot,
} from "../application/statistics";
import { syncSteamIfAvailable } from "../application/steam";
import { Icon, type IconName } from "./icons";
import { greetingKeyForHour } from "./greeting";
import { supportedLocales, type Locale, type TranslationKey } from "./i18n";
import { ReaderScreen } from "./ReaderScreen";
import { SpecialReaderScreen } from "./SpecialReaderScreen";
import { AchievementsPage, StatisticsPage } from "./StatisticsPages";
import { AudiobookDetails, AudiobooksPage } from "./AudiobooksPage";
import { AudiobookPlayer } from "./AudiobookPlayer";
import { readLocalValue, writeLocalValue } from "./localStorage";
import { useLocale } from "./useLocale";
import { useCurrentHour } from "./useCurrentHour";
import {
  displayNameMaxLength,
  normalizeDisplayName,
  useLocalProfile,
} from "./useLocalProfile";
import {
  normalizeBookLanguage,
  useScreenReaderSupport,
} from "./useScreenReaderSupport";
import { uiThemes, useUiTheme, type UiMode, type UiTheme } from "./useUiTheme";

type Route = { id: string; label: TranslationKey; icon: IconName };
const routes: Route[] = [
  { id: "library", label: "library", icon: "library" },
  { id: "audiobooks", label: "audiobooks", icon: "audio" },
  { id: "reading", label: "readingNow", icon: "reading" },
  { id: "collections", label: "collections", icon: "collections" },
  { id: "authors", label: "authors", icon: "authors" },
  { id: "series", label: "series", icon: "series" },
  { id: "favorites", label: "favorites", icon: "favorite" },
  { id: "achievements", label: "achievements", icon: "achievement" },
  { id: "statistics", label: "statistics", icon: "statistics" },
  { id: "settings", label: "settings", icon: "settings" },
];

class ReaderCrashBoundary extends Component<
  {
    children: ReactNode;
    t: ReturnType<typeof useLocale>["t"];
    onClose: () => void;
    document: DocumentModel;
  },
  { failed: boolean }
> {
  state = { failed: false };

  static getDerivedStateFromError() {
    return { failed: true };
  }

  componentDidCatch(error: unknown) {
    console.error("Reader render failed", error);
  }

  render() {
    if (!this.state.failed) return this.props.children;
    return <SafeReaderScreen {...this.props} />;
  }
}

/**
 * A deliberately dependency-free reader used only if the enhanced reader
 * fails. It keeps a valid document readable instead of replacing it with an
 * error page on a device-specific WebView failure.
 */
function SafeReaderScreen({
  document,
  t,
  onClose,
}: {
  document: DocumentModel;
  t: ReturnType<typeof useLocale>["t"];
  onClose: () => void;
}) {
  const [sectionIndex, setSectionIndex] = useState(document.lastSection);
  const section = document.sections[sectionIndex] ?? document.sections[0];
  if (!section) return null;

  return (
    <div className="reader-screen theme-paper safe-reader-screen">
      <header className="reader-toolbar">
        <button
          type="button"
          className="reader-icon-button"
          aria-label={t("readerBack")}
          onClick={onClose}
        >
          ←
        </button>
        <div className="reader-book-title">
          <strong>{document.title}</strong>
          <span>{document.author}</span>
        </div>
      </header>
      <main className="reader-scroll layout-continuous">
        <article className="reader-document reader-document-continuous">
          <p className="reader-kicker">
            {sectionIndex + 1} / {document.sections.length}
          </p>
          <h1>{section.title}</h1>
          <div className="reader-blocks">
            {section.blocks.map((block, index) => (
              <p
                className={
                  block.kind === "heading"
                    ? "reader-block-heading"
                    : block.kind === "quote"
                      ? "reader-block-quote"
                      : block.kind === "code"
                        ? "reader-block-code"
                        : block.kind === "listItem"
                          ? "reader-block-list"
                          : ""
                }
                key={`${section.id}-${index}`}
              >
                {block.text}
              </p>
            ))}
          </div>
          <footer className="reader-section-nav">
            <button
              type="button"
              disabled={sectionIndex === 0}
              onClick={() => setSectionIndex((value) => value - 1)}
            >
              ← {t("previousSection")}
            </button>
            <button
              type="button"
              disabled={sectionIndex === document.sections.length - 1}
              onClick={() => setSectionIndex((value) => value + 1)}
            >
              {t("nextSection")} →
            </button>
          </footer>
        </article>
      </main>
    </div>
  );
}

export function App() {
  const { locale, t, selectLocale, confirmLocale, languageSelected } =
    useLocale();
  const currentHour = useCurrentHour();
  const {
    onboardingComplete,
    displayName,
    completeOnboarding,
    saveDisplayName,
  } = useLocalProfile();
  const { screenReaderSupport, setScreenReaderSupport } =
    useScreenReaderSupport();
  const { uiTheme, uiMode, setUiTheme, setUiMode } = useUiTheme();
  const [audioCloseBehavior, setAudioCloseBehaviorState] =
    useState<AudioCloseBehavior>(readAudioCloseBehavior);
  const [route, setRoute] = useState("library");
  const [books, setBooks] = useState<Book[]>([]);
  const [folders, setFolders] = useState<WatchedFolder[]>([]);
  const [audiobooks, setAudiobooks] = useState<AudiobookRecord[]>([]);
  const [audioFolders, setAudioFolders] = useState<WatchedAudioFolder[]>([]);
  const [selectedAudiobookId, setSelectedAudiobookId] = useState<number | null>(
    null,
  );
  const [selectedAudioParts, setSelectedAudioParts] = useState<
    AudiobookPartRecord[]
  >([]);
  const [audioLoading, setAudioLoading] = useState(false);
  const [audioPartsLoading, setAudioPartsLoading] = useState(false);
  const [activeAudiobook, setActiveAudiobook] =
    useState<AudiobookRecord | null>(null);
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
  const [platformCapabilities, setPlatformCapabilities] = useState(
    conservativePlatformCapabilities,
  );
  const mainRef = useRef<HTMLElement>(null);
  const previousRoute = useRef(route);
  const launchFileWork = useRef<Promise<void>>(Promise.resolve());
  const readerRequest = useRef(0);
  const audioRequest = useRef(0);
  const visibleRoutes = routes;
  const current = visibleRoutes.find((item) => item.id === route) ?? routes[0]!;
  const selected = books.find((book) => book.id === selectedId) ?? null;
  const selectedAudiobook =
    audiobooks.find((book) => book.id === selectedAudiobookId) ?? null;

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

  const refreshAudiobooks = useCallback(async () => {
    setAudioLoading(true);
    try {
      const [nextBooks, nextFolders] = await Promise.all([
        listAudiobooks(),
        listWatchedAudioFolders(),
      ]);
      setAudiobooks(nextBooks);
      setAudioFolders(nextFolders);
      setSelectedAudiobookId((currentId) =>
        nextBooks.some((book) => book.id === currentId) ? currentId : null,
      );
      setError("");
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setAudioLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
    void getPlatformCapabilities()
      .then((capabilities) => {
        setPlatformCapabilities(capabilities);
        window.document.documentElement.dataset.platform =
          capabilities.platform;
        if (capabilities.steamIntegration) {
          void syncSteamIfAvailable().catch((reason: unknown) =>
            console.error("Steam achievement sync failed", reason),
          );
        }
      })
      /*
       * Losing this leaves the app on the fail-closed capability set, which
       * quietly hides watched folders and the tray. Report it rather than
       * leaving a degraded window with no explanation anywhere.
       */
      .catch((reason: unknown) =>
        console.error("Platform capability probe failed", reason),
      );
    void getStartupHealth()
      .then(setStartupHealth)
      .catch((reason: unknown) =>
        console.error("Startup health probe failed", reason),
      );
  }, [refresh]);

  useEffect(() => {
    if (platformCapabilities.desktop) {
      void syncAudioCloseBehavior(audioCloseBehavior).catch(() => undefined);
    }
  }, [audioCloseBehavior, platformCapabilities.desktop]);

  useEffect(() => {
    if (route === "audiobooks") void refreshAudiobooks();
    if (route === "settings")
      setAudioCloseBehaviorState(readAudioCloseBehavior());
  }, [refreshAudiobooks, route]);

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

  const runAudioImport = async (
    operation: () => Promise<AudioImportSummary | null>,
  ) => {
    setBusy(true);
    setMessage("");
    setError("");
    try {
      const summary = await operation();
      if (summary) {
        setMessage(audioSummaryMessage(t("audioImportDone"), summary));
        if (summary.errors.length > 0) setError(summary.errors.join("\n"));
        await refreshAudiobooks();
      }
    } catch (reason) {
      setError(
        `${t("audioImportError")}: ${reason instanceof Error ? reason.message : String(reason)}`,
      );
    } finally {
      setBusy(false);
    }
  };

  const selectAudiobook = useCallback(async (id: number) => {
    const request = ++audioRequest.current;
    setSelectedAudiobookId(id);
    setSelectedAudioParts([]);
    setAudioPartsLoading(true);
    try {
      const parts = await listAudiobookParts(id);
      if (request === audioRequest.current) setSelectedAudioParts(parts);
    } catch (reason) {
      if (request === audioRequest.current) {
        setError(reason instanceof Error ? reason.message : String(reason));
      }
    } finally {
      if (request === audioRequest.current) setAudioPartsLoading(false);
    }
  }, []);

  const updateAudiobookProgress = useCallback((updated: AudiobookRecord) => {
    setAudiobooks((items) =>
      items.map((item) => (item.id === updated.id ? updated : item)),
    );
  }, []);

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

  const stopWatchingFolder = async (folder: WatchedFolder) => {
    if (
      !window.confirm(
        t("stopWatchingFolderConfirm").replace("{path}", folder.path),
      )
    ) {
      return;
    }
    setBusy(true);
    setError("");
    try {
      await removeWatchedFolder(folder.id);
      setFolders(await listWatchedFolders());
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusy(false);
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

  const openBook = useCallback(
    async (book: Book) => {
      const request = ++readerRequest.current;
      setReaderLoading(true);
      setError("");
      try {
        const language = normalizeBookLanguage(book.language);
        if (isSpecialFormat(book.format)) {
          const nextDocument = await loadSpecialDocument(book.id);
          if (request !== readerRequest.current) return;
          setDocument(null);
          setSpecialDocument(nextDocument);
        } else {
          const nextDocument = await loadDocument(book.id);
          if (request !== readerRequest.current) return;
          setSpecialDocument(null);
          setDocument(nextDocument);
        }
        setReaderLanguage(language);
        setSelectedId(null);
      } catch (reason) {
        if (request !== readerRequest.current) return;
        setError(
          `${t("readerError")}: ${reason instanceof Error ? reason.message : String(reason)}`,
        );
      } finally {
        if (request === readerRequest.current) setReaderLoading(false);
      }
    },
    [t],
  );

  const closeReader = useCallback(() => {
    readerRequest.current += 1;
    setReaderLoading(false);
    setDocument(null);
    setSpecialDocument(null);
    void refresh();
  }, [refresh]);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;

    const consumeLaunchFiles = () => {
      launchFileWork.current = launchFileWork.current
        .then(async () => {
          const paths = await takeLaunchPaths();
          for (const path of paths) {
            if (disposed) return;
            const opened = await openLaunchPath(path);
            if (opened.kind === "book") {
              setActiveAudiobook(null);
              await refresh();
              await openBook(opened.item);
              continue;
            }

            readerRequest.current += 1;
            setDocument(null);
            setSpecialDocument(null);
            setReaderLoading(false);
            setRoute("audiobooks");
            const parts = await listAudiobookParts(opened.item.id);
            if (disposed) return;
            await refreshAudiobooks();
            setSelectedAudiobookId(opened.item.id);
            setSelectedAudioParts(parts);
            setActiveAudiobook(opened.item);
          }
        })
        .catch((reason: unknown) => {
          if (!disposed) {
            setError(
              `${t("readerError")}: ${reason instanceof Error ? reason.message : String(reason)}`,
            );
          }
        });
    };

    void listenForLaunchFiles(consumeLaunchFiles).then((stopListening) => {
      if (disposed) {
        stopListening();
        return;
      }
      unlisten = stopListening;
      consumeLaunchFiles();
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [openBook, refresh, refreshAudiobooks, t]);

  if (!languageSelected) {
    return <LanguageWelcomeScreen locale={locale} onConfirm={confirmLocale} />;
  }

  if (!onboardingComplete) {
    return (
      <WelcomeScreen
        locale={locale}
        t={t}
        onSelectLocale={selectLocale}
        onContinue={completeOnboarding}
        onSkip={() => completeOnboarding()}
      />
    );
  }

  if (document) {
    return (
      <ReaderCrashBoundary
        key={document.bookId}
        document={document}
        t={t}
        onClose={closeReader}
      >
        <ReaderScreen
          document={document}
          t={t}
          language={readerLanguage}
          screenReaderSupport={screenReaderSupport}
          allowCloudTts={platformCapabilities.protectedCloudCredentials}
          onClose={closeReader}
          onProgress={(progress) => {
            setBooks((items) =>
              items.map((book) =>
                book.id === document.bookId ? { ...book, progress } : book,
              ),
            );
          }}
        />
      </ReaderCrashBoundary>
    );
  }

  if (specialDocument) {
    return (
      <SpecialReaderScreen
        document={specialDocument}
        t={t}
        language={readerLanguage}
        screenReaderSupport={screenReaderSupport}
        onClose={closeReader}
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

  if (activeAudiobook) {
    return (
      <AudiobookPlayer
        book={activeAudiobook}
        parts={selectedAudioParts}
        t={t}
        onProgress={updateAudiobookProgress}
        onClose={() => setActiveAudiobook(null)}
      />
    );
  }

  return (
    <div className="app-shell" data-platform={platformCapabilities.platform}>
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
          {visibleRoutes.map((item) => (
            <button
              className={`nav-item ${route === item.id ? "active" : ""}`}
              type="button"
              key={item.id}
              aria-label={t(item.label)}
              aria-current={route === item.id ? "page" : undefined}
              title={t(item.label)}
              onClick={() => {
                setRoute(item.id);
                if (item.id === "audiobooks") setSelectedId(null);
                else setSelectedAudiobookId(null);
              }}
            >
              <Icon name={item.icon} />
              <span className="nav-label">{t(item.label)}</span>
            </button>
          ))}
        </nav>
        <div className="sidebar-foot">
          <LanguageSelect
            className="sidebar-language"
            locale={locale}
            t={t}
            onSelect={selectLocale}
          />
        </div>
      </aside>

      <main
        ref={mainRef}
        id="main-content"
        className="main-content"
        tabIndex={-1}
        aria-labelledby="page-title"
      >
        <section className="page route-page" key={route}>
          <header className="page-header">
            <div>
              <p className="eyebrow">{t("personalLibrary")}</p>
              <h1 id="page-title">
                {route === "library"
                  ? `${t(greetingKeyForHour(currentHour))}${
                      displayName ? `, ${displayName}` : ""
                    }!`
                  : t(current.label)}
              </h1>
            </div>
            {route === "library" && (
              <label className="search">
                <span className="sr-only">{t("search")}</span>
                <Icon name="search" />
                <input
                  type="search"
                  placeholder={t("search")}
                  value={query}
                  onChange={(event) => setQuery(event.target.value)}
                />
              </label>
            )}
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
              onWatch={
                platformCapabilities.watchedFolders
                  ? () => void runImport(chooseAndWatchFolder)
                  : null
              }
            />
          ) : route === "audiobooks" ? (
            <AudiobooksPage
              audiobooks={audiobooks}
              folders={audioFolders}
              selectedId={selectedAudiobookId}
              loading={audioLoading}
              busy={busy}
              locale={locale}
              t={t}
              onSelect={(id) => void selectAudiobook(id)}
              onImportFiles={() =>
                void runAudioImport(chooseAndImportAudiobooks)
              }
              onImportFolder={
                platformCapabilities.watchedFolders
                  ? () => void runAudioImport(chooseAndImportAudiobookFolder)
                  : null
              }
              onWatchFolder={
                platformCapabilities.watchedFolders
                  ? () => void runAudioImport(chooseAndWatchAudioFolder)
                  : null
              }
              onScan={
                platformCapabilities.watchedFolders
                  ? () =>
                      void runAudioImport(async () => scanWatchedAudioFolders())
                  : null
              }
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
              onUnwatch={(folder) => void stopWatchingFolder(folder)}
            />
          ) : route === "settings" ? (
            <SettingsPage
              t={t}
              displayName={displayName}
              onDisplayNameChange={saveDisplayName}
              screenReaderSupport={screenReaderSupport}
              onScreenReaderSupportChange={setScreenReaderSupport}
              uiTheme={uiTheme}
              onUiThemeChange={setUiTheme}
              uiMode={uiMode}
              onUiModeChange={setUiMode}
              audioCloseBehavior={audioCloseBehavior}
              onAudioCloseBehaviorChange={(behavior) => {
                setAudioCloseBehaviorState(behavior);
                void setAudioCloseBehavior(behavior).catch((reason: unknown) =>
                  setError(
                    reason instanceof Error ? reason.message : String(reason),
                  ),
                );
              }}
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
          ) : null}
        </section>
      </main>

      {route === "audiobooks" ? (
        <AudiobookDetails
          key={selectedAudiobook?.id ?? "empty-audio-details"}
          book={selectedAudiobook}
          parts={selectedAudioParts}
          loading={audioPartsLoading}
          locale={locale}
          t={t}
          onOpenPlayer={setActiveAudiobook}
          onChanged={updateAudiobookProgress}
          onClose={() => {
            audioRequest.current += 1;
            setSelectedAudiobookId(null);
            setSelectedAudioParts([]);
          }}
        />
      ) : (
        <BookDetails
          key={selected?.id ?? "empty-details"}
          book={selected}
          locale={locale === "ru" ? "ru" : "en"}
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
      )}
      {(route === "audiobooks" ? selectedAudiobook : selected) && (
        <button
          className="drawer-scrim"
          type="button"
          aria-label={
            route === "audiobooks" ? t("closeAudioDetails") : t("closeDetails")
          }
          onClick={() => {
            if (route === "audiobooks") {
              audioRequest.current += 1;
              setSelectedAudiobookId(null);
              setSelectedAudioParts([]);
            } else setSelectedId(null);
          }}
        />
      )}
    </div>
  );
}

const languageIntroductions: Record<Locale, string> = {
  ru: "Добро пожаловать",
  en: "Welcome",
  az: "Xoş gəlmisiniz",
  it: "Benvenuto",
  de: "Willkommen",
  tr: "Hoş geldiniz",
};

function LanguageWelcomeScreen({
  locale,
  onConfirm,
}: {
  locale: Locale;
  onConfirm: (locale: Locale) => void;
}) {
  const [selected, setSelected] = useState(locale);
  const [leaving, setLeaving] = useState(false);

  const confirm = () => {
    if (leaving) return;
    setLeaving(true);
    window.setTimeout(() => onConfirm(selected), 260);
  };

  return (
    <main
      className={`language-welcome-screen ${leaving ? "is-leaving" : ""}`}
      aria-labelledby="language-welcome-title"
    >
      <div className="language-welcome-glow" aria-hidden="true" />
      <section className="language-welcome-card">
        <header className="language-welcome-header">
          <span className="language-welcome-mark" aria-hidden="true">
            <Icon name="reading" />
          </span>
          <p className="language-welcome-brand">ApriReader</p>
          <h1 id="language-welcome-title">Choose your reading language</h1>
          <p>Выберите язык · Dilinizi seçin · Scegli la lingua</p>
        </header>

        <div className="language-cards" role="radiogroup" aria-label="Language">
          {supportedLocales.map((language, index) => (
            <button
              type="button"
              role="radio"
              aria-checked={selected === language.id}
              className={`language-card ${selected === language.id ? "selected" : ""}`}
              key={language.id}
              style={
                { animationDelay: `${100 + index * 48}ms` } as CSSProperties
              }
              onClick={() => setSelected(language.id)}
              onDoubleClick={confirm}
            >
              <span className="language-card-code">{language.code}</span>
              <span className="language-card-copy">
                <strong>{language.label}</strong>
                <small>{languageIntroductions[language.id]}</small>
              </span>
              <span className="language-card-check" aria-hidden="true">
                ✓
              </span>
            </button>
          ))}
        </div>

        <button
          className="language-confirm-button"
          type="button"
          onClick={confirm}
          disabled={leaving}
        >
          <span>{languageIntroductions[selected]}</span>
          <span aria-hidden="true">→</span>
        </button>
        <p className="language-welcome-footnote">
          You can change this later in the library.
        </p>
      </section>
    </main>
  );
}

function LanguageSelect({
  className,
  locale,
  t,
  onSelect,
}: {
  className?: string;
  locale: Locale;
  t: Translator;
  onSelect: (locale: Locale) => void;
}) {
  return (
    <label className={`language-select ${className ?? ""}`.trim()}>
      <span className="visually-hidden">{t("switchLanguage")}</span>
      <select
        value={locale}
        aria-label={t("switchLanguage")}
        title={t("switchLanguage")}
        onChange={(event) => onSelect(event.currentTarget.value as Locale)}
      >
        {supportedLocales.map((language) => (
          <option value={language.id} key={language.id}>
            {language.code} · {language.label}
          </option>
        ))}
      </select>
    </label>
  );
}

function WelcomeScreen({
  locale,
  t,
  onSelectLocale,
  onContinue,
  onSkip,
}: {
  locale: Locale;
  t: Translator;
  onSelectLocale: (locale: Locale) => void;
  onContinue: (displayName: string) => void;
  onSkip: () => void;
}) {
  const [displayName, setDisplayName] = useState("");
  const normalized = normalizeDisplayName(displayName);

  return (
    <main className="welcome-screen">
      <LanguageSelect
        className="welcome-language"
        locale={locale}
        t={t}
        onSelect={onSelectLocale}
      />
      <section className="welcome-card" aria-labelledby="welcome-title">
        <span className="welcome-mark" aria-hidden="true">
          <Icon name="reading" />
        </span>
        <p className="eyebrow">{t("appName")}</p>
        <h1 id="welcome-title">{t("welcomeTitle")}</h1>
        <p className="welcome-copy">{t("welcomeHint")}</p>
        <form
          onSubmit={(event) => {
            event.preventDefault();
            if (normalized) onContinue(normalized);
          }}
        >
          <label htmlFor="welcome-display-name">{t("displayNameLabel")}</label>
          <input
            id="welcome-display-name"
            autoFocus
            autoComplete="name"
            maxLength={displayNameMaxLength}
            value={displayName}
            placeholder={t("displayNamePlaceholder")}
            onChange={(event) => setDisplayName(event.target.value)}
          />
          <p className="welcome-privacy">{t("profilePrivacyHint")}</p>
          <div className="welcome-actions">
            <button
              className="primary-button"
              type="submit"
              disabled={!normalized}
            >
              {t("continue")}
            </button>
            <button className="secondary-button" type="button" onClick={onSkip}>
              {t("skip")}
            </button>
          </div>
        </form>
      </section>
    </main>
  );
}

export function SettingsPage({
  t,
  displayName,
  onDisplayNameChange,
  screenReaderSupport,
  onScreenReaderSupportChange,
  uiTheme,
  onUiThemeChange,
  uiMode,
  onUiModeChange,
  audioCloseBehavior,
  onAudioCloseBehaviorChange,
}: {
  t: Translator;
  displayName: string;
  onDisplayNameChange: (displayName: string) => void;
  screenReaderSupport: boolean;
  onScreenReaderSupportChange: (enabled: boolean) => void;
  uiTheme: UiTheme;
  onUiThemeChange: (theme: UiTheme) => void;
  uiMode: UiMode;
  onUiModeChange: (mode: UiMode) => void;
  audioCloseBehavior: AudioCloseBehavior;
  onAudioCloseBehaviorChange: (behavior: AudioCloseBehavior) => void;
}) {
  const [profileName, setProfileName] = useState(displayName);
  const [profileSaved, setProfileSaved] = useState(false);

  const saveProfile = (nextName: string) => {
    const normalized = normalizeDisplayName(nextName);
    onDisplayNameChange(normalized);
    setProfileName(normalized);
    setProfileSaved(true);
  };

  return (
    <div className="settings-page">
      <section className="profile-settings settings-section">
        <p className="eyebrow">{t("localProfile")}</p>
        <h2>{t("profileSettingsTitle")}</h2>
        <p className="settings-hint">{t("profileSettingsHint")}</p>
        <form
          className="profile-form"
          onSubmit={(event) => {
            event.preventDefault();
            saveProfile(profileName);
          }}
        >
          <label htmlFor="settings-display-name">{t("displayNameLabel")}</label>
          <input
            id="settings-display-name"
            maxLength={displayNameMaxLength}
            value={profileName}
            placeholder={t("displayNamePlaceholder")}
            onChange={(event) => {
              setProfileName(event.target.value);
              setProfileSaved(false);
            }}
          />
          <div className="profile-actions">
            <button className="primary-button" type="submit">
              {t("save")}
            </button>
            {displayName && (
              <button
                className="secondary-button"
                type="button"
                onClick={() => saveProfile("")}
              >
                {t("removeName")}
              </button>
            )}
          </div>
          {profileSaved && (
            <p className="profile-saved" role="status">
              {t("profileSaved")}
            </p>
          )}
        </form>
      </section>
      <section className="audio-settings settings-section">
        <p className="eyebrow">{t("audiobooks")}</p>
        <h2>{t("audioCloseBehavior")}</h2>
        <p className="settings-hint">{t("audioCloseBehaviorHint")}</p>
        <label className="settings-select" htmlFor="audio-close-behavior">
          <span>{t("audioCloseBehavior")}</span>
          <select
            id="audio-close-behavior"
            value={audioCloseBehavior}
            onChange={(event) =>
              onAudioCloseBehaviorChange(
                event.target.value as AudioCloseBehavior,
              )
            }
          >
            <option value="ask">{t("audioCloseAsk")}</option>
            <option value="tray">{t("audioCloseTray")}</option>
            <option value="exit">{t("audioCloseExit")}</option>
          </select>
        </label>
      </section>
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
      <section className="ui-theme-settings settings-section">
        <p className="eyebrow">{t("uiThemeEyebrow")}</p>
        <h2>{t("uiThemeSectionTitle")}</h2>
        <p className="settings-hint">{t("uiThemeSectionHint")}</p>
        <fieldset className="theme-choices ui-theme-choices">
          <legend>{t("uiThemeStyleLabel")}</legend>
          {uiThemes.map((theme) => (
            <button
              type="button"
              className={uiTheme === theme ? "active" : ""}
              aria-pressed={uiTheme === theme}
              key={theme}
              onClick={() => onUiThemeChange(theme)}
            >
              <span className={`theme-swatch ui-theme-swatch-${theme}`} />
              {t(uiThemeLabelKey(theme))}
            </button>
          ))}
        </fieldset>
        <div
          className="ui-mode-choices"
          role="group"
          aria-label={t("uiThemeModeLabel")}
        >
          <button
            type="button"
            className={uiMode === "light" ? "active" : ""}
            aria-pressed={uiMode === "light"}
            onClick={() => onUiModeChange("light")}
          >
            {t("uiThemeLight")}
          </button>
          <button
            type="button"
            className={uiMode === "dark" ? "active" : ""}
            aria-pressed={uiMode === "dark"}
            onClick={() => onUiModeChange("dark")}
          >
            {t("uiThemeDark")}
          </button>
        </div>
      </section>
    </div>
  );
}

function uiThemeLabelKey(theme: UiTheme): TranslationKey {
  const keys: Record<UiTheme, TranslationKey> = {
    default: "uiThemeDefault",
    classic: "uiThemeClassic",
    bookish: "uiThemeBookish",
    glass: "uiThemeGlass",
    "liquid-glass": "uiThemeLiquidGlass",
    neumorphism: "uiThemeNeumorphism",
  };
  return keys[theme];
}

type Translator = ReturnType<typeof useLocale>["t"];
type LibraryView = "grid" | "list";
const libraryViewPreferenceKey = "aprireader.library.view";
/** The key this preference used before the mobile build was dropped. */
const legacyLibraryViewPreferenceKey = "aprireader.library.mobileView";

function readLibraryView(): LibraryView {
  const stored =
    readLocalValue(libraryViewPreferenceKey) ??
    readLocalValue(legacyLibraryViewPreferenceKey);
  return stored === "list" ? "list" : "grid";
}

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
  onWatch: (() => void) | null;
}) {
  const [renderLimit, setRenderLimit] = useState(120);
  const [selectionMode, setSelectionMode] = useState(false);
  const [libraryView, setLibraryView] = useState<LibraryView>(readLibraryView);
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
  const chooseLibraryView = (view: LibraryView) => {
    setLibraryView(view);
    writeLocalValue(libraryViewPreferenceKey, view);
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
          <div
            className="library-view-switch"
            role="group"
            aria-label={t("libraryDisplayMode")}
          >
            <button
              className={libraryView === "grid" ? "active" : ""}
              type="button"
              aria-label={t("libraryGridView")}
              aria-pressed={libraryView === "grid"}
              onClick={() => chooseLibraryView("grid")}
            >
              <span aria-hidden="true" className="view-grid-icon" />
            </button>
            <button
              className={libraryView === "list" ? "active" : ""}
              type="button"
              aria-label={t("libraryListView")}
              aria-pressed={libraryView === "list"}
              onClick={() => chooseLibraryView("list")}
            >
              <span aria-hidden="true" className="view-list-icon" />
            </button>
          </div>
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
              {onWatch && (
                <button
                  className="secondary-button"
                  type="button"
                  disabled={busy}
                  onClick={onWatch}
                >
                  <Icon name="folder" />
                  {t("watchFolder")}
                </button>
              )}
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
        <EmptyState
          title={t("noSearchResults")}
          hint={t("noSearchResultsHint")}
        />
      ) : (
        <section
          className={`book-grid library-shelf shelf-${libraryView}`}
          aria-label={t("library")}
        >
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
  onUnwatch,
}: {
  folders: WatchedFolder[];
  formats: string[];
  busy: boolean;
  t: Translator;
  onWatch: () => void;
  onScan: () => void;
  onUnwatch: (folder: WatchedFolder) => void;
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
              <button
                type="button"
                className="secondary-button folder-unwatch"
                disabled={busy}
                onClick={() => onUnwatch(folder)}
              >
                {t("stopWatchingFolder")}
              </button>
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
  locale = "en",
  t,
  busy,
  onRead,
  onFavorite,
  onRemove,
  onUpdated,
  onClose,
}: {
  book: Book | null;
  locale?: MetadataLanguage;
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
  const [metadataLanguage, setMetadataLanguage] =
    useState<MetadataLanguage>(locale);
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

  const runCoverAction = async (action: () => Promise<Book | null>) => {
    setActionBusy(true);
    setActionError("");
    try {
      const updated = await action();
      if (updated) {
        onUpdated(updated);
        setStatus(t("coverUpdated"));
      }
    } catch (reason) {
      setActionError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setActionBusy(false);
    }
  };

  const cover = book?.coverPath ? (
    <img className="details-cover" src={coverUrl(book.coverPath)} alt="" />
  ) : book ? (
    <div className="details-cover fallback-cover">
      <Icon name="reading" />
      <strong>{book.title}</strong>
      <small>{book.author || t("unknownAuthor")}</small>
    </div>
  ) : null;

  return (
    <aside
      className={`details-panel ${book ? "open" : ""}`}
      aria-label={book ? undefined : t("detailsEmpty")}
      aria-labelledby={book ? "book-details-title" : undefined}
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
            aria-label={t("closeDetails")}
          >
            ×
          </button>
          {mode === "edit" ? (
            <>
              <button
                type="button"
                className="details-cover-button"
                aria-label={t("changeCover")}
                title={t("changeCover")}
                disabled={actionBusy}
                onClick={() =>
                  void runCoverAction(() => chooseAndSetLocalCover(book.id))
                }
              >
                {cover}
                <span>{t("changeCover")}</span>
              </button>
              <p className="cover-edit-hint">{t("coverEditHint")}</p>
            </>
          ) : (
            cover
          )}
          {mode === "edit" && metadata ? (
            <MetadataEditor
              value={metadata}
              t={t}
              busy={actionBusy}
              error={actionError}
              onChange={setMetadata}
              onCancel={() => setMode("view")}
              hasCustomCover={book.coverSource !== "embedded"}
              onRestoreCover={() =>
                void runCoverAction(() => removeExternalCover(book.id))
              }
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
              language={metadataLanguage}
              candidates={candidates}
              searched={metadataSearched}
              t={t}
              busy={actionBusy}
              error={actionError}
              onQuery={setMetadataQuery}
              onLanguage={setMetadataLanguage}
              onCancel={() => setMode("view")}
              onSearch={() => {
                setActionBusy(true);
                setActionError("");
                void searchMetadata(book.id, metadataQuery, metadataLanguage)
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
              <h2 id="book-details-title">{book.title}</h2>
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
  hasCustomCover,
  onRestoreCover,
  onSave,
}: {
  value: BookMetadataInput;
  t: Translator;
  busy: boolean;
  error: string;
  onChange: (value: BookMetadataInput) => void;
  onCancel: () => void;
  hasCustomCover: boolean;
  onRestoreCover: () => void;
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
      <div className="external-cover-policy">
        <p>{t("localCoverPolicy")}</p>
        {hasCustomCover && (
          <button
            type="button"
            className="text-button danger"
            disabled={busy}
            onClick={onRestoreCover}
          >
            {t("restoreEmbeddedCover")}
          </button>
        )}
      </div>
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
  language,
  candidates,
  searched,
  t,
  busy,
  error,
  onQuery,
  onLanguage,
  onCancel,
  onSearch,
  onApply,
}: {
  query: string;
  language: MetadataLanguage;
  candidates: MetadataCandidate[];
  searched: boolean;
  t: Translator;
  busy: boolean;
  error: string;
  onQuery: (query: string) => void;
  onLanguage: (language: MetadataLanguage) => void;
  onCancel: () => void;
  onSearch: () => void;
  onApply: (candidate: MetadataCandidate) => void;
}) {
  return (
    <section className="metadata-search-panel">
      <h2>{t("findMetadata")}</h2>
      <p>{t("metadataSearchHint")}</p>
      <fieldset className="metadata-language-switch">
        <legend>{t("metadataLanguage")}</legend>
        <label>
          <input
            type="radio"
            name="metadata-language"
            value="ru"
            checked={language === "ru"}
            onChange={() => onLanguage("ru")}
          />
          <span>{t("metadataLanguageRussian")}</span>
        </label>
        <label>
          <input
            type="radio"
            name="metadata-language"
            value="en"
            checked={language === "en"}
            onChange={() => onLanguage("en")}
          />
          <span>{t("metadataLanguageEnglish")}</span>
        </label>
      </fieldset>
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
          <article key={`${candidate.provider}:${candidate.providerId}`}>
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
              {candidate.series && (
                <div>
                  <dt>{t("seriesField")}</dt>
                  <dd>{candidate.series}</dd>
                </div>
              )}
              {candidate.genres && (
                <div>
                  <dt>{t("genresField")}</dt>
                  <dd>{candidate.genres}</dd>
                </div>
              )}
              {candidate.description && (
                <div>
                  <dt>{t("descriptionField")}</dt>
                  <dd>{candidate.description}</dd>
                </div>
              )}
            </dl>
            {(candidate.coverId || candidate.coverPath) && (
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
  if (source === "inventaire") return t("inventaire");
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
    .replace("{updated}", String(summary.updated))
    .replace("{duplicates}", String(summary.duplicates))
    .replace("{failed}", String(summary.failed));
}

function audioSummaryMessage(template: string, summary: AudioImportSummary) {
  return template
    .replace("{books}", String(summary.importedBooks))
    .replace("{parts}", String(summary.importedParts))
    .replace("{duplicates}", String(summary.duplicateParts))
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
