use std::collections::VecDeque;
use std::fs;
use std::path::Path;

const HISTORY_SIZE: usize = 300;

#[derive(Debug, Clone)]
pub struct MonitorSnapshot {
    pub cpu_temp_c: Option<f64>,
    pub cpu_usage_pct: f32,
    pub ram_used_bytes: u64,
    pub ram_total_bytes: u64,
    pub gpu_temp_c: Option<f64>,
    pub gpu_usage_pct: Option<f32>,
    pub vram_used_bytes: u64,
    pub vram_total_bytes: u64,
}

fn parse_stat_cpu(line: &str) -> Option<(u64, u64)> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 5 || parts[0] != "cpu" {
        return None;
    }
    let vals: Vec<u64> = parts[1..]
        .iter()
        .filter_map(|s| s.parse::<u64>().ok())
        .collect();
    if vals.len() < 3 {
        return None;
    }
    let user = vals[0];
    let nice = vals[1];
    let system = vals[2];
    let idle = vals[3];
    let iowait = vals.get(4).copied().unwrap_or(0);
    let total = user + nice + system + idle + iowait;
    let idle_total = idle + iowait;
    Some((total, idle_total))
}

fn read_cpu_usage(prev_total: &mut u64, prev_idle: &mut u64) -> f32 {
    let content = fs::read_to_string("/proc/stat").unwrap_or_default();
    let first_line = content.lines().next().unwrap_or("");
    if let Some((total, idle)) = parse_stat_cpu(first_line) {
        if *prev_total == 0 {
            *prev_total = total;
            *prev_idle = idle;
            return 0.0;
        }
        let d_total = total.saturating_sub(*prev_total);
        let d_idle = idle.saturating_sub(*prev_idle);
        *prev_total = total;
        *prev_idle = idle;
        if d_total == 0 {
            return 0.0;
        }
        ((d_total - d_idle) as f32 / d_total as f32) * 100.0
    } else {
        0.0
    }
}

fn read_memory() -> (u64, u64) {
    let content = fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let mut total: u64 = 0;
    let mut available: u64 = 0;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if let Some(kb) = parts.first().and_then(|s| s.parse::<u64>().ok()) {
                total = kb * 1024;
            }
        }
        if let Some(rest) = line.strip_prefix("MemAvailable:") {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if let Some(kb) = parts.first().and_then(|s| s.parse::<u64>().ok()) {
                available = kb * 1024;
            }
        }
    }
    let used = total.saturating_sub(available);
    (used, total)
}

fn read_cpu_temp_from_thermal() -> Option<f64> {
    let thermal_base = Path::new("/sys/class/thermal");
    if thermal_base.is_dir() {
        let re = regex::Regex::new(r"^thermal_zone\d+$").unwrap();
        if let Ok(entries) = fs::read_dir(thermal_base) {
            let mut readings: Vec<f64> = Vec::new();
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !re.is_match(&name) {
                    continue;
                }
                let temp_path = entry.path().join("temp");
                if let Ok(content) = fs::read_to_string(&temp_path) {
                    if let Ok(millic) = content.trim().parse::<f64>() {
                        let c = millic / 1000.0;
                        if c > 0.0 && c < 200.0 {
                            readings.push(c);
                        }
                    }
                }
            }
            if !readings.is_empty() {
                let avg = readings.iter().sum::<f64>() / readings.len() as f64;
                return Some((avg * 10.0).round() / 10.0);
            }
        }
    }
    None
}

