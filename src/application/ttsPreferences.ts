import type { CloudTtsTiming } from "./cloudTts";

export type TtsProvider = "local" | "elevenlabs" | "google" | "azure";

export interface TtsVoicePreset {
  id: string;
  name: string;
  provider: TtsProvider;
  voiceId: string;
  rate: number;
  region?: string;
  stability?: number;
  similarityBoost?: number;
  style?: number;
  speakerBoost?: boolean;
  pitch?: number;
}

export interface TtsPronunciationRule {
  id: string;
  source: string;
  replacement: string;
}

export interface TtsPreferences {
  version: 1;
  dictionaryEnabled: boolean;
  presets: TtsVoicePreset[];
  pronunciations: TtsPronunciationRule[];
}

export interface PronouncedText {
  text: string;
  sourceOffsets: number[];
}

export const maxTtsVoicePresets = 20;
export const maxTtsPronunciationRules = 100;
export const maxTtsSpokenChunkCharacters = 2_000;
export const ttsPreferencesStorageKey = "aprireader.tts.preferences.v1";

const allowedRates = new Set([0.5, 0.75, 1, 1.25, 1.5, 1.75, 2]);
const idPattern = /^[A-Za-z0-9_-]{1,64}$/u;
const wordCharacterPattern = /[\p{L}\p{N}]/u;

export function emptyTtsPreferences(): TtsPreferences {
  return {
    version: 1,
    dictionaryEnabled: true,
    presets: [],
    pronunciations: [],
  };
}

function validText(value: unknown, maximum: number): value is string {
  return (
    typeof value === "string" &&
    value.trim().length > 0 &&
    value.trim().length <= maximum &&
    !Array.from(value).some((character) => {
      const code = character.codePointAt(0) ?? 0;
      return code <= 31 || code === 127;
    })
  );
}

function validId(value: unknown): value is string {
  return typeof value === "string" && idPattern.test(value);
}

function validProvider(value: unknown): value is TtsProvider {
  return (
    value === "local" ||
    value === "elevenlabs" ||
    value === "google" ||
    value === "azure"
  );
}

export function parseTtsPreferences(raw: string | null): TtsPreferences {
  if (!raw || raw.length > 65_536) return emptyTtsPreferences();
  try {
    const value = JSON.parse(raw) as Partial<TtsPreferences>;
    const presets = Array.isArray(value.presets)
      ? value.presets
          .filter(
            (preset): preset is TtsVoicePreset =>
              typeof preset === "object" &&
              preset !== null &&
              validId(preset.id) &&
              validText(preset.name, 40) &&
              validProvider(preset.provider) &&
              validText(preset.voiceId, 128) &&
              allowedRates.has(preset.rate) &&
              (preset.region === undefined ||
                (typeof preset.region === "string" &&
                  /^[a-z0-9]{2,32}$/u.test(preset.region))),
          )
          .slice(0, maxTtsVoicePresets)
          .map((preset) => ({ ...preset, name: preset.name.trim() }))
      : [];
    const seen = new Set<string>();
    const pronunciations = Array.isArray(value.pronunciations)
      ? value.pronunciations
          .filter(
            (rule): rule is TtsPronunciationRule =>
              typeof rule === "object" &&
              rule !== null &&
              validId(rule.id) &&
              validText(rule.source, 80) &&
              validText(rule.replacement, 160),
          )
          .filter((rule) => {
            const key = rule.source.trim().toLocaleLowerCase();
            if (seen.has(key)) return false;
            seen.add(key);
            return true;
          })
          .slice(0, maxTtsPronunciationRules)
          .map((rule) => ({
            ...rule,
            source: rule.source.trim(),
            replacement: rule.replacement.trim(),
          }))
      : [];
    return {
      version: 1,
      dictionaryEnabled: value.dictionaryEnabled !== false,
      presets,
      pronunciations,
    };
  } catch {
    return emptyTtsPreferences();
  }
}

export function createTtsPreferenceId(): string {
  return (
    globalThis.crypto?.randomUUID().replaceAll("-", "") ??
    `${Date.now()}_${Math.random().toString(36).slice(2)}`
  );
}

