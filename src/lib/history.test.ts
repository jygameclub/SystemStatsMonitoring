import { describe, expect, it } from "vitest";
import { historyRangeToStartTs } from "./history";

describe("history utilities", () => {
  it("converts 1h history range to a start timestamp", () => {
    const now = 1_783_317_600_000;
    expect(historyRangeToStartTs("1h", now)).toBe(now - 60 * 60 * 1000);
  });

  it("converts 24h history range to a start timestamp", () => {
    const now = 1_783_317_600_000;
    expect(historyRangeToStartTs("24h", now)).toBe(now - 24 * 60 * 60 * 1000);
  });
});
