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
            let Ok(driver) = fs::read_to_string(&name_path) else { continue; };
            if !matches!(driver.trim(), "k10temp" | "coretemp") {
                continue;
            }
            if let Ok(files) = fs::read_dir(entry.path()) {
                for file in files.flatten() {
                    let fname = file.file_name().to_string_lossy().to_string();
                    if let Some(caps) = input_re.captures(&fname) {
                        let index = caps[1].to_string();
                        let input_path = file.path();
                        let Ok(content) = fs::read_to_string(&input_path) else { continue; };
                        let Ok(millic) = content.trim().parse::<f64>() else { continue; };
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

    let ccd_readings: Vec<f64> = readings.iter()
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

    readings.first().map(|(_, temp)| (*temp * 10.0).round() / 10.0)
}

fn read_cpu_temp() -> Option<f64> {
    read_cpu_temp_from_thermal().or_else(read_cpu_temp_from_hwmon)
}

fn read_cpu_freq() -> f64 {
    let cpu_base = Path::new("/sys/devices/system/cpu");
    if cpu_base.is_dir() {
        if let Ok(entries) = fs::read_dir(cpu_base) {
            for entry in entries.flatten() {
                let fname = entry.file_name().to_string_lossy().to_string();
                if fname.starts_with("cpu") && fname.len() > 3
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

        MonitorSnapshot {
            cpu_temp_c: temp,
            cpu_usage_pct: usage,
            ram_used_bytes: ram_used,
            ram_total_bytes: ram_total,
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
}

