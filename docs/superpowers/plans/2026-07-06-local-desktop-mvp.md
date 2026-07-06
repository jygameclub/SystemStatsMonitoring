# SystemStatsMonitoring V0.1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the V0.1 local-first desktop monitoring MVP with Tauri v2, React, Rust, SQLite, tray entry, live metrics, local history, charts, settings, tests, and README.

**Architecture:** React owns polling and rendering. Rust exposes Tauri commands for metrics collection, settings validation, SQLite initialization, persistence, and history queries. SQLite stores local samples and settings under the app data directory.

**Tech Stack:** Tauri v2, Rust, sysinfo, rusqlite, React, TypeScript, Vite, ECharts, Vitest.

---

## File Structure

- Create `package.json`: npm scripts and frontend dependencies.
- Create `index.html`, `src/main.tsx`, `src/App.tsx`, `src/styles.css`: React app shell.
- Create `src/lib/format.ts`, `src/lib/settings.ts`, `src/lib/history.ts`: pure TypeScript utility functions.
- Create `src/lib/*.test.ts`: TypeScript unit tests.
- Create `src-tauri/Cargo.toml`, `src-tauri/build.rs`, `src-tauri/tauri.conf.json`: Tauri Rust project.
- Create `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`: Tauri app entry and tray/command wiring.
- Create `src-tauri/src/models.rs`: shared Rust DTOs.
- Create `src-tauri/src/settings.rs`: settings validation and defaults.
- Create `src-tauri/src/store.rs`: SQLite repository.
- Create `src-tauri/src/metrics.rs`: sysinfo collector.
- Create `src-tauri/icons/icon.svg`, `src-tauri/icons/icon.png`: app/tray icons.
- Create `README.md`: setup, dev, test, build instructions.

## Task 1: Scaffold Frontend Package

**Files:**
- Create: `package.json`
- Create: `index.html`
- Create: `tsconfig.json`
- Create: `tsconfig.node.json`
- Create: `vite.config.ts`
- Create: `.gitignore`

- [ ] **Step 1: Create npm project files**

Use React, Vite, TypeScript, Vitest, ECharts, and Tauri CLI:

```json
{
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview",
    "test": "vitest run",
    "test:watch": "vitest",
    "tauri": "tauri",
    "tauri:dev": "tauri dev",
    "tauri:build": "tauri build"
  }
}
```

- [ ] **Step 2: Install npm dependencies**

Run:

```bash
npm install
```

Expected: `package-lock.json` is created and dependencies install successfully.

- [ ] **Step 3: Verify frontend scaffold**

Run:

```bash
npm run build
```

Expected: TypeScript and Vite build complete with exit code 0.

## Task 2: Add TypeScript Utilities with Tests

**Files:**
- Create: `src/lib/settings.ts`
- Create: `src/lib/settings.test.ts`
- Create: `src/lib/format.ts`
- Create: `src/lib/format.test.ts`
- Create: `src/lib/history.ts`
- Create: `src/lib/history.test.ts`

- [ ] **Step 1: Write failing settings tests**

Tests must cover default settings and invalid intervals:

```ts
expect(validateSettings({ sample_interval_sec: 1, local_save_interval_sec: 5 }).valid).toBe(true);
expect(validateSettings({ sample_interval_sec: 0, local_save_interval_sec: 5 }).valid).toBe(false);
expect(validateSettings({ sample_interval_sec: 10, local_save_interval_sec: 5 }).valid).toBe(false);
```

Run:

```bash
npm test -- src/lib/settings.test.ts
```

Expected before implementation: fail because the modules do not exist.

- [ ] **Step 2: Implement settings utilities**

Implement `DEFAULT_SETTINGS`, `validateSettings`, and `normalizeSettings` with the spec limits:

```text
sample_interval_sec: 1..60
local_save_interval_sec: 5..300
local_save_interval_sec >= sample_interval_sec
```

- [ ] **Step 3: Write and implement formatting/history utilities**

Cover bytes, percentages, network rates, time labels, and range-to-start timestamp conversion.

- [ ] **Step 4: Run TypeScript tests**

Run:

```bash
npm test
```

Expected: all Vitest tests pass.

## Task 3: Add Rust Domain, Settings, and SQLite Tests

