import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { DocumentModel } from "../application/reader";
import { translations, type TranslationKey } from "./i18n";
import { ReaderScreen } from "./ReaderScreen";

const { openUrl } = vi.hoisted(() => ({
  openUrl: vi.fn<(url: string) => Promise<void>>().mockResolvedValue(undefined),
}));

const ttsMocks = vi.hoisted(() => ({
  listVoices: vi.fn(),
  prepare: vi.fn(),
  load: vi.fn(),
  play: vi.fn(),
  pause: vi.fn(),
  rate: vi.fn(),
  snapshot: vi.fn(),
  stop: vi.fn(),
}));

const cloudTtsMocks = vi.hoisted(() => ({
  status: vi.fn(),
  saveKey: vi.fn(),
  deleteKey: vi.fn(),
  listVoices: vi.fn(),
  prepare: vi.fn(),
}));

const googleTtsMocks = vi.hoisted(() => ({
  status: vi.fn(),
  saveKey: vi.fn(),
  deleteKey: vi.fn(),
  listVoices: vi.fn(),
  prepare: vi.fn(),
}));

const azureTtsMocks = vi.hoisted(() => ({
  status: vi.fn(),
  regions: vi.fn(),
  saveKey: vi.fn(),
  deleteKey: vi.fn(),
  listVoices: vi.fn(),
  prepare: vi.fn(),
}));

const ttsAssetMocks = vi.hoisted(() => ({
  summary: vi.fn(),
  clear: vi.fn(),
  choose: vi.fn(),
  begin: vi.fn(),
  append: vi.fn(),
  finish: vi.fn(),
  cancel: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl }));
vi.mock("../application/tts", () => ({
  maxTtsCharacters: 20_000,
  listTtsVoices: ttsMocks.listVoices,
  prepareTtsSection: ttsMocks.prepare,
}));
vi.mock("../application/audioPlayer", () => ({
  loadAudioFile: ttsMocks.load,
  playAudio: ttsMocks.play,
  pauseAudio: ttsMocks.pause,
  setAudioRate: ttsMocks.rate,
  getAudioSnapshot: ttsMocks.snapshot,
  stopAudio: ttsMocks.stop,
}));
vi.mock("../application/cloudTts", () => ({
  getCloudTtsStatus: cloudTtsMocks.status,
  saveCloudTtsKey: cloudTtsMocks.saveKey,
  deleteCloudTtsKey: cloudTtsMocks.deleteKey,
  listCloudTtsVoices: cloudTtsMocks.listVoices,
  prepareCloudTtsSection: cloudTtsMocks.prepare,
}));
vi.mock("../application/googleTts", () => ({
  getGoogleTtsStatus: googleTtsMocks.status,
  saveGoogleTtsKey: googleTtsMocks.saveKey,
  deleteGoogleTtsKey: googleTtsMocks.deleteKey,
  listGoogleTtsVoices: googleTtsMocks.listVoices,
  prepareGoogleTtsSection: googleTtsMocks.prepare,
}));
vi.mock("../application/azureTts", () => ({
  getAzureTtsStatus: azureTtsMocks.status,
  listAzureTtsRegions: azureTtsMocks.regions,
  saveAzureTtsKey: azureTtsMocks.saveKey,
  deleteAzureTtsKey: azureTtsMocks.deleteKey,
  listAzureTtsVoices: azureTtsMocks.listVoices,
  prepareAzureTtsSection: azureTtsMocks.prepare,
}));
vi.mock("../application/ttsAssets", () => ({
  maxTtsExportParts: 5_000,
  getTtsCacheSummary: ttsAssetMocks.summary,
  clearTtsCache: ttsAssetMocks.clear,
  chooseTtsExportPath: ttsAssetMocks.choose,
  beginTtsExport: ttsAssetMocks.begin,
  appendTtsExportPart: ttsAssetMocks.append,
  finishTtsExport: ttsAssetMocks.finish,
  cancelTtsExport: ttsAssetMocks.cancel,
}));

