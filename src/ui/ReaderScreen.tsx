import {
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type FormEvent,
  type MouseEvent,
  type ReactNode,
} from "react";
import {
  chooseAndExportAnnotations,
  createAnnotation,
  deleteAnnotation,
  listAnnotations,
  searchBook,
  updateAnnotationNote,
  type AnnotationKind,
  type AnnotationRecord,
  type SearchResult,
} from "../application/annotations";
import {
  saveReadingPosition,
  type DocumentBlock,
  type DocumentModel,
} from "../application/reader";
import {
  externalTranslationConsentKey,
  externalTranslationMaxCharacters,
  openExternalTranslation,
  type ExternalTranslationProvider,
} from "../application/externalTranslation";
import {
  chooseAndImportReaderFont,
  readerFontUrl,
  type ImportedReaderFont,
} from "../application/fonts";
import { Icon } from "./icons";
import type { TranslationKey } from "./i18n";
import { readLocalValue, writeLocalValue } from "./localStorage";
import { useReadingSession } from "./useReadingSession";

type Translator = (key: TranslationKey) => string;
type ReaderTheme = "paper" | "sepia" | "night";
type ReaderFontChoice =
  | "literary"
  | "book"
  | "classic"
  | "clear"
  | "literata"
  | "lora"
  | "merriweather"
  | "sourceSerif"
  | "charis"
  | "ibmPlex"
  | "custom";
type ReaderFontStyle = "normal" | "italic";
type ReaderLayout = "continuous" | "spread";
type ReaderPanel = "contents" | "search" | "annotations" | "settings" | null;

type ReaderPreferences = {
  fontSize: number;
  lineHeight: number;
  columnWidth: number;
  letterSpacing: number;
  wordSpacing: number;
  paragraphSpacing: number;
  fontWeight: number;
  fontStyle: ReaderFontStyle;
  fontChoice: ReaderFontChoice;
  customFont: ImportedReaderFont | null;
  textAlign: "left" | "justify";
  bionicReading: boolean;
  pageWheel: boolean;
  layout: ReaderLayout;
  theme: ReaderTheme;
};

type PendingSelection = {
  blockIndex: number;
  startOffset: number;
  endOffset: number;
  text: string;
};

type ReaderViewport = {
  width: number;
  height: number;
};

type PageMeasurement = {
  key: string;
  index: number;
  counts: number[];
};

type PendingPositionSave = {
  bookId: number;
  section: number;
  sectionProgress: number;
  progress: number;
};

const preferenceKey = "aprireader.reader.preferences";
const defaultPreferences: ReaderPreferences = {
  fontSize: 20,
  lineHeight: 1.75,
  columnWidth: 720,
  letterSpacing: 0,
  wordSpacing: 0,
  paragraphSpacing: 1.15,
  fontWeight: 400,
  fontStyle: "normal",
  fontChoice: "literary",
  customFont: null,
  textAlign: "left",
  bionicReading: false,
  pageWheel: true,
  layout: "continuous",
  theme: "paper",
};

type ReaderFontDefinition = {
  family: string;
  weights: readonly number[];
};

const readerFonts: Record<
  Exclude<ReaderFontChoice, "custom">,
  ReaderFontDefinition
> = {
  literary: {
    family: 'Georgia, "Times New Roman", serif',
    weights: [400, 700],
  },
  book: {
    family: '"Palatino Linotype", "Book Antiqua", Palatino, serif',
    weights: [400, 700],
  },
  classic: {
    family: "Cambria, Constantia, Georgia, serif",
    weights: [400, 700],
  },
  clear: {
    family: '"Segoe UI", Arial, sans-serif',
    weights: [300, 400, 600, 700],
  },
  literata: {
    family: '"ApriReader Literata", Georgia, serif',
    weights: [200, 300, 400, 500, 600, 700, 800, 900],
  },
  lora: {
    family: '"ApriReader Lora", Georgia, serif',
    weights: [400, 500, 600, 700],
  },
  merriweather: {
    family: '"ApriReader Merriweather", Georgia, serif',
    weights: [300, 400, 500, 600, 700, 800, 900],
  },
  sourceSerif: {
    family: '"ApriReader Source Serif 4", Georgia, serif',
    weights: [200, 300, 400, 500, 600, 700, 800, 900],
  },
  charis: {
    family: '"ApriReader Charis SIL", Georgia, serif',
    weights: [400, 700],
  },
  ibmPlex: {
    family: '"ApriReader IBM Plex Serif", Georgia, serif',
    weights: [100, 200, 300, 400, 500, 600, 700],
  },
};

const customFontWeights = [300, 400, 500, 600, 700] as const;

function fontWeightsForChoice(fontChoice: ReaderFontChoice) {
  return fontChoice === "custom"
    ? customFontWeights
    : readerFonts[fontChoice].weights;
}

function closestFontWeight(requested: number, available: readonly number[]) {
  return available.reduce((closest, candidate) =>
    Math.abs(candidate - requested) < Math.abs(closest - requested)
      ? candidate
      : closest,
  );
}

