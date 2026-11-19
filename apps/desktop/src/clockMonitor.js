export const CLOCK_JUMP_THRESHOLD_MS = 5_000;

/**
 * Detects a wall-clock step by comparing wall-clock elapsed time with a
 * monotonic elapsed-time measurement. This is diagnostic only: it cannot
 * establish whether the initial device clock is correct or NTP-synchronised.
 */
export function hasWallClockJump(
  previousWallMs,
  currentWallMs,
  previousMonotonicMs,
  currentMonotonicMs,
  thresholdMs = CLOCK_JUMP_THRESHOLD_MS,
) {
  const values = [previousWallMs, currentWallMs, previousMonotonicMs, currentMonotonicMs, thresholdMs];
  if (!values.every(Number.isFinite) || thresholdMs < 0) return false;

  const wallElapsed = currentWallMs - previousWallMs;
  const monotonicElapsed = currentMonotonicMs - previousMonotonicMs;
  return Math.abs(wallElapsed - monotonicElapsed) > thresholdMs;
}
