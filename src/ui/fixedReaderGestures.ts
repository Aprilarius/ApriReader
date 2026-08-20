export type TouchPoint = { clientX: number; clientY: number };

export function touchDistance(first: TouchPoint, second: TouchPoint) {
  return Math.hypot(
    second.clientX - first.clientX,
    second.clientY - first.clientY,
  );
}

export function fixedReaderSwipeDirection(start: TouchPoint, end: TouchPoint) {
  const horizontal = end.clientX - start.clientX;
  const vertical = end.clientY - start.clientY;
  if (
    Math.abs(horizontal) < 64 ||
    Math.abs(horizontal) < Math.abs(vertical) * 1.5
  ) {
    return 0;
  }
  return horizontal < 0 ? 1 : -1;
}

export function clampReaderZoom(value: number) {
  return Math.min(3, Math.max(0.5, value));
}
