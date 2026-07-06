# SystemStatsMonitoring

本地优先的跨平台系统监控桌面工具。V0.1 使用 Tauri v2 + React + Rust + SQLite，聚焦本机实时指标、历史采样和桌面托盘入口。

## 功能范围

已包含：

- macOS / Windows 桌面应用骨架。
- 系统托盘 / macOS 菜单栏入口。
- CPU、内存、磁盘容量、磁盘读写速度、网络速率采集。
- 内存同时记录已用字节数和总字节数，不只记录百分比。
- GPU 信息采集：macOS / Windows 当前记录显卡名称和显存容量；GPU 使用率字段已预留，采不到时显示未支持。
- 温度采集：使用系统组件传感器，记录当前最高有效温度，并在 Overview 显示传感器明细列表。
- 功率采集：macOS 当前优先读取 `PowerTelemetryData.SystemPowerIn / SystemVoltageIn / SystemCurrentIn`，否则通过电池电压和瞬时电流估算功率；系统不暴露相关字段时显示未支持。
- 传感器明细：支持温度、电压、电流、功率、能耗、风扇分类，能读取到 CPU / GPU / NAND 等标签时会按传感器逐条展示。
- SQLite 本地历史记录。
- Rust 后台任务负责采样和本地保存，窗口被遮挡或前端 WebView 暂停时也会继续写入历史。
- Overview 实时面板。
- History 支持最近 1 小时 / 24 小时 / week / 月切换。
- History 支持查看磁盘读取和写入速度曲线，单位为每秒字节数。
- History 图表会对大范围数据做前端均匀抽样，避免 week / 月视图渲染过多点导致卡顿。
- Settings 采样间隔和保存间隔配置。
- Settings 可设置本机名称，用于本机展示、采样设备 ID 和 S3 同步目录。
- 默认简体中文，支持切换 English。
- Settings 可查看本地数据库大小、历史采样数量和数据库路径。
- 支持清理本地历史采样，保留用户设置。
- 支持 CPU、内存、磁盘、网络、GPU、温度、功率、电池指标开关。
- 支持可选 S3 同步：本机上传历史采样到自己的设备目录，也可以拉取同一 S3 目录下其他机器的历史数据。

权限说明：

- macOS 普通桌面应用可以读取一部分 `ioreg` 供电遥测和系统组件温度。
- 截图里类似 CPU Power / GPU Power / ANE Power / RAM Power 这类分项功率，macOS 通常要求 `powermetrics` 超级用户权限或安装 privileged helper。
- 当前版本不会静默要求管理员密码；后续如果要做到和 iStat Menus / TG Pro 一样完整，需要增加可选 helper 安装流程。

暂不包含：

- GPU 使用率、CPU/GPU/ANE/RAM 分项功率、完整风扇控制、电池细节、进程 Top N。
- 局域网多机器同步。
- 登录、账号和官方云同步。

## 环境要求

- Node.js 20 或更高版本。
- npm。
- Rust 工具链。
- macOS 构建需要 Xcode Command Line Tools 或 Xcode。
- Windows 构建需要 Microsoft C++ Build Tools。

## 安装依赖

```bash
npm install
```

## 前端开发预览

普通浏览器预览会使用 demo 数据，适合检查 UI：

```bash
npm run dev
```

默认地址：

```text
http://localhost:1420
```

## 桌面应用开发

启动 Tauri 桌面应用：

```bash
npm run tauri:dev
```

## 测试

运行 TypeScript 单元测试：

```bash
npm test
```

运行 Rust 单元测试：

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

## 构建

构建前端：

```bash
npm run build
```

构建桌面应用：

```bash
npm run tauri:build
```

## 数据保存位置

桌面应用会在 Tauri app data 目录创建 SQLite 数据库：

```text
system-stats-monitoring.sqlite3
```

数据库保存本机指标采样和用户设置。历史采样默认保留最近 30 天，支持月视图。V0.1 不依赖网络服务。

## 本地数据管理

在 `设置` 页面可以查看：

```text
数据库大小
历史采样数量
数据库路径
```

点击 `清理历史数据` 只会删除 `metric_samples` 历史采样，不会删除语言、采样间隔和指标开关设置。

## 指标开关

默认开启：

```text
CPU
内存
磁盘
网络
GPU
温度
功率
电池
```

关闭某项指标后：

```text
Overview 不显示对应指标
History 不显示对应曲线
保存历史采样时对应字段写入 null
```

## S3 同步

在 `设置` 页面可以配置：

```text
本机名称
启用 S3 同步
S3 地址
Region
Bucket
目录
Access Key
Secret Key
同步间隔（分钟）
Path-style 地址
```

对象目录结构：

```text
{目录}/devices/{device_id}/manifest.json
{目录}/devices/{device_id}/samples/{YYYY-MM-DD}.json
```

同步行为：

```text
本机数据仍然优先写入本地 SQLite
启用 S3 后按同步间隔自动上报
点击「立即同步」会上传本机最近 30 天历史，并拉取同目录下其他机器数据
History 页面可以通过设备下拉切换查看本机或其他机器历史
```

兼容目标：

```text
AWS S3
Cloudflare R2
MinIO
其他标准 S3 兼容对象存储
```

GPU 当前原生记录显卡名称和显存容量，GPU 使用率显示为未支持。温度会记录最高有效传感器温度；功率会在系统能提供 power telemetry 或电池电压/电流时记录瓦数。采不到的数据会显示为未支持，并在历史采样中写入 null。

磁盘速度记录的是采样周期内所有已挂载磁盘的读写字节数，并归一化为 B/s、KB/s、MB/s 等速率展示。首次采样没有上一个周期可比较，会显示为 0 B/s。
