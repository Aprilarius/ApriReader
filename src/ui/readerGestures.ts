export function readerSwipeDirection(
  startX: number,
  startY: number,
  endX: number,
  endY: number,
) {
  const horizontal = endX - startX;
  const vertical = endY - startY;
  if (
    Math.abs(horizontal) < 64 ||
    Math.abs(horizontal) < Math.abs(vertical) * 1.5
  ) {
    return 0;
  }
  return horizontal < 0 ? 1 : -1;
}
