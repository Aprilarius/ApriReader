import { useEffect, useRef, useState, type TouchEvent } from "react";
import { readFile } from "@tauri-apps/plugin-fs";
import type { PDFDocumentProxy, RenderTask } from "pdfjs-dist";
import pdfWorkerUrl from "pdfjs-dist/legacy/build/pdf.worker.min.mjs?url";
import {
  localAssetUrl,
  type SpecialDocument,
} from "../application/fixedReader";
import { saveReadingPosition } from "../application/reader";
import type { TranslationKey } from "./i18n";
import {
  clampReaderZoom,
  fixedReaderSwipeDirection,
  touchDistance,
  type TouchPoint,
} from "./fixedReaderGestures";
import { ensurePdfWebViewCompatibility } from "./pdfCompatibility";
import { useReadingSession } from "./useReadingSession";

type Translator = (key: TranslationKey) => string;
type ComicLayout = "single" | "double";
type ReadingDirection = "ltr" | "rtl";

type FixedTouchGesture = {
  point: TouchPoint | null;
  pinchDistance: number;
  pinchZoom: number;
};

export function SpecialReaderScreen({
  document,
  t,
  onClose,
  onProgress,
  language,
  screenReaderSupport = true,
}: {
  document: SpecialDocument;
  t: Translator;
  onClose: () => void;
  onProgress: (progress: number) => void;
  language?: string;
  screenReaderSupport?: boolean;
}) {
  if (document.kind === "pdf") {
    return (
      <PdfReader
        document={document}
        t={t}
        onClose={onClose}
        onProgress={onProgress}
        language={language}
        screenReaderSupport={screenReaderSupport}
      />
    );
  }
  return (
    <ComicReader
      document={document}
      t={t}
      onClose={onClose}
      onProgress={onProgress}
      language={language}
      screenReaderSupport={screenReaderSupport}
    />
  );
}

