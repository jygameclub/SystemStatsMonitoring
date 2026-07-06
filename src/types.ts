import type { AppSettings } from "./lib/settings";

export interface MetricSample {
  id: number | null;
  device_id: string;
  ts: number;
  cpu_usage: number;
  memory_used: number;
  memory_total: number;
  disk_used: number;
  disk_total: number;
  network_rx: number;
  network_tx: number;
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

export type { AppSettings };
