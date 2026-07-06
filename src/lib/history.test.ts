import { describe, expect, it } from "vitest";
import { downsampleHistory, historyRangeToStartTs } from "./history";

describe("history utilities", () => {
  it("converts 1h history range to a start timestamp", () => {
    const now = 1_783_317_600_000;
    expect(historyRangeToStartTs("1h", now)).toBe(now - 60 * 60 * 1000);
  });

  it("converts 24h history range to a start timestamp", () => {
    const now = 1_783_317_600_000;
    expect(historyRangeToStartTs("24h", now)).toBe(now - 24 * 60 * 60 * 1000);
  });

  it("converts week history range to a start timestamp", () => {
    const now = 1_783_317_600_000;
    expect(historyRangeToStartTs("week", now)).toBe(
      now - 7 * 24 * 60 * 60 * 1000,
    );
  });

  it("converts month history range to a start timestamp", () => {
    const now = 1_783_317_600_000;
    expect(historyRangeToStartTs("month", now)).toBe(
      now - 30 * 24 * 60 * 60 * 1000,
    );
  });

  it("downsamples history while keeping the first and last samples", () => {
    expect(downsampleHistory([0, 1, 2, 3, 4], 3)).toEqual([0, 2, 4]);
  });
});