function PdfReader({
  document,
  t,
  onClose,
  onProgress,
  language,
  screenReaderSupport,
}: {
  document: SpecialDocument;
  t: Translator;
  onClose: () => void;
  onProgress: (progress: number) => void;
  language?: string;
  screenReaderSupport: boolean;
}) {
  const stageRef = useRef<HTMLElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const pdfRef = useRef<PDFDocumentProxy | null>(null);
  const renderRef = useRef<RenderTask | null>(null);
  const gestureRef = useRef<FixedTouchGesture>({
    point: null,
    pinchDistance: 0,
    pinchZoom: 1,
  });
  const onProgressRef = useRef(onProgress);
  const [page, setPage] = useState(Math.max(1, document.lastPage + 1));
  const [pageCount, setPageCount] = useState(0);
  const [zoom, setZoom] = useState(1);
  const [stageWidth, setStageWidth] = useState(0);
  const [rendering, setRendering] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  useReadingSession({
    bookId: document.bookId,
    progress: pageCount > 0 ? page / pageCount : document.progress,
    words: 0,
    pages: page,
  });

  useEffect(() => {
    onProgressRef.current = onProgress;
  }, [onProgress]);

  useEffect(() => {
    const stage = stageRef.current;
    if (!stage) return;
    const updateWidth = () => setStageWidth(stage.clientWidth);
    updateWidth();
    const observer = new ResizeObserver(updateWidth);
    observer.observe(stage);
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    if (!document.sourcePath) {
      setError(t("fixedReaderError"));
      return;
    }
    const sourcePath = document.sourcePath;
    let active = true;
    let destroy: (() => Promise<void>) | undefined;
    ensurePdfWebViewCompatibility();
    void Promise.all([
      import("pdfjs-dist/legacy/build/pdf.mjs"),
      readFile(sourcePath),
    ])
      .then(([{ GlobalWorkerOptions, getDocument }, data]) => {
        GlobalWorkerOptions.workerSrc = pdfWorkerUrl;
        const task = getDocument({
          data,
          useWorkerFetch: false,
        });
        destroy = () => task.destroy();
        return task.promise;
      })
      .then((pdf) => {
        if (!active) {
          return;
        }
        pdfRef.current = pdf;
        setPageCount(pdf.numPages);
        setPage((value) => Math.min(pdf.numPages, value));
        setLoading(false);
      })
      .catch((reason: unknown) => {
        setError(reason instanceof Error ? reason.message : String(reason));
        setLoading(false);
      });
    return () => {
      active = false;
      renderRef.current?.cancel();
      if (destroy) void destroy();
      pdfRef.current = null;
    };
  }, [document.sourcePath, t]);

  useEffect(() => {
    const pdf = pdfRef.current;
    const canvas = canvasRef.current;
    if (!pdf || !canvas || pageCount === 0) return;
    let active = true;
    renderRef.current?.cancel();
    void pdf
      .getPage(page)
      .then((pdfPage) => {
        if (!active) return;
        const naturalViewport = pdfPage.getViewport({ scale: 1 });
        const availableWidth = Math.max(240, stageWidth - 24);
        const fitScale = Math.min(1.6, availableWidth / naturalViewport.width);
        const viewport = pdfPage.getViewport({ scale: fitScale * zoom });
        const pixelRatio = Math.min(2, window.devicePixelRatio || 1);
        canvas.width = Math.floor(viewport.width * pixelRatio);
        canvas.height = Math.floor(viewport.height * pixelRatio);
        canvas.style.width = `${viewport.width}px`;
        canvas.style.height = `${viewport.height}px`;
        const context = canvas.getContext("2d");
        if (!context) throw new Error("Canvas rendering is unavailable.");
        setRendering(true);
        const renderTask = pdfPage.render({
          canvasContext: context,
          canvas,
          viewport,
          transform:
            pixelRatio === 1 ? undefined : [pixelRatio, 0, 0, pixelRatio, 0, 0],
        });
        renderRef.current = renderTask;
        return renderTask.promise.then(() => setRendering(false));
      })
      .catch((reason: unknown) => {
        if (
          active &&
          !(
            reason instanceof Error &&
            reason.name === "RenderingCancelledException"
          )
        ) {
          setError(reason instanceof Error ? reason.message : String(reason));
          setRendering(false);
        }
      });
    return () => {
      active = false;
      renderRef.current?.cancel();
    };
  }, [page, pageCount, stageWidth, zoom]);

  useEffect(() => {
    if (pageCount === 0) return;
    const progress = page / pageCount;
    onProgressRef.current(progress);
    void saveReadingPosition(document.bookId, page - 1, 0, progress);
  }, [document.bookId, page, pageCount]);

  const changePage = (next: number) =>
    setPage(Math.min(pageCount, Math.max(1, next)));
  const touchHandlers = fixedTouchHandlers(
    gestureRef,
    zoom,
    setZoom,
    (direction) => changePage(page + direction),
  );

  return (
    <div
      className="fixed-reader pdf-reader"
      lang={language}
      onKeyDown={(event) => {
        if (event.key === "ArrowLeft" || event.key === "PageUp")
          changePage(page - 1);
        if (event.key === "ArrowRight" || event.key === "PageDown")
          changePage(page + 1);
        if (event.key === "Escape") onClose();
      }}
    >
      <ReaderHeader
        title={document.title}
        format="PDF"
        backLabel={t("readerBack")}
        onClose={onClose}
      >
        <button
          type="button"
          aria-label={t("zoomOut")}
          onClick={() => setZoom((value) => Math.max(0.5, value - 0.15))}
        >
          −
        </button>
        <output>{Math.round(zoom * 100)}%</output>
        <button
          type="button"
          aria-label={t("zoomIn")}
          onClick={() => setZoom((value) => Math.min(3, value + 0.15))}
        >
          +
        </button>
      </ReaderHeader>
      <main
        ref={stageRef}
        className={`fixed-reader-stage${rendering ? " rendering" : ""}`}
        tabIndex={0}
        {...touchHandlers}
      >
        {loading && <p className="fixed-reader-state">{t("readerLoading")}</p>}
        {error && (
          <p className="fixed-reader-state error-message" role="alert">
            {t("fixedReaderError")}: {error}
          </p>
        )}
        <div className="pdf-page-surface">
          <canvas ref={canvasRef} aria-label={t("pdfPage")} />
        </div>
      </main>
      <PageControls
        page={page}
        pageCount={pageCount}
        t={t}
        onChange={changePage}
        announce={screenReaderSupport}
      />
    </div>
  );
}

