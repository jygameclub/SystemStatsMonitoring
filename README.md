# SystemStatsMonitoring

本地优先的跨平台系统监控桌面工具。V0.1 使用 Tauri v2 + React + Rust + SQLite，聚焦本机实时指标、历史采样和桌面托盘入口。

## 功能范围

已包含：

- macOS / Windows 桌面应用骨架。
- 系统托盘 / macOS 菜单栏入口。
- CPU、内存、磁盘容量、网络速率采集。
- SQLite 本地历史记录。
- Overview 实时面板。
- History 最近 1 小时 / 24 小时曲线。
- Settings 采样间隔和保存间隔配置。

暂不包含：

- 温度、电池细节、进程 Top N。
- 局域网多机器同步。
- S3/R2 同步。
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

数据库保存本机指标采样和用户设置。V0.1 不依赖网络服务。
