# SystemStatsMonitoring V0.1 本地桌面 MVP 设计

## 目标

构建一个可在 macOS 和 Windows 上运行的本地优先系统监控桌面应用。V0.1 聚焦“本机可用”：托盘入口、实时面板、基础系统指标采集、SQLite 历史记录和最近 1 小时 / 24 小时图表。

## 范围

### 包含

- Tauri v2 桌面应用骨架。
- React + TypeScript 前端面板。
- Rust 后端命令，负责系统指标采集和 SQLite 读写。
- macOS 顶部菜单栏 / Windows 系统托盘入口。
- 点击托盘图标打开主窗口。
- 采集 CPU、内存、磁盘容量、网络速率。
- 每 1 秒刷新实时指标。
- 每 5 秒保存一条本地历史采样。
- 最近 1 小时和最近 24 小时历史图表。
- 设置页支持修改采样间隔和本地保存间隔。
- README 说明本地开发、运行和构建方式。

### 不包含

- 温度采集。
- 电池细节。
- 进程 Top N。
- 局域网多机器发现、配对和访问。
- S3/R2 同步。
- 登录、账号、官方云同步。
- 告警通知。
- Windows 实机验证。

## 技术栈

- 桌面壳：Tauri v2。
- 后端：Rust。
- 系统指标：`sysinfo`。
- 本地数据库：SQLite，通过 `rusqlite` 访问。
- 前端：React、TypeScript、Vite。
- 图表：ECharts。
- 样式：普通 CSS，避免引入重量级 UI 框架。
- 测试：Rust 单元测试，TypeScript 单元测试。

## 架构

```text
Tauri Desktop App
  ├─ Tray
  │   ├─ macOS menu bar
  │   └─ Windows system tray
  ├─ React UI
  │   ├─ Overview page
  │   ├─ History page
  │   └─ Settings page
  └─ Rust Backend
      ├─ Metrics collector
      ├─ SQLite repository
      ├─ Settings repository
      └─ Tauri commands
```

本机采集和保存完全在桌面端完成，不依赖网络。云同步和局域网功能后续通过新增模块接入，不影响 V0.1 的本地数据模型。

## 模块设计

### Metrics Collector

职责：

- 调用 `sysinfo` 读取 CPU、内存、磁盘和网络数据。
- 计算 CPU 使用率、内存使用率、磁盘使用率。
- 通过相邻两次网络累计字节差值计算上传 / 下载速率。
- 返回统一的 `MetricSample` 结构。

接口：

```rust
pub struct MetricSample {
    pub device_id: String,
    pub ts: i64,
    pub cpu_usage: f64,
    pub memory_used: u64,
    pub memory_total: u64,
    pub disk_used: u64,
    pub disk_total: u64,
    pub network_rx: f64,
    pub network_tx: f64,
}
```

### SQLite Repository

职责：

- 初始化数据库和表结构。
- 保存原始采样。
- 查询指定时间范围内的采样。
- 保存和读取用户设置。
- 删除超过 24 小时的原始采样，避免数据库无限增长。

数据库位置：

- 使用 Tauri app data 目录。
- 文件名为 `system-stats-monitoring.sqlite3`。

表结构：

```sql
CREATE TABLE IF NOT EXISTS devices (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  os TEXT NOT NULL,
  arch TEXT NOT NULL,
  agent_version TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  last_seen_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS metric_samples (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  device_id TEXT NOT NULL,
  ts INTEGER NOT NULL,
  cpu_usage REAL NOT NULL,
  memory_used INTEGER NOT NULL,
  memory_total INTEGER NOT NULL,
  disk_used INTEGER NOT NULL,
  disk_total INTEGER NOT NULL,
  network_rx REAL NOT NULL,
  network_tx REAL NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_metric_samples_device_ts
ON metric_samples(device_id, ts);

CREATE TABLE IF NOT EXISTS app_settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
```

### Tauri Commands

前端只通过 Tauri command 与 Rust 后端交互。

命令：

```rust
get_latest_metrics() -> MetricSample
save_metric_sample(sample: MetricSample) -> ()
get_metric_history(range: HistoryRange) -> Vec<MetricSample>
get_settings() -> AppSettings
update_settings(settings: AppSettings) -> AppSettings
```

### React UI

页面：

- `Overview`：显示当前 CPU、内存、磁盘、网络速率和本机设备信息。
- `History`：切换最近 1 小时 / 24 小时，展示 CPU、内存、磁盘和网络图表。
- `Settings`：修改采样间隔和本地保存间隔。

主布局：

```text
┌─────────────────────────────────────────┐
│ Header: device name + status             │
├───────────────┬─────────────────────────┤
│ Sidebar tabs  │ Current page             │
│ Overview      │                         │
│ History       │                         │
│ Settings      │                         │
└───────────────┴─────────────────────────┘
```

视觉方向：

- 面板式运维工具风格。
- 信息密度适中，优先可扫读。
- 不做营销页。
- 不做复杂动效。

## 数据流

```text
App starts
  ↓
Rust initializes app data dir and SQLite
  ↓
React starts polling get_latest_metrics every sample_interval_sec
  ↓
React updates Overview and in-memory latest state
  ↓
Every local_save_interval_sec React calls save_metric_sample
  ↓
History page queries SQLite with get_metric_history
  ↓
Charts render returned samples
```

V0.1 由前端定时触发采样和保存，降低后台复杂度。后续做局域网或云同步时，再把定时任务下沉到 Rust 后台 worker。

## 设置

默认值：

```json
{
  "sample_interval_sec": 1,
  "local_save_interval_sec": 5
}
```

校验规则：

- `sample_interval_sec` 最小 1，最大 60。
- `local_save_interval_sec` 最小 5，最大 300。
- 保存间隔不能小于采样间隔。

无效输入时，前端显示错误信息，不调用 `update_settings`。

## 错误处理

- 后端命令返回结构化错误字符串，前端在页面顶部显示。
- SQLite 初始化失败时，主窗口显示不可继续的错误状态。
- 指标采集失败时，显示错误状态并保留上一次成功数据。
- 历史查询为空时，显示空状态，不显示失败。

## 测试策略

Rust：

- 测试设置校验。
- 测试 SQLite 初始化。
- 测试保存并查询采样。
- 测试按时间范围查询。
- 测试过期采样清理。

TypeScript：

- 测试设置校验逻辑。
- 测试历史范围转换。
- 测试指标格式化函数。

手动验证：

- `npm run dev` 可以启动前端。
- `npm run tauri dev` 可以打开桌面应用。
- Overview 每秒刷新。
- History 能看到保存后的曲线。
- Settings 修改间隔后生效。

## 验收标准

- 仓库包含完整 Tauri + React + Rust 工程。
- macOS 本机可以运行 `npm run tauri dev` 打开应用。
- 托盘图标存在，点击可打开主窗口。
- Overview 显示 CPU、内存、磁盘、网络实时数据。
- SQLite 数据库自动创建。
- 运行 20 秒后，History 能看到至少 3 条历史采样。
- 最近 1 小时和最近 24 小时切换可用。
- Settings 能保存合法间隔，拒绝非法间隔。
- Rust 测试通过。
- TypeScript 测试通过。
- README 包含安装、开发、测试和构建命令。