export function ReaderScreen({
  document,
  t,
  onClose,
  onProgress,
  language,
  screenReaderSupport = true,
}: {
  document: DocumentModel;
  t: Translator;
  onClose: () => void;
  onProgress: (progress: number) => void;
  language?: string;
  screenReaderSupport?: boolean;
}) {
  const [sectionIndex, setSectionIndex] = useState(document.lastSection);
  const [panel, setPanel] = useState<ReaderPanel>(null);
  const [preferences, setPreferences] = useState(readPreferences);
  const [displayProgress, setDisplayProgress] = useState(document.progress);
  const [sectionProgress, setSectionProgress] = useState(
    document.sectionProgress,
  );
  const [readerViewport, setReaderViewport] = useState<ReaderViewport>({
    width: 0,
    height: 0,
  });
  const [fontRevision, setFontRevision] = useState(0);
  const [annotations, setAnnotations] = useState<AnnotationRecord[]>([]);
  const [selection, setSelection] = useState<PendingSelection | null>(null);
  const [noteDraft, setNoteDraft] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState<SearchResult[]>([]);
  const [searching, setSearching] = useState(false);
  const [searchSubmitted, setSearchSubmitted] = useState(false);
  const [message, setMessage] = useState("");
  const [translationMenuOpen, setTranslationMenuOpen] = useState(false);
  const [pendingTranslationProvider, setPendingTranslationProvider] =
    useState<ExternalTranslationProvider | null>(null);
  const [translationBusy, setTranslationBusy] = useState(false);
  const [fontBusy, setFontBusy] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);
  const measurementRef = useRef<HTMLDivElement>(null);
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pendingPositionSave = useRef<PendingPositionSave | null>(null);
  const pendingSectionProgress = useRef<number | null>(null);
  const lastWheelPage = useRef(Number.NEGATIVE_INFINITY);
  const changeSectionRef = useRef<(direction: number) => void>(() => undefined);
  const initialPosition = useRef({
    section: document.lastSection,
    progress: document.sectionProgress,
  });
  const section = document.sections[sectionIndex] ?? document.sections[0]!;
  const sectionAnnotations = useMemo(
    () =>
      annotations.filter(
        (annotation) => annotation.locator.sectionId === section.id,
      ),
    [annotations, section.id],
  );
  const currentBookmark = sectionAnnotations.find(
    (annotation) => annotation.kind === "bookmark",
  );
  const totalWords = useMemo(
    () =>
      document.sections.reduce(
        (sectionTotal, documentSection) =>
          sectionTotal +
          documentSection.blocks.reduce(
            (blockTotal, block) => blockTotal + wordCount(block.text),
            0,
          ),
        0,
      ),
    [document.sections],
  );
  const pagesPerSpread =
    preferences.layout === "spread" &&
    (readerViewport.width === 0 || readerViewport.width > 980)
      ? 2
      : 1;
  const fontFamily =
    preferences.fontChoice === "custom" && preferences.customFont
      ? `"${preferences.customFont.family}", Georgia, serif`
      : readerFonts[
          preferences.fontChoice === "custom"
            ? "literary"
            : preferences.fontChoice
        ].family;
  const pageMeasurementKey = useMemo(
    () =>
      JSON.stringify({
        bookId: document.bookId,
        width: Math.round(readerViewport.width),
        height: Math.round(readerViewport.height),
        layout: preferences.layout,
        fontSize: preferences.fontSize,
        lineHeight: preferences.lineHeight,
        columnWidth: preferences.columnWidth,
        letterSpacing: preferences.letterSpacing,
        wordSpacing: preferences.wordSpacing,
        paragraphSpacing: preferences.paragraphSpacing,
        fontWeight: preferences.fontWeight,
        fontStyle: preferences.fontStyle,
        fontChoice: preferences.fontChoice,
        customFont: preferences.customFont?.family ?? "",
        textAlign: preferences.textAlign,
        bionicReading: preferences.bionicReading,
        fontRevision,
      }),
    [
      document.bookId,
      fontRevision,
      preferences,
      readerViewport.height,
      readerViewport.width,
    ],
  );
  const [pageMeasurement, setPageMeasurement] = useState<PageMeasurement>(() =>
    createPageMeasurement(
      pageMeasurementKey,
      document.sections.length,
      pagesPerSpread,
    ),
  );
  const measurementSection =
    document.sections[
      Math.min(pageMeasurement.index, document.sections.length - 1)
    ] ?? section;
  const pagePosition = calculatePagePosition(
    pageMeasurement.counts,
    sectionIndex,
    sectionProgress,
    preferences.layout === "spread" ? pagesPerSpread : 1,
  );
  useReadingSession({
    bookId: document.bookId,
    progress: displayProgress,
    words: Math.round(totalWords * displayProgress),
    pages: 0,
  });

  useEffect(() => {
    writeLocalValue(preferenceKey, JSON.stringify(preferences));
  }, [preferences]);

  useEffect(() => {
    void listAnnotations(document.bookId)
      .then(setAnnotations)
      .catch((reason) => setMessage(String(reason)));
  }, [document.bookId]);

  useEffect(() => {
    const container = scrollRef.current;
    if (!container) return;
    const restored =
      pendingSectionProgress.current ??
      (sectionIndex === initialPosition.current.section
        ? initialPosition.current.progress
        : 0);
    pendingSectionProgress.current = null;
    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        const spread = preferences.layout === "spread";
        const range = readerScrollRange(container, spread);
        setSectionProgress(restored);
        if (spread) {
          container.scrollLeft = spreadOffsetForProgress(container, restored);
          container.scrollTop = 0;
        } else {
          container.scrollTop = range * restored;
          container.scrollLeft = 0;
        }
      });
    });
  }, [preferences.layout, sectionIndex]);

  useEffect(() => {
    const container = scrollRef.current;
    if (!container) return;
    const updateViewport = () => {
      const width = container.clientWidth;
      const height = container.clientHeight;
      setReaderViewport((value) =>
        value.width === width && value.height === height
          ? value
          : { width, height },
      );
    };
    updateViewport();
    if (typeof ResizeObserver === "undefined") return;
    let previousWidth = container.clientWidth;
    const observer = new ResizeObserver(() => {
      const nextWidth = container.clientWidth;
      const spread = preferences.layout === "spread";
      const progress = readerSectionProgress(container, spread);
      updateViewport();
      if (
        !spread ||
        nextWidth <= 0 ||
        Math.abs(nextWidth - previousWidth) < 2
      ) {
        previousWidth = nextWidth;
        return;
      }
      previousWidth = nextWidth;
      requestAnimationFrame(() => {
        container.scrollLeft = spreadOffsetForProgress(container, progress);
      });
    });
    observer.observe(container);
    return () => observer.disconnect();
  }, [preferences.layout]);

  useEffect(() => {
    setPageMeasurement(
      createPageMeasurement(
        pageMeasurementKey,
        document.sections.length,
        pagesPerSpread,
      ),
    );
  }, [document.sections.length, pageMeasurementKey, pagesPerSpread]);

  useLayoutEffect(() => {
    if (
      pageMeasurement.key !== pageMeasurementKey ||
      pageMeasurement.index >= document.sections.length ||
      readerViewport.width <= 0 ||
      readerViewport.height <= 0
    ) {
      return;
    }
    const measurer = measurementRef.current;
    if (!measurer) return;
    const measured = measureRenderedPages(
      measurer,
      preferences.layout,
      pagesPerSpread,
    );
    setPageMeasurement((value) => {
      if (
        value.key !== pageMeasurementKey ||
        value.index >= document.sections.length
      ) {
        return value;
      }
      const counts = [...value.counts];
      counts[value.index] = measured;
      return {
        key: value.key,
        index: value.index + 1,
        counts,
      };
    });
  }, [
    document.sections.length,
    pageMeasurement.index,
    pageMeasurement.key,
    pageMeasurementKey,
    pagesPerSpread,
    preferences.layout,
    readerViewport.height,
    readerViewport.width,
  ]);

  useEffect(() => {
    const imported = preferences.customFont;
    if (!imported || preferences.fontChoice !== "custom") return;
    const face = new FontFace(
      imported.family,
      `url("${readerFontUrl(imported.path)}")`,
    );
    let active = true;
    void face
      .load()
      .then((loaded) => {
        if (active) {
          window.document.fonts.add(loaded);
          setFontRevision((value) => value + 1);
        }
      })
      .catch((reason) => {
        if (active) setMessage(String(reason));
      });
    return () => {
      active = false;
      window.document.fonts.delete(face);
    };
  }, [preferences.customFont, preferences.fontChoice]);

  useEffect(() => {
    const fontSet = window.document.fonts;
    if (!fontSet?.load) return;
    let active = true;
    const descriptor = `${preferences.fontStyle} ${preferences.fontWeight} 20px ${fontFamily}`;
    void fontSet
      .load(descriptor, "ApriReader Абзац Əla")
      .then(() => {
        if (active) setFontRevision((value) => value + 1);
      })
      .catch(() => undefined);
    return () => {
      active = false;
    };
  }, [fontFamily, preferences.fontStyle, preferences.fontWeight]);

  useEffect(
    () => () => {
      if (saveTimer.current) clearTimeout(saveTimer.current);
      const pending = pendingPositionSave.current;
      if (pending) {
        void saveReadingPosition(
          pending.bookId,
          pending.section,
          pending.sectionProgress,
          pending.progress,
        );
      }
    },
    [],
  );

  const cancelPendingPositionSave = () => {
    if (saveTimer.current) clearTimeout(saveTimer.current);
    saveTimer.current = null;
    pendingPositionSave.current = null;
  };

  const savePosition = (sectionProgress: number) => {
    setSectionProgress(sectionProgress);
    const progress =
      document.sections.length === 0
        ? 0
        : Math.min(
            1,
            (sectionIndex + sectionProgress) / document.sections.length,
          );
    setDisplayProgress(progress);
    onProgress(progress);
    cancelPendingPositionSave();
    pendingPositionSave.current = {
      bookId: document.bookId,
      section: sectionIndex,
      sectionProgress,
      progress,
    };
    saveTimer.current = setTimeout(() => {
      const pending = pendingPositionSave.current;
      saveTimer.current = null;
      pendingPositionSave.current = null;
      if (!pending) return;
      void saveReadingPosition(
        pending.bookId,
        pending.section,
        pending.sectionProgress,
        pending.progress,
      );
    }, 350);
  };

  const selectSection = (index: number, blockIndex?: number) => {
    cancelPendingPositionSave();
    setSectionIndex(index);
    setPanel(null);
    setSelection(null);
    setSectionProgress(0);
    const progress = index / document.sections.length;
    setDisplayProgress(progress);
    onProgress(progress);
    void saveReadingPosition(document.bookId, index, 0, progress);
    if (blockIndex !== undefined) {
      requestAnimationFrame(() =>
        requestAnimationFrame(() => {
          scrollRef.current
            ?.querySelector<HTMLElement>(`[data-reader-block="${blockIndex}"]`)
            ?.scrollIntoView({ block: "center" });
        }),
      );
    }
  };

  const navigateTo = (sectionId: string, blockIndex: number) => {
    const index = document.sections.findIndex((item) => item.id === sectionId);
    if (index >= 0) selectSection(index, blockIndex);
  };

  const changeLayout = (layout: ReaderLayout) => {
    const container = scrollRef.current;
    if (container) {
      pendingSectionProgress.current = readerSectionProgress(
        container,
        preferences.layout === "spread",
      );
    }
    setSelection(null);
    setPreferences((value) => ({ ...value, layout }));
  };

  const changeSection = (direction: number) => {
    selectSection(
      Math.min(
        document.sections.length - 1,
        Math.max(0, sectionIndex + direction),
      ),
    );
  };
  changeSectionRef.current = changeSection;

  useEffect(() => {
    const container = scrollRef.current;
    const spread = preferences.layout === "spread";
    if (!container || (!spread && !preferences.pageWheel)) return;
    const handlePageWheel = (event: WheelEvent) => {
      if (event.ctrlKey || Math.abs(event.deltaY) < 12) return;
      event.preventDefault();
      const now = performance.now();
      if (now - lastWheelPage.current < 260) return;
      lastWheelPage.current = now;
      const direction = Math.sign(event.deltaY);
      const position = spread ? container.scrollLeft : container.scrollTop;
      const maximum = readerScrollRange(container, spread);
      const atStart = position <= 2;
      const atEnd = position >= maximum - 2;
      if (
        direction > 0 &&
        atEnd &&
        sectionIndex < document.sections.length - 1
      ) {
        pendingSectionProgress.current = 0;
        changeSectionRef.current(1);
        return;
      }
      if (direction < 0 && atStart && sectionIndex > 0) {
        pendingSectionProgress.current = 1;
        changeSectionRef.current(-1);
        return;
      }
      if (spread) {
        const spreadWidth = Math.max(1, container.clientWidth);
        const targetSpread = Math.round(position / spreadWidth) + direction;
        container.scrollTo({
          left: Math.min(maximum, Math.max(0, targetSpread * spreadWidth)),
          behavior: "smooth",
        });
      } else {
        container.scrollBy({
          top: direction * Math.max(240, container.clientHeight * 0.86),
          behavior: "smooth",
        });
      }
    };
    container.addEventListener("wheel", handlePageWheel, { passive: false });
    return () => container.removeEventListener("wheel", handlePageWheel);
  }, [
    document.sections.length,
    preferences.layout,
    preferences.pageWheel,
    sectionIndex,
  ]);

  const handleScroll = () => {
    const container = scrollRef.current;
    if (!container) return;
    savePosition(
      readerSectionProgress(container, preferences.layout === "spread"),
    );
  };

  const captureSelection = (event: MouseEvent<HTMLElement>) => {
    const selected = window.getSelection();
    if (!selected || selected.rangeCount === 0 || selected.isCollapsed) {
      setSelection(null);
      setNoteDraft(null);
      return;
    }
    const range = selected.getRangeAt(0);
    const startBlock = closestReaderBlock(range.startContainer);
    const endBlock = closestReaderBlock(range.endContainer);
    if (!startBlock || startBlock !== endBlock) {
      setMessage(t("selectionSingleBlock"));
      return;
    }
    const text = selected.toString().trim();
    if (!text || text.length > 4_000) return;
    const prefix = window.document.createRange();
    prefix.selectNodeContents(startBlock);
    prefix.setEnd(range.startContainer, range.startOffset);
    const startOffset = prefix.toString().length;
    setSelection({
      blockIndex: Number(startBlock.dataset.readerBlock),
      startOffset,
      endOffset: startOffset + selected.toString().length,
      text,
    });
    setTranslationMenuOpen(false);
    setPendingTranslationProvider(null);
    setNoteDraft(null);
    event.stopPropagation();
  };

  const openSelectedTranslation = async (
    provider: ExternalTranslationProvider,
  ) => {
    if (!selection) return;
    setTranslationBusy(true);
    try {
      await openExternalTranslation(provider, selection.text);
      setMessage("");
      setTranslationMenuOpen(false);
      setPendingTranslationProvider(null);
    } catch (reason) {
      console.error("Could not open external translator", reason);
      setMessage(t("externalTranslationOpenError"));
    } finally {
      setTranslationBusy(false);
    }
  };

  const chooseTranslationProvider = (provider: ExternalTranslationProvider) => {
    if (!selection) return;
    if (
      Array.from(selection.text.trim()).length >
      externalTranslationMaxCharacters
    ) {
      setMessage(
        t("externalTranslationTooLong").replace(
          "{count}",
          String(externalTranslationMaxCharacters),
        ),
      );
      setTranslationMenuOpen(false);
      return;
    }
    if (readLocalValue(externalTranslationConsentKey) === "accepted") {
      void openSelectedTranslation(provider);
      return;
    }
    setPendingTranslationProvider(provider);
    setTranslationMenuOpen(false);
  };

  const confirmExternalTranslation = () => {
    if (!pendingTranslationProvider) return;
    writeLocalValue(externalTranslationConsentKey, "accepted");
    void openSelectedTranslation(pendingTranslationProvider);
  };

  const addSelectionAnnotation = async (
    kind: Exclude<AnnotationKind, "bookmark">,
    note = "",
  ) => {
    if (!selection) return;
    try {
      const created = await createAnnotation({
        bookId: document.bookId,
        kind,
        sectionId: section.id,
        blockIndex: selection.blockIndex,
        startOffset: selection.startOffset,
        endOffset: selection.endOffset,
        selectedText: selection.text,
        note,
      });
      setAnnotations((items) => [...items, created]);
      if (kind === "quote") {
        let copied = false;
        try {
          if (navigator.clipboard) {
            await navigator.clipboard.writeText(selection.text);
            copied = true;
          }
        } catch {
          // The quote is still safely stored when clipboard access is denied.
        }
        setMessage(t(copied ? "quoteCopied" : "quoteSaved"));
      } else {
        setMessage(t("annotationSaved"));
      }
      window.getSelection()?.removeAllRanges();
      setSelection(null);
      setNoteDraft(null);
    } catch (reason) {
      setMessage(String(reason));
    }
  };

  const toggleBookmark = async () => {
    try {
      if (currentBookmark) {
        await deleteAnnotation(currentBookmark.id);
        setAnnotations((items) =>
          items.filter((annotation) => annotation.id !== currentBookmark.id),
        );
        setMessage(t("bookmarkRemoved"));
      } else {
        const created = await createAnnotation({
          bookId: document.bookId,
          kind: "bookmark",
          sectionId: section.id,
          blockIndex: 0,
          startOffset: 0,
          endOffset: 0,
          selectedText: "",
          note: "",
        });
        setAnnotations((items) => [...items, created]);
        setMessage(t("bookmarkSaved"));
      }
    } catch (reason) {
      setMessage(String(reason));
    }
  };

  const removeAnnotation = async (annotationId: number) => {
    try {
      await deleteAnnotation(annotationId);
      setAnnotations((items) =>
        items.filter((annotation) => annotation.id !== annotationId),
      );
    } catch (reason) {
      setMessage(String(reason));
    }
  };

  const saveEditedNote = async (annotation: AnnotationRecord, note: string) => {
    try {
      const updated = await updateAnnotationNote(annotation.id, note);
      setAnnotations((items) =>
        items.map((item) => (item.id === updated.id ? updated : item)),
      );
      setMessage(t("noteUpdated"));
    } catch (reason) {
      setMessage(String(reason));
    }
  };

  const runSearch = async (event: FormEvent) => {
    event.preventDefault();
    if (!searchQuery.trim()) {
      setSearchResults([]);
      setSearchSubmitted(false);
      return;
    }
    setSearching(true);
    setSearchSubmitted(true);
    try {
      setSearchResults(await searchBook(document.bookId, searchQuery));
      setMessage("");
    } catch (reason) {
      setMessage(String(reason));
    } finally {
      setSearching(false);
    }
  };

  const exportNotes = async () => {
    try {
      if (await chooseAndExportAnnotations(document.bookId, document.title)) {
        setMessage(t("annotationsExported"));
      }
    } catch (reason) {
      setMessage(String(reason));
    }
  };

  const importFont = async () => {
    setFontBusy(true);
    try {
      const imported = await chooseAndImportReaderFont();
      if (imported) {
        setPreferences((value) => ({
          ...value,
          customFont: imported,
          fontChoice: "custom",
          fontStyle: "normal",
          fontWeight: 400,
        }));
        setMessage(t("fontImported"));
      }
    } catch (reason) {
      setMessage(String(reason));
    } finally {
      setFontBusy(false);
    }
  };

  const availableFontWeights = fontWeightsForChoice(preferences.fontChoice);
  const style = {
    "--reader-font-size": `${preferences.fontSize}px`,
    "--reader-line-height": String(preferences.lineHeight),
    "--reader-column-width": `${preferences.columnWidth}px`,
    "--reader-font-family": fontFamily,
    "--reader-letter-spacing": `${preferences.letterSpacing}em`,
    "--reader-word-spacing": `${preferences.wordSpacing}em`,
    "--reader-paragraph-spacing": `${preferences.paragraphSpacing}em`,
    "--reader-font-weight": String(preferences.fontWeight),
    "--reader-font-style": preferences.fontStyle,
    "--reader-text-align": preferences.textAlign,
  } as CSSProperties;

  return (
    <div
      className={`reader-screen theme-${preferences.theme} ${preferences.bionicReading ? "bionic-reading" : ""}`}
      lang={language}
      style={style}
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          if (pendingTranslationProvider) setPendingTranslationProvider(null);
          else if (translationMenuOpen) setTranslationMenuOpen(false);
          else if (selection) setSelection(null);
          else if (panel) setPanel(null);
          else onClose();
        }
      }}
    >
      <header className="reader-toolbar">
        <button
          type="button"
          className="reader-icon-button"
          aria-label={t("readerBack")}
          autoFocus
          onClick={onClose}
        >
          ←
        </button>
        <div className="reader-book-title">
          <strong>{document.title}</strong>
          <span>{document.author}</span>
        </div>
        <div className="reader-actions">
          <button
            type="button"
            aria-label={t("previousSection")}
            disabled={sectionIndex === 0}
            onClick={() => changeSection(-1)}
          >
            ←
          </button>
          <button
            type="button"
            aria-label={t("nextSection")}
            disabled={sectionIndex === document.sections.length - 1}
            onClick={() => changeSection(1)}
          >
            →
          </button>
          <ReaderAction
            active={panel === "contents"}
            icon="collections"
            label={t("tableOfContents")}
            onClick={() =>
              setPanel((value) => (value === "contents" ? null : "contents"))
            }
          />
          <ReaderAction
            active={panel === "search"}
            icon="search"
            label={t("searchInBook")}
            onClick={() =>
              setPanel((value) => (value === "search" ? null : "search"))
            }
          />
          <ReaderAction
            active={Boolean(currentBookmark)}
            icon="bookmark"
            label={t("bookmark")}
            onClick={() => void toggleBookmark()}
          />
          <ReaderAction
            active={panel === "annotations"}
            icon="notes"
            label={t("annotations")}
            onClick={() =>
              setPanel((value) =>
                value === "annotations" ? null : "annotations",
              )
            }
          />
          <ReaderAction
            active={panel === "settings"}
            icon="settings"
            label={t("typography")}
            onClick={() =>
              setPanel((value) => (value === "settings" ? null : "settings"))
            }
          />
        </div>
      </header>

      <div className="reader-progress-track" aria-hidden="true">
        <span style={{ width: `${displayProgress * 100}%` }} />
      </div>

      <ReaderSidePanel
        open={panel === "contents"}
        side="left"
        title={t("tableOfContents")}
        closeLabel={t("closeContents")}
        onClose={() => setPanel(null)}
      >
        <nav className="reader-location-list">
          {document.sections.map((item, index) => (
            <button
              type="button"
              className={index === sectionIndex ? "active" : ""}
              key={item.id}
              onClick={() => selectSection(index)}
              aria-current={index === sectionIndex ? "location" : undefined}
            >
              <span>{String(index + 1).padStart(2, "0")}</span>
              {item.title}
            </button>
          ))}
        </nav>
      </ReaderSidePanel>

      <ReaderSidePanel
        open={panel === "search"}
        side="right"
        title={t("searchInBook")}
        closeLabel={t("closeSearch")}
        onClose={() => setPanel(null)}
      >
        <form
          className="reader-search-form"
          onSubmit={(event) => void runSearch(event)}
        >
          <input
            type="search"
            value={searchQuery}
            onChange={(event) => {
              setSearchQuery(event.target.value);
              setSearchSubmitted(false);
            }}
            placeholder={t("searchBookPlaceholder")}
            aria-label={t("searchInBook")}
          />
          <button type="submit">{t("find")}</button>
        </form>
        <p className="reader-panel-meta" role="status">
          {searching
            ? t("searching")
            : searchSubmitted && searchResults.length === 0
              ? t("nothingFound")
              : `${searchResults.length} ${t("searchResults")}`}
        </p>
        <div className="reader-result-list">
          {searchResults.map((result, index) => (
            <button
              type="button"
              key={`${result.sectionId}-${result.blockIndex}-${index}`}
              onClick={() => navigateTo(result.sectionId, result.blockIndex)}
            >
              <strong>{result.sectionTitle}</strong>
              <span>{result.excerpt}</span>
            </button>
          ))}
        </div>
      </ReaderSidePanel>

      <ReaderSidePanel
        open={panel === "annotations"}
        side="right"
        title={t("annotations")}
        closeLabel={t("closeAnnotations")}
        onClose={() => setPanel(null)}
      >
        <button
          className="reader-export-button"
          type="button"
          disabled={annotations.length === 0}
          onClick={() => void exportNotes()}
        >
          {t("exportAnnotations")}
        </button>
        {annotations.length === 0 ? (
          <p className="reader-empty-panel">{t("noAnnotations")}</p>
        ) : (
          <div className="annotation-list">
            {annotations.map((annotation) => (
              <AnnotationCard
                annotation={annotation}
                key={annotation.id}
                t={t}
                onNavigate={() =>
                  navigateTo(
                    annotation.locator.sectionId,
                    annotation.locator.blockIndex,
                  )
                }
                onDelete={() => void removeAnnotation(annotation.id)}
                onSaveNote={(note) => void saveEditedNote(annotation, note)}
              />
            ))}
          </div>
        )}
      </ReaderSidePanel>

      <ReaderSidePanel
        open={panel === "settings"}
        side="right"
        title={t("typography")}
        closeLabel={t("closeSettings")}
        onClose={() => setPanel(null)}
      >
        <fieldset className="reader-choice-group reader-layout-choice">
          <legend>{t("readingLayout")}</legend>
          <button
            type="button"
            className={preferences.layout === "continuous" ? "active" : ""}
            aria-pressed={preferences.layout === "continuous"}
            onClick={() => changeLayout("continuous")}
          >
            <strong>{t("layoutContinuous")}</strong>
            <small>{t("layoutContinuousHint")}</small>
          </button>
          <button
            type="button"
            className={preferences.layout === "spread" ? "active" : ""}
            aria-pressed={preferences.layout === "spread"}
            onClick={() => changeLayout("spread")}
          >
            <strong>{t("layoutSpread")}</strong>
            <small>{t("layoutSpreadHint")}</small>
          </button>
        </fieldset>
        <label className="reader-select">
          <span>{t("readerFont")}</span>
          <select
            value={preferences.fontChoice}
            onChange={(event) => {
              const fontChoice = event.target.value as ReaderFontChoice;
              setPreferences((value) => ({
                ...value,
                fontChoice,
                fontWeight: closestFontWeight(
                  value.fontWeight,
                  fontWeightsForChoice(fontChoice),
                ),
              }));
            }}
          >
            <option value="literary">{t("fontLiterary")}</option>
            <option value="book">{t("fontBook")}</option>
            <option value="classic">{t("fontClassic")}</option>
            <option value="clear">{t("fontClear")}</option>
            <option value="literata">Literata</option>
            <option value="lora">Lora</option>
            <option value="merriweather">Merriweather</option>
            <option value="sourceSerif">Source Serif 4</option>
            <option value="charis">Charis SIL</option>
            <option value="ibmPlex">IBM Plex Serif</option>
            {preferences.customFont && (
              <option value="custom">{preferences.customFont.name}</option>
            )}
          </select>
        </label>
        <div className="reader-font-variants">
          <label className="reader-select">
            <span>{t("fontStyle")}</span>
            <select
              value={preferences.fontStyle}
              onChange={(event) =>
                setPreferences((value) => ({
                  ...value,
                  fontStyle: event.target.value as ReaderFontStyle,
                }))
              }
            >
              <option value="normal">{t("fontStyleNormal")}</option>
              <option value="italic">{t("fontStyleItalic")}</option>
            </select>
          </label>
          <label className="reader-select">
            <span>{t("fontWeight")}</span>
            <select
              value={preferences.fontWeight}
              onChange={(event) =>
                setPreferences((value) => ({
                  ...value,
                  fontWeight: Number(event.target.value),
                }))
              }
            >
              {availableFontWeights.map((weight) => (
                <option value={weight} key={weight}>
                  {fontWeightLabel(weight, t)}
                </option>
              ))}
            </select>
          </label>
        </div>
        <p className="reader-font-preview">{t("fontPreview")}</p>
        <button
          className="reader-import-font"
          type="button"
          disabled={fontBusy}
          onClick={() => void importFont()}
        >
          {fontBusy ? t("fontImporting") : t("importFont")}
        </button>
        <p className="reader-setting-hint">{t("fontImportHint")}</p>
        <ReaderRange
          label={t("fontSize")}
          value={preferences.fontSize}
          min={14}
          max={36}
          step={1}
          onChange={(fontSize) =>
            setPreferences((value) => ({ ...value, fontSize }))
          }
        />
        <ReaderRange
          label={t("lineHeight")}
          value={preferences.lineHeight}
          min={1.2}
          max={2.4}
          step={0.05}
          onChange={(lineHeight) =>
            setPreferences((value) => ({ ...value, lineHeight }))
          }
        />
        <ReaderRange
          label={t("columnWidth")}
          value={preferences.columnWidth}
          min={480}
          max={1000}
          step={20}
          onChange={(columnWidth) =>
            setPreferences((value) => ({ ...value, columnWidth }))
          }
        />
        <ReaderRange
          label={t("letterSpacing")}
          value={preferences.letterSpacing}
          min={-0.02}
          max={0.12}
          step={0.01}
          onChange={(letterSpacing) =>
            setPreferences((value) => ({ ...value, letterSpacing }))
          }
        />
        <ReaderRange
          label={t("wordSpacing")}
          value={preferences.wordSpacing}
          min={0}
          max={0.3}
          step={0.02}
          onChange={(wordSpacing) =>
            setPreferences((value) => ({ ...value, wordSpacing }))
          }
        />
        <ReaderRange
          label={t("paragraphSpacing")}
          value={preferences.paragraphSpacing}
          min={0.5}
          max={2}
          step={0.05}
          onChange={(paragraphSpacing) =>
            setPreferences((value) => ({ ...value, paragraphSpacing }))
          }
        />
        <fieldset className="reader-choice-group">
          <legend>{t("textAlignment")}</legend>
          <button
            type="button"
            className={preferences.textAlign === "left" ? "active" : ""}
            aria-pressed={preferences.textAlign === "left"}
            onClick={() =>
              setPreferences((value) => ({ ...value, textAlign: "left" }))
            }
          >
            {t("alignLeft")}
          </button>
          <button
            type="button"
            className={preferences.textAlign === "justify" ? "active" : ""}
            aria-pressed={preferences.textAlign === "justify"}
            onClick={() =>
              setPreferences((value) => ({ ...value, textAlign: "justify" }))
            }
          >
            {t("alignJustify")}
          </button>
        </fieldset>
        <label className="reader-toggle">
          <input
            type="checkbox"
            checked={preferences.bionicReading}
            onChange={(event) =>
              setPreferences((value) => ({
                ...value,
                bionicReading: event.target.checked,
              }))
            }
          />
          <span>
            <strong>{t("bionicReading")}</strong>
            <small>{t("bionicReadingHint")}</small>
          </span>
        </label>
        <label className="reader-toggle">
          <input
            type="checkbox"
            checked={preferences.pageWheel}
            onChange={(event) =>
              setPreferences((value) => ({
                ...value,
                pageWheel: event.target.checked,
              }))
            }
          />
          <span>
            <strong>{t("pageWheel")}</strong>
            <small>{t("pageWheelHint")}</small>
          </span>
        </label>
        <fieldset className="theme-choices">
          <legend>{t("readerTheme")}</legend>
          {(
            [
              ["paper", t("themePaper")],
              ["sepia", t("themeSepia")],
              ["night", t("themeNight")],
            ] as const
          ).map(([theme, label]) => (
            <button
              type="button"
              className={preferences.theme === theme ? "active" : ""}
              aria-pressed={preferences.theme === theme}
              key={theme}
              onClick={() => setPreferences((value) => ({ ...value, theme }))}
            >
              <span className={`theme-swatch ${theme}`} />
              {label}
            </button>
          ))}
        </fieldset>
      </ReaderSidePanel>

      <p
        className="sr-only"
        role="status"
        aria-live={screenReaderSupport ? "polite" : "off"}
        aria-atomic="true"
      >
        {screenReaderSupport
          ? `${t("chapterAnnouncement")}: ${section.title}`
          : ""}
      </p>

      <main
        className={`reader-scroll layout-${preferences.layout}`}
        ref={scrollRef}
        onScroll={handleScroll}
        tabIndex={0}
      >
        <article
          className={`reader-document reader-document-${preferences.layout}`}
        >
          <p className="reader-kicker">
            {sectionIndex + 1} / {document.sections.length}
          </p>
          <h1>{section.title}</h1>
          <div className="reader-blocks" onMouseUp={captureSelection}>
            {section.blocks.map((block, index) => (
              <ReaderBlock
                block={block}
                annotations={sectionAnnotations.filter(
                  (annotation) =>
                    annotation.locator.blockIndex === index &&
                    annotation.kind !== "bookmark",
                )}
                bionic={preferences.bionicReading}
                blockIndex={index}
                key={`${section.id}-${index}-${block.kind}`}
              />
            ))}
          </div>
          <footer className="reader-section-nav">
            <button
              type="button"
              disabled={sectionIndex === 0}
              onClick={() => changeSection(-1)}
            >
              ← {t("previousSection")}
            </button>
            <button
              type="button"
              disabled={sectionIndex === document.sections.length - 1}
              onClick={() => changeSection(1)}
            >
              {t("nextSection")} →
            </button>
          </footer>
        </article>
      </main>

      <p
        className="reader-page-status"
        role="status"
        aria-live={screenReaderSupport ? "polite" : "off"}
        aria-atomic="true"
      >
        {preferences.layout === "spread"
          ? `${t("pages")} ${pagePosition.start}–${pagePosition.end} ${t("pageOf")} ${pagePosition.total}`
          : `${t("page")} ${pagePosition.start} ${t("pageOf")} ${pagePosition.total}`}
      </p>

      <div
        className={`reader-page-measurer reader-scroll layout-${preferences.layout}`}
        ref={measurementRef}
        aria-hidden="true"
        style={{
          width: readerViewport.width,
          height: readerViewport.height,
        }}
      >
        <article
          className={`reader-document reader-document-${preferences.layout}`}
        >
          <p className="reader-kicker">
            {Math.min(pageMeasurement.index + 1, document.sections.length)} /{" "}
            {document.sections.length}
          </p>
          <h1>{measurementSection.title}</h1>
          <div className="reader-blocks">
            {measurementSection.blocks.map((block, index) => (
              <ReaderBlock
                block={block}
                annotations={[]}
                bionic={preferences.bionicReading}
                blockIndex={index}
                key={`measure-${measurementSection.id}-${index}-${block.kind}`}
              />
            ))}
          </div>
          <footer className="reader-section-nav">
            <button type="button">{t("previousSection")}</button>
            <button type="button">{t("nextSection")}</button>
          </footer>
        </article>
      </div>

      {selection && (
        <div
          className="selection-toolbar"
          role="dialog"
          aria-label={t("selectionActions")}
        >
          <p>“{selection.text}”</p>
          {noteDraft === null ? (
            <div>
              <button
                type="button"
                onClick={() => void addSelectionAnnotation("highlight")}
              >
                {t("highlight")}
              </button>
              <button type="button" onClick={() => setNoteDraft("")}>
                {t("addNote")}
              </button>
              <button
                type="button"
                onClick={() => void addSelectionAnnotation("quote")}
              >
                {t("copyQuote")}
              </button>
              <button
                type="button"
                aria-haspopup="menu"
                aria-expanded={translationMenuOpen}
                onClick={() => {
                  setPendingTranslationProvider(null);
                  setTranslationMenuOpen((open) => !open);
                }}
              >
                {t("translate")}
              </button>
              <button
                type="button"
                onClick={() => {
                  setTranslationMenuOpen(false);
                  setPendingTranslationProvider(null);
                  setSelection(null);
                }}
              >
                {t("cancel")}
              </button>
            </div>
          ) : (
            <form
              onSubmit={(event) => {
                event.preventDefault();
                void addSelectionAnnotation("note", noteDraft);
              }}
            >
              <textarea
                autoFocus
                value={noteDraft}
                maxLength={20_000}
                placeholder={t("notePlaceholder")}
                onChange={(event) => setNoteDraft(event.target.value)}
              />
              <button type="submit" disabled={!noteDraft.trim()}>
                {t("saveNote")}
              </button>
              <button type="button" onClick={() => setNoteDraft(null)}>
                {t("cancel")}
              </button>
            </form>
          )}
          {translationMenuOpen && (
            <div className="translation-provider-menu" role="menu">
              <button
                type="button"
                role="menuitem"
                onClick={() => chooseTranslationProvider("google")}
              >
                {t("googleTranslate")}
              </button>
              <button
                type="button"
                role="menuitem"
                onClick={() => chooseTranslationProvider("yandex")}
              >
                {t("yandexTranslate")}
              </button>
            </div>
          )}
          {pendingTranslationProvider && (
            <section
              className="external-translation-consent"
              role="group"
              aria-live="assertive"
              aria-labelledby="external-translation-title"
            >
              <div>
                <strong id="external-translation-title">
                  {t("externalTranslationDisclosureTitle")}
                </strong>
                <p>
                  {t("externalTranslationDisclosure").replace(
                    "{provider}",
                    t(
                      pendingTranslationProvider === "google"
                        ? "googleTranslate"
                        : "yandexTranslate",
                    ),
                  )}
                </p>
              </div>
              <div className="external-translation-actions">
                <button
                  type="button"
                  autoFocus
                  disabled={translationBusy}
                  onClick={confirmExternalTranslation}
                >
                  {t("continue")}
                </button>
                <button
                  type="button"
                  disabled={translationBusy}
                  onClick={() => setPendingTranslationProvider(null)}
                >
                  {t("cancel")}
                </button>
              </div>
            </section>
          )}
        </div>
      )}

      {message && (
        <p className="reader-toast" role="status">
          {message}
        </p>
      )}
    </div>
  );
}

