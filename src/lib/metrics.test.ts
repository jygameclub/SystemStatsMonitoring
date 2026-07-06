import { describe, expect, it } from "vitest";
import { DEFAULT_SETTINGS } from "./settings";
import { filterSampleBySettings } from "./metrics";
import type { MetricSample } from "../types";

const sample: MetricSample = {
  id: null,
  device_id: "test-device",
  ts: 1_783_317_600_000,
  cpu_usage: 35,
  memory_used: 4_000,
  memory_total: 8_000,
  disk_used: 100_000,
  disk_total: 200_000,
  network_rx: 120,
  network_tx: 80,
  gpu_usage: 48,
  gpu_memory_total: 16_000,
  gpu_name: "Apple M4 Pro",
  temperature_celsius: 63.5,
  power_watts: 18.2,
  sensor_readings: [
    {
      id: "cpu-core-1",
      label: "CPU performance core 1",
      category: "temperature",
      value: 63.5,
      unit: "celsius",
    },
    {
      id: "system-power",
      label: "System Total",
      category: "power",
      value: 18.2,
      unit: "watt",
    },
    {
      id: "dc-in-voltage",
      label: "DC In",
      category: "voltage",
      value: 12.06,
      unit: "volt",
    },
  ],
};

describe("metrics utilities", () => {
  it("keeps every metric enabled by default", () => {
    expect(filterSampleBySettings(sample, DEFAULT_SETTINGS)).toEqual(sample);
  });

  it("sets disabled metrics to null before saving", () => {
    expect(
      filterSampleBySettings(sample, {
        ...DEFAULT_SETTINGS,
        metrics: {
          ...DEFAULT_SETTINGS.metrics,
          cpu: false,
          network: false,
          gpu: false,
          temperature: false,
          power: false,
        },
      }),
    ).toEqual({
      ...sample,
      cpu_usage: null,
      network_rx: null,
      network_tx: null,
      gpu_usage: null,
      gpu_memory_total: null,
      gpu_name: null,
      temperature_celsius: null,
      power_watts: null,
      sensor_readings: [],
    });
  });
});
