import { useCallback, useState } from "react";
import { readLocalValue, writeLocalValue } from "./localStorage";

export const localProfileKey = "aprireader.localProfile";
export const displayNameMaxLength = 40;

type StoredLocalProfile = {
  onboardingComplete: boolean;
  displayName: string;
};

const emptyProfile: StoredLocalProfile = {
  onboardingComplete: false,
  displayName: "",
};

export function normalizeDisplayName(value: string) {
  const normalized = value.replace(/\p{C}/gu, "").trim().replace(/\s+/gu, " ");
  return Array.from(normalized).slice(0, displayNameMaxLength).join("");
}

function readLocalProfile(): StoredLocalProfile {
  try {
    const value = JSON.parse(
      readLocalValue(localProfileKey) ?? "",
    ) as Partial<StoredLocalProfile>;
    return {
      onboardingComplete: value.onboardingComplete === true,
      displayName:
        typeof value.displayName === "string"
          ? normalizeDisplayName(value.displayName)
          : "",
    };
  } catch {
    return emptyProfile;
  }
}

export function useLocalProfile() {
  const [profile, setProfile] = useState(readLocalProfile);

  const store = useCallback((next: StoredLocalProfile) => {
    setProfile(next);
    writeLocalValue(localProfileKey, JSON.stringify(next));
  }, []);

  const completeOnboarding = useCallback(
    (displayName = "") => {
      store({
        onboardingComplete: true,
        displayName: normalizeDisplayName(displayName),
      });
    },
    [store],
  );

  const saveDisplayName = useCallback(
    (displayName: string) => {
      store({
        onboardingComplete: true,
        displayName: normalizeDisplayName(displayName),
      });
    },
    [store],
  );

  return {
    ...profile,
    completeOnboarding,
    saveDisplayName,
  };
}