function ReaderAction({
  active,
  icon,
  label,
  onClick,
}: {
  active: boolean;
  icon: "collections" | "search" | "bookmark" | "notes" | "settings";
  label: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      className={active ? "active" : ""}
      aria-label={label}
      onClick={onClick}
    >
      <Icon name={icon} />
      <span>{label}</span>
    </button>
  );
}

function ReaderSidePanel({
  open,
  side,
  title,
  closeLabel,
  onClose,
  children,
}: {
  open: boolean;
  side: "left" | "right";
  title: string;
  closeLabel: string;
  onClose: () => void;
  children: ReactNode;
}) {
  return (
    <aside
      className={`reader-side-panel ${side} ${open ? "open" : ""}`}
      aria-hidden={!open}
    >
      <div className="reader-panel-heading">
        <h2>{title}</h2>
        <button type="button" aria-label={closeLabel} onClick={onClose}>
          ×
        </button>
      </div>
      {children}
    </aside>
  );
}

function AnnotationCard({
  annotation,
  t,
  onNavigate,
  onDelete,
  onSaveNote,
}: {
  annotation: AnnotationRecord;
  t: Translator;
  onNavigate: () => void;
  onDelete: () => void;
  onSaveNote: (note: string) => void;
}) {
  const [note, setNote] = useState(annotation.note);
  return (
    <article className={`annotation-card kind-${annotation.kind}`}>
      <button
        className="annotation-location"
        type="button"
        onClick={onNavigate}
      >
        <strong>{t(annotationKindKey(annotation.kind))}</strong>
        <span>
          {annotation.locator.sectionId} · {annotation.locator.blockIndex + 1}
        </span>
      </button>
      {annotation.selectedText && (
        <blockquote>{annotation.selectedText}</blockquote>
      )}
      {annotation.kind === "note" && (
        <textarea
          value={note}
          aria-label={t("editNote")}
          maxLength={20_000}
          onChange={(event) => setNote(event.target.value)}
          onBlur={() => {
            if (note !== annotation.note) onSaveNote(note);
          }}
        />
      )}
      <button className="annotation-delete" type="button" onClick={onDelete}>
        {t("deleteAnnotation")}
      </button>
    </article>
  );
}

