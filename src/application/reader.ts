import { invoke, isTauri } from "@tauri-apps/api/core";

export type BlockKind =
  | "heading"
  | "paragraph"
  | "quote"
  | "listItem"
  | "code"
  | "divider";

/**
 * A hyperlink from the source markup, expressed as a range of the block text.
 *
 * Offsets index the same string the reader renders, so annotations and speech
 * highlighting keep addressing the text exactly as they did before links were
 * carried through the pipeline.
 */
export type InlineLink = {
  start: number;
  end: number;
  href: string;
};

export type DocumentBlock = {
  kind: BlockKind;
  text: string;
  links: InlineLink[];
};

export type DocumentSection = {
  id: string;
  title: string;
  blocks: DocumentBlock[];
  /** Archive-relative path the section came from; empty for single-file books. */
  source: string;
  /** `id` attributes found in that file, used to resolve `#fragment` links. */
  anchors: string[];
};

export type DocumentModel = {
  bookId: number;
  title: string;
  author: string;
  format: string;
  sections: DocumentSection[];
  progress: number;
  lastSection: number;
  sectionProgress: number;
};

const blockKinds = new Set<BlockKind>([
  "heading",
  "paragraph",
  "quote",
  "listItem",
  "code",
  "divider",
]);

/**
 * Drops link ranges that do not describe a real slice of the block. A stale or
 * malformed range would otherwise put a clickable region over arbitrary words.
 */
function normalizeInlineLinks(value: unknown, text: string): InlineLink[] {
  if (!Array.isArray(value)) return [];
  return value.flatMap((link) => {
    if (!link || typeof link !== "object") return [];
    const { start, end, href } = link as Partial<InlineLink>;
    if (typeof href !== "string" || !href.trim()) return [];
    if (!Number.isInteger(start) || !Number.isInteger(end)) return [];
    const from = start as number;
    const to = end as number;
    if (from < 0 || to > text.length || to <= from) return [];
    return [{ start: from, end: to, href }];
  });
}

/** Web links leave the book and belong in the system browser, not the reader. */
export function isExternalLink(href: string) {
  return /^[a-z][a-z0-9+.-]*:/i.test(href) && !href.startsWith("#");
}

function decodePart(value: string) {
  try {
    return decodeURIComponent(value);
  } catch {
    return value;
  }
}

/** Applies a relative archive href to the directory of the current entry. */
function resolveArchivePath(from: string, href: string) {
  const base = from.includes("/") ? from.slice(0, from.lastIndexOf("/")) : "";
  const segments = href.startsWith("/") ? [] : base.split("/").filter(Boolean);
  for (const segment of href.split("/")) {
    if (!segment || segment === ".") continue;
    if (segment === "..") segments.pop();
    else segments.push(segment);
  }
  return segments.join("/");
}

/**
 * Maps a hyperlink from the book onto the section that should be shown.
 *
 * Returns `null` when nothing in this book matches, which the reader reports
 * rather than silently doing nothing.
 */
export function resolveLinkTarget(
  href: string,
  sections: DocumentSection[],
  fromSource: string,
): number | null {
  if (isExternalLink(href)) return null;
  const hash = href.indexOf("#");
  const path = hash < 0 ? href : href.slice(0, hash);
  const fragment = hash < 0 ? "" : decodePart(href.slice(hash + 1));

  if (!path) {
    // A bare `#id` points inside the current file first, then anywhere.
    const sameFile = sections.findIndex(
      (section) =>
        section.source === fromSource && section.anchors.includes(fragment),
    );
    if (sameFile >= 0) return sameFile;
    const anywhere = sections.findIndex((section) =>
      section.anchors.includes(fragment),
    );
    return anywhere >= 0 ? anywhere : null;
  }

  const target = resolveArchivePath(fromSource, decodePart(path));
  const matches = sections.flatMap((section, index) =>
    section.source === target ? [index] : [],
  );
  if (matches.length === 0) {
    // Some books link by bare file name across directories.
    const name = target.slice(target.lastIndexOf("/") + 1);
    const byName = sections.findIndex(
      (section) =>
        section.source.slice(section.source.lastIndexOf("/") + 1) === name,
    );
    return byName >= 0 ? byName : null;
  }
  if (fragment) {
    const withAnchor = matches.find((index) =>
      sections[index]!.anchors.includes(fragment),
    );
    if (withAnchor !== undefined) return withAnchor;
  }
  return matches[0]!;
}

function clampProgress(value: unknown) {
  return typeof value === "number" && Number.isFinite(value)
    ? Math.min(1, Math.max(0, value))
    : 0;
}

/**
 * Tauri command results are an external boundary at runtime. Keep malformed
 * documents out of the reader so every supported text format fails with an
 * actionable error instead of leaving the WebView on a blank screen.
 */
export function normalizeDocumentModel(document: DocumentModel): DocumentModel {
  const sections = Array.isArray(document.sections)
    ? document.sections.flatMap((section, index) => {
        if (!section || !Array.isArray(section.blocks)) return [];
        const blocks = section.blocks.flatMap((block) =>
          block && typeof block.text === "string" && blockKinds.has(block.kind)
            ? [
                {
                  kind: block.kind,
                  text: block.text,
                  links: normalizeInlineLinks(block.links, block.text),
                },
              ]
            : [],
        );
        if (blocks.length === 0) return [];
        return [
          {
            id:
              typeof section.id === "string" && section.id.trim()
                ? section.id
                : `section-${index + 1}`,
            title:
              typeof section.title === "string" && section.title.trim()
                ? section.title
                : `Section ${index + 1}`,
            blocks,
            source: typeof section.source === "string" ? section.source : "",
            anchors: Array.isArray(section.anchors)
              ? section.anchors.filter(
                  (anchor): anchor is string => typeof anchor === "string",
                )
              : [],
          },
        ];
      })
    : [];

  if (sections.length === 0) {
    throw new Error("The document does not contain readable text.");
  }

  const lastSection =
    typeof document.lastSection === "number" &&
    Number.isFinite(document.lastSection)
      ? Math.min(
          sections.length - 1,
          Math.max(0, Math.trunc(document.lastSection)),
        )
      : 0;

  return {
    ...document,
    title: typeof document.title === "string" ? document.title : "Untitled",
    author: typeof document.author === "string" ? document.author : "",
    format: typeof document.format === "string" ? document.format : "",
    sections,
    progress: clampProgress(document.progress),
    lastSection,
    sectionProgress: clampProgress(document.sectionProgress),
  };
}

export async function loadDocument(bookId: number): Promise<DocumentModel> {
  if (!isTauri()) {
    throw new Error("The reader is available in the ApriReader application.");
  }
  return normalizeDocumentModel(
    await invoke<DocumentModel>("load_document", { bookId }),
  );
}

export async function saveReadingPosition(
  bookId: number,
  section: number,
  sectionProgress: number,
  progress: number,
): Promise<void> {
  if (!isTauri()) return;
  await invoke("save_reading_position", {
    bookId,
    section,
    sectionProgress,
    progress,
  });
}
