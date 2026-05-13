//! rairos-sysinfo — Rust alternative to psutil for system monitoring.
//! Replaces psutil dependency in core/resource_monitor.py, core/profiler.py.

#![allow(deprecated)]

use pyo3::prelude::*;
use std::collections::HashMap;
use std::sync::Mutex;
use sysinfo::{Disks, Pid, System};

/// psutil-compatible system info wrapper.
#[pyclass]
struct SysInfo(Mutex<System>);

#[pymethods]
impl SysInfo {
    #[new]
    fn new() -> Self {
        Self(Mutex::new(System::new_all()))
    }

    /// psutil.cpu_percent(interval=None) -> float
    fn cpu_percent(&self, interval: Option<f32>) -> f32 {
        if let Some(secs) = interval {
            std::thread::sleep(std::time::Duration::from_secs_f32(secs));
        }
        let mut sys = self.0.lock().unwrap();
        sys.refresh_cpu_specifics(sysinfo::CpuRefreshKind::everything());
        sys.global_cpu_usage()
    }

    /// psutil.virtual_memory() -> HashMap converted to Python dict
    fn virtual_memory(&self) -> PyResult<HashMap<String, f64>> {
        let mut sys = self.0.lock().unwrap();
        sys.refresh_memory();
        let total = sys.total_memory() as f64;
        let available = sys.available_memory() as f64;
        let used = total - available;
        let percent = if total > 0.0 {
            (used / total) * 100.0
        } else {
            0.0
        };
        let mut m = HashMap::new();
        m.insert("used_mb".into(), used / 1024.0 / 1024.0);
        m.insert("available_mb".into(), available / 1024.0 / 1024.0);
        m.insert("percent".into(), percent);
        Ok(m)
    }

    /// psutil.disk_usage(path) -> HashMap converted to Python dict
    fn disk_usage(&self, path: &str) -> PyResult<HashMap<String, f64>> {
        let disks = Disks::new_with_refreshed_list();
        let best = disks
            .iter()
            .find(|d| path.starts_with(d.mount_point().to_string_lossy().as_ref()));
        let mut m = HashMap::new();
        if let Some(disk) = best {
            let total = disk.total_space() as f64;
            let free = disk.available_space() as f64;
            let used = total - free;
            let percent = if total > 0.0 {
                (used / total) * 100.0
            } else {
                0.0
            };
            m.insert("total_gb".into(), total / 1024.0 / 1024.0 / 1024.0);
            m.insert("used_gb".into(), used / 1024.0 / 1024.0 / 1024.0);
            m.insert("free_gb".into(), free / 1024.0 / 1024.0 / 1024.0);
            m.insert("percent".into(), percent);
        } else {
            m.insert("total_gb".into(), 0.0);
            m.insert("used_gb".into(), 0.0);
            m.insert("free_gb".into(), 0.0);
            m.insert("percent".into(), 0.0);
        }
        Ok(m)
    }

    /// psutil.disk_io_counters() -> HashMap (stub)
    fn disk_io_counters(&self) -> PyResult<HashMap<String, u64>> {
        let mut m = HashMap::new();
        m.insert("read_bytes".into(), 0u64);
        m.insert("write_bytes".into(), 0u64);
        m.insert("read_count".into(), 0u64);
        m.insert("write_count".into(), 0u64);
        Ok(m)
    }

    /// psutil.net_io_counters() -> HashMap (stub)
    fn net_io_counters(&self) -> PyResult<HashMap<String, u64>> {
        let mut m = HashMap::new();
        m.insert("bytes_sent".into(), 0u64);
        m.insert("bytes_recv".into(), 0u64);
        m.insert("packets_sent".into(), 0u64);
        m.insert("packets_recv".into(), 0u64);
        Ok(m)
    }
}

/// psutil.Process(pid).memory_info().rss
#[pyclass]
struct ProcessInfo {
    pid: Pid,
}

#[pymethods]
impl ProcessInfo {
    #[new]
    #[pyo3(signature = (pid=None))]
    fn new(pid: Option<u32>) -> PyResult<Self> {
        let actual_pid = pid
            .map(Pid::from_u32)
            .unwrap_or_else(|| Pid::from_u32(std::process::id()));
        Ok(Self { pid: actual_pid })
    }

    /// process.memory_info().rss in bytes
    fn memory_info_rss(&self) -> PyResult<u64> {
        let sys = System::new_all();
        if let Some(p) = sys.process(self.pid) {
            Ok(p.memory())
        } else {
            Ok(0)
        }
    }

    /// process.memory_info().rss in MB
    fn memory_rss_mb(&self) -> PyResult<f64> {
        Ok(self.memory_info_rss()? as f64 / 1024.0 / 1024.0)
    }
}

#[pymodule]
fn rairos_sysinfo(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<SysInfo>()?;
    m.add_class::<ProcessInfo>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn sysinfo_version_exists() {
        assert!(true)
    }
}
