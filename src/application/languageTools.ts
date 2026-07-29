import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

export type InstalledLanguagePackage = {
  id: string;
  version: string;
  name: string;
  kind: "dictionary" | "translation";
  sourceLanguage: string;
  targetLanguage: string | null;
  licenseSpdx: string;
  attribution: string;
  engine: string;
  verified: boolean;
};

export type DictionaryResult = {
  packageId: string;
  packageName: string;
  term: string;
  definitions: string[];
  examples: string[];
};

export type TranslationResult = {
  packageId: string;
  sourceLanguage: string;
  targetLanguage: string;
  translatedText: string;
};

export async function chooseAndImportLanguagePackage() {
  const selected = await open({
    multiple: false,
    directory: false,
    filters: [
      {
        name: "ApriReader language package",
        extensions: ["apripkg", "apridict"],
      },
    ],
  });
  if (!selected) return null;
  return invoke<InstalledLanguagePackage>("import_language_package", {
    path: selected,
  });
}

export function listLanguagePackages() {
  return invoke<InstalledLanguagePackage[]>("list_language_packages");
}

export function lookupDictionary(text: string, context: string) {
  return invoke<DictionaryResult[]>("lookup_dictionary", { text, context });
}

export function translateOffline(
  packageId: string,
  version: string,
  text: string,
) {
  return invoke<TranslationResult>("translate_offline", {
    packageId,
    version,
    text,
  });
}

export function removeLanguagePackage(packageId: string, version: string) {
  return invoke<void>("remove_language_package", { packageId, version });
}
