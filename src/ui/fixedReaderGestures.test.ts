import { describe, expect, it } from "vitest";
import {
  clampReaderZoom,
  fixedReaderSwipeDirection,
  touchDistance,
} from "./fixedReaderGestures";

describe("fixed reader gestures", () => {
  it("distinguishes page swipes from vertical reading gestures", () => {
    expect(
      fixedReaderSwipeDirection(
        { clientX: 240, clientY: 100 },
        { clientX: 100, clientY: 110 },
      ),
    ).toBe(1);
    expect(
      fixedReaderSwipeDirection(
        { clientX: 100, clientY: 100 },
        { clientX: 40, clientY: 220 },
      ),
    ).toBe(0);
  });

  it("measures pinch distance and bounds zoom", () => {
    expect(
      touchDistance({ clientX: 0, clientY: 0 }, { clientX: 30, clientY: 40 }),
    ).toBe(50);
    expect(clampReaderZoom(0.1)).toBe(0.5);
    expect(clampReaderZoom(4)).toBe(3);
  });
});
