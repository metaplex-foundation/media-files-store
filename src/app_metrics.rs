use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use sysinfo::System;

pub const MET_DOWNLOADS: &str = "downloads";
pub const CAT_STATUS: &str = "status";

pub fn setup_metrics_recorder() -> PrometheusHandle {
    PrometheusBuilder::new()
        .install_recorder()
        .unwrap()
}

pub async fn run_sys_metrics_collector() {
    tokio::spawn(async move {
        let mut sys = System::new();
        loop {

            // RAM usage
            use memory_stats::memory_stats;
            if let Some(usage) = memory_stats() {
                metrics::gauge!("physical_mem").set(usage.physical_mem as u32);
                metrics::gauge!("virtual_mem").set(usage.virtual_mem as u32);
            }

            // CPU usage
            sys.refresh_cpu_usage();
            for cpu in sys.cpus() {
                metrics::gauge!("cpu", "name" => cpu.name().to_string()).set(cpu.cpu_usage());
            }

            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        }
    });
}
