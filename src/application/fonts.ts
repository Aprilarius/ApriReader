import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

export type ImportedReaderFont = {
  name: string;
  family: string;
  path: string;
};

export async function chooseAndImportReaderFont(): Promise<ImportedReaderFont | null> {
  const selected = await open({
    multiple: false,
    directory: false,
    filters: [
      {
        name: "Fonts",
        extensions: ["ttf", "otf", "woff", "woff2"],
      },
    ],
  });
  if (!selected || Array.isArray(selected)) return null;
  return invoke<ImportedReaderFont>("import_reader_font", { path: selected });
}

export function readerFontUrl(path: string): string {
  return convertFileSrc(path);
}
