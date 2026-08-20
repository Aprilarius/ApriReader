import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  conservativePlatformCapabilities,
  getPlatformCapabilities,
} from "./platform";

const invokeMock = vi.hoisted(() => vi.fn());
const isTauriMock = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
  isTauri: isTauriMock,
}));

describe("platform capabilities", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    isTauriMock.mockReset();
  });

  it("uses a fail-closed capability set outside Tauri", async () => {
    isTauriMock.mockReturnValue(false);

    await expect(getPlatformCapabilities()).resolves.toEqual(
      conservativePlatformCapabilities,
    );
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("loads the authoritative capability set from Rust", async () => {
    isTauriMock.mockReturnValue(true);
    const windows = {
      ...conservativePlatformCapabilities,
      platform: "windows",
      desktop: true,
      filesystemPaths: true,
      watchedFolders: true,
      systemTray: true,
    };
    invokeMock.mockResolvedValue(windows);

    await expect(getPlatformCapabilities()).resolves.toEqual(windows);
    expect(invokeMock).toHaveBeenCalledWith("get_platform_capabilities");
  });
});
