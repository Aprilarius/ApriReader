import type { TranslationKey } from "./i18n";

export function greetingKeyForHour(hour: number): TranslationKey {
  if (hour < 12) return "greetingMorning";
  if (hour < 18) return "greetingAfternoon";
  return "greetingEvening";
}
