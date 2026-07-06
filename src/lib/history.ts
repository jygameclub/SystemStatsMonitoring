export type HistoryRange = "1h" | "24h" | "week" | "month";

export const MAX_HISTORY_POINTS = 2_000;

const RANGE_MS: Record<HistoryRange, number> = {
  "1h": 60 * 60 * 1000,
  "24h": 24 * 60 * 60 * 1000,
  week: 7 * 24 * 60 * 60 * 1000,
  month: 30 * 24 * 60 * 60 * 1000,
};

export function historyRangeToStartTs(range: HistoryRange, now: number): number {
  return now - RANGE_MS[range];
}

export function downsampleHistory<T>(
  samples: T[],
  maxPoints = MAX_HISTORY_POINTS,
): T[] {
  if (samples.length <= maxPoints || maxPoints <= 0) {
    return samples;
  }

  if (maxPoints === 1) {
    return [samples[0]];
  }

  const step = (samples.length - 1) / (maxPoints - 1);

  return Array.from({ length: maxPoints }, (_, index) => {
    const sampleIndex = Math.round(index * step);
    return samples[sampleIndex];
  });
}
