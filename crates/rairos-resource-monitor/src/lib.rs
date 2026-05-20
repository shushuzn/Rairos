use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use sysinfo::{CpuRefreshKind, Disks, System};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceStats {
    pub timestamp: f64,
    pub cpu_percent: f32,
    pub memory_used_mb: f64,
    pub memory_percent: f32,
    pub disk_used_gb: f64,
    pub disk_percent: f32,
    pub disk_io_reads: u64,
    pub disk_io_writes: u64,
    pub network_sent_mb: f64,
    pub network_recv_mb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    pub path: String,
    pub total_gb: f64,
    pub used_gb: f64,
    pub free_gb: f64,
    pub percent: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiCallRecord {
    pub timestamp: f64,
    pub provider: String,
    pub endpoint: String,
    pub tokens: u32,
    pub cost: f64,
    pub total_cost: f64,
}

#[derive(Debug)]
pub struct ResourceMonitor {
    sys: Mutex<System>,
    data_dir: PathBuf,
    history: Mutex<Vec<ResourceStats>>,
    max_history: usize,
}

impl ResourceMonitor {
    pub fn new(data_dir: Option<PathBuf>) -> Self {
        let dir = data_dir.unwrap_or_else(|| {
            let home = std::env::var("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("."));
            home.join(".cache").join("ai_research_os")
        });
        Self {
            sys: Mutex::new(System::new()),
            data_dir: dir,
            history: Mutex::new(Vec::new()),
            max_history: 1000,
        }
    }

    pub fn get_disk_info(&self) -> DiskInfo {
        let disks = Disks::new_with_refreshed_list();
        let target = self.data_dir.to_string_lossy().to_string();
        for disk in &disks {
            let mount = disk.mount_point().to_string_lossy();
            if target.starts_with(mount.as_ref()) {
                let total = disk.total_space() as f64;
                let free = disk.available_space() as f64;
                let used = total - free;
                let pct = if total > 0.0 {
                    (used / total * 100.0) as f32
                } else {
                    0.0
                };
                return DiskInfo {
                    path: mount.to_string(),
                    total_gb: total / 1_073_741_824.0,
                    used_gb: used / 1_073_741_824.0,
                    free_gb: free / 1_073_741_824.0,
                    percent: pct,
                };
            }
        }
        DiskInfo {
            path: target,
            total_gb: 0.0,
            used_gb: 0.0,
            free_gb: 0.0,
            percent: 0.0,
        }
    }

    pub fn get_memory_info(&self) -> (f64, f64, f32) {
        let mut sys = self.sys.lock();
        sys.refresh_memory();
        let total = sys.total_memory() as f64;
        let used = sys.used_memory() as f64;
        let pct = if total > 0.0 {
            (used / total * 100.0) as f32
        } else {
            0.0
        };
        (used / 1_048_576.0, (total - used) / 1_048_576.0, pct)
    }

    pub fn get_cpu_info(&self) -> (f32, usize) {
        let mut sys = self.sys.lock();
        sys.refresh_cpu_specifics(CpuRefreshKind::everything());
        let pct = sys.global_cpu_usage();
        let count = sys.cpus().len();
        (pct, count)
    }

    pub fn collect_stats(&self) -> ResourceStats {
        let _mem_used_mb = {
            let mut sys = self.sys.lock();
            sys.refresh_memory();
            sys.used_memory() as f64 / 1_048_576.0
        };
        let (mem_used, _mem_avail, mem_pct) = self.get_memory_info();
        let (cpu_pct, _cpu_count) = self.get_cpu_info();
        let disk = self.get_disk_info();

        let stats = ResourceStats {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
            cpu_percent: cpu_pct,
            memory_used_mb: mem_used,
            memory_percent: mem_pct,
            disk_used_gb: disk.used_gb,
            disk_percent: disk.percent,
            disk_io_reads: 0,
            disk_io_writes: 0,
            network_sent_mb: 0.0,
            network_recv_mb: 0.0,
        };

        let mut history = self.history.lock();
        history.push(stats.clone());
        while history.len() > self.max_history {
            history.remove(0);
        }
        stats
    }

    pub fn get_average_stats(&self, count: usize) -> HashMap<String, f64> {
        let history = self.history.lock();
        let samples: Vec<_> = history.iter().rev().take(count).collect();
        if samples.is_empty() {
            return HashMap::new();
        }
        let len = samples.len() as f64;
        let mut map = HashMap::new();
        map.insert(
            "avg_cpu_percent".to_string(),
            samples.iter().map(|s| s.cpu_percent as f64).sum::<f64>() / len,
        );
        map.insert(
            "avg_memory_percent".to_string(),
            samples.iter().map(|s| s.memory_percent as f64).sum::<f64>() / len,
        );
        map.insert(
            "avg_disk_percent".to_string(),
            samples.iter().map(|s| s.disk_percent as f64).sum::<f64>() / len,
        );
        map.insert(
            "total_disk_io_reads".to_string(),
            samples.iter().map(|s| s.disk_io_reads as f64).sum::<f64>(),
        );
        map.insert(
            "total_disk_io_writes".to_string(),
            samples.iter().map(|s| s.disk_io_writes as f64).sum::<f64>(),
        );
        map.insert(
            "total_network_sent_mb".to_string(),
            samples.iter().map(|s| s.network_sent_mb).sum::<f64>(),
        );
        map.insert(
            "total_network_recv_mb".to_string(),
            samples.iter().map(|s| s.network_recv_mb).sum::<f64>(),
        );
        map
    }

    pub fn get_resource_report(&self) -> String {
        let stats = self.collect_stats();
        let avg = self.get_average_stats(100);
        let disk = self.get_disk_info();
        let ts = chrono::DateTime::from_timestamp(stats.timestamp as i64, 0)
            .map(|d| d.format("%Y-%m-%dT%H:%M:%S").to_string())
            .unwrap_or_default();

        format!(
            "=== Resource Usage Report ===\n\
             Timestamp: {ts}\n\
             \n\
             Current Usage:\n\
               CPU:     {cpu:.1}%\n\
               Memory:  {mem:.0} MB ({mem_pct:.1}%)\n\
               Disk:    {disk_used:.2} GB ({disk_pct:.1}%)\n\
             \n\
             Disk Info:\n\
               Total:   {total:.2} GB\n\
               Used:    {used:.2} GB\n\
               Free:    {free:.2} GB\n\
             \n\
             Recent Averages (last 100 samples):\n\
               CPU:     {avg_cpu:.1}%\n\
               Memory:  {avg_mem:.1}%\n\
               Disk:    {avg_disk:.1}%\n\
             \n\
             I/O Statistics:\n\
               Disk Reads:  {io_reads}\n\
               Disk Writes: {io_writes}\n\
               Network Sent:    {net_sent:.2} MB\n\
               Network Recv:   {net_recv:.2} MB\n",
            ts = ts,
            cpu = stats.cpu_percent,
            mem = stats.memory_used_mb,
            mem_pct = stats.memory_percent,
            disk_used = stats.disk_used_gb,
            disk_pct = stats.disk_percent,
            total = disk.total_gb,
            used = disk.used_gb,
            free = disk.free_gb,
            avg_cpu = avg.get("avg_cpu_percent").unwrap_or(&0.0),
            avg_mem = avg.get("avg_memory_percent").unwrap_or(&0.0),
            avg_disk = avg.get("avg_disk_percent").unwrap_or(&0.0),
            io_reads = *avg.get("total_disk_io_reads").unwrap_or(&0.0) as u64,
            io_writes = *avg.get("total_disk_io_writes").unwrap_or(&0.0) as u64,
            net_sent = avg.get("total_network_sent_mb").unwrap_or(&0.0),
            net_recv = avg.get("total_network_recv_mb").unwrap_or(&0.0),
        )
    }
}

#[derive(Debug)]
pub struct ResourceGuard {
    min_disk_gb: f64,
    max_memory_percent: f32,
    monitor: Option<ResourceMonitor>,
}

impl Default for ResourceGuard {
    fn default() -> Self {
        Self {
            min_disk_gb: 1.0,
            max_memory_percent: 90.0,
            monitor: None,
        }
    }
}

impl ResourceGuard {
    pub fn new(min_disk_gb: f64, max_memory_percent: f32, monitor: Option<ResourceMonitor>) -> Self {
        Self {
            min_disk_gb,
            max_memory_percent,
            monitor,
        }
    }

    pub fn check(&self) -> Result<(), String> {
        let monitor = self.monitor.as_ref().ok_or("No monitor configured")?;
        let disk = monitor.get_disk_info();
        if disk.free_gb < self.min_disk_gb {
            return Err(format!("Low disk space: only {:.2} GB free", disk.free_gb));
        }
        let (_mem_used, _mem_avail, mem_pct) = monitor.get_memory_info();
        if mem_pct > self.max_memory_percent {
            return Err(format!("High memory usage: {:.1}%", mem_pct));
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct ApiBudgetTracker {
    monthly_budget_usd: f64,
    call_count: u64,
    cost_estimate: f64,
    recent_calls: Vec<ApiCallRecord>,
}

impl Default for ApiBudgetTracker {
    fn default() -> Self {
        Self::new(100.0)
    }
}

impl ApiBudgetTracker {
    pub fn new(monthly_budget_usd: f64) -> Self {
        Self {
            monthly_budget_usd,
            call_count: 0,
            cost_estimate: 0.0,
            recent_calls: Vec::new(),
        }
    }

    pub fn record_api_call(
        &mut self,
        provider: &str,
        endpoint: &str,
        tokens_used: u32,
        cost_per_1k: f64,
    ) {
        self.call_count += 1;
        let call_cost = (tokens_used as f64 / 1000.0) * cost_per_1k;
        self.cost_estimate += call_cost;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();
        self.recent_calls.push(ApiCallRecord {
            timestamp: now,
            provider: provider.to_string(),
            endpoint: endpoint.to_string(),
            tokens: tokens_used,
            cost: call_cost,
            total_cost: self.cost_estimate,
        });
        if self.recent_calls.len() > 100 {
            self.recent_calls.remove(0);
        }
    }

    pub fn get_usage_report(&self) -> serde_json::Value {
        let remaining = self.monthly_budget_usd - self.cost_estimate;
        let used_pct = if self.monthly_budget_usd > 0.0 {
            self.cost_estimate / self.monthly_budget_usd * 100.0 
        } else {
            0.0
        };
        serde_json::json!({
            "total_calls": self.call_count,
            "estimated_cost_usd": self.cost_estimate,
            "budget_remaining_usd": remaining.max(0.0),
            "budget_used_percent": used_pct,
            "recent_calls": self.recent_calls.len(),
        })
    }

    pub fn should_make_api_call(&self, estimated_cost: f64) -> bool {
        (self.cost_estimate + estimated_cost) <= self.monthly_budget_usd
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_disk_info() {
        let rm = ResourceMonitor::new(None);
        let info = rm.get_disk_info();
        assert!(info.total_gb > 0.0);
        assert!(info.free_gb >= 0.0);
    }

    #[test]
    fn test_get_memory_info() {
        let rm = ResourceMonitor::new(None);
        let (used, avail, pct) = rm.get_memory_info();
        assert!(used > 0.0);
        assert!(avail > 0.0);
        assert!(pct > 0.0 && pct <= 100.0);
    }

    #[test]
    fn test_collect_stats() {
        let rm = ResourceMonitor::new(None);
        let stats = rm.collect_stats();
        assert!(stats.memory_percent > 0.0);
        assert!(stats.timestamp > 0.0);
    }

    #[test]
    fn test_get_average_stats() {
        let rm = ResourceMonitor::new(None);
        rm.collect_stats();
        let avg = rm.get_average_stats(10);
        assert!(avg.contains_key("avg_memory_percent"));
    }

    #[test]
    fn test_get_resource_report() {
        let rm = ResourceMonitor::new(None);
        let report = rm.get_resource_report();
        assert!(report.contains("Resource Usage Report"));
    }

    #[test]
    fn test_resource_guard() {
        let rm = ResourceMonitor::new(None);
        let guard = ResourceGuard::new(0.0, 100.0, Some(rm));
        assert!(guard.check().is_ok());
    }

    #[test]
    fn test_api_budget_tracker() {
        let mut tracker = ApiBudgetTracker::new(100.0);
        tracker.record_api_call("openai", "gpt-4", 500, 0.03);
        let report = tracker.get_usage_report();
        assert!(report["total_calls"].as_u64() == Some(1));
        assert!(report["budget_used_percent"].as_f64().unwrap() > 0.0);
        assert!(tracker.should_make_api_call(0.01));
    }

    #[test]
    fn test_api_budget_exceeded() {
        let mut tracker = ApiBudgetTracker::new(0.05);
        tracker.record_api_call("openai", "gpt-4", 1000, 0.03);
        assert!(!tracker.should_make_api_call(0.10));
    }
}
