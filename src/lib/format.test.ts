import { describe, expect, it } from "vitest";
import {
  formatBytes,
  formatNetworkRate,
  formatPower,
  formatPercent,
  formatTemperature,
  formatTimeLabel,
} from "./format";

describe("format utilities", () => {
  it("formats bytes using binary units", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(1024)).toBe("1.0 KB");
    expect(formatBytes(1536)).toBe("1.5 KB");
    expect(formatBytes(1024 * 1024)).toBe("1.0 MB");
  });

  it("formats percentage values with one decimal place", () => {
    expect(formatPercent(0)).toBe("0.0%");
    expect(formatPercent(12.345)).toBe("12.3%");
    expect(formatPercent(100)).toBe("100.0%");
  });

  it("formats network bytes per second", () => {
    expect(formatNetworkRate(512)).toBe("512 B/s");
    expect(formatNetworkRate(2048)).toBe("2.0 KB/s");
    expect(formatNetworkRate(5 * 1024 * 1024)).toBe("5.0 MB/s");
  });

  it("formats temperatures", () => {
    expect(formatTemperature(63.456)).toBe("63.5°C");
  });

  it("formats power in watts", () => {
    expect(formatPower(18.24)).toBe("18.2 W");
  });

  it("formats timestamps as compact local time labels", () => {
    expect(formatTimeLabel(new Date("2026-07-06T09:08:00+09:00").getTime())).toBe(
      "09:08",
    );
  });
});
