export const CLOCK_JUMP_THRESHOLD_MS: number;

export function hasWallClockJump(
  previousWallMs: number,
  currentWallMs: number,
  previousMonotonicMs: number,
  currentMonotonicMs: number,
  thresholdMs?: number,
): boolean;
