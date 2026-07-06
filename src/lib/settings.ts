export interface AppSettings {
  sample_interval_sec: number;
  local_save_interval_sec: number;
}

export type SettingsInput = {
  sample_interval_sec: number | string;
  local_save_interval_sec: number | string;
};

export type SettingsValidation =
  | { valid: true }
  | { valid: false; message: string };

export const DEFAULT_SETTINGS: AppSettings = {
  sample_interval_sec: 1,
  local_save_interval_sec: 5,
};

export function normalizeSettings(input: SettingsInput): AppSettings {
  return {
    sample_interval_sec: Number(input.sample_interval_sec),
    local_save_interval_sec: Number(input.local_save_interval_sec),
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

  return { valid: true };
}
