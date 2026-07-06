import { describe, expect, it } from "vitest";
import {
  DEFAULT_SETTINGS,
  normalizeSettings,
  validateSettings,
} from "./settings";

describe("settings utilities", () => {
  it("accepts the default sampling settings", () => {
    expect(DEFAULT_SETTINGS).toEqual({
      sample_interval_sec: 1,
      local_save_interval_sec: 5,
    });
    expect(validateSettings(DEFAULT_SETTINGS)).toEqual({ valid: true });
  });

  it("rejects sample intervals outside the supported range", () => {
    expect(
      validateSettings({ sample_interval_sec: 0, local_save_interval_sec: 5 }),
    ).toEqual({
      valid: false,
      message: "采样间隔必须在 1 到 60 秒之间",
    });

    expect(
      validateSettings({ sample_interval_sec: 61, local_save_interval_sec: 65 }),
    ).toEqual({
      valid: false,
      message: "采样间隔必须在 1 到 60 秒之间",
    });
  });

  it("rejects save intervals outside the supported range", () => {
    expect(
      validateSettings({ sample_interval_sec: 1, local_save_interval_sec: 4 }),
    ).toEqual({
      valid: false,
      message: "保存间隔必须在 5 到 300 秒之间",
    });

    expect(
      validateSettings({ sample_interval_sec: 1, local_save_interval_sec: 301 }),
    ).toEqual({
      valid: false,
      message: "保存间隔必须在 5 到 300 秒之间",
    });
  });

  it("rejects a save interval shorter than the sample interval", () => {
    expect(
      validateSettings({ sample_interval_sec: 10, local_save_interval_sec: 5 }),
    ).toEqual({
      valid: false,
      message: "保存间隔不能小于采样间隔",
    });
  });

  it("normalizes string form input to numeric settings", () => {
    expect(
      normalizeSettings({
        sample_interval_sec: "2",
        local_save_interval_sec: "10",
      }),
    ).toEqual({
      sample_interval_sec: 2,
      local_save_interval_sec: 10,
    });
  });
});
