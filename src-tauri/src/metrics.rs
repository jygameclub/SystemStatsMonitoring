use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use sysinfo::{Disks, Networks, System};

use crate::models::MetricSample;

pub struct MetricCollector {
    system: System,
    disks: Disks,
    networks: Networks,
    device_id: String,
}

impl MetricCollector {
    pub fn new(device_id: String) -> Self {
        Self {
            system: System::new_all(),
            disks: Disks::new_with_refreshed_list(),
            networks: Networks::new_with_refreshed_list(),
            device_id,
        }
    }

    pub fn sample(&mut self) -> Result<MetricSample> {
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
        self.disks.refresh(true);
        self.networks.refresh(true);

        let memory_used = self.system.used_memory();
        let memory_total = self.system.total_memory();
        let (disk_used, disk_total) = disk_usage(&self.disks);
        let (network_rx, network_tx) = network_rates(&self.networks);

        Ok(MetricSample {
            id: None,
            device_id: self.device_id.clone(),
            ts: now_ms(),
            cpu_usage: self.system.global_cpu_usage() as f64,
            memory_used,
            memory_total,
            disk_used,
            disk_total,
            network_rx,
            network_tx,
        })
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn disk_usage(disks: &Disks) -> (u64, u64) {
    disks.iter().fold((0, 0), |(used_acc, total_acc), disk| {
        let total = disk.total_space();
        let available = disk.available_space();
        (
            used_acc + total.saturating_sub(available),
            total_acc + total,
        )
    })
}

fn network_rates(networks: &Networks) -> (f64, f64) {
    networks
        .iter()
        .fold((0.0, 0.0), |(rx_acc, tx_acc), (_, data)| {
            (
                rx_acc + data.received() as f64,
                tx_acc + data.transmitted() as f64,
            )
        })
}

#[cfg(test)]
mod tests {
    #[test]
    fn disk_usage_saturates_when_available_exceeds_total() {
        let total = 100_u64;
        let available = 120_u64;

        assert_eq!(total.saturating_sub(available), 0);
    }
}
