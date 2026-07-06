import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { LineChart as EchartsLineChart } from "echarts/charts";
import {
  GridComponent,
  LegendComponent,
  TooltipComponent,
} from "echarts/components";
import * as echarts from "echarts/core";
import { CanvasRenderer } from "echarts/renderers";
import type { AppSettings, DeviceInfo, MetricSample } from "./types";
import {
  formatBytes,
  formatNetworkRate,
  formatPercent,
  formatTimeLabel,
} from "./lib/format";
import { historyRangeToStartTs, type HistoryRange } from "./lib/history";
import {
  DEFAULT_SETTINGS,
  normalizeSettings,
  validateSettings,
} from "./lib/settings";
import { invokeCommand } from "./lib/tauri";

echarts.use([
  CanvasRenderer,
  GridComponent,
  LegendComponent,
  TooltipComponent,
  EchartsLineChart,
]);

type View = "overview" | "history" | "settings";

const viewLabels: Record<View, string> = {
  overview: "Overview",
  history: "History",
  settings: "Settings",
};

function App() {
  const [activeView, setActiveView] = useState<View>("overview");
  const [device, setDevice] = useState<DeviceInfo | null>(null);
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS);
  const [latest, setLatest] = useState<MetricSample | null>(null);
  const [history, setHistory] = useState<MetricSample[]>([]);
  const [historyRange, setHistoryRange] = useState<HistoryRange>("1h");
  const [error, setError] = useState<string | null>(null);
  const latestRef = useRef<MetricSample | null>(null);

  useEffect(() => {
    latestRef.current = latest;
  }, [latest]);

  useEffect(() => {
    let cancelled = false;

    async function bootstrap() {
      try {
        const [deviceInfo, savedSettings] = await Promise.all([
          invokeCommand<DeviceInfo>("get_device_info"),
          invokeCommand<AppSettings>("get_settings"),
        ]);

        if (!cancelled) {
          setDevice(deviceInfo);
          setSettings(savedSettings);
        }
      } catch (unknownError) {
        if (!cancelled) {
          setError(errorMessage(unknownError));
        }
      }
    }

    void bootstrap();
    return () => {
      cancelled = true;
    };
  }, []);

  const refreshLatest = useCallback(async () => {
    try {
      const sample = await invokeCommand<MetricSample>("get_latest_metrics");
      setLatest(sample);
      setError(null);
    } catch (unknownError) {
      setError(errorMessage(unknownError));
    }
  }, []);

  useEffect(() => {
    void refreshLatest();
    const timer = window.setInterval(
      () => void refreshLatest(),
      settings.sample_interval_sec * 1000,
    );

    return () => window.clearInterval(timer);
  }, [refreshLatest, settings.sample_interval_sec]);

  useEffect(() => {
    const timer = window.setInterval(() => {
      const sample = latestRef.current;
      if (!sample) {
        return;
      }

      void invokeCommand<void>("save_metric_sample", { sample }).catch(
        (unknownError) => setError(errorMessage(unknownError)),
      );
    }, settings.local_save_interval_sec * 1000);

    return () => window.clearInterval(timer);
  }, [settings.local_save_interval_sec]);

  const refreshHistory = useCallback(async () => {
    const now = Date.now();
    const startTs = historyRangeToStartTs(historyRange, now);

    try {
      const samples = await invokeCommand<MetricSample[]>("get_metric_history", {
        query: {
          device_id: device?.id,
          start_ts: startTs,
          end_ts: now,
        },
      });
      setHistory(samples);
      setError(null);
    } catch (unknownError) {
      setError(errorMessage(unknownError));
    }
  }, [device?.id, historyRange]);

  useEffect(() => {
    if (activeView !== "history") {
      return;
    }

    void refreshHistory();
    const timer = window.setInterval(() => void refreshHistory(), 5_000);
    return () => window.clearInterval(timer);
  }, [activeView, refreshHistory]);

  const usage = useMemo(() => deriveUsage(latest), [latest]);

  return (
    <main className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark">S</div>
          <div>
            <div className="brand-title">SystemStats</div>
            <div className="brand-subtitle">Local Monitor</div>
          </div>
        </div>

        <nav className="nav-tabs">
          {(Object.keys(viewLabels) as View[]).map((view) => (
            <button
              key={view}
              className={activeView === view ? "nav-tab active" : "nav-tab"}
              type="button"
              onClick={() => setActiveView(view)}
            >
              {viewLabels[view]}
            </button>
          ))}
        </nav>
      </aside>

      <section className="workspace">
        <header className="topbar">
          <div>
            <h1>{device?.name ?? "Local Machine"}</h1>
            <p>
              {device
                ? `${device.os} / ${device.arch} / Agent ${device.agent_version}`
                : "Loading device profile"}
            </p>
          </div>
          <div className={latest ? "status online" : "status"}>
            <span />
            {latest ? "Online" : "Waiting"}
          </div>
        </header>

        {error ? <div className="error-banner">{error}</div> : null}

        {activeView === "overview" ? (
          <Overview latest={latest} usage={usage} />
        ) : null}
        {activeView === "history" ? (
          <History
            history={history}
            range={historyRange}
            onRangeChange={setHistoryRange}
            onRefresh={() => void refreshHistory()}
          />
        ) : null}
        {activeView === "settings" ? (
          <Settings
            settings={settings}
            onSaved={(nextSettings) => setSettings(nextSettings)}
            onError={setError}
          />
        ) : null}
      </section>
    </main>
  );
}