**Files:**
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/build.rs`
- Create: `src-tauri/src/main.rs`
- Create: `src-tauri/src/lib.rs`
- Create: `src-tauri/src/models.rs`
- Create: `src-tauri/src/settings.rs`
- Create: `src-tauri/src/store.rs`

- [ ] **Step 1: Write failing Rust tests**

Tests must cover:

```rust
assert!(AppSettings { sample_interval_sec: 1, local_save_interval_sec: 5 }.validate().is_ok());
assert!(AppSettings { sample_interval_sec: 0, local_save_interval_sec: 5 }.validate().is_err());
assert!(AppSettings { sample_interval_sec: 10, local_save_interval_sec: 5 }.validate().is_err());
```

SQLite tests must use an in-memory connection, insert two samples, query by range, and prune old samples.

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected before implementation: fail because Rust project files do not exist.

- [ ] **Step 2: Implement Rust models and settings**

Define `MetricSample`, `HistoryRange`, `AppSettings`, and validation helpers.

- [ ] **Step 3: Implement SQLite store**

Implement:

```rust
Store::new(path: &Path) -> Result<Store>
Store::in_memory() -> Result<Store>
Store::init() -> Result<()>
Store::save_metric_sample(&self, sample: &MetricSample) -> Result<()>
Store::metric_history(&self, device_id: &str, start_ts: i64, end_ts: i64) -> Result<Vec<MetricSample>>
Store::prune_metric_samples_before(&self, cutoff_ts: i64) -> Result<usize>
Store::get_settings(&self) -> Result<AppSettings>
Store::update_settings(&self, settings: &AppSettings) -> Result<AppSettings>
```

- [ ] **Step 4: Run Rust tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: Rust unit tests pass.

## Task 4: Implement Metrics Collection and Tauri Commands

**Files:**
- Create: `src-tauri/src/metrics.rs`
- Modify: `src-tauri/src/lib.rs`
- Create: `src-tauri/tauri.conf.json`
- Create: `src-tauri/capabilities/default.json`

- [ ] **Step 1: Add metrics collector tests for calculations**

Test network delta math with previous/current byte counters and elapsed seconds.

- [ ] **Step 2: Implement sysinfo collector**

Use `System::new_all`, refresh CPU and memory, `Disks::new_with_refreshed_list`, and `Networks::new_with_refreshed_list`.

- [ ] **Step 3: Implement Tauri commands**

Expose:

```rust
get_latest_metrics
save_metric_sample
get_metric_history
get_settings
update_settings
```

- [ ] **Step 4: Implement tray setup**

Use Tauri v2 Rust tray API with `TrayIconBuilder`, a `Show` menu item, and a `Quit` menu item. Left click and `Show` open/focus the main window.

- [ ] **Step 5: Run Rust tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: all Rust tests pass.

## Task 5: Build React UI

**Files:**
- Create: `src/types.ts`
- Modify: `src/App.tsx`
- Modify: `src/main.tsx`
- Modify: `src/styles.css`

- [ ] **Step 1: Create app shell**

Implement header, sidebar tabs, Overview, History, and Settings views.

- [ ] **Step 2: Implement polling**

Use `invoke("get_latest_metrics")` every `sample_interval_sec`.

- [ ] **Step 3: Implement local save timer**

Call `invoke("save_metric_sample", { sample })` every `local_save_interval_sec`.

- [ ] **Step 4: Implement history charts**

Use ECharts line charts for CPU, memory, disk, network RX, and network TX. Support `1h` and `24h` range buttons.

- [ ] **Step 5: Implement settings form**

Validate settings in TypeScript before invoking `update_settings`.

- [ ] **Step 6: Run frontend tests and build**

Run:

```bash
npm test
npm run build
```

Expected: tests and build pass.

## Task 6: Documentation and Final Verification

**Files:**
- Create: `README.md`
- Modify: `.gitignore`

- [ ] **Step 1: Write README**

Include:

```bash
npm install
npm run dev
npm run tauri:dev
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
```

Also state that Rust is required for Tauri commands and desktop builds.

- [ ] **Step 2: Run final verification**

Run:

```bash
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: all available verification passes. If Rust is unavailable, record the exact blocker and still report frontend verification separately.

- [ ] **Step 3: Check git status**

Run:

```bash
git status --short
```

Expected: only intended project files are modified or untracked.
