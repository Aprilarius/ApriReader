import { useEffect, useRef } from "react";
import {
  endReadingSession,
  recordReadingActivity,
  startReadingSession,
} from "../application/statistics";

const heartbeatMilliseconds = 15_000;
const recentInteractionMilliseconds = 60_000;

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
  const current = useRef({ progress, words, pages });
  const initial = useRef({ progress, words, pages });
  current.current = { progress, words, pages };

  useEffect(() => {
    let disposed = false;
    let token: string | null = null;
    let lastInteraction = Date.now();
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

    const startingValue = initial.current;
    void startReadingSession(
      bookId,
      startingValue.progress,
      startingValue.words,
      startingValue.pages,
    )
      .then((sessionToken) => {
        if (disposed && sessionToken) {
          void endReadingSession(sessionToken);
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
      const value = current.current;
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
        const value = current.current;
        void recordReadingActivity(
          token,
          false,
          value.progress,
          value.words,
          value.pages,
        ).finally(() => {
          if (token) void endReadingSession(token);
        });
      }
    };
  }, [bookId]);
}
