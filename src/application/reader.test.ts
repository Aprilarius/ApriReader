import { describe, expect, it } from "vitest";
import {
  isExternalLink,
  normalizeDocumentModel,
  resolveLinkTarget,
  type DocumentModel,
  type DocumentSection,
} from "./reader";

const validDocument: DocumentModel = {
  bookId: 7,
  title: "Format fixture",
  author: "ApriReader",
  format: "FB2",
  progress: 0.25,
  lastSection: 0,
  sectionProgress: 0.5,
  sections: [
    {
      id: "chapter-1",
      title: "Chapter 1",
      source: "",
      anchors: [],
      blocks: [{ kind: "paragraph", text: "Readable text.", links: [] }],
    },
  ],
};

describe("normalizeDocumentModel", () => {
  it("clamps restored reader positions for every reflowable format", () => {
    const document = normalizeDocumentModel({
      ...validDocument,
      format: "EPUB",
      progress: 4,
      lastSection: 99,
      sectionProgress: -2,
    });

    expect(document.progress).toBe(1);
    expect(document.lastSection).toBe(0);
    expect(document.sectionProgress).toBe(0);
  });

  it("rejects malformed or textless documents before the reader can render", () => {
    expect(() =>
      normalizeDocumentModel({ ...validDocument, format: "FB2", sections: [] }),
    ).toThrow("readable text");

    expect(() =>
      normalizeDocumentModel({
        ...validDocument,
        format: "DOCX",
        sections: [
          {
            id: "broken",
            title: "Broken",
            source: "",
            anchors: [],
            blocks: [{ kind: "paragraph", text: 42 } as never],
          },
        ],
      }),
    ).toThrow("readable text");
  });
});

const linkedSections: DocumentSection[] = [
  {
    id: "section-1",
    title: "Chapter",
    source: "OPS/ch1-15.xhtml",
    anchors: ["id10"],
    blocks: [{ kind: "paragraph", text: "Body [85]", links: [] }],
  },
  {
    id: "section-2",
    title: "85",
    source: "OPS/ch2-85.xhtml",
    anchors: ["id45"],
    blocks: [{ kind: "paragraph", text: "Note body.", links: [] }],
  },
  {
    id: "section-3",
    title: "Notes",
    source: "OPS/notes.xhtml",
    anchors: ["n1", "n2"],
    blocks: [{ kind: "paragraph", text: "Collected notes.", links: [] }],
  },
];

describe("resolveLinkTarget", () => {
  it("follows a note link that points at another file in the book", () => {
    expect(
      resolveLinkTarget(
        "ch2-85.xhtml#id45",
        linkedSections,
        "OPS/ch1-15.xhtml",
      ),
    ).toBe(1);
  });

  it("resolves a bare fragment through the anchors of a section", () => {
    expect(resolveLinkTarget("#n2", linkedSections, "OPS/ch1-15.xhtml")).toBe(
      2,
    );
  });

  it("walks relative segments from the linking file", () => {
    expect(
      resolveLinkTarget(
        "../OPS/notes.xhtml",
        linkedSections,
        "OPS/sub/a.xhtml",
      ),
    ).toBe(2);
  });

  it("reports links the book cannot satisfy instead of guessing", () => {
    expect(
      resolveLinkTarget("missing.xhtml#x", linkedSections, "OPS/ch1-15.xhtml"),
    ).toBeNull();
    // External destinations are the browser's job, not the reader's.
    expect(
      resolveLinkTarget(
        "https://example.com",
        linkedSections,
        "OPS/ch1-15.xhtml",
      ),
    ).toBeNull();
    expect(isExternalLink("https://example.com")).toBe(true);
    expect(isExternalLink("#id45")).toBe(false);
    expect(isExternalLink("ch2-85.xhtml")).toBe(false);
  });
});

describe("inline link normalization", () => {
  it("keeps a valid range and drops one that does not fit the text", () => {
    const document = normalizeDocumentModel({
      ...validDocument,
      sections: [
        {
          id: "chapter-1",
          title: "Chapter 1",
          source: "OPS/a.xhtml",
          anchors: ["top"],
          blocks: [
            {
              kind: "paragraph",
              text: "He called it [85] a myth.",
              links: [
                { start: 13, end: 17, href: "ch2-85.xhtml#id45" },
                { start: 13, end: 900, href: "broken.xhtml" },
                { start: 0, end: 3, href: "   " },
              ],
            },
          ],
        },
      ],
    });
    const links = document.sections[0]!.blocks[0]!.links;
    expect(links).toEqual([{ start: 13, end: 17, href: "ch2-85.xhtml#id45" }]);
    // The text itself must survive untouched: annotations index into it.
    expect(document.sections[0]!.blocks[0]!.text).toBe(
      "He called it [85] a myth.",
    );
    expect(document.sections[0]!.source).toBe("OPS/a.xhtml");
    expect(document.sections[0]!.anchors).toEqual(["top"]);
  });
});