function ComicReader({
  document,
  t,
  onClose,
  onProgress,
  language,
  screenReaderSupport,
}: {
  document: SpecialDocument;
  t: Translator;
  onClose: () => void;
  onProgress: (progress: number) => void;
  language?: string;
  screenReaderSupport: boolean;
}) {
  const [page, setPage] = useState(
    Math.min(document.pages.length, document.lastPage + 1),
  );
  const [layout, setLayout] = useState<ComicLayout>("single");
  const [direction, setDirection] = useState<ReadingDirection>("ltr");
  const [zoom, setZoom] = useState(1);
  const compact = useCompactFixedReader();
  const gestureRef = useRef<FixedTouchGesture>({
    point: null,
    pinchDistance: 0,
    pinchZoom: 1,
  });
  const onProgressRef = useRef(onProgress);
  const pageCount = document.pages.length;
  useReadingSession({
    bookId: document.bookId,
    progress: pageCount > 0 ? page / pageCount : document.progress,
    words: 0,
    pages: page,
  });
  const visible =
    layout === "double"
      ? direction === "ltr"
        ? [page - 1, page]
        : [page, page - 1]
      : [page - 1];

  useEffect(() => {
    onProgressRef.current = onProgress;
  }, [onProgress]);

  useEffect(() => {
    const progress = page / pageCount;
    onProgressRef.current(progress);
    void saveReadingPosition(document.bookId, page - 1, 0, progress);
  }, [document.bookId, page, pageCount]);

  useEffect(() => {
    if (compact) setLayout("single");
  }, [compact]);

  const step = layout === "double" ? 2 : 1;
  const changePage = (next: number) =>
    setPage(Math.min(pageCount, Math.max(1, next)));
  const touchHandlers = fixedTouchHandlers(gestureRef, zoom, setZoom, (swipe) =>
    changePage(page + (direction === "rtl" ? -swipe * step : swipe * step)),
  );

  return (
    <div
      className="fixed-reader comic-reader"
      lang={language}
      onKeyDown={(event) => {
        if (event.key === "ArrowLeft")
          changePage(page + (direction === "rtl" ? step : -step));
        if (event.key === "ArrowRight")
          changePage(page + (direction === "rtl" ? -step : step));
        if (event.key === "Escape") onClose();
      }}
    >
      <ReaderHeader
        title={document.title}
        format={document.format}
        backLabel={t("readerBack")}
        onClose={onClose}
      >
        <button
          type="button"
          className={layout === "single" ? "active" : ""}
          aria-pressed={layout === "single"}
          onClick={() => setLayout("single")}
        >
          {t("singlePage")}
        </button>
        {!compact && (
          <button
            type="button"
            className={layout === "double" ? "active" : ""}
            aria-pressed={layout === "double"}
            onClick={() => setLayout("double")}
          >
            {t("doublePage")}
          </button>
        )}
        <button
          type="button"
          onClick={() =>
            setDirection((value) => (value === "ltr" ? "rtl" : "ltr"))
          }
        >
          {direction === "ltr" ? t("leftToRight") : t("rightToLeft")}
        </button>
        <button
          type="button"
          aria-label={t("zoomOut")}
          onClick={() => setZoom((value) => clampReaderZoom(value - 0.15))}
        >
          −
        </button>
        <output>{Math.round(zoom * 100)}%</output>
        <button
          type="button"
          aria-label={t("zoomIn")}
          onClick={() => setZoom((value) => clampReaderZoom(value + 0.15))}
        >
          +
        </button>
      </ReaderHeader>
      <main
        className={`comic-stage layout-${layout} ${zoom > 1 ? "zoomed" : ""}`}
        data-direction={direction}
        tabIndex={0}
        {...touchHandlers}
      >
        {visible.map((index) => {
          const comicPage = document.pages[index];
          return comicPage ? (
            <img
              src={localAssetUrl(comicPage.path)}
              alt={`${t("comicPage")} ${index + 1}`}
              key={comicPage.path}
              draggable={false}
              style={
                zoom === 1
                  ? undefined
                  : {
                      width: `${zoom * 100}%`,
                      maxWidth: "none",
                      maxHeight: "none",
                    }
              }
            />
          ) : null;
        })}
      </main>
      <PageControls
        page={page}
        pageCount={pageCount}
        step={step}
        t={t}
        onChange={changePage}
        announce={screenReaderSupport}
      />
    </div>
  );
}