fn read_cpu_temp_from_hwmon() -> Option<f64> {
    let hwmon_base = Path::new("/sys/class/hwmon");
    if !hwmon_base.is_dir() {
        return None;
    }

    let dir_re = regex::Regex::new(r"^hwmon\d+$").unwrap();
    let input_re = regex::Regex::new(r"^temp(\d+)_input$").unwrap();

    let mut readings: Vec<(String, f64)> = Vec::new();

    if let Ok(entries) = fs::read_dir(hwmon_base) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !dir_re.is_match(&name) {
                continue;
            }
            let name_path = entry.path().join("name");
            let Ok(driver) = fs::read_to_string(&name_path) else {
                continue;
            };
            if !matches!(driver.trim(), "k10temp" | "coretemp") {
                continue;
            }
            if let Ok(files) = fs::read_dir(entry.path()) {
                for file in files.flatten() {
                    let fname = file.file_name().to_string_lossy().to_string();
                    if let Some(caps) = input_re.captures(&fname) {
                        let index = caps[1].to_string();
                        let input_path = file.path();
                        let Ok(content) = fs::read_to_string(&input_path) else {
                            continue;
                        };
                        let Ok(millic) = content.trim().parse::<f64>() else {
                            continue;
                        };
                        let c = millic / 1000.0;
                        if !(c > 0.0 && c < 200.0) {
                            continue;
                        }
                        let label_path = entry.path().join(format!("temp{}_label", index));
                        let label = fs::read_to_string(&label_path)
                            .ok()
                            .map(|s| s.trim().to_string())
                            .unwrap_or_default();
                        readings.push((label, c));
                    }
                }
            }
        }
    }

    if readings.is_empty() {
        return None;
    }

    for (label, temp) in &readings {
        if label == "Tdie" || label == "Package id 0" {
            return Some((*temp * 10.0).round() / 10.0);
        }
    }

    for (label, temp) in &readings {
        if label == "Tctl" {
            return Some((*temp * 10.0).round() / 10.0);
        }
    }

    let ccd_readings: Vec<f64> = readings
        .iter()
        .filter(|(label, _)| label.starts_with("Tccd"))
        .map(|(_, temp)| *temp)
        .collect();
    if !ccd_readings.is_empty() {
        let avg = ccd_readings.iter().sum::<f64>() / ccd_readings.len() as f64;
        return Some((avg * 10.0).round() / 10.0);
    }

    if let Some((_, temp)) = readings.iter().find(|(label, _)| !label.is_empty()) {
        return Some((*temp * 10.0).round() / 10.0);
    }

    readings
        .first()
        .map(|(_, temp)| (*temp * 10.0).round() / 10.0)
}

fn read_cpu_temp() -> Option<f64> {
    read_cpu_temp_from_hwmon().or_else(read_cpu_temp_from_thermal)
}

