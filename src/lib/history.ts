export type HistoryRange = "1h" | "24h";

const RANGE_MS: Record<HistoryRange, number> = {
  "1h": 60 * 60 * 1000,
  "24h": 24 * 60 * 60 * 1000,
};

export function historyRangeToStartTs(range: HistoryRange, now: number): number {
  return now - RANGE_MS[range];
}
