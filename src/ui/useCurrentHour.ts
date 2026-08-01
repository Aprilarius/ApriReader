import { useEffect, useState } from "react";

const CLOCK_REFRESH_INTERVAL_MS = 60_000;

export function useCurrentHour(): number {
  const [hour, setHour] = useState(() => new Date().getHours());

  useEffect(() => {
    const refresh = () => setHour(new Date().getHours());
    const timer = window.setInterval(refresh, CLOCK_REFRESH_INTERVAL_MS);
    window.addEventListener("focus", refresh);
    document.addEventListener("visibilitychange", refresh);

    return () => {
      window.clearInterval(timer);
      window.removeEventListener("focus", refresh);
      document.removeEventListener("visibilitychange", refresh);
    };
  }, []);

  return hour;
}
