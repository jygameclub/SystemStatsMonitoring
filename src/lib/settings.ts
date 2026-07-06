export type SupportedLanguage = "zh-CN" | "en";

export interface MetricSettings {
  cpu: boolean;
  memory: boolean;
  disk: boolean;
  network: boolean;
  gpu: boolean;
  temperature: boolean;
  power: boolean;
  battery: boolean;
}

export interface AppSettings {
  sample_interval_sec: number;
  local_save_interval_sec: number;
  machine_name: string;
  language: SupportedLanguage;
  metrics: MetricSettings;
  s3_sync: S3SyncSettings;
}

export type SettingsInput = {
  sample_interval_sec: number | string;
  local_save_interval_sec: number | string;
  machine_name?: string;
  language?: string;
  metrics?: Partial<MetricSettings>;
  s3_sync?: Partial<S3SyncSettingsInput>;
};

export interface S3SyncSettings {
  enabled: boolean;
  endpoint_url: string;
  region: string;
  bucket: string;
  access_key_id: string;
  secret_access_key: string;
  prefix: string;
  sync_interval_min: number;
  path_style: boolean;
}

export type S3SyncSettingsInput = Omit<S3SyncSettings, "sync_interval_min"> & {
  sync_interval_min: number | string;
};

export type SettingsValidation =
  | { valid: true }
  | { valid: false; message: string };

export const DEFAULT_SETTINGS: AppSettings = {
  sample_interval_sec: 1,
  local_save_interval_sec: 5,
  machine_name: "",
  language: "zh-CN",
  metrics: {
    cpu: true,
    memory: true,
    disk: true,
    network: true,
    gpu: true,
    temperature: true,
    power: true,
    battery: true,
  },
  s3_sync: {
    enabled: false,
    endpoint_url: "",
    region: "us-east-1",
    bucket: "",
    access_key_id: "",
    secret_access_key: "",
    prefix: "system-stats-monitoring",
    sync_interval_min: 10,
    path_style: true,
  },
};

export function normalizeSettings(input: SettingsInput): AppSettings {
  return {
    sample_interval_sec: Number(input.sample_interval_sec),
    local_save_interval_sec: Number(input.local_save_interval_sec),
    machine_name: input.machine_name?.trim() ?? DEFAULT_SETTINGS.machine_name,
    language: normalizeLanguage(input.language),
    metrics: {
      ...DEFAULT_SETTINGS.metrics,
      ...input.metrics,
    },
    s3_sync: normalizeS3Sync(input.s3_sync),
  };
}

export function validateSettings(settings: AppSettings): SettingsValidation {
  if (
    !Number.isFinite(settings.sample_interval_sec) ||
    settings.sample_interval_sec < 1 ||
    settings.sample_interval_sec > 60
  ) {
    return { valid: false, message: "采样间隔必须在 1 到 60 秒之间" };
  }

  if (
    !Number.isFinite(settings.local_save_interval_sec) ||
    settings.local_save_interval_sec < 5 ||
    settings.local_save_interval_sec > 300
  ) {
    return { valid: false, message: "保存间隔必须在 5 到 300 秒之间" };
  }

  if (settings.local_save_interval_sec < settings.sample_interval_sec) {
    return { valid: false, message: "保存间隔不能小于采样间隔" };
  }

  if (settings.machine_name.trim().length > 64) {
    return { valid: false, message: "机器名称必须少于 64 个字符" };
  }

  if (!isSupportedLanguage(settings.language)) {
    return { valid: false, message: "不支持的语言" };
  }

  const s3Validation = validateS3Sync(settings.s3_sync);
  if (!s3Validation.valid) {
    return s3Validation;
  }

  return { valid: true };
}

function normalizeS3Sync(
  input: Partial<S3SyncSettingsInput> | undefined,
): S3SyncSettings {
  return {
    ...DEFAULT_SETTINGS.s3_sync,
    ...input,
    endpoint_url: input?.endpoint_url?.trim() ?? DEFAULT_SETTINGS.s3_sync.endpoint_url,
    region: input?.region?.trim() ?? DEFAULT_SETTINGS.s3_sync.region,
    bucket: input?.bucket?.trim() ?? DEFAULT_SETTINGS.s3_sync.bucket,
    access_key_id: input?.access_key_id?.trim() ?? DEFAULT_SETTINGS.s3_sync.access_key_id,
    secret_access_key:
      input?.secret_access_key?.trim() ?? DEFAULT_SETTINGS.s3_sync.secret_access_key,
    prefix: input?.prefix?.trim() ?? DEFAULT_SETTINGS.s3_sync.prefix,
    sync_interval_min: Number(
      input?.sync_interval_min ?? DEFAULT_SETTINGS.s3_sync.sync_interval_min,
    ),
    path_style: input?.path_style ?? DEFAULT_SETTINGS.s3_sync.path_style,
  };
}

function validateS3Sync(settings: S3SyncSettings): SettingsValidation {
  if (
    !Number.isFinite(settings.sync_interval_min) ||
    settings.sync_interval_min < 1 ||
    settings.sync_interval_min > 1440
  ) {
    return { valid: false, message: "S3 同步间隔必须在 1 到 1440 分钟之间" };
  }

  if (!settings.enabled) {
    return { valid: true };
  }

  if (
    settings.endpoint_url.trim() === "" ||
    settings.bucket.trim() === "" ||
    settings.access_key_id.trim() === "" ||
    settings.secret_access_key.trim() === ""
  ) {
    return {
      valid: false,
      message: "启用 S3 同步后必须填写 S3 地址、Bucket、Access Key 和 Secret Key",
    };
  }

  if (
    !settings.endpoint_url.startsWith("https://") &&
    !settings.endpoint_url.startsWith("http://")
  ) {
    return { valid: false, message: "S3 地址必须以 http:// 或 https:// 开头" };
  }

  if (settings.bucket.includes("/")) {
    return { valid: false, message: "S3 Bucket 不能包含 /" };
  }

  if (settings.prefix.startsWith("/") || settings.prefix.includes("..")) {
    return { valid: false, message: "S3 目录不能以 / 开头，且不能包含 .." };
  }

  return { valid: true };
}

function normalizeLanguage(language: string | undefined): SupportedLanguage {
  return isSupportedLanguage(language) ? language : DEFAULT_SETTINGS.language;
}

function isSupportedLanguage(
  language: string | undefined,
): language is SupportedLanguage {
  return language === "zh-CN" || language === "en";
}
