import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { LineChart as EchartsLineChart } from "echarts/charts";
import {
  GridComponent,
  LegendComponent,
  TooltipComponent,
} from "echarts/components";
import * as echarts from "echarts/core";
import { CanvasRenderer } from "echarts/renderers";
import type {
  AppSettings,
  DeviceInfo,
  LocalDataStats,
  MetricSample,
} from "./types";
import {
  formatBytes,
  formatNetworkRate,
  formatPercent,
  formatTimeLabel,
} from "./lib/format";
import { historyRangeToStartTs, type HistoryRange } from "./lib/history";
import { t } from "./lib/i18n";
import { filterSampleBySettings } from "./lib/metrics";
import {
  DEFAULT_SETTINGS,
  normalizeSettings,
  validateSettings,
  type MetricSettings,
  type SupportedLanguage,
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
type ChartValue = number | null;

const navKeys: Record<View, Parameters<typeof t>[0]> = {
  overview: "nav.overview",
  history: "nav.history",
  settings: "nav.settings",
};

function App() {
  const [activeView, setActiveView] = useState<View>("overview");
  const [device, setDevice] = useState<DeviceInfo | null>(null);
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS);
  const [latest, setLatest] = useState<MetricSample | null>(null);
  const [history, setHistory] = useState<MetricSample[]>([]);
  const [historyRange, setHistoryRange] = useState<HistoryRange>("1h");
  const [localData, setLocalData] = useState<LocalDataStats | null>(null);
  const [error, setError] = useState<string | null>(null);
  const latestRef = useRef<MetricSample | null>(null);
  const settingsRef = useRef<AppSettings>(settings);

  useEffect(() => {
    latestRef.current = latest;
  }, [latest]);

  useEffect(() => {
    settingsRef.current = settings;
  }, [settings]);

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

      const filtered = filterSampleBySettings(sample, settingsRef.current);
      void invokeCommand<void>("save_metric_sample", { sample: filtered }).catch(
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

  const refreshLocalData = useCallback(async () => {
    try {
      const stats = await invokeCommand<LocalDataStats>("get_local_data_stats");
      setLocalData(stats);
      setError(null);
    } catch (unknownError) {
      setError(errorMessage(unknownError));
    }
  }, []);

  useEffect(() => {
    if (activeView !== "history") {
      return;
    }

    void refreshHistory();
    const timer = window.setInterval(() => void refreshHistory(), 5_000);
    return () => window.clearInterval(timer);
  }, [activeView, refreshHistory]);

  useEffect(() => {
    if (activeView !== "settings") {
      return;
    }

    void refreshLocalData();
  }, [activeView, refreshLocalData]);

  const usage = useMemo(() => deriveUsage(latest), [latest]);
  const language = settings.language;

  return (
    <main className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark">S</div>
          <div>
            <div className="brand-title">SystemStats</div>
            <div className="brand-subtitle">{t("app.localMonitor", language)}</div>
          </div>
        </div>

        <nav className="nav-tabs">
          {(Object.keys(navKeys) as View[]).map((view) => (
            <button
              key={view}
              className={activeView === view ? "nav-tab active" : "nav-tab"}
              type="button"
              onClick={() => setActiveView(view)}
            >
              {t(navKeys[view], language)}
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
                : t("device.loading", language)}
            </p>
          </div>
          <div className={latest ? "status online" : "status"}>
            <span />
            {latest ? t("common.online", language) : t("common.waiting", language)}
          </div>
        </header>

        {error ? <div className="error-banner">{error}</div> : null}

        {activeView === "overview" ? (
          <Overview
            latest={latest}
            usage={usage}
            settings={settings}
            language={language}
          />
        ) : null}
        {activeView === "history" ? (
          <History
            history={history}
            range={historyRange}
            settings={settings}
            language={language}
            onRangeChange={setHistoryRange}
            onRefresh={() => void refreshHistory()}
          />
        ) : null}
        {activeView === "settings" ? (
          <Settings
            settings={settings}
            localData={localData}
            language={language}
            onSaved={(nextSettings) => setSettings(nextSettings)}
            onError={setError}
            onLocalDataChanged={() => void refreshLocalData()}
          />
        ) : null}
      </section>
    </main>
  );
}

interface UsageSummary {
  cpu: number | null;
  memory: number | null;
  disk: number | null;
}

function deriveUsage(sample: MetricSample | null): UsageSummary {
  if (!sample) {
    return { cpu: null, memory: null, disk: null };
  }

  return {
    cpu: sample.cpu_usage,
    memory:
      sample.memory_used !== null &&
      sample.memory_total !== null &&
      sample.memory_total > 0
        ? (sample.memory_used / sample.memory_total) * 100
        : null,
    disk:
      sample.disk_used !== null &&
      sample.disk_total !== null &&
      sample.disk_total > 0
        ? (sample.disk_used / sample.disk_total) * 100
        : null,
  };
}

function Overview({
  latest,
  usage,
  settings,
  language,
}: {
  latest: MetricSample | null;
  usage: UsageSummary;
  settings: AppSettings;
  language: SupportedLanguage;
}) {
  return (
    <div className="page-stack">
      <section className="metric-grid">
        {settings.metrics.cpu ? (
          <MetricTile
            label={t("metrics.cpu", language)}
            value={nullablePercent(usage.cpu, language)}
            tone="green"
          />
        ) : null}
        {settings.metrics.memory ? (
          <MetricTile
            label={t("metrics.memory", language)}
            value={nullablePercent(usage.memory, language)}
            tone="teal"
          />
        ) : null}
        {settings.metrics.disk ? (
          <MetricTile
            label={t("metrics.disk", language)}
            value={nullablePercent(usage.disk, language)}
            tone="amber"
          />
        ) : null}
        {settings.metrics.network ? (
          <MetricTile
            label={t("metrics.network", language)}
            value={
              latest?.network_rx !== null &&
              latest?.network_rx !== undefined &&
              latest?.network_tx !== null &&
              latest?.network_tx !== undefined
                ? `↓ ${formatNetworkRate(latest.network_rx)}  ↑ ${formatNetworkRate(
                    latest.network_tx,
                  )}`
                : t("common.waiting", language)
            }
            tone="blue"
          />
        ) : null}
        {settings.metrics.temperature ? (
          <MetricTile
            label={t("metrics.temperature", language)}
            value={t("common.notSupported", language)}
            tone="gray"
          />
        ) : null}
        {settings.metrics.battery ? (
          <MetricTile
            label={t("metrics.battery", language)}
            value={t("common.notSupported", language)}
            tone="gray"
          />
        ) : null}
      </section>

      <section className="detail-band">
        <div>
          <h2>{t("overview.currentSnapshot", language)}</h2>
          <p>
            {latest
              ? formatTimeLabel(latest.ts)
              : t("overview.waitingFirstSample", language)}
          </p>
        </div>
        <div className="snapshot-grid">
          {settings.metrics.memory ? (
            <SnapshotItem
              label={t("metrics.memoryUsed", language)}
              value={
                latest?.memory_used !== null &&
                latest?.memory_used !== undefined &&
                latest?.memory_total !== null &&
                latest?.memory_total !== undefined
                  ? `${formatBytes(latest.memory_used)} / ${formatBytes(latest.memory_total)}`
                  : t("common.waiting", language)
              }
            />
          ) : null}
          {settings.metrics.disk ? (
            <SnapshotItem
              label={t("metrics.diskUsed", language)}
              value={
                latest?.disk_used !== null &&
                latest?.disk_used !== undefined &&
                latest?.disk_total !== null &&
                latest?.disk_total !== undefined
                  ? `${formatBytes(latest.disk_used)} / ${formatBytes(latest.disk_total)}`
                  : t("common.waiting", language)
              }
            />
          ) : null}
          {settings.metrics.network ? (
            <>
              <SnapshotItem
                label={t("metrics.download", language)}
                value={
                  latest?.network_rx !== null && latest?.network_rx !== undefined
                    ? formatNetworkRate(latest.network_rx)
                    : t("common.waiting", language)
                }
              />
              <SnapshotItem
                label={t("metrics.upload", language)}
                value={
                  latest?.network_tx !== null && latest?.network_tx !== undefined
                    ? formatNetworkRate(latest.network_tx)
                    : t("common.waiting", language)
                }
              />
            </>
          ) : null}
        </div>
      </section>
    </div>
  );
}

function nullablePercent(value: number | null, language: SupportedLanguage): string {
  return value === null ? t("common.waiting", language) : formatPercent(value);
}

function MetricTile({
  label,
  value,
  tone,
}: {
  label: string;
  value: string;
  tone: "green" | "teal" | "amber" | "blue" | "gray";
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
  settings,
  language,
  onRangeChange,
  onRefresh,
}: {
  history: MetricSample[];
  range: HistoryRange;
  settings: AppSettings;
  language: SupportedLanguage;
  onRangeChange: (range: HistoryRange) => void;
  onRefresh: () => void;
}) {
  const charts = buildHistoryCharts(history, settings, language);

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
          {t("history.refresh", language)}
        </button>
      </div>

      {history.length === 0 || charts.length === 0 ? (
        <div className="empty-state">
          <h2>{t("history.emptyTitle", language)}</h2>
          <p>{t("history.emptyBody", language)}</p>
        </div>
      ) : (
        <div className="chart-grid">
          {charts.map((chart) => (
            <LineChart
              key={chart.title}
              title={chart.title}
              samples={history}
              series={chart.series}
              valueFormatter={chart.valueFormatter}
            />
          ))}
        </div>
      )}
    </div>
  );
}

interface HistoryChart {
  title: string;
  series: ChartSeries[];
  valueFormatter?: (value: number) => string;
}

function buildHistoryCharts(
  history: MetricSample[],
  settings: AppSettings,
  language: SupportedLanguage,
): HistoryChart[] {
  const charts: HistoryChart[] = [];

  if (settings.metrics.cpu) {
    charts.push({
      title: t("metrics.cpu", language),
      series: [
        {
          name: t("history.cpuSeries", language),
          color: "#1f9d55",
          values: history.map((sample) => sample.cpu_usage),
        },
      ],
    });
  }

  const memoryDiskSeries: ChartSeries[] = [];
  if (settings.metrics.memory) {
    memoryDiskSeries.push({
      name: t("history.memorySeries", language),
      color: "#0f766e",
      values: history.map((sample) =>
        sample.memory_used !== null &&
        sample.memory_total !== null &&
        sample.memory_total > 0
          ? (sample.memory_used / sample.memory_total) * 100
          : null,
      ),
    });
  }
  if (settings.metrics.disk) {
    memoryDiskSeries.push({
      name: t("history.diskSeries", language),
      color: "#b7791f",
      values: history.map((sample) =>
        sample.disk_used !== null &&
        sample.disk_total !== null &&
        sample.disk_total > 0
          ? (sample.disk_used / sample.disk_total) * 100
          : null,
      ),
    });
  }
  if (memoryDiskSeries.length > 0) {
    charts.push({
      title: t("history.memoryDisk", language),
      series: memoryDiskSeries,
    });
  }

  if (settings.metrics.network) {
    charts.push({
      title: t("history.networkSeries", language),
      valueFormatter: formatNetworkRate,
      series: [
        {
          name: t("metrics.download", language),
          color: "#2563eb",
          values: history.map((sample) => sample.network_rx),
        },
        {
          name: t("metrics.upload", language),
          color: "#7c3aed",
          values: history.map((sample) => sample.network_tx),
        },
      ],
    });
  }

  return charts;
}

interface ChartSeries {
  name: string;
  color: string;
  values: ChartValue[];
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
        valueFormatter: (value: unknown) =>
          typeof value === "number" ? valueFormatter(value) : "-",
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
          formatter: (value: number) => valueFormatter(value),
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
  localData,
  language,
  onSaved,
  onError,
  onLocalDataChanged,
}: {
  settings: AppSettings;
  localData: LocalDataStats | null;
  language: SupportedLanguage;
  onSaved: (settings: AppSettings) => void;
  onError: (message: string | null) => void;
  onLocalDataChanged: () => void;
}) {
  const [form, setForm] = useState({
    sample_interval_sec: String(settings.sample_interval_sec),
    local_save_interval_sec: String(settings.local_save_interval_sec),
    language: settings.language,
    metrics: settings.metrics,
  });
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    setForm({
      sample_interval_sec: String(settings.sample_interval_sec),
      local_save_interval_sec: String(settings.local_save_interval_sec),
      language: settings.language,
      metrics: settings.metrics,
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
      setMessage(t("settings.saved", saved.language));
    } catch (unknownError) {
      const nextMessage = errorMessage(unknownError);
      onError(nextMessage);
      setMessage(nextMessage);
    }
  }

  async function clearHistoryData() {
    if (!window.confirm(t("common.confirmClear", language))) {
      return;
    }

    try {
      await invokeCommand<number>("clear_local_metric_samples");
      onLocalDataChanged();
      setMessage(t("settings.localData.cleared", language));
    } catch (unknownError) {
      const nextMessage = errorMessage(unknownError);
      onError(nextMessage);
      setMessage(nextMessage);
    }
  }

  function updateMetric(metric: keyof MetricSettings, enabled: boolean) {
    setForm((current) => ({
      ...current,
      metrics: {
        ...current.metrics,
        [metric]: enabled,
      },
    }));
  }

  return (
    <div className="settings-stack">
      <section className="settings-panel">
        <h2>{t("settings.sampling", language)}</h2>
        <div className="form-grid">
          <label>
            <span>{t("settings.sampleInterval", language)}</span>
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
            <span>{t("settings.saveInterval", language)}</span>
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
          <label>
            <span>{t("settings.language", language)}</span>
            <select
              value={form.language}
              onChange={(event) =>
                setForm((current) => ({
                  ...current,
                  language: event.target.value as SupportedLanguage,
                }))
              }
            >
              <option value="zh-CN">简体中文</option>
              <option value="en">English</option>
            </select>
          </label>
        </div>
      </section>

      <section className="settings-panel">
        <h2>{t("settings.metrics", language)}</h2>
        <div className="toggle-grid">
          {metricToggleItems(language).map((item) => (
            <label key={item.key} className="toggle-row">
              <span>
                <strong>{item.label}</strong>
                {item.unsupported ? (
                  <em>{t("settings.unsupportedHint", language)}</em>
                ) : null}
              </span>
              <input
                checked={form.metrics[item.key]}
                type="checkbox"
                onChange={(event) => updateMetric(item.key, event.target.checked)}
              />
            </label>
          ))}
        </div>
        <button className="primary-button" type="button" onClick={() => void saveSettings()}>
          {t("settings.save", language)}
        </button>
        {message ? <p className="form-message">{message}</p> : null}
      </section>

      <section className="settings-panel">
        <h2>{t("settings.localData.title", language)}</h2>
        <div className="snapshot-grid">
          <SnapshotItem
            label={t("settings.localData.databaseSize", language)}
            value={
              localData ? formatBytes(localData.database_size_bytes) : t("common.waiting", language)
            }
          />
          <SnapshotItem
            label={t("settings.localData.sampleCount", language)}
            value={localData ? String(localData.sample_count) : t("common.waiting", language)}
          />
          <SnapshotItem
            label={t("settings.localData.databasePath", language)}
            value={localData?.database_path ?? t("common.waiting", language)}
          />
        </div>
        <button
          className="danger-button"
          type="button"
          onClick={() => void clearHistoryData()}
        >
          {t("settings.localData.clearHistory", language)}
        </button>
      </section>
    </div>
  );
}

function metricToggleItems(language: SupportedLanguage): Array<{
  key: keyof MetricSettings;
  label: string;
  unsupported?: boolean;
}> {
  return [
    { key: "cpu", label: t("metrics.cpu", language) },
    { key: "memory", label: t("metrics.memory", language) },
    { key: "disk", label: t("metrics.disk", language) },
    { key: "network", label: t("metrics.network", language) },
    { key: "temperature", label: t("metrics.temperature", language), unsupported: true },
    { key: "battery", label: t("metrics.battery", language), unsupported: true },
  ];
}

function errorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }

  return String(error);
}

export default App;
