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
  });
});
