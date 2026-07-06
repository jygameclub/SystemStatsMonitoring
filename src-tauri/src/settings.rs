use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppSettings {
    pub sample_interval_sec: u64,
    pub local_save_interval_sec: u64,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            sample_interval_sec: 1,
            local_save_interval_sec: 5,
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SettingsError {
    #[error("采样间隔必须在 1 到 60 秒之间")]
    InvalidSampleInterval,
    #[error("保存间隔必须在 5 到 300 秒之间")]
    InvalidSaveInterval,
    #[error("保存间隔不能小于采样间隔")]
    SaveIntervalShorterThanSampleInterval,
}

impl AppSettings {
    pub fn validate(&self) -> Result<(), SettingsError> {
        if self.sample_interval_sec < 1 || self.sample_interval_sec > 60 {
            return Err(SettingsError::InvalidSampleInterval);
        }

        if self.local_save_interval_sec < 5 || self.local_save_interval_sec > 300 {
            return Err(SettingsError::InvalidSaveInterval);
        }

        if self.local_save_interval_sec < self.sample_interval_sec {
            return Err(SettingsError::SaveIntervalShorterThanSampleInterval);
        }

        Ok(())
    }
}