function ReaderBlock({
  block,
  blockIndex,
  annotations,
  bionic,
}: {
  block: DocumentBlock;
  blockIndex: number;
  annotations: AnnotationRecord[];
  bionic: boolean;
}) {
  const text = (
    <AnnotatedText
      text={block.text}
      annotations={annotations}
      bionic={bionic}
    />
  );
  const data = { "data-reader-block": blockIndex };
  switch (block.kind) {
    case "heading":
      return <h2 {...data}>{text}</h2>;
    case "quote":
      return <blockquote {...data}>{text}</blockquote>;
    case "listItem":
      return (
        <ul>
          <li {...data}>{text}</li>
        </ul>
      );
    case "code":
      return <pre {...data}>{text}</pre>;
    case "divider":
      return <hr {...data} />;
    default:
      return <p {...data}>{text}</p>;
  }
}

function AnnotatedText({
  text,
  annotations,
  bionic,
}: {
  text: string;
  annotations: AnnotationRecord[];
  bionic: boolean;
}) {
  const ranges = annotations
    .map((annotation) => ({
      annotation,
      start: Math.min(text.length, annotation.locator.startOffset),
      end: Math.min(text.length, annotation.locator.endOffset),
    }))
    .filter((range) => range.end > range.start)
    .sort((left, right) => left.start - right.start);
  const output: ReactNode[] = [];
  let cursor = 0;
  for (const range of ranges) {
    if (range.start < cursor) continue;
    output.push(
      <span key={`plain-${cursor}`}>
        {renderFocusText(text.slice(cursor, range.start), bionic)}
      </span>,
    );
    output.push(
      <mark
        className={`reader-annotation kind-${range.annotation.kind}`}
        key={range.annotation.id}
        title={range.annotation.note || range.annotation.kind}
      >
        {renderFocusText(text.slice(range.start, range.end), bionic)}
      </mark>,
    );
    cursor = range.end;
  }
  output.push(
    <span key={`plain-${cursor}-end`}>
      {renderFocusText(text.slice(cursor), bionic)}
    </span>,
  );
  return output;
}