interface UsageSummary {
  cpu: number;
  memory: number;
  disk: number;
}

function deriveUsage(sample: MetricSample | null): UsageSummary {
  if (!sample) {
    return { cpu: 0, memory: 0, disk: 0 };
  }

  return {
    cpu: sample.cpu_usage,
    memory:
      sample.memory_total > 0
        ? (sample.memory_used / sample.memory_total) * 100
        : 0,
    disk:
      sample.disk_total > 0 ? (sample.disk_used / sample.disk_total) * 100 : 0,
  };
}

function Overview({
  latest,
  usage,
}: {
  latest: MetricSample | null;
  usage: UsageSummary;
}) {
  return (
    <div className="page-stack">
      <section className="metric-grid">
        <MetricTile label="CPU" value={formatPercent(usage.cpu)} tone="green" />
        <MetricTile label="Memory" value={formatPercent(usage.memory)} tone="teal" />
        <MetricTile label="Disk" value={formatPercent(usage.disk)} tone="amber" />
        <MetricTile
          label="Network"
          value={
            latest
              ? `↓ ${formatNetworkRate(latest.network_rx)}  ↑ ${formatNetworkRate(
                  latest.network_tx,
                )}`
              : "Waiting"
          }
          tone="blue"
        />
      </section>

      <section className="detail-band">
        <div>
          <h2>Current Snapshot</h2>
          <p>{latest ? formatTimeLabel(latest.ts) : "Waiting for first sample"}</p>
        </div>
        <div className="snapshot-grid">
          <SnapshotItem
            label="Memory Used"
            value={
              latest
                ? `${formatBytes(latest.memory_used)} / ${formatBytes(latest.memory_total)}`
                : "Waiting"
            }
          />
          <SnapshotItem
            label="Disk Used"
            value={
              latest
                ? `${formatBytes(latest.disk_used)} / ${formatBytes(latest.disk_total)}`
                : "Waiting"
            }
          />
          <SnapshotItem
            label="Download"
            value={latest ? formatNetworkRate(latest.network_rx) : "Waiting"}
          />
          <SnapshotItem
            label="Upload"
            value={latest ? formatNetworkRate(latest.network_tx) : "Waiting"}
          />
        </div>
      </section>
    </div>
  );
}

function MetricTile({
  label,
  value,
  tone,
}: {
  label: string;
  value: string;
  tone: "green" | "teal" | "amber" | "blue";
}) {
  return (
    <article className={`metric-tile ${tone}`}>
      <div className="metric-label">{label}</div>
      <div className="metric-value">{value}</div>
    </article>
  );
}