export function normalizeVoicePreset(
  value: TtsVoicePreset,
): TtsVoicePreset | null {
  if (
    !validId(value.id) ||
    !validText(value.name, 40) ||
    !validProvider(value.provider) ||
    !validText(value.voiceId, 128) ||
    !allowedRates.has(value.rate) ||
    (value.region !== undefined && !/^[a-z0-9]{2,32}$/u.test(value.region))
  ) {
    return null;
  }
  const normalized = {
    ...value,
    name: value.name.trim(),
    voiceId: value.voiceId.trim(),
  };
  if (value.provider === "elevenlabs") {
    normalized.stability = value.stability ?? 0.5;
    normalized.similarityBoost = value.similarityBoost ?? 0.75;
    normalized.style = value.style ?? 0;
    normalized.speakerBoost = value.speakerBoost ?? true;
    if (
      !validUnitValue(normalized.stability) ||
      !validUnitValue(normalized.similarityBoost) ||
      !validUnitValue(normalized.style) ||
      typeof normalized.speakerBoost !== "boolean"
    ) {
      return null;
    }
  } else if (value.provider === "google") {
    normalized.pitch = value.pitch ?? 0;
    if (
      !Number.isFinite(normalized.pitch) ||
      normalized.pitch < -20 ||
      normalized.pitch > 20
    )
      return null;
  } else if (value.provider === "azure") {
    normalized.pitch = value.pitch ?? 0;
    if (
      !Number.isInteger(normalized.pitch) ||
      normalized.pitch < -50 ||
      normalized.pitch > 50
    )
      return null;
  }
  return normalized;
}

function validUnitValue(value: unknown): value is number {
  return (
    typeof value === "number" &&
    Number.isFinite(value) &&
    value >= 0 &&
    value <= 1
  );
}

export function normalizePronunciationRule(
  value: TtsPronunciationRule,
): TtsPronunciationRule | null {
  if (
    !validId(value.id) ||
    !validText(value.source, 80) ||
    !validText(value.replacement, 160)
  ) {
    return null;
  }
  return {
    ...value,
    source: value.source.trim(),
    replacement: value.replacement.trim(),
  };
}

function escapeRegularExpression(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
}

function hasWholeWordBoundaries(
  text: string,
  start: number,
  end: number,
  source: string,
): boolean {
  const first = Array.from(source)[0] ?? "";
  const last = Array.from(source).at(-1) ?? "";
  const before =
    start > 0 ? (Array.from(text.slice(0, start)).at(-1) ?? "") : "";
  const after = end < text.length ? (Array.from(text.slice(end))[0] ?? "") : "";
  return !(
    (wordCharacterPattern.test(first) && wordCharacterPattern.test(before)) ||
    (wordCharacterPattern.test(last) && wordCharacterPattern.test(after))
  );
}

export function applyPronunciationDictionary(
  text: string,
  rules: TtsPronunciationRule[],
): PronouncedText {
  const matches: Array<{
    start: number;
    end: number;
    replacement: string;
  }> = [];
  for (const rule of rules) {
    const normalized = normalizePronunciationRule(rule);
    if (!normalized) continue;
    const expression = new RegExp(
      escapeRegularExpression(normalized.source),
      "giu",
    );
    for (const match of text.matchAll(expression)) {
      const start = match.index;
      const end = start + match[0].length;
      if (hasWholeWordBoundaries(text, start, end, match[0])) {
        matches.push({ start, end, replacement: normalized.replacement });
      }
    }
  }
  matches.sort((left, right) =>
    left.start === right.start
      ? right.end - right.start - (left.end - left.start)
      : left.start - right.start,
  );

  let spoken = "";
  let cursor = 0;
  const sourceOffsets = [0];
  const appendIdentity = (value: string, sourceStart: number) => {
    spoken += value;
    for (let index = 0; index < value.length; index += 1) {
      sourceOffsets.push(sourceStart + index + 1);
    }
  };
  const appendReplacement = (
    value: string,
    sourceStart: number,
    sourceLength: number,
  ) => {
    spoken += value;
    for (let index = 0; index < value.length; index += 1) {
      sourceOffsets.push(
        sourceStart + Math.round(((index + 1) / value.length) * sourceLength),
      );
    }
  };

  for (const match of matches) {
    if (match.start < cursor) continue;
    appendIdentity(text.slice(cursor, match.start), cursor);
    appendReplacement(match.replacement, match.start, match.end - match.start);
    cursor = match.end;
    if (spoken.length > maxTtsSpokenChunkCharacters) {
      throw new Error("TTS_PRONUNCIATION_EXPANSION_LIMIT");
    }
  }
  appendIdentity(text.slice(cursor), cursor);
  if (spoken.length > maxTtsSpokenChunkCharacters) {
    throw new Error("TTS_PRONUNCIATION_EXPANSION_LIMIT");
  }
  return { text: spoken, sourceOffsets };
}

export function remapCloudTtsTimings(
  timings: CloudTtsTiming[],
  sourceOffsets: number[],
): CloudTtsTiming[] {
  if (sourceOffsets.length === 0) return [];
  const maximum = sourceOffsets.length - 1;
  return timings.map((timing) => ({
    ...timing,
    startOffset: sourceOffsets[Math.min(maximum, timing.startOffset)] ?? 0,
    endOffset: sourceOffsets[Math.min(maximum, timing.endOffset)] ?? 0,
  }));
}
