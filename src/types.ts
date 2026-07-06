import type { AppSettings } from "./lib/settings";

export type SensorCategory =
  | "temperature"
  | "voltage"
  | "current"
  | "power"
  | "energy"
  | "fan";

export type SensorUnit =
  | "celsius"
  | "volt"
  | "ampere"
  | "watt"
  | "watt_hour"
  | "percent";

export interface SensorReading {
  id: string;
  label: string;
  category: SensorCategory;
  value: number;
  unit: SensorUnit;
}

export interface MetricSample {
  id: number | null;
  device_id: string;
  ts: number;
  cpu_usage: number | null;
  memory_used: number | null;
  memory_total: number | null;
  disk_used: number | null;
  disk_total: number | null;
  network_rx: number | null;
  network_tx: number | null;
  gpu_usage: number | null;
  gpu_memory_total: number | null;
  gpu_name: string | null;
  temperature_celsius: number | null;
  power_watts: number | null;
  sensor_readings: SensorReading[];
}

export interface DeviceInfo {
  id: string;
  name: string;
  os: string;
  arch: string;
  agent_version: string;
}

export interface HistoryQuery {
  device_id?: string;
  start_ts: number;
  end_ts: number;
}

export interface LocalDataStats {
  database_path: string;
  database_size_bytes: number;
  sample_count: number;
}

export type { AppSettings };
