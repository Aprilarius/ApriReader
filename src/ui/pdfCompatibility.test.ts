import { describe, expect, it } from "vitest";
import { ensurePdfWebViewCompatibility } from "./pdfCompatibility";

describe("PDF WebView compatibility", () => {
  it("provides Promise.try with value and rejection semantics", async () => {
    ensurePdfWebViewCompatibility();
    const promiseTry = (
      Promise as PromiseConstructor & {
        try: <T>(callback: () => T | PromiseLike<T>) => Promise<T>;
      }
    ).try;

    await expect(promiseTry.call(Promise, () => 42)).resolves.toBe(42);
    await expect(
      promiseTry.call(Promise, () => {
        throw new Error("expected");
      }),
    ).rejects.toThrow("expected");
  });

  it("provides URL.parse without throwing for malformed input", () => {
    ensurePdfWebViewCompatibility();
    const parse = (URL as typeof URL & { parse: (url: string) => URL | null })
      .parse;

    expect(parse("https://example.com/book.pdf")?.hostname).toBe("example.com");
    expect(parse("http://[invalid")).toBeNull();
  });
});
