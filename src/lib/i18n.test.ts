import { describe, expect, it } from "vitest";
import { DEFAULT_LANGUAGE, t } from "./i18n";

describe("i18n", () => {
  it("uses simplified Chinese as the default language", () => {
    expect(DEFAULT_LANGUAGE).toBe("zh-CN");
    expect(t("nav.overview", DEFAULT_LANGUAGE)).toBe("概览");
  });

  it("translates labels to English", () => {
    expect(t("nav.overview", "en")).toBe("Overview");
    expect(t("settings.localData.clearHistory", "en")).toBe("Clear history data");
    expect(t("metrics.gpu", "en")).toBe("GPU");
    expect(t("metrics.power", "en")).toBe("Power");
  });

  it("shows seconds units on sampling settings labels", () => {
    expect(t("settings.sampleInterval", "zh-CN")).toBe("采样间隔（秒）");
    expect(t("settings.saveInterval", "zh-CN")).toBe("本地保存间隔（秒）");
    expect(t("settings.sampleInterval", "en")).toBe("Sample interval (seconds)");
    expect(t("settings.saveInterval", "en")).toBe("Local save interval (seconds)");
  });
});
