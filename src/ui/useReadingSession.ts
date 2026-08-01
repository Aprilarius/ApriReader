import { useEffect, useRef } from "react";
import {
  endReadingSession,
  recordReadingActivity,
  startReadingSession,
} from "../application/statistics";

const heartbeatMilliseconds = 15_000;
const recentInteractionMilliseconds = 60_000;

type ReadingValue = {
  progress: number;
  words: number;
  pages: number;
};

export function useReadingSession({
  bookId,
  progress,
  words,
  pages,
}: {
  bookId: number;
  progress: number;
  words: number;
  pages: number;
}) {
  const valuesByBook = useRef(new Map<number, ReadingValue>());
  valuesByBook.current.set(bookId, { progress, words, pages });

  useEffect(() => {
    let disposed = false;
    let token: string | null = null;
    let finished = false;
    let lastInteraction = Date.now();
    const startingValue = valuesByBook.current.get(bookId);
    if (!startingValue) return;
    const currentValue = () =>
      valuesByBook.current.get(bookId) ?? startingValue;
    const finishSession = async (sessionToken: string) => {
      if (finished) return;
      finished = true;
      const value = currentValue();
      await recordReadingActivity(
        sessionToken,
        false,
        value.progress,
        value.words,
        value.pages,
      ).catch(() => undefined);
      await endReadingSession(sessionToken).catch(() => undefined);
      valuesByBook.current.delete(bookId);
    };
    const markInteraction = () => {
      lastInteraction = Date.now();
    };
    const activityEvents = [
      "keydown",
      "pointerdown",
      "scroll",
      "touchstart",
      "wheel",
    ] as const;
    for (const event of activityEvents) {
      window.addEventListener(event, markInteraction, { passive: true });
    }

    void startReadingSession(
      bookId,
      startingValue.progress,
      startingValue.words,
      startingValue.pages,
    )
      .then((sessionToken) => {
        if (disposed && sessionToken) {
          void finishSession(sessionToken);
          return;
        }
        token = sessionToken;
      })
      .catch(() => undefined);

    const heartbeat = window.setInterval(() => {
      if (!token) return;
      const active =
        document.visibilityState === "visible" &&
        document.hasFocus() &&
        Date.now() - lastInteraction <= recentInteractionMilliseconds;
      const value = currentValue();
      void recordReadingActivity(
        token,
        active,
        value.progress,
        value.words,
        value.pages,
      ).catch(() => undefined);
    }, heartbeatMilliseconds);

    return () => {
      disposed = true;
      window.clearInterval(heartbeat);
      for (const event of activityEvents) {
        window.removeEventListener(event, markInteraction);
      }
      if (token) {
        void finishSession(token);
      }
    };
  }, [bookId]);
}
