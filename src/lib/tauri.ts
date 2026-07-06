import { invoke } from "@tauri-apps/api/core";
import type { AppSettings } from "./settings";
import { DEFAULT_SETTINGS } from "./settings";
import type { DeviceInfo, HistoryQuery, MetricSample } from "../types";

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
      return DEMO_DEVICE as T;
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
    case "save_metric_sample": {
      const sample = args?.sample as MetricSample | undefined;
      if (sample) {
        historyStore.push(sample);
        const cutoff = Date.now() - 24 * 60 * 60 * 1000;
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

function readDemoSettings(): AppSettings {
  const raw = localStorage.getItem("demo-settings");
  if (!raw) {
    return DEFAULT_SETTINGS;
  }

  try {
    return JSON.parse(raw) as AppSettings;
  } catch {
    return DEFAULT_SETTINGS;
  }
}

function createDemoSample(): MetricSample {
  const now = Date.now();
  const phase = now / 10_000;
  const cpu = 34 + Math.sin(phase) * 18 + Math.random() * 4;
  const memoryTotal = 32 * 1024 * 1024 * 1024;
  const memoryUsed = memoryTotal * (0.52 + Math.sin(phase / 2) * 0.07);
  const diskTotal = 1_000 * 1024 * 1024 * 1024;
  const diskUsed = diskTotal * 0.62;

  return {
    id: null,
    device_id: DEMO_DEVICE.id,
    ts: now,
    cpu_usage: clamp(cpu, 0, 100),
    memory_used: Math.round(memoryUsed),
    memory_total: memoryTotal,
    disk_used: Math.round(diskUsed),
    disk_total: diskTotal,
    network_rx: 800_000 + Math.random() * 4_000_000,
    network_tx: 120_000 + Math.random() * 800_000,
  };
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}
