use std::collections::{BTreeMap, BTreeSet};

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use futures_util::TryStreamExt;
use object_store::aws::AmazonS3Builder;
use object_store::path::Path;
use object_store::{ObjectStore, ObjectStoreExt, PutPayload};
use serde::{Deserialize, Serialize};

use crate::models::{DeviceInfo, MetricSample};
use crate::settings::S3SyncSettings;

#[derive(Debug, Clone)]
pub struct DownloadedDeviceSamples {
    pub device: DeviceInfo,
    pub samples: Vec<MetricSample>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SyncManifest {
    version: u32,
    device: DeviceInfo,
    uploaded_at: i64,
    sample_days: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SyncSamplesFile {
    version: u32,
    device: DeviceInfo,
    day: String,
    samples: Vec<MetricSample>,
}

pub fn normalize_prefix(prefix: &str) -> Result<String> {
    let prefix = prefix.trim().trim_matches('/').to_string();

    if prefix.contains("..") {
        bail!("S3 目录不能包含 ..");
    }

    Ok(prefix)
}

pub fn manifest_key(prefix: &str, device_id: &str) -> String {
    join_key([
        prefix,
        "devices",
        &sanitize_path_segment(device_id),
        "manifest.json",
    ])
}

pub fn samples_key(prefix: &str, device_id: &str, day: &str) -> String {
    join_key([
        prefix,
        "devices",
        &sanitize_path_segment(device_id),
        "samples",
        &format!("{day}.json"),
    ])
}

pub async fn test_connection(settings: &S3SyncSettings) -> Result<()> {
    let store = build_store(settings)?;
    let prefix = normalize_prefix(&settings.prefix)?;
    let prefix_path = Path::from(join_key([prefix.as_str(), "devices"]));
    let mut stream = store.list(Some(&prefix_path));
    let _ = stream.try_next().await.context("list S3 prefix")?;
    Ok(())
}

pub async fn upload_and_download(
    settings: &S3SyncSettings,
    device: &DeviceInfo,
    local_samples: &[MetricSample],
    now_ms: i64,
) -> Result<(usize, Vec<DownloadedDeviceSamples>)> {
    settings.validate()?;
    if !settings.enabled {
        bail!("S3 同步未启用");
    }

    let store = build_store(settings)?;
    let prefix = normalize_prefix(&settings.prefix)?;
    let uploaded_days = upload_local_samples(&store, &prefix, device, local_samples, now_ms)
        .await
        .context("upload local samples")?;
    let downloaded = download_remote_samples(&store, &prefix, &device.id)
        .await
        .context("download remote samples")?;

    Ok((uploaded_days, downloaded))
}

async fn upload_local_samples(
    store: &impl ObjectStore,
    prefix: &str,
    device: &DeviceInfo,
    samples: &[MetricSample],
    now_ms: i64,
) -> Result<usize> {
    let grouped = group_samples_by_day(samples);
    let mut sample_days = Vec::with_capacity(grouped.len());

    for (day, day_samples) in &grouped {
        let file = SyncSamplesFile {
            version: 1,
            device: device.clone(),
            day: day.clone(),
            samples: day_samples.clone(),
        };
        let key = samples_key(prefix, &device.id, day);
        put_json(store, &key, &file).await?;
        sample_days.push(day.clone());
    }

    let manifest = SyncManifest {
        version: 1,
        device: device.clone(),
        uploaded_at: now_ms,
        sample_days,
    };
    put_json(store, &manifest_key(prefix, &device.id), &manifest).await?;

    Ok(grouped.len())
}

async fn download_remote_samples(
    store: &impl ObjectStore,
    prefix: &str,
    local_device_id: &str,
) -> Result<Vec<DownloadedDeviceSamples>> {
    let manifest_keys = list_manifest_keys(store, prefix).await?;
    let mut devices = Vec::new();

    for key in manifest_keys {
        let manifest: SyncManifest = get_json(store, &key)
            .await
            .with_context(|| format!("read manifest {key}"))?;
        if manifest.device.id == local_device_id {
            continue;
        }

        let mut samples = Vec::new();
        let days: BTreeSet<String> = manifest.sample_days.into_iter().collect();
        for day in days {
            let key = samples_key(prefix, &manifest.device.id, &day);
            let file = match get_json::<SyncSamplesFile>(store, &key).await {
                Ok(file) => file,
                Err(error) => {
                    let text = error.to_string();
                    if text.contains("NotFound") || text.contains("not found") {
                        continue;
                    }
                    return Err(error).with_context(|| format!("read sample day {key}"));
                }
            };

            samples.extend(file.samples.into_iter().map(|mut sample| {
                sample.id = None;
                sample.device_id = manifest.device.id.clone();
                sample
            }));
        }

        devices.push(DownloadedDeviceSamples {
            device: manifest.device,
            samples,
        });
    }

    Ok(devices)
}

async fn list_manifest_keys(store: &impl ObjectStore, prefix: &str) -> Result<Vec<String>> {
    let prefix_path = Path::from(join_key([prefix, "devices"]));
    let mut stream = store.list(Some(&prefix_path));
    let mut keys = Vec::new();

    while let Some(meta) = stream.try_next().await? {
        let key = meta.location.as_ref().to_string();
        if key.ends_with("/manifest.json") {
            keys.push(key);
        }
    }

    keys.sort();
    Ok(keys)
}

async fn put_json<T: Serialize>(store: &impl ObjectStore, key: &str, value: &T) -> Result<()> {
    let bytes = serde_json::to_vec(value).context("serialize S3 sync JSON")?;
    store
        .put(&Path::from(key), PutPayload::from(bytes))
        .await
        .with_context(|| format!("put S3 object {key}"))?;
    Ok(())
}

async fn get_json<T: for<'de> Deserialize<'de>>(store: &impl ObjectStore, key: &str) -> Result<T> {
    let bytes = store
        .get(&Path::from(key))
        .await
        .with_context(|| format!("get S3 object {key}"))?
        .bytes()
        .await
        .with_context(|| format!("read S3 object {key}"))?;
    serde_json::from_slice::<T>(&bytes).with_context(|| format!("parse S3 object {key}"))
}

fn build_store(settings: &S3SyncSettings) -> Result<object_store::aws::AmazonS3> {
    let endpoint = settings.endpoint_url.trim();
    let allow_http = endpoint.starts_with("http://");
    AmazonS3Builder::new()
        .with_endpoint(endpoint)
        .with_region(settings.region.trim())
        .with_bucket_name(settings.bucket.trim())
        .with_access_key_id(settings.access_key_id.trim())
        .with_secret_access_key(settings.secret_access_key.trim())
        .with_virtual_hosted_style_request(!settings.path_style)
        .with_allow_http(allow_http)
        .build()
        .context("build S3 client")
}

fn group_samples_by_day(samples: &[MetricSample]) -> BTreeMap<String, Vec<MetricSample>> {
    let mut grouped = BTreeMap::new();

    for sample in samples {
        if let Some(day) = day_from_timestamp(sample.ts) {
            grouped
                .entry(day)
                .or_insert_with(Vec::new)
                .push(sample.clone());
        }
    }

    grouped
}

fn day_from_timestamp(ts: i64) -> Option<String> {
    let datetime = DateTime::<Utc>::from_timestamp_millis(ts)?;
    Some(datetime.format("%Y-%m-%d").to_string())
}

fn join_key<'a>(segments: impl IntoIterator<Item = &'a str>) -> String {
    segments
        .into_iter()
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .map(|segment| segment.trim_matches('/'))
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("/")
}

fn sanitize_path_segment(value: &str) -> String {
    let normalized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    if normalized.is_empty() {
        "machine".to_string()
    } else {
        normalized
    }
}

#[cfg(test)]
mod tests {
    use super::{day_from_timestamp, group_samples_by_day};
    use crate::models::MetricSample;

    fn sample(ts: i64) -> MetricSample {
        MetricSample {
            id: None,
            device_id: "studio-mac".to_string(),
            ts,
            cpu_usage: Some(10.0),
            memory_used: None,
            memory_total: None,
            disk_used: None,
            disk_total: None,
            network_rx: None,
            network_tx: None,
            gpu_usage: None,
            gpu_memory_total: None,
            gpu_name: None,
            temperature_celsius: None,
            power_watts: None,
            sensor_readings: Vec::new(),
        }
    }

    #[test]
    fn derives_utc_day_from_timestamp() {
        assert_eq!(
            day_from_timestamp(1_783_180_800_000).as_deref(),
            Some("2026-07-04")
        );
    }

    #[test]
    fn groups_samples_by_utc_day() {
        let grouped = group_samples_by_day(&[sample(1_783_180_800_000), sample(1_783_267_200_000)]);

        assert_eq!(grouped.len(), 2);
        assert!(grouped.contains_key("2026-07-04"));
        assert!(grouped.contains_key("2026-07-05"));
    }
}