const document: DocumentModel = {
  bookId: 1,
  title: "A quiet fixture",
  author: "Test Author",
  format: "EPUB",
  progress: 0.25,
  lastSection: 0,
  sectionProgress: 0.5,
  sections: [
    {
      id: "opening",
      title: "Opening",
      blocks: [
        { kind: "paragraph", text: "Only safe text is rendered." },
        { kind: "quote", text: "A quoted thought." },
      ],
    },
    {
      id: "ending",
      title: "Ending",
      blocks: [{ kind: "paragraph", text: "The end." }],
    },
  ],
};

const t = (key: TranslationKey) => translations.en[key];

describe("ReaderScreen", () => {
  beforeEach(() => {
    localStorage.clear();
    openUrl.mockReset();
    for (const mock of Object.values(ttsMocks)) mock.mockReset();
    for (const mock of Object.values(cloudTtsMocks)) mock.mockReset();
    for (const mock of Object.values(googleTtsMocks)) mock.mockReset();
    for (const mock of Object.values(azureTtsMocks)) mock.mockReset();
    for (const mock of Object.values(ttsAssetMocks)) mock.mockReset();
    const emptyCache = {
      totalFiles: 0,
      totalBytes: 0,
      providers: [
        { provider: "local", files: 0, bytes: 0 },
        { provider: "elevenlabs", files: 0, bytes: 0 },
        { provider: "google", files: 0, bytes: 0 },
        { provider: "azure", files: 0, bytes: 0 },
      ],
    };
    ttsAssetMocks.summary.mockResolvedValue(emptyCache);
    ttsAssetMocks.clear.mockResolvedValue(emptyCache);
    ttsAssetMocks.choose.mockResolvedValue("C:\\Exports\\fixture.m3u8");
    ttsAssetMocks.begin.mockResolvedValue({
      sessionId: "export-session",
      expectedParts: 3,
    });
    ttsAssetMocks.append.mockResolvedValue(1);
    ttsAssetMocks.finish.mockResolvedValue({
      playlistPath: "C:\\Exports\\fixture.m3u8",
      mediaDirectory: "C:\\Exports\\fixture-media",
      parts: 3,
      bytes: 1024,
    });
    ttsAssetMocks.cancel.mockResolvedValue(undefined);
    cloudTtsMocks.status.mockResolvedValue({ configured: false });
    cloudTtsMocks.saveKey.mockResolvedValue({ configured: true });
    cloudTtsMocks.deleteKey.mockResolvedValue({ configured: false });
    cloudTtsMocks.listVoices.mockResolvedValue([]);
    cloudTtsMocks.prepare.mockResolvedValue({
      path: "C:\\AppData\\tts\\cloud-section.mp3",
      voiceId: "eleven-voice",
      characterCount: 7,
      timings: [
        {
          startOffset: 0,
          endOffset: 7,
          startSeconds: 0,
          endSeconds: 0.5,
        },
      ],
    });
    googleTtsMocks.status.mockResolvedValue({ configured: false });
    googleTtsMocks.saveKey.mockResolvedValue({ configured: true });
    googleTtsMocks.deleteKey.mockResolvedValue({ configured: false });
    googleTtsMocks.listVoices.mockResolvedValue([]);
    googleTtsMocks.prepare.mockResolvedValue({
      path: "C:\\AppData\\tts\\google-section.mp3",
      voiceId: "en-US-Wavenet-A",
      characterCount: 7,
    });
    azureTtsMocks.status.mockResolvedValue({ configured: false });
    azureTtsMocks.regions.mockResolvedValue([
      { id: "westeurope", name: "West Europe" },
      { id: "eastus", name: "East US" },
    ]);
    azureTtsMocks.saveKey.mockResolvedValue({ configured: true });
    azureTtsMocks.deleteKey.mockResolvedValue({ configured: false });
    azureTtsMocks.listVoices.mockResolvedValue([]);
    azureTtsMocks.prepare.mockResolvedValue({
      path: "C:\\AppData\\tts\\azure-section.mp3",
      voiceId: "en-US-AvaNeural",
      characterCount: 7,
    });
    ttsMocks.listVoices.mockResolvedValue([
      {
        id: "windows-en-voice",
        name: "Local Voice",
        language: "en-US",
        gender: "female",
        isDefault: true,
      },
    ]);
    ttsMocks.prepare.mockResolvedValue({
      path: "C:\\AppData\\tts\\section.wav",
      voiceId: "windows-en-voice",
      characterCount: 54,
    });
    const ready = {
      phase: "ready",
      path: "C:\\AppData\\tts\\section.wav",
      positionSeconds: 0,
      durationSeconds: 10,
      playbackRate: 1,
      volume: 1,
      canSeek: true,
      canPause: true,
      lastError: null,
    };
    ttsMocks.load.mockResolvedValue(ready);
    ttsMocks.rate.mockResolvedValue(ready);
    ttsMocks.play.mockResolvedValue({ ...ready, phase: "playing" });
    ttsMocks.pause.mockResolvedValue({ ...ready, phase: "paused" });
    ttsMocks.snapshot.mockResolvedValue({ ...ready, phase: "playing" });
    ttsMocks.stop.mockResolvedValue({ ...ready, phase: "idle" });
  });

  it("starts a bounded local narration queue with an installed Windows voice", async () => {
    render(
      <ReaderScreen
        document={document}
        language="en"
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Read aloud" }));
    expect(
      await screen.findByRole("combobox", { name: "Windows voice" }),
    ).toHaveValue("windows-en-voice");
    fireEvent.click(screen.getByRole("button", { name: "Read this section" }));

    await waitFor(() =>
      expect(ttsMocks.prepare).toHaveBeenCalledWith(
        "Opening",
        "windows-en-voice",
        1,
      ),
    );
    expect(ttsMocks.load).toHaveBeenCalledWith("C:\\AppData\\tts\\section.wav");
    expect(ttsMocks.play).toHaveBeenCalledOnce();
    expect(within(screen.getByRole("main")).getByText("Opening")).toHaveClass(
      "reader-speech-focus",
    );
  });

  it("continues from a short section title into the first text block", async () => {
    ttsMocks.snapshot.mockResolvedValue({
      phase: "ended",
      path: "C:\\AppData\\tts\\section.wav",
      positionSeconds: 1,
      durationSeconds: 1,
      playbackRate: 1,
      volume: 1,
      canSeek: true,
      canPause: true,
      lastError: null,
    });
    render(
      <ReaderScreen
        document={document}
        language="en"
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Read aloud" }));
    expect(
      await screen.findByRole("combobox", { name: "Windows voice" }),
    ).toHaveValue("windows-en-voice");
    fireEvent.click(screen.getByRole("button", { name: "Read this section" }));

    await waitFor(
      () =>
        expect(ttsMocks.prepare).toHaveBeenCalledWith(
          "Only safe text is rendered.",
          "windows-en-voice",
          1,
        ),
      { timeout: 2_500 },
    );
    await waitFor(() => expect(ttsMocks.play).toHaveBeenCalledTimes(2), {
      timeout: 2_500,
    });
  });

  it("exports generated parts immediately into a bounded M3U8 audiobook package", async () => {
    render(
      <ReaderScreen
        document={document}
        language="en"
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Read aloud" }));
    await screen.findByRole("combobox", { name: "Windows voice" });
    fireEvent.click(screen.getByRole("button", { name: "Export narration" }));
    expect(
      screen.getByRole("group", { name: "Create a local audiobook?" }),
    ).toBeInTheDocument();
    fireEvent.click(
      screen.getByRole("button", { name: "Choose destination and start" }),
    );

    await waitFor(() =>
      expect(ttsAssetMocks.finish).toHaveBeenCalledWith("export-session"),
    );
    expect(ttsAssetMocks.choose).toHaveBeenCalledWith("A quiet fixture");
    expect(ttsAssetMocks.begin).toHaveBeenCalledWith(
      "C:\\Exports\\fixture.m3u8",
      3,
    );
    expect(ttsAssetMocks.append).toHaveBeenCalledTimes(3);
    expect(ttsAssetMocks.cancel).not.toHaveBeenCalled();
  });

  it("applies a local pronunciation rule without changing the book text", async () => {
    render(
      <ReaderScreen
        document={document}
        language="en"
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Read aloud" }));
    fireEvent.click(await screen.findByText("Pronunciation dictionary: 0"));
    fireEvent.change(screen.getByLabelText("Word or phrase in the book"), {
      target: { value: "Opening" },
    });
    fireEvent.change(screen.getByLabelText("How to pronounce it"), {
      target: { value: "Oh-pen-ing" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Add rule" }));
    fireEvent.click(screen.getByRole("button", { name: "Read this section" }));

    await waitFor(() =>
      expect(ttsMocks.prepare).toHaveBeenCalledWith(
        "Oh-pen-ing",
        "windows-en-voice",
        1,
      ),
    );
    expect(within(screen.getByRole("main")).getByText("Opening")).toBeVisible();
  });

  it("saves, renames, applies, and deletes a voice preset locally", async () => {
    render(
      <ReaderScreen
        document={document}
        language="en"
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Read aloud" }));
    const summary = await screen.findByText("Voice presets");
    fireEvent.click(summary);
    const details = summary.closest("details")!;
    fireEvent.change(within(details).getByLabelText("Preset name"), {
      target: { value: "Calm reading" },
    });
    fireEvent.click(
      within(details).getByRole("button", { name: "Save preset" }),
    );
    expect(within(details).getByLabelText("Saved preset")).not.toHaveValue("");

    fireEvent.change(within(details).getByLabelText("Preset name"), {
      target: { value: "Evening reading" },
    });
    fireEvent.click(
      within(details).getByRole("button", { name: "Update and rename" }),
    );
    expect(localStorage.getItem("aprireader.tts.preferences.v1")).toContain(
      "Evening reading",
    );
    fireEvent.click(within(details).getByRole("button", { name: "Apply" }));
    expect(screen.getByRole("alert")).toHaveTextContent(
      "Voice preset applied.",
    );
    fireEvent.click(within(details).getByRole("button", { name: "Delete" }));
    expect(localStorage.getItem("aprireader.tts.preferences.v1")).not.toContain(
      "Evening reading",
    );
  });

  it("continues narration into the next section and updates the visible reader", async () => {
    ttsMocks.snapshot.mockResolvedValue({
      phase: "ended",
      path: "C:\\AppData\\tts\\section.wav",
      positionSeconds: 10,
      durationSeconds: 10,
      playbackRate: 1,
      volume: 1,
      canSeek: true,
      canPause: true,
      lastError: null,
    });
    render(
      <ReaderScreen
        document={document}
        language="en"
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Read aloud" }));
    fireEvent.click(
      await screen.findByRole("radio", {
        name: "Whole book from this section",
      }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Read the book" }));

    await waitFor(
      () =>
        expect(
          within(screen.getByRole("main")).getByRole("heading", {
            level: 1,
            name: "Ending",
          }),
        ).toBeInTheDocument(),
      { timeout: 2_500 },
    );
    expect(ttsMocks.prepare).toHaveBeenCalledWith(
      "Ending",
      "windows-en-voice",
      1,
    );
  });

  it("does not overlap native playback polling while a snapshot request is pending", async () => {
    ttsMocks.snapshot.mockImplementation(() => new Promise(() => undefined));
    const { unmount } = render(
      <ReaderScreen
        document={document}
        language="en"
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Read aloud" }));
    fireEvent.click(
      await screen.findByRole("button", { name: "Read this section" }),
    );
    await waitFor(() => expect(ttsMocks.play).toHaveBeenCalledOnce());
    await new Promise((resolve) => window.setTimeout(resolve, 650));

    expect(ttsMocks.snapshot).toHaveBeenCalledOnce();
    unmount();
  });

  it("requires consent before sending a fragment to a BYOK cloud voice", async () => {
    cloudTtsMocks.status.mockResolvedValue({ configured: true });
    cloudTtsMocks.listVoices.mockResolvedValue([
      {
        id: "eleven-voice",
        name: "Multilingual Voice",
        language: "multilingual",
        category: "premade",
      },
    ]);
    render(
      <ReaderScreen
        document={document}
        language="en"
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Read aloud" }));
    fireEvent.click(
      await screen.findByRole("radio", {
        name: "ElevenLabs — your API key",
      }),
    );
    expect(
      await screen.findByRole("combobox", { name: "ElevenLabs voice" }),
    ).toHaveValue("eleven-voice");
    fireEvent.click(screen.getByRole("button", { name: "Read this section" }));

    expect(
      screen.getByRole("group", { name: "Send text to ElevenLabs?" }),
    ).toBeInTheDocument();
    expect(cloudTtsMocks.prepare).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Agree and start" }));

    await waitFor(() =>
      expect(cloudTtsMocks.prepare).toHaveBeenCalledWith(
        "Opening",
        "eleven-voice",
        {
          stability: 0.5,
          similarityBoost: 0.75,
          style: 0,
          speakerBoost: true,
        },
      ),
    );
    expect(ttsMocks.load).toHaveBeenCalledWith(
      "C:\\AppData\\tts\\cloud-section.mp3",
    );
  });

  it("requires separate consent before Google Cloud narration", async () => {
    googleTtsMocks.status.mockResolvedValue({ configured: true });
    googleTtsMocks.listVoices.mockResolvedValue([
      {
        id: "en-US-Wavenet-A",
        name: "en-US-Wavenet-A",
        language: "en-US",
        category: "WaveNet",
        gender: "female",
      },
    ]);
    render(
      <ReaderScreen
        document={document}
        language="en"
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Read aloud" }));
    fireEvent.click(
      await screen.findByRole("radio", {
        name: "Google Cloud TTS — your API key",
      }),
    );
    expect(
      await screen.findByRole("combobox", { name: "Google Cloud voice" }),
    ).toHaveValue("en-US-Wavenet-A");
    expect(googleTtsMocks.listVoices).toHaveBeenCalledWith("en");
    fireEvent.click(screen.getByRole("button", { name: "Read this section" }));

    expect(
      screen.getByRole("group", { name: "Send text to Google Cloud TTS?" }),
    ).toBeInTheDocument();
    expect(googleTtsMocks.prepare).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Agree and start" }));
    await waitFor(() =>
      expect(googleTtsMocks.prepare).toHaveBeenCalledWith(
        "Opening",
        "en-US-Wavenet-A",
        "en-US",
        { pitch: 0 },
      ),
    );
    expect(ttsMocks.load).toHaveBeenCalledWith(
      "C:\\AppData\\tts\\google-section.mp3",
    );
  });

  it("hands a Google key to native credential storage without persisting it", async () => {
    render(
      <ReaderScreen
        document={document}
        language="en"
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Read aloud" }));
    fireEvent.click(
      await screen.findByRole("radio", {
        name: "Google Cloud TTS — your API key",
      }),
    );
    const keyField = screen.getByLabelText("Google Cloud API key");
    fireEvent.change(keyField, { target: { value: "AIza-disposable-test" } });
    fireEvent.click(screen.getByRole("button", { name: "Save and connect" }));

    await waitFor(() =>
      expect(googleTtsMocks.saveKey).toHaveBeenCalledWith(
        "AIza-disposable-test",
      ),
    );
    expect(
      screen.queryByDisplayValue("AIza-disposable-test"),
    ).not.toBeInTheDocument();
    expect(Object.values(localStorage).join("\n")).not.toContain(
      "AIza-disposable-test",
    );
  });

  it("uses an allowlisted Azure region and separate first-send consent", async () => {
    azureTtsMocks.status.mockResolvedValue({ configured: true });
    azureTtsMocks.listVoices.mockResolvedValue([
      {
        id: "en-US-AvaNeural",
        name: "Ava",
        language: "en-US",
        category: "Neural",
        gender: "female",
      },
    ]);
    render(
      <ReaderScreen
        document={document}
        language="en"
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Read aloud" }));
    fireEvent.click(
      await screen.findByRole("radio", {
        name: "Azure AI Speech — your resource key",
      }),
    );
    expect(
      await screen.findByRole("combobox", { name: "Azure Speech voice" }),
    ).toHaveValue("en-US-AvaNeural");
    expect(azureTtsMocks.listVoices).toHaveBeenCalledWith("westeurope", "en");
    fireEvent.click(screen.getByRole("button", { name: "Read this section" }));
    expect(
      screen.getByRole("group", { name: "Send text to Azure AI Speech?" }),
    ).toBeInTheDocument();
    expect(azureTtsMocks.prepare).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Agree and start" }));
    await waitFor(() =>
      expect(azureTtsMocks.prepare).toHaveBeenCalledWith(
        "Opening",
        "en-US-AvaNeural",
        "en-US",
        "westeurope",
        { pitchPercent: 0 },
      ),
    );
  });

  it("hands an Azure key to native storage and keeps only the region locally", async () => {
    render(
      <ReaderScreen
        document={document}
        language="en"
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Read aloud" }));
    fireEvent.click(
      await screen.findByRole("radio", {
        name: "Azure AI Speech — your resource key",
      }),
    );
    const keyField = screen.getByLabelText("Azure Speech resource key");
    fireEvent.change(keyField, { target: { value: "azure-disposable-test" } });
    fireEvent.click(screen.getByRole("button", { name: "Save and connect" }));
    await waitFor(() =>
      expect(azureTtsMocks.saveKey).toHaveBeenCalledWith(
        "azure-disposable-test",
      ),
    );
    expect(Object.values(localStorage).join("\n")).not.toContain(
      "azure-disposable-test",
    );
    expect(screen.getByLabelText("Azure Speech resource region")).toHaveValue(
      "westeurope",
    );
  });

  it("hands a BYOK key to native credential storage and clears the field", async () => {
    render(
      <ReaderScreen
        document={document}
        language="en"
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Read aloud" }));
    fireEvent.click(
      await screen.findByRole("radio", {
        name: "ElevenLabs — your API key",
      }),
    );
    const keyField = screen.getByLabelText("ElevenLabs API key");
    fireEvent.change(keyField, { target: { value: "sk_disposable_test" } });
    fireEvent.click(screen.getByRole("button", { name: "Save and connect" }));

    await waitFor(() =>
      expect(cloudTtsMocks.saveKey).toHaveBeenCalledWith("sk_disposable_test"),
    );
    expect(
      screen.queryByDisplayValue("sk_disposable_test"),
    ).not.toBeInTheDocument();
    expect(Object.values(localStorage).join("\n")).not.toContain(
      "sk_disposable_test",
    );
  });

  it("asks for consent once before opening a selected phrase in a translator", async () => {
    render(
      <ReaderScreen
        document={document}
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );
    const paragraph = within(screen.getByRole("main")).getByText(
      "Only safe text is rendered.",
    );
    const range = window.document.createRange();
    range.setStart(paragraph.firstChild!, 0);
    range.setEnd(paragraph.firstChild!, 9);
    const selected = window.getSelection()!;
    selected.removeAllRanges();
    selected.addRange(range);
    fireEvent.mouseUp(paragraph);

    fireEvent.click(screen.getByRole("button", { name: "Translate" }));
    fireEvent.click(screen.getByRole("menuitem", { name: "Google Translate" }));
    expect(
      screen.getByRole("group", { name: "Open an external translator?" }),
    ).toHaveTextContent("The selected text will be sent to Google Translate");
    expect(openUrl).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "Continue" }));
    await waitFor(() => expect(openUrl).toHaveBeenCalledOnce());
    const opened = new URL(openUrl.mock.calls[0]![0]);
    expect(opened.origin).toBe("https://translate.google.com");
    expect(opened.searchParams.get("text")).toBe("Only safe");
    expect(opened.searchParams.get("sl")).toBe("en");
    expect(opened.searchParams.get("tl")).toBe("ru");
    expect(
      localStorage.getItem("aprireader.external-translation-consent.v1"),
    ).toBe("accepted");
  });

  it("keeps a quote saved when clipboard access is unavailable", async () => {
    render(
      <ReaderScreen
        document={document}
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );
    const paragraph = within(screen.getByRole("main")).getByText(
      "Only safe text is rendered.",
    );
    const range = window.document.createRange();
    range.setStart(paragraph.firstChild!, 0);
    range.setEnd(paragraph.firstChild!, 9);
    const selected = window.getSelection()!;
    selected.removeAllRanges();
    selected.addRange(range);
    fireEvent.mouseUp(paragraph);

    fireEvent.click(screen.getByRole("button", { name: "Copy quote" }));

    expect(
      await screen.findByText(
        "Quote saved, but clipboard access was unavailable",
      ),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Annotations" }));
    expect(await screen.findAllByText("Only safe")).toHaveLength(2);
  });

  it("places initial keyboard focus on the reader toolbar", () => {
    render(
      <ReaderScreen
        document={document}
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );
    expect(
      screen.getByRole("button", { name: "Back to library" }),
    ).toHaveFocus();
  });

  it("tags book language and respects optional reader announcements", () => {
    const { container } = render(
      <ReaderScreen
        document={document}
        t={t}
        language="en-US"
        screenReaderSupport={false}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );
    expect(container.querySelector(".reader-screen")).toHaveAttribute(
      "lang",
      "en-US",
    );
    expect(container.querySelector(".reader-page-status")).toHaveAttribute(
      "aria-live",
      "off",
    );
    expect(screen.queryByText("Chapter: Opening")).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Back to library" }),
    ).toBeInTheDocument();
  });

  it("renders normalized book text and navigates through the table of contents", () => {
    render(
      <ReaderScreen
        document={document}
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );
    expect(
      within(screen.getByRole("main")).getByText("Only safe text is rendered."),
    ).toBeInTheDocument();
    expect(screen.getByText("Page 1 of 2")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Table of contents" }));
    fireEvent.click(screen.getByRole("button", { name: /02 Ending/ }));
    expect(screen.getByText("The end.")).toBeInTheDocument();
  });

  it("persists typography choices locally", () => {
    render(
      <ReaderScreen
        document={document}
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Text settings" }));
    const size = screen.getByRole("slider", { name: /Font size/ });
    fireEvent.change(size, { target: { value: "24" } });
    expect(localStorage.getItem("aprireader.reader.preferences")).toContain(
      '"fontSize":24',
    );
    fireEvent.change(screen.getByLabelText("Reading font"), {
      target: { value: "literata" },
    });
    fireEvent.change(screen.getByLabelText("Font style"), {
      target: { value: "italic" },
    });
    fireEvent.change(screen.getByLabelText("Font weight"), {
      target: { value: "800" },
    });
    fireEvent.click(
      screen.getByRole("checkbox", { name: /Bionic highlighting/ }),
    );
    const spread = screen.getByRole("button", { name: /Book spread/ });
    expect(spread).toHaveAttribute("aria-pressed", "false");
    fireEvent.click(spread);
    expect(spread).toHaveAttribute("aria-pressed", "true");
    expect(localStorage.getItem("aprireader.reader.preferences")).toContain(
      '"fontChoice":"literata"',
    );
    expect(localStorage.getItem("aprireader.reader.preferences")).toContain(
      '"fontStyle":"italic"',
    );
    expect(localStorage.getItem("aprireader.reader.preferences")).toContain(
      '"fontWeight":800',
    );
    expect(localStorage.getItem("aprireader.reader.preferences")).toContain(
      '"bionicReading":true',
    );
    expect(localStorage.getItem("aprireader.reader.preferences")).toContain(
      '"layout":"spread"',
    );
    expect(screen.getByRole("main")).toHaveClass("layout-spread");
    expect(
      globalThis.document.querySelector(".reader-document-spread"),
    ).toBeInTheDocument();
    expect(screen.getByText("Pages 1–2 of 4")).toBeInTheDocument();
  });

  it("offers bundled families and only real weights for the selected font", () => {
    render(
      <ReaderScreen
        document={document}
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Text settings" }));
    const family = screen.getByLabelText("Reading font");
    for (const name of [
      "Literata",
      "Lora",
      "Merriweather",
      "Source Serif 4",
      "Charis SIL",
      "IBM Plex Serif",
    ]) {
      expect(within(family).getByRole("option", { name })).toBeInTheDocument();
    }

    fireEvent.change(family, { target: { value: "lora" } });
    const weight = screen.getByLabelText("Font weight");
    expect(within(weight).getAllByRole("option")).toHaveLength(4);
    expect(
      within(weight).queryByRole("option", { name: "Black" }),
    ).not.toBeInTheDocument();
    expect(within(weight).getByRole("option", { name: "Bold" })).toHaveValue(
      "700",
    );
    expect(
      screen.getByText(/The quick brown fox jumps over the lazy dog/),
    ).toHaveClass("reader-font-preview");
  });

  it("renders optional focus highlighting without changing the text", () => {
    localStorage.setItem(
      "aprireader.reader.preferences",
      JSON.stringify({ bionicReading: true }),
    );
    render(
      <ReaderScreen
        document={document}
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );
    expect(
      globalThis.document.querySelector('[data-reader-block="0"]'),
    ).toHaveTextContent("Only safe text is rendered.");
    expect(
      globalThis.document.querySelector(".bionic-word strong"),
    ).toHaveTextContent("On");
  });

  it("turns a wheel gesture at the chapter edge into the next chapter", () => {
    render(
      <ReaderScreen
        document={document}
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );
    const reader = screen.getByRole("main");
    Object.defineProperties(reader, {
      clientHeight: { configurable: true, value: 600 },
      scrollHeight: { configurable: true, value: 1200 },
      scrollTop: { configurable: true, value: 600, writable: true },
    });
    fireEvent.wheel(reader, { deltaY: 120 });
    expect(screen.getByText("The end.")).toBeInTheDocument();
  });

  it("turns a wheel gesture into one horizontal spread", () => {
    localStorage.setItem(
      "aprireader.reader.preferences",
      JSON.stringify({ layout: "spread" }),
    );
    render(
      <ReaderScreen
        document={document}
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );
    const reader = screen.getByRole("main");
    const scrollTo = vi.fn();
    Object.defineProperties(reader, {
      clientWidth: { configurable: true, value: 800 },
      scrollWidth: { configurable: true, value: 2400 },
      scrollLeft: { configurable: true, value: 0, writable: true },
      scrollTo: { configurable: true, value: scrollTo },
    });
    fireEvent.wheel(reader, { deltaY: 120 });
    expect(scrollTo).toHaveBeenCalledWith({
      left: 800,
      behavior: "smooth",
    });
  });

  it("creates a bookmark and exposes it in the annotations panel", async () => {
    render(
      <ReaderScreen
        document={document}
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Bookmark" }));
    fireEvent.click(screen.getByRole("button", { name: "Annotations" }));
    await waitFor(() =>
      expect(screen.getByText("opening · 1")).toBeInTheDocument(),
    );
    expect(
      screen.getByRole("button", { name: "Export as Markdown" }),
    ).toBeEnabled();
  });

  it("opens the full-text search panel without leaving the reader", () => {
    render(
      <ReaderScreen
        document={document}
        t={t}
        onClose={vi.fn()}
        onProgress={vi.fn()}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Search this book" }));
    expect(
      screen.getByRole("searchbox", { name: "Search this book" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Find" })).toBeInTheDocument();
  });
});
