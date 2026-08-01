import { render, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useReadingSession } from "./useReadingSession";

const statistics = vi.hoisted(() => ({
  startReadingSession: vi.fn<(bookId: number) => Promise<string>>(),
  recordReadingActivity: vi.fn<() => Promise<void>>(),
  endReadingSession: vi.fn<() => Promise<void>>(),
}));

vi.mock("../application/statistics", () => statistics);

function SessionHarness({
  bookId,
  progress,
}: {
  bookId: number;
  progress: number;
}) {
  useReadingSession({
    bookId,
    progress,
    words: Math.round(progress * 100),
    pages: Math.round(progress * 10),
  });
  return null;
}

describe("useReadingSession", () => {
  beforeEach(() => {
    statistics.startReadingSession.mockReset();
    statistics.recordReadingActivity.mockReset().mockResolvedValue(undefined);
    statistics.endReadingSession.mockReset().mockResolvedValue(undefined);
  });

  it("finishes the previous book with that book's latest values", async () => {
    statistics.startReadingSession
      .mockResolvedValueOnce("first-session")
      .mockResolvedValueOnce("second-session");
    const view = render(<SessionHarness bookId={1} progress={0.2} />);
    await waitFor(() =>
      expect(statistics.startReadingSession).toHaveBeenCalledTimes(1),
    );

    view.rerender(<SessionHarness bookId={1} progress={0.4} />);
    view.rerender(<SessionHarness bookId={2} progress={0.8} />);

    await waitFor(() =>
      expect(statistics.recordReadingActivity).toHaveBeenCalledWith(
        "first-session",
        false,
        0.4,
        40,
        4,
      ),
    );
    expect(statistics.recordReadingActivity).not.toHaveBeenCalledWith(
      "first-session",
      false,
      0.8,
      80,
      8,
    );
  });
});