function SnapshotItem({ label, value }: { label: string; value: string }) {
  return (
    <div className="snapshot-item">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function History({
  history,
  range,
  onRangeChange,
  onRefresh,
}: {
  history: MetricSample[];
  range: HistoryRange;
  onRangeChange: (range: HistoryRange) => void;
  onRefresh: () => void;
}) {
  return (
    <div className="page-stack">
      <div className="toolbar">
        <div className="segmented">
          <button
            type="button"
            className={range === "1h" ? "active" : ""}
            onClick={() => onRangeChange("1h")}
          >
            1h
          </button>
          <button
            type="button"
            className={range === "24h" ? "active" : ""}
            onClick={() => onRangeChange("24h")}
          >
            24h
          </button>
        </div>
        <button className="secondary-button" type="button" onClick={onRefresh}>
          Refresh
        </button>
      </div>

      {history.length === 0 ? (
        <div className="empty-state">
          <h2>No history yet</h2>
          <p>Keep the app running until local samples are saved.</p>
        </div>
      ) : (
        <div className="chart-grid">
          <LineChart
            title="CPU"
            samples={history}
            series={[
              {
                name: "CPU %",
                color: "#1f9d55",
                values: history.map((sample) => sample.cpu_usage),
              },
            ]}
          />
          <LineChart
            title="Memory / Disk"
            samples={history}
            series={[
              {
                name: "Memory %",
                color: "#0f766e",
                values: history.map((sample) =>
                  sample.memory_total > 0
                    ? (sample.memory_used / sample.memory_total) * 100
                    : 0,
                ),
              },
              {
                name: "Disk %",
                color: "#b7791f",
                values: history.map((sample) =>
                  sample.disk_total > 0
                    ? (sample.disk_used / sample.disk_total) * 100
                    : 0,
                ),
              },
            ]}
          />
          <LineChart
            title="Network"
            samples={history}
            series={[
              {
                name: "Download",
                color: "#2563eb",
                values: history.map((sample) => sample.network_rx),
              },
              {
                name: "Upload",
                color: "#7c3aed",
                values: history.map((sample) => sample.network_tx),
              },
            ]}
            valueFormatter={formatNetworkRate}
          />
        </div>
      )}
    </div>
  );
}

interface ChartSeries {
  name: string;
  color: string;
  values: number[];
}

function LineChart({
  title,
  samples,
  series,
  valueFormatter = formatPercent,
}: {
  title: string;
  samples: MetricSample[];
  series: ChartSeries[];
  valueFormatter?: (value: number) => string;
}) {
  const chartRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!chartRef.current) {
      return;
    }

    const chart = echarts.init(chartRef.current);
    const labels = samples.map((sample) => formatTimeLabel(sample.ts));

    chart.setOption({
      animation: false,
      color: series.map((item) => item.color),
      grid: { left: 52, right: 24, top: 44, bottom: 36 },
      tooltip: {
        trigger: "axis",
        valueFormatter,
      },
      legend: {
        top: 4,
        right: 8,
      },
      xAxis: {
        type: "category",
        boundaryGap: false,
        data: labels,
      },
      yAxis: {
        type: "value",
        axisLabel: {
          formatter: valueFormatter,
        },
      },
      series: series.map((item) => ({
        name: item.name,
        type: "line",
        smooth: true,
        showSymbol: false,
        data: item.values,
      })),
    });

    const resize = () => chart.resize();
    window.addEventListener("resize", resize);

    return () => {
      window.removeEventListener("resize", resize);
      chart.dispose();
    };
  }, [samples, series, valueFormatter]);

  return (
    <section className="chart-panel">
      <h2>{title}</h2>
      <div ref={chartRef} className="chart-surface" />
    </section>
  );
}

function Settings({
  settings,
  onSaved,
  onError,
}: {
  settings: AppSettings;
  onSaved: (settings: AppSettings) => void;
  onError: (message: string | null) => void;
}) {
  const [form, setForm] = useState({
    sample_interval_sec: String(settings.sample_interval_sec),
    local_save_interval_sec: String(settings.local_save_interval_sec),
  });
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    setForm({
      sample_interval_sec: String(settings.sample_interval_sec),
      local_save_interval_sec: String(settings.local_save_interval_sec),
    });
  }, [settings]);

  async function saveSettings() {
    const normalized = normalizeSettings(form);
    const validation = validateSettings(normalized);

    if (!validation.valid) {
      setMessage(validation.message);
      return;
    }

    try {
      const saved = await invokeCommand<AppSettings>("update_settings", {
        settings: normalized,
      });
      onSaved(saved);
      onError(null);
      setMessage("Settings saved");
    } catch (unknownError) {
      const nextMessage = errorMessage(unknownError);
      onError(nextMessage);
      setMessage(nextMessage);
    }
  }

  return (
    <section className="settings-panel">
      <h2>Sampling</h2>
      <div className="form-grid">
        <label>
          <span>Sample interval</span>
          <input
            min={1}
            max={60}
            type="number"
            value={form.sample_interval_sec}
            onChange={(event) =>
              setForm((current) => ({
                ...current,
                sample_interval_sec: event.target.value,
              }))
            }
          />
        </label>
        <label>
          <span>Local save interval</span>
          <input
            min={5}
            max={300}
            type="number"
            value={form.local_save_interval_sec}
            onChange={(event) =>
              setForm((current) => ({
                ...current,
                local_save_interval_sec: event.target.value,
              }))
            }
          />
        </label>
      </div>
      <button className="primary-button" type="button" onClick={() => void saveSettings()}>
        Save
      </button>
      {message ? <p className="form-message">{message}</p> : null}
    </section>
  );
}

function errorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }

  return String(error);
}

export default App;
