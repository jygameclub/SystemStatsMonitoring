use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MetricSettings {
    pub cpu: bool,
    pub memory: bool,
    pub disk: bool,
    pub network: bool,
    pub gpu: bool,
    pub temperature: bool,
    pub power: bool,
    pub battery: bool,
}

impl Default for MetricSettings {
    fn default() -> Self {
        Self {
            cpu: true,
            memory: true,
            disk: true,
            network: true,
            gpu: true,
            temperature: true,
            power: true,
            battery: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct S3SyncSettings {
    pub enabled: bool,
    pub endpoint_url: String,
    pub region: String,
    pub bucket: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub prefix: String,
    pub sync_interval_min: u64,
    pub path_style: bool,
}

impl Default for S3SyncSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint_url: String::new(),
            region: "us-east-1".to_string(),
            bucket: String::new(),
            access_key_id: String::new(),
            secret_access_key: String::new(),
            prefix: "system-stats-monitoring".to_string(),
            sync_interval_min: 10,
            path_style: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppSettings {
    pub sample_interval_sec: u64,
    pub local_save_interval_sec: u64,
    pub machine_name: String,
    pub language: String,
    pub metrics: MetricSettings,
    pub s3_sync: S3SyncSettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            sample_interval_sec: 1,
            local_save_interval_sec: 5,
            machine_name: String::new(),
            language: "zh-CN".to_string(),
            metrics: MetricSettings::default(),
            s3_sync: S3SyncSettings::default(),
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
    #[error("机器名称必须少于 64 个字符")]
    InvalidMachineName,
    #[error("不支持的语言")]
    UnsupportedLanguage,
    #[error("启用 S3 同步后必须填写 S3 地址、Bucket、Access Key 和 Secret Key")]
    IncompleteS3SyncSettings,
    #[error("S3 地址必须以 http:// 或 https:// 开头")]
    InvalidS3Endpoint,
    #[error("S3 Bucket 不能包含 /")]
    InvalidS3Bucket,
    #[error("S3 目录不能以 / 开头，且不能包含 ..")]
    InvalidS3Prefix,
    #[error("S3 同步间隔必须在 1 到 1440 分钟之间")]
    InvalidS3SyncInterval,
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

        let machine_name = self.machine_name.trim();
        if machine_name.chars().count() > 64 {
            return Err(SettingsError::InvalidMachineName);
        }

        if self.language != "zh-CN" && self.language != "en" {
            return Err(SettingsError::UnsupportedLanguage);
        }

        self.s3_sync.validate()?;

        Ok(())
    }
}

impl S3SyncSettings {
    pub fn validate(&self) -> Result<(), SettingsError> {
        if self.sync_interval_min < 1 || self.sync_interval_min > 1440 {
            return Err(SettingsError::InvalidS3SyncInterval);
        }

        let endpoint_url = self.endpoint_url.trim();
        let bucket = self.bucket.trim();
        let access_key_id = self.access_key_id.trim();
        let secret_access_key = self.secret_access_key.trim();
        let prefix = self.prefix.trim();

        if !self.enabled {
            return Ok(());
        }

        if endpoint_url.is_empty()
            || bucket.is_empty()
            || access_key_id.is_empty()
            || secret_access_key.is_empty()
        {
            return Err(SettingsError::IncompleteS3SyncSettings);
        }

        if !endpoint_url.starts_with("https://") && !endpoint_url.starts_with("http://") {
            return Err(SettingsError::InvalidS3Endpoint);
        }

        if bucket.contains('/') {
            return Err(SettingsError::InvalidS3Bucket);
        }

        if prefix.starts_with('/') || prefix.contains("..") {
            return Err(SettingsError::InvalidS3Prefix);
        }

        Ok(())
    }
}
