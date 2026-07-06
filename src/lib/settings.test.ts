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
      machine_name: "",
      language: "zh-CN",
      metrics: {
        cpu: true,
        memory: true,
        disk: true,
        network: true,
        gpu: true,
        temperature: true,
        power: true,
        battery: true,
      },
      s3_sync: {
        enabled: false,
        endpoint_url: "",
        region: "us-east-1",
        bucket: "",
        access_key_id: "",
        secret_access_key: "",
        prefix: "system-stats-monitoring",
        sync_interval_min: 10,
        path_style: true,
      },
    });
    expect(validateSettings(DEFAULT_SETTINGS)).toEqual({ valid: true });
  });

  it("rejects sample intervals outside the supported range", () => {
    expect(
      validateSettings({
        ...DEFAULT_SETTINGS,
        sample_interval_sec: 0,
        local_save_interval_sec: 5,
      }),
    ).toEqual({
      valid: false,
      message: "采样间隔必须在 1 到 60 秒之间",
    });

    expect(
      validateSettings({
        ...DEFAULT_SETTINGS,
        sample_interval_sec: 61,
        local_save_interval_sec: 65,
      }),
    ).toEqual({
      valid: false,
      message: "采样间隔必须在 1 到 60 秒之间",
    });
  });

  it("rejects save intervals outside the supported range", () => {
    expect(
      validateSettings({
        ...DEFAULT_SETTINGS,
        sample_interval_sec: 1,
        local_save_interval_sec: 4,
      }),
    ).toEqual({
      valid: false,
      message: "保存间隔必须在 5 到 300 秒之间",
    });

    expect(
      validateSettings({
        ...DEFAULT_SETTINGS,
        sample_interval_sec: 1,
        local_save_interval_sec: 301,
      }),
    ).toEqual({
      valid: false,
      message: "保存间隔必须在 5 到 300 秒之间",
    });
  });

  it("rejects a save interval shorter than the sample interval", () => {
    expect(
      validateSettings({
        ...DEFAULT_SETTINGS,
        sample_interval_sec: 10,
        local_save_interval_sec: 5,
      }),
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
        machine_name: "Studio Mac",
        language: "en",
        metrics: {
          cpu: true,
          memory: false,
          disk: true,
          network: false,
          gpu: false,
          temperature: true,
          power: false,
          battery: false,
        },
        s3_sync: {
          enabled: true,
          endpoint_url: "https://s3.example.com",
          region: "auto",
          bucket: "monitoring",
          access_key_id: "ak",
          secret_access_key: "sk",
          prefix: "team-a",
          sync_interval_min: "15",
          path_style: false,
        },
      }),
    ).toEqual({
      sample_interval_sec: 2,
      local_save_interval_sec: 10,
      machine_name: "Studio Mac",
      language: "en",
      metrics: {
        cpu: true,
        memory: false,
        disk: true,
        network: false,
        gpu: false,
        temperature: true,
        power: false,
        battery: false,
      },
      s3_sync: {
        enabled: true,
        endpoint_url: "https://s3.example.com",
        region: "auto",
        bucket: "monitoring",
        access_key_id: "ak",
        secret_access_key: "sk",
        prefix: "team-a",
        sync_interval_min: 15,
        path_style: false,
      },
    });
  });

  it("rejects incomplete enabled S3 sync settings", () => {
    expect(
      validateSettings({
        ...DEFAULT_SETTINGS,
        s3_sync: {
          ...DEFAULT_SETTINGS.s3_sync,
          enabled: true,
        },
      }),
    ).toEqual({
      valid: false,
      message: "启用 S3 同步后必须填写 S3 地址、Bucket、Access Key 和 Secret Key",
    });
  });

  it("rejects invalid S3 sync intervals", () => {
    expect(
      validateSettings({
        ...DEFAULT_SETTINGS,
        s3_sync: {
          ...DEFAULT_SETTINGS.s3_sync,
          enabled: true,
          endpoint_url: "https://s3.example.com",
          bucket: "monitoring",
          access_key_id: "ak",
          secret_access_key: "sk",
          sync_interval_min: 0,
        },
      }),
    ).toEqual({
      valid: false,
      message: "S3 同步间隔必须在 1 到 1440 分钟之间",
    });
  });

  it("rejects unsupported languages", () => {
    expect(
      validateSettings({
        ...DEFAULT_SETTINGS,
        language: "ja-JP" as never,
      }),
    ).toEqual({
      valid: false,
      message: "不支持的语言",
    });
  });
});
