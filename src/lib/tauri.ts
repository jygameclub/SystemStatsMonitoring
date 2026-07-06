import { invoke } from "@tauri-apps/api/core";
import type { AppSettings } from "./settings";
import { DEFAULT_SETTINGS, normalizeSettings } from "./settings";
import type {
  DeviceInfo,
  HistoryQuery,
  LocalDataStats,
  MetricSample,
  S3SyncReport,
} from "../types";

const DEMO_DEVICE: DeviceInfo = {
  id: "browser-preview",
  name: "Browser Preview",
  os: "web",
  arch: "preview",
  agent_version: "0.1.0",
};

const historyStore: MetricSample[] = [];

function hasTauriRuntime(): boolean {
  return "__TAURI_INTERNALS__" in window;
}

export async function invokeCommand<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (hasTauriRuntime()) {
    return invoke<T>(command, args);
  }

  return demoInvoke<T>(command, args);
}

async function demoInvoke<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  switch (command) {
    case "get_device_info":
      return readDemoDevice() as T;
    case "list_devices":
      return [readDemoDevice()] as T;
    case "get_settings":
      return readDemoSettings() as T;
    case "update_settings": {
      const settings = args?.settings as AppSettings;
      localStorage.setItem("demo-settings", JSON.stringify(settings));
      return settings as T;
    }
    case "get_latest_metrics": {
      const sample = createDemoSample();
      return sample as T;
    }
    case "test_s3_connection": {
      return undefined as T;
    }
    case "sync_s3_now": {
      return {
        uploaded_days: historyStore.length > 0 ? 1 : 0,
        downloaded_devices: 0,
        imported_samples: 0,
      } satisfies S3SyncReport as T;
    }
    case "get_local_data_stats": {
      return {
        database_path: "browser localStorage preview",
        database_size_bytes: estimateDemoStorageBytes(),
        sample_count: historyStore.length,
      } satisfies LocalDataStats as T;
    }
    case "clear_local_metric_samples": {
      const deleted = historyStore.length;
      historyStore.splice(0, historyStore.length);
      return deleted as T;
    }
    case "save_metric_sample": {
      const sample = args?.sample as MetricSample | undefined;
      if (sample) {
        historyStore.push(sample);
        const cutoff = Date.now() - 30 * 24 * 60 * 60 * 1000;
        while (historyStore.length > 0 && historyStore[0].ts < cutoff) {
          historyStore.shift();
        }
      }
      return undefined as T;
    }
    case "get_metric_history": {
      const query = args?.query as HistoryQuery;
      return historyStore
        .filter((sample) => sample.ts >= query.start_ts && sample.ts <= query.end_ts)
        .slice(-720) as T;
    }
    default:
      throw new Error(`Unknown demo command: ${command}`);
  }
}

function estimateDemoStorageBytes(): number {
  const settings = localStorage.getItem("demo-settings") ?? "";
  return settings.length + JSON.stringify(historyStore).length;
}

function readDemoSettings(): AppSettings {
  const raw = localStorage.getItem("demo-settings");
  if (!raw) {
    return DEFAULT_SETTINGS;
  }

  try {
    return normalizeSettings(JSON.parse(raw) as AppSettings);
  } catch {
    return DEFAULT_SETTINGS;
  }
}

function readDemoDevice(): DeviceInfo {
  const settings = readDemoSettings();
  const name = settings.machine_name.trim() || DEMO_DEVICE.name;

  return {
    ...DEMO_DEVICE,
    id: sanitizeDeviceId(`${name}-${DEMO_DEVICE.os}-${DEMO_DEVICE.arch}`),
    name,
  };
}

function createDemoSample(): MetricSample {
  const now = Date.now();
  const phase = now / 10_000;
  const cpu = 34 + Math.sin(phase) * 18 + Math.random() * 4;
  const memoryTotal = 32 * 1024 * 1024 * 1024;
  const memoryUsed = memoryTotal * (0.52 + Math.sin(phase / 2) * 0.07);
  const diskTotal = 1_000 * 1024 * 1024 * 1024;
  const diskUsed = diskTotal * 0.62;
  const gpuMemoryTotal = 16 * 1024 * 1024 * 1024;
  const gpuUsage = 42 + Math.sin(phase / 1.7) * 16 + Math.random() * 4;
  const temperature = 58 + Math.sin(phase / 2.2) * 8 + Math.random() * 2;
  const power = 22 + Math.sin(phase / 1.9) * 7 + Math.random() * 2;

  return {
    id: null,
    device_id: readDemoDevice().id,
    ts: now,
    cpu_usage: clamp(cpu, 0, 100),
    memory_used: Math.round(memoryUsed),
    memory_total: memoryTotal,
    disk_used: Math.round(diskUsed),
    disk_total: diskTotal,
    network_rx: 800_000 + Math.random() * 4_000_000,
    network_tx: 120_000 + Math.random() * 800_000,
    gpu_usage: clamp(gpuUsage, 0, 100),
    gpu_memory_total: gpuMemoryTotal,
    gpu_name: "Demo GPU",
    temperature_celsius: clamp(temperature, 20, 110),
    power_watts: Math.max(0, power),
    sensor_readings: [
      {
        id: "cpu-performance-core-1",
        label: "CPU performance core 1",
        category: "temperature",
        value: clamp(temperature + 2.4, 20, 110),
        unit: "celsius",
      },
      {
        id: "gpu-1",
        label: "GPU 1",
        category: "temperature",
        value: clamp(temperature - 7.2, 20, 110),
        unit: "celsius",
      },
      {
        id: "nand",
        label: "NAND",
        category: "temperature",
        value: clamp(temperature - 18.5, 20, 110),
        unit: "celsius",
      },
      {
        id: "system-voltage-in",
        label: "DC In",
        category: "voltage",
        value: 12.06,
        unit: "volt",
      },
      {
        id: "system-current-in",
        label: "DC In",
        category: "current",
        value: 3.11,
        unit: "ampere",
      },
      {
        id: "system-power-in",
        label: "System Total",
        category: "power",
        value: Math.max(0, power),
        unit: "watt",
      },
    ],
  };
}

function sanitizeDeviceId(value: string): string {
  const id = value
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");

  return id || "browser-preview";
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}