fn read_gpu_temp() -> Option<f64> {
    let drm_base = Path::new("/sys/class/drm");
    if !drm_base.is_dir() {
        return None;
    }
    let card_re = regex::Regex::new(r"^card\d+$").unwrap();
    let hwmon_re = regex::Regex::new(r"^hwmon\d+$").unwrap();
    let input_re = regex::Regex::new(r"^temp(\d+)_input$").unwrap();
    let mut max_temp: Option<f64> = None;

    if let Ok(entries) = fs::read_dir(drm_base) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !card_re.is_match(&name) {
                continue;
            }
            let dev_dir = entry.path().join("device");
            let hwmon_dir = dev_dir.join("hwmon");
            if !hwmon_dir.is_dir() {
                continue;
            }
            if let Ok(hwmons) = fs::read_dir(&hwmon_dir) {
                for hwmon_entry in hwmons.flatten() {
                    let hname = hwmon_entry.file_name().to_string_lossy().to_string();
                    if !hwmon_re.is_match(&hname) {
                        continue;
                    }
                    if let Ok(files) = fs::read_dir(hwmon_entry.path()) {
                        for file in files.flatten() {
                            let fname = file.file_name().to_string_lossy().to_string();
                            if input_re.is_match(&fname) {
                                if let Ok(content) = fs::read_to_string(file.path()) {
                                    if let Ok(millic) = content.trim().parse::<f64>() {
                                        let c = millic / 1000.0;
                                        if c > 0.0 && c < 200.0 {
                                            max_temp = Some(max_temp.map_or(c, |m| m.max(c)));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    max_temp.map(|t| (t * 10.0).round() / 10.0)
}

fn read_gpu_usage() -> Option<f32> {
    let drm_base = Path::new("/sys/class/drm");
    if !drm_base.is_dir() {
        return None;
    }
    let card_re = regex::Regex::new(r"^card\d+$").unwrap();
    let mut max_usage: Option<f32> = None;

    if let Ok(entries) = fs::read_dir(drm_base) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !card_re.is_match(&name) {
                continue;
            }
            let busy_path = entry.path().join("device").join("gpu_busy_percent");
            if let Ok(content) = fs::read_to_string(&busy_path) {
                if let Ok(pct) = content.trim().parse::<f32>() {
                    if (0.0..=100.0).contains(&pct) {
                        max_usage = Some(max_usage.map_or(pct, |m| m.max(pct)));
                    }
                }
            }
        }
    }
    max_usage
}

fn read_vram() -> (u64, u64) {
    let drm_base = Path::new("/sys/class/drm");
    if !drm_base.is_dir() {
        return (0, 0);
    }
    let card_re = regex::Regex::new(r"^card\d+$").unwrap();
    let mut total: u64 = 0;
    let mut used: u64 = 0;

    if let Ok(entries) = fs::read_dir(drm_base) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !card_re.is_match(&name) {
                continue;
            }
            let dev_dir = entry.path().join("device");
            let total_path = dev_dir.join("mem_info_vram_total");
            let used_path = dev_dir.join("mem_info_vram_used");

            if let Ok(content) = fs::read_to_string(&total_path) {
                if let Ok(t) = content.trim().parse::<u64>() {
                    total += t;
                }
            }
            if let Ok(content) = fs::read_to_string(&used_path) {
                if let Ok(u) = content.trim().parse::<u64>() {
                    used += u;
                }
            }
        }
    }
    (used, total)
}

fn read_nvidia_gpu_utilization() -> Option<(Option<f64>, Option<f32>, u64, u64)> {
    let output = std::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=temperature.gpu,utilization.gpu,memory.used,memory.total",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut temp: Option<f64> = None;
    let mut usage: Option<f32> = None;
    let mut vram_used: u64 = 0;
    let mut vram_total: u64 = 0;

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        if parts.len() < 4 {
            continue;
        }
        if let Ok(t) = parts[0].parse::<f64>() {
            temp = Some(temp.map_or(t, |m| m.max(t)));
        }
        if let Ok(u) = parts[1].parse::<f32>() {
            usage = Some(usage.map_or(u, |m| m.max(u)));
        }
        if let Ok(u) = parts[2].parse::<u64>() {
            vram_used += u * 1024 * 1024;
        }
        if let Ok(t) = parts[3].parse::<u64>() {
            vram_total += t * 1024 * 1024;
        }
    }
    if temp.is_none() && usage.is_none() && vram_used == 0 {
        return None;
    }
    Some((temp, usage, vram_used, vram_total))
}

fn read_cpu_freq() -> f64 {
    let cpu_base = Path::new("/sys/devices/system/cpu");
    if cpu_base.is_dir() {
        if let Ok(entries) = fs::read_dir(cpu_base) {
            for entry in entries.flatten() {
                let fname = entry.file_name().to_string_lossy().to_string();
                if fname.starts_with("cpu")
                    && fname.len() > 3
                    && fname[3..].chars().all(|c| c.is_ascii_digit())
                {
                    let freq_path = entry.path().join("cpufreq").join("scaling_cur_freq");
                    if let Ok(content) = fs::read_to_string(&freq_path) {
                        if let Ok(khz) = content.trim().parse::<f64>() {
                            if khz > 0.0 {
                                return khz / 1000.0;
                            }
                        }
                    }
                }
            }
        }
    }
    0.0
}

pub struct SystemMonitor {
    freq_history: VecDeque<f64>,
    temp_history: VecDeque<Option<f64>>,
    prev_total: u64,
    prev_idle: u64,
}

impl SystemMonitor {
    pub fn new() -> Self {
        SystemMonitor {
            freq_history: VecDeque::with_capacity(HISTORY_SIZE),
            temp_history: VecDeque::with_capacity(HISTORY_SIZE),
            prev_total: 0,
            prev_idle: 0,
        }
    }

    pub fn poll(&mut self) -> MonitorSnapshot {
        let temp = read_cpu_temp();
        let freq = read_cpu_freq();
        let usage = read_cpu_usage(&mut self.prev_total, &mut self.prev_idle);
        let (ram_used, ram_total) = read_memory();

        self.freq_history.push_back(freq);
        self.temp_history.push_back(temp);

        if self.freq_history.len() > HISTORY_SIZE {
            self.freq_history.pop_front();
        }
        if self.temp_history.len() > HISTORY_SIZE {
            self.temp_history.pop_front();
        }

        let gpu_temp = read_gpu_temp();
        let gpu_usage =
            read_gpu_usage().or_else(|| read_nvidia_gpu_utilization().and_then(|(_, u, _, _)| u));
        let (vram_used, vram_total) = {
            let (u, t) = read_vram();
            if t > 0 {
                (u, t)
            } else {
                read_nvidia_gpu_utilization()
                    .map(|(_, _, u, t)| (u, t))
                    .unwrap_or((0, 0))
            }
        };

        MonitorSnapshot {
            cpu_temp_c: temp,
            cpu_usage_pct: usage,
            ram_used_bytes: ram_used,
            ram_total_bytes: ram_total,
            gpu_temp_c: gpu_temp,
            gpu_usage_pct: gpu_usage,
            vram_used_bytes: vram_used,
            vram_total_bytes: vram_total,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_stat_cpu_valid() {
        // A typical cpu line: cpu user nice system idle iowait ...
        let line = "cpu 100 200 300 400 500";
        let parsed = parse_stat_cpu(line);
        assert!(parsed.is_some());
        let (total, idle_total) = parsed.unwrap();
        // total = 100 + 200 + 300 + 400 + 500 = 1500
        // idle_total = 400 + 500 = 900
        assert_eq!(total, 1500);
        assert_eq!(idle_total, 900);
    }

    #[test]
    fn test_parse_stat_cpu_extra_spaces() {
        let line = "cpu   100   200   300   400   500   600   700";
        let parsed = parse_stat_cpu(line);
        assert!(parsed.is_some());
        let (total, idle_total) = parsed.unwrap();
        // total = 100 + 200 + 300 + 400 + 500 = 1500
        // idle_total = 400 + 500 = 900
        assert_eq!(total, 1500);
        assert_eq!(idle_total, 900);
    }

    #[test]
    fn test_parse_stat_cpu_invalid_prefix() {
        let line = "cpu0 100 200 300 400 500";
        let parsed = parse_stat_cpu(line);
        assert!(parsed.is_none());
    }

    #[test]
    fn test_parse_stat_cpu_too_few_fields() {
        let line = "cpu 100 200 300";
        let parsed = parse_stat_cpu(line);
        assert!(parsed.is_none());
    }

    #[test]
    fn test_parse_stat_cpu_non_numeric() {
        let line = "cpu abc def 300 400";
        let parsed = parse_stat_cpu(line);
        assert!(parsed.is_none());
    }

    #[test]
    fn test_system_monitor_history_limiting() {
        let mut monitor = SystemMonitor::new();
        assert_eq!(monitor.freq_history.len(), 0);
        assert_eq!(monitor.temp_history.len(), 0);

        // Poll more than HISTORY_SIZE times
        for _ in 0..(HISTORY_SIZE + 50) {
            let _snapshot = monitor.poll();
        }

        assert_eq!(monitor.freq_history.len(), HISTORY_SIZE);
        assert_eq!(monitor.temp_history.len(), HISTORY_SIZE);
    }

    #[test]
    fn test_gpu_monitoring_sysfs_does_not_crash() {
        // These functions read from /sys/class/drm which may not exist in all
        // environments. They should not panic regardless.
        let _temp = read_gpu_temp();
        let _usage = read_gpu_usage();
        let (_used, _total) = read_vram();
        // Just verify they complete without panicking
    }

    #[test]
    fn test_gpu_snapshot_fields_have_defaults() {
        let mut monitor = SystemMonitor::new();
        let snapshot = monitor.poll();
        // GPU fields should be valid (None/0) even if no GPU is present
        assert!(snapshot.gpu_temp_c.is_none() || snapshot.gpu_temp_c.unwrap() > 0.0);
        assert!(
            snapshot.gpu_usage_pct.is_none()
                || (snapshot.gpu_usage_pct.unwrap() >= 0.0
                    && snapshot.gpu_usage_pct.unwrap() <= 100.0)
        );
        assert!(
            snapshot.vram_total_bytes == 0 || snapshot.vram_used_bytes <= snapshot.vram_total_bytes
        );
    }
}