function useCompactFixedReader() {
  const [compact, setCompact] = useState(
    () => typeof window !== "undefined" && window.innerWidth <= 700,
  );
  useEffect(() => {
    const update = () => setCompact(window.innerWidth <= 700);
    window.addEventListener("resize", update);
    return () => window.removeEventListener("resize", update);
  }, []);
  return compact;
}

function fixedTouchHandlers(
  gestureRef: React.MutableRefObject<FixedTouchGesture>,
  zoom: number,
  setZoom: React.Dispatch<React.SetStateAction<number>>,
  onSwipe: (direction: number) => void,
) {
  return {
    onTouchStart(event: TouchEvent<HTMLElement>) {
      if (event.touches.length === 1) {
        const touch = event.touches[0];
        gestureRef.current.point = touch
          ? { clientX: touch.clientX, clientY: touch.clientY }
          : null;
        gestureRef.current.pinchDistance = 0;
      } else if (event.touches.length === 2) {
        const first = event.touches[0];
        const second = event.touches[1];
        if (!first || !second) return;
        gestureRef.current.point = null;
        gestureRef.current.pinchDistance = touchDistance(first, second);
        gestureRef.current.pinchZoom = zoom;
      }
    },
    onTouchMove(event: TouchEvent<HTMLElement>) {
      if (event.touches.length !== 2) return;
      const first = event.touches[0];
      const second = event.touches[1];
      const startDistance = gestureRef.current.pinchDistance;
      if (!first || !second || startDistance <= 0) return;
      event.preventDefault();
      setZoom(
        clampReaderZoom(
          gestureRef.current.pinchZoom *
            (touchDistance(first, second) / startDistance),
        ),
      );
    },
    onTouchEnd(event: TouchEvent<HTMLElement>) {
      const start = gestureRef.current.point;
      gestureRef.current.point = null;
      gestureRef.current.pinchDistance = 0;
      if (!start || event.changedTouches.length !== 1) return;
      const touch = event.changedTouches[0];
      if (!touch) return;
      const direction = fixedReaderSwipeDirection(start, touch);
      if (direction !== 0) onSwipe(direction);
    },
    onTouchCancel() {
      gestureRef.current.point = null;
      gestureRef.current.pinchDistance = 0;
    },
  };
}

function ReaderHeader({
  title,
  format,
  backLabel,
  onClose,
  children,
}: {
  title: string;
  format: string;
  backLabel: string;
  onClose: () => void;
  children: React.ReactNode;
}) {
  return (
    <header className="fixed-reader-toolbar">
      <button
        type="button"
        className="fixed-reader-back"
        aria-label={backLabel}
        autoFocus
        onClick={onClose}
      >
        ←
      </button>
      <div>
        <strong>{title}</strong>
        <span>{format}</span>
      </div>
      <nav>{children}</nav>
    </header>
  );
}

function PageControls({
  page,
  pageCount,
  step = 1,
  t,
  onChange,
  announce,
}: {
  page: number;
  pageCount: number;
  step?: number;
  t: Translator;
  onChange: (page: number) => void;
  announce: boolean;
}) {
  return (
    <footer className="fixed-page-controls">
      <p
        className="sr-only"
        role="status"
        aria-live={announce ? "polite" : "off"}
        aria-atomic="true"
      >
        {announce ? `${t("currentPage")}: ${page} / ${pageCount}` : ""}
      </p>
      <button
        type="button"
        aria-label={t("previousPage")}
        disabled={page <= 1}
        onClick={() => onChange(page - step)}
      >
        ‹ <span>{t("previousPage")}</span>
      </button>
      <label>
        <span className="sr-only">{t("currentPage")}</span>
        <input
          type="number"
          min={1}
          max={Math.max(1, pageCount)}
          value={page}
          onChange={(event) => onChange(Number(event.target.value))}
        />
        <span>/ {pageCount}</span>
      </label>
      <button
        type="button"
        aria-label={t("nextPage")}
        disabled={page >= pageCount}
        onClick={() => onChange(page + step)}
      >
        › <span>{t("nextPage")}</span>
      </button>
    </footer>
  );
}
