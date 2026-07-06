import type { AppSettings } from "./settings";
import type { MetricSample } from "../types";

export function filterSampleBySettings(
  sample: MetricSample,
  settings: AppSettings,
): MetricSample {
  return {
    ...sample,
    cpu_usage: settings.metrics.cpu ? sample.cpu_usage : null,
    memory_used: settings.metrics.memory ? sample.memory_used : null,
    memory_total: settings.metrics.memory ? sample.memory_total : null,
    disk_used: settings.metrics.disk ? sample.disk_used : null,
    disk_total: settings.metrics.disk ? sample.disk_total : null,
    network_rx: settings.metrics.network ? sample.network_rx : null,
    network_tx: settings.metrics.network ? sample.network_tx : null,
    gpu_usage: settings.metrics.gpu ? sample.gpu_usage : null,
    gpu_memory_total: settings.metrics.gpu ? sample.gpu_memory_total : null,
    gpu_name: settings.metrics.gpu ? sample.gpu_name : null,
    temperature_celsius: settings.metrics.temperature
      ? sample.temperature_celsius
      : null,
    power_watts: settings.metrics.power ? sample.power_watts : null,
    sensor_readings: (sample.sensor_readings ?? []).filter((reading) => {
      if (reading.category === "temperature") {
        return settings.metrics.temperature;
      }

      if (
        reading.category === "power" ||
        reading.category === "current" ||
        reading.category === "voltage" ||
        reading.category === "energy"
      ) {
        return settings.metrics.power;
      }

      return true;
    }),
  };
}