function renderFocusText(text: string, enabled: boolean): ReactNode {
  if (!enabled || !text) return text;
  return text.split(/(\p{L}[\p{L}\p{M}\p{N}'’-]*)/gu).map((part, index) => {
    if (!/^\p{L}/u.test(part)) return part;
    const characters = Array.from(part);
    const focusLength = Math.max(1, Math.ceil(characters.length * 0.45));
    return (
      <span className="bionic-word" key={`${index}-${part}`}>
        <strong>{characters.slice(0, focusLength).join("")}</strong>
        {characters.slice(focusLength).join("")}
      </span>
    );
  });
}

function ReaderRange({
  label,
  value,
  min,
  max,
  step,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  onChange: (value: number) => void;
}) {
  return (
    <label className="reader-range">
      <span>
        {label}
        <output>{value}</output>
      </span>
      <input
        type="range"
        aria-label={label}
        value={value}
        min={min}
        max={max}
        step={step}
        onChange={(event) => onChange(Number(event.target.value))}
      />
    </label>
  );
}

function closestReaderBlock(node: Node) {
  const element = node instanceof Element ? node : node.parentElement;
  return element?.closest<HTMLElement>("[data-reader-block]") ?? null;
}

function annotationKindKey(kind: AnnotationKind): TranslationKey {
  return {
    bookmark: "bookmark",
    highlight: "highlight",
    note: "note",
    quote: "quote",
  }[kind] as TranslationKey;
}

function fontWeightLabel(weight: number, t: Translator) {
  const keys: Record<number, TranslationKey> = {
    100: "fontWeightThin",
    200: "fontWeightExtraLight",
    300: "fontWeightLight",
    400: "fontWeightRegular",
    500: "fontWeightMedium",
    600: "fontWeightSemiBold",
    700: "fontWeightBold",
    800: "fontWeightExtraBold",
    900: "fontWeightBlack",
  };
  return t(keys[weight] ?? "fontWeightRegular");
}

function readPreferences(): ReaderPreferences {
  try {
    const value = JSON.parse(
      readLocalValue(preferenceKey) ?? "",
    ) as Partial<ReaderPreferences>;
    const customFont = validCustomFont(value.customFont)
      ? value.customFont
      : null;
    const requestedFont = [
      "literary",
      "book",
      "classic",
      "clear",
      "literata",
      "lora",
      "merriweather",
      "sourceSerif",
      "charis",
      "ibmPlex",
      "custom",
    ].includes(value.fontChoice ?? "")
      ? (value.fontChoice as ReaderFontChoice)
      : defaultPreferences.fontChoice;
    const fontChoice =
      requestedFont === "custom" && !customFont ? "literary" : requestedFont;
    const requestedWeight = clamp(
      value.fontWeight,
      100,
      900,
      defaultPreferences.fontWeight,
    );
    return {
      fontSize: clamp(value.fontSize, 14, 36, defaultPreferences.fontSize),
      lineHeight: clamp(
        value.lineHeight,
        1.2,
        2.4,
        defaultPreferences.lineHeight,
      ),
      columnWidth: clamp(
        value.columnWidth,
        480,
        1000,
        defaultPreferences.columnWidth,
      ),
      letterSpacing: clamp(
        value.letterSpacing,
        -0.02,
        0.12,
        defaultPreferences.letterSpacing,
      ),
      wordSpacing: clamp(
        value.wordSpacing,
        0,
        0.3,
        defaultPreferences.wordSpacing,
      ),
      paragraphSpacing: clamp(
        value.paragraphSpacing,
        0.5,
        2,
        defaultPreferences.paragraphSpacing,
      ),
      fontWeight: closestFontWeight(
        requestedWeight,
        fontWeightsForChoice(fontChoice),
      ),
      fontStyle: value.fontStyle === "italic" ? "italic" : "normal",
      fontChoice,
      customFont,
      textAlign: value.textAlign === "justify" ? "justify" : "left",
      bionicReading: value.bionicReading === true,
      pageWheel: value.pageWheel !== false,
      layout: value.layout === "spread" ? "spread" : "continuous",
      theme: ["paper", "sepia", "night"].includes(value.theme ?? "")
        ? (value.theme as ReaderTheme)
        : defaultPreferences.theme,
    };
  } catch {
    return defaultPreferences;
  }
}

function readerScrollRange(container: HTMLElement, spread: boolean) {
  return Math.max(
    0,
    spread
      ? container.scrollWidth - container.clientWidth
      : container.scrollHeight - container.clientHeight,
  );
}

function createPageMeasurement(
  key: string,
  sectionCount: number,
  minimumPages: number,
): PageMeasurement {
  return {
    key,
    index: 0,
    counts: Array.from({ length: sectionCount }, () =>
      Math.max(1, minimumPages),
    ),
  };
}

function measureRenderedPages(
  container: HTMLElement,
  layout: ReaderLayout,
  pagesPerSpread: number,
) {
  if (layout === "continuous") {
    return Math.max(
      1,
      Math.ceil(container.scrollHeight / Math.max(1, container.clientHeight)),
    );
  }
  const article = container.querySelector<HTMLElement>(".reader-document");
  const terminal = article?.querySelector<HTMLElement>(".reader-section-nav");
  const pageWidth = Math.max(1, container.clientWidth / pagesPerSpread);
  let rawPages = 0;
  if (article && terminal) {
    const articleRect = article.getBoundingClientRect();
    const terminalRect = terminal.getBoundingClientRect();
    rawPages = Math.ceil(
      Math.max(0, terminalRect.right - articleRect.left) / pageWidth,
    );
  }
  if (rawPages <= 0 && article) {
    rawPages = Math.ceil(article.scrollWidth / pageWidth);
  }
  return Math.max(
    pagesPerSpread,
    Math.ceil(Math.max(1, rawPages) / pagesPerSpread) * pagesPerSpread,
  );
}

function calculatePagePosition(
  counts: number[],
  sectionIndex: number,
  sectionProgress: number,
  pagesPerView: number,
) {
  const safeCounts = counts.length > 0 ? counts : [1];
  const currentSection = Math.min(
    safeCounts.length - 1,
    Math.max(0, sectionIndex),
  );
  const sectionPages = Math.max(1, safeCounts[currentSection] ?? 1);
  const precedingPages = safeCounts
    .slice(0, currentSection)
    .reduce((total, count) => total + Math.max(1, count), 0);
  const total = safeCounts.reduce((sum, count) => sum + Math.max(1, count), 0);
  const progress = Math.min(1, Math.max(0, sectionProgress));
  const viewCount = Math.max(1, Math.ceil(sectionPages / pagesPerView));
  const viewIndex = Math.min(viewCount - 1, Math.floor(progress * viewCount));
  const start = precedingPages + viewIndex * pagesPerView + 1;
  const end = Math.min(precedingPages + sectionPages, start + pagesPerView - 1);
  return { start, end, total: Math.max(1, total) };
}

function readerSectionProgress(container: HTMLElement, spread: boolean) {
  const range = readerScrollRange(container, spread);
  const position = spread ? container.scrollLeft : container.scrollTop;
  return range <= 0 ? 1 : Math.min(1, Math.max(0, position / range));
}

function spreadOffsetForProgress(container: HTMLElement, progress: number) {
  const range = readerScrollRange(container, true);
  const spreadWidth = container.clientWidth;
  const target = range * Math.min(1, Math.max(0, progress));
  if (spreadWidth <= 0) return target;
  return Math.min(range, Math.round(target / spreadWidth) * spreadWidth);
}

function validCustomFont(
  value: ImportedReaderFont | null | undefined,
): value is ImportedReaderFont {
  return Boolean(
    value &&
      typeof value.name === "string" &&
      typeof value.family === "string" &&
      value.family.startsWith("ApriReaderImported_") &&
      typeof value.path === "string",
  );
}

function clamp(
  value: number | undefined,
  min: number,
  max: number,
  fallback: number,
) {
  return typeof value === "number" && Number.isFinite(value)
    ? Math.min(max, Math.max(min, value))
    : fallback;
}

function wordCount(text: string) {
  return text.trim() ? text.trim().split(/\s+/u).length : 0;
}
