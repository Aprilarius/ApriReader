import { useEffect, useRef, useState } from "react";
import type { PDFDocumentProxy, RenderTask } from "pdfjs-dist";
import pdfWorkerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
import {
  localAssetUrl,
  type SpecialDocument,
} from "../application/fixedReader";
import { saveReadingPosition } from "../application/reader";
import type { TranslationKey } from "./i18n";
import { useReadingSession } from "./useReadingSession";

type Translator = (key: TranslationKey) => string;
type ComicLayout = "single" | "double";
type ReadingDirection = "ltr" | "rtl";

export function SpecialReaderScreen({
  document,
  t,
  onClose,
  onProgress,
}: {
  document: SpecialDocument;
  t: Translator;
  onClose: () => void;
  onProgress: (progress: number) => void;
}) {
  if (document.kind === "pdf") {
    return (
      <PdfReader
        document={document}
        t={t}
        onClose={onClose}
        onProgress={onProgress}
      />
    );
  }
  return (
    <ComicReader
      document={document}
      t={t}
      onClose={onClose}
      onProgress={onProgress}
    />
  );
}

function PdfReader({
  document,
  t,
  onClose,
  onProgress,
}: {
  document: SpecialDocument;
  t: Translator;
  onClose: () => void;
  onProgress: (progress: number) => void;
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const pdfRef = useRef<PDFDocumentProxy | null>(null);
  const renderRef = useRef<RenderTask | null>(null);
  const onProgressRef = useRef(onProgress);
  const [page, setPage] = useState(Math.max(1, document.lastPage + 1));
  const [pageCount, setPageCount] = useState(0);
  const [zoom, setZoom] = useState(1.15);
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
    if (!document.sourcePath) {
      setError(t("fixedReaderError"));
      return;
    }
    const sourcePath = document.sourcePath;
    let active = true;
    let destroy: (() => Promise<void>) | undefined;
    void import("pdfjs-dist")
      .then(({ GlobalWorkerOptions, getDocument }) => {
        GlobalWorkerOptions.workerSrc = pdfWorkerUrl;
        const task = getDocument({
          url: localAssetUrl(sourcePath),
          useWorkerFetch: false,
        });
        destroy = () => task.destroy();
        return task.promise;
      })
      .then((pdf) => {
        if (!active) {
          void pdf.destroy();
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
        const viewport = pdfPage.getViewport({ scale: zoom });
        const pixelRatio = Math.min(2, window.devicePixelRatio || 1);
        canvas.width = Math.floor(viewport.width * pixelRatio);
        canvas.height = Math.floor(viewport.height * pixelRatio);
        canvas.style.width = `${viewport.width}px`;
        canvas.style.height = `${viewport.height}px`;
        const context = canvas.getContext("2d");
        if (!context) throw new Error("Canvas rendering is unavailable.");
        const renderTask = pdfPage.render({
          canvasContext: context,
          canvas,
          viewport,
          transform:
            pixelRatio === 1 ? undefined : [pixelRatio, 0, 0, pixelRatio, 0, 0],
        });
        renderRef.current = renderTask;
        return renderTask.promise;
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
        }
      });
    return () => {
      active = false;
      renderRef.current?.cancel();
    };
  }, [page, pageCount, zoom]);

  useEffect(() => {
    if (pageCount === 0) return;
    const progress = page / pageCount;
    onProgressRef.current(progress);
    void saveReadingPosition(document.bookId, page - 1, 0, progress);
  }, [document.bookId, page, pageCount]);

  const changePage = (next: number) =>
    setPage(Math.min(pageCount, Math.max(1, next)));

  return (
    <div
      className="fixed-reader pdf-reader"
      onKeyDown={(event) => {
        if (event.key === "ArrowLeft" || event.key === "PageUp")
          changePage(page - 1);
        if (event.key === "ArrowRight" || event.key === "PageDown")
          changePage(page + 1);
        if (event.key === "Escape") onClose();
      }}
    >
      <ReaderHeader title={document.title} format="PDF" onClose={onClose}>
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
      <main className="fixed-reader-stage" tabIndex={0}>
        {loading && <p className="fixed-reader-state">{t("readerLoading")}</p>}
        {error && (
          <p className="fixed-reader-state error-message" role="alert">
            {t("fixedReaderError")}: {error}
          </p>
        )}
        <canvas ref={canvasRef} aria-label={t("pdfPage")} />
      </main>
      <PageControls
        page={page}
        pageCount={pageCount}
        t={t}
        onChange={changePage}
      />
    </div>
  );
}

function ComicReader({
  document,
  t,
  onClose,
  onProgress,
}: {
  document: SpecialDocument;
  t: Translator;
  onClose: () => void;
  onProgress: (progress: number) => void;
}) {
  const [page, setPage] = useState(
    Math.min(document.pages.length, document.lastPage + 1),
  );
  const [layout, setLayout] = useState<ComicLayout>("single");
  const [direction, setDirection] = useState<ReadingDirection>("ltr");
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

  const step = layout === "double" ? 2 : 1;
  const changePage = (next: number) =>
    setPage(Math.min(pageCount, Math.max(1, next)));

  return (
    <div
      className="fixed-reader comic-reader"
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
        onClose={onClose}
      >
        <button
          type="button"
          className={layout === "single" ? "active" : ""}
          onClick={() => setLayout("single")}
        >
          {t("singlePage")}
        </button>
        <button
          type="button"
          className={layout === "double" ? "active" : ""}
          onClick={() => setLayout("double")}
        >
          {t("doublePage")}
        </button>
        <button
          type="button"
          onClick={() =>
            setDirection((value) => (value === "ltr" ? "rtl" : "ltr"))
          }
        >
          {direction === "ltr" ? t("leftToRight") : t("rightToLeft")}
        </button>
      </ReaderHeader>
      <main
        className={`comic-stage layout-${layout}`}
        data-direction={direction}
        tabIndex={0}
      >
        {visible.map((index) => {
          const comicPage = document.pages[index];
          return comicPage ? (
            <img
              src={localAssetUrl(comicPage.path)}
              alt={`${t("comicPage")} ${index + 1}`}
              key={comicPage.path}
              draggable={false}
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
      />
    </div>
  );
}

function ReaderHeader({
  title,
  format,
  onClose,
  children,
}: {
  title: string;
  format: string;
  onClose: () => void;
  children: React.ReactNode;
}) {
  return (
    <header className="fixed-reader-toolbar">
      <button type="button" className="fixed-reader-back" onClick={onClose}>
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
}: {
  page: number;
  pageCount: number;
  step?: number;
  t: Translator;
  onChange: (page: number) => void;
}) {
  return (
    <footer className="fixed-page-controls">
      <button
        type="button"
        disabled={page <= 1}
        onClick={() => onChange(page - step)}
      >
        ← {t("previousPage")}
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
        disabled={page >= pageCount}
        onClick={() => onChange(page + step)}
      >
        {t("nextPage")} →
      </button>
    </footer>
  );
}
