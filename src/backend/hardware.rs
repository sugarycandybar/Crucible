use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct GpuInfo {
    pub name: String,
    pub vendor: String,
    pub is_integrated: bool,
}

#[derive(Debug, Clone)]
pub struct SystemSpecs {
    pub os_name: String,
    pub os_version: String,
    pub kernel: String,
    pub cpu_model: String,
    pub cpu_cores_physical: usize,
    pub cpu_cores_logical: usize,
    pub cpu_freq_max_mhz: f64,
    pub gpus: Vec<GpuInfo>,
    pub ram_total_bytes: u64,
}

impl SystemSpecs {
    pub fn ram_total_gb(&self) -> f64 {
        self.ram_total_bytes as f64 / 1_000_000_000.0
    }

    pub fn cpu_freq_max_ghz(&self) -> f64 {
        if self.cpu_freq_max_mhz > 0.0 {
            self.cpu_freq_max_mhz / 1000.0
        } else {
            0.0
        }
    }

    pub fn to_plain_text(&self) -> String {
        let mut lines = vec!["System Specs".to_string(), String::new()];

        let os_display = if !self.os_version.is_empty()
            && !self.os_name.contains(&self.os_version)
        {
            format!("{} {}", self.os_name, self.os_version)
        } else {
            self.os_name.clone()
        };
        lines.push(format!("OS: {os_display}"));
        lines.push(format!("Kernel: {}", self.kernel));
        lines.push(format!(
            "CPU: {} ({}C/{}T @ {:.2} GHz)",
            self.cpu_model,
            self.cpu_cores_physical,
            self.cpu_cores_logical,
            self.cpu_freq_max_ghz()
        ));

        if self.gpus.is_empty() {
            lines.push("GPU: Unknown".to_string());
        } else {
            for (i, gpu) in self.gpus.iter().enumerate() {
                let tag = if gpu.is_integrated { " (integrated)" } else { "" };
                lines.push(format!("GPU {i}: {}{tag}", gpu.name));
            }
        }

        lines.push(format!("RAM: {:.1} GB", self.ram_total_gb()));
        lines.join("\n")
    }
}

fn read_os_release(path: &str) -> HashMap<String, String> {
    let mut info = HashMap::new();
    if let Ok(content) = fs::read_to_string(path) {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                info.insert(k.to_string(), v.trim_matches('"').trim_matches('\'').to_string());
            }
        }
    }
    info
}

fn get_os_info() -> (String, String) {
    let (name, ver) = if Path::new("/run/host/os-release").exists() {
        let info = read_os_release("/run/host/os-release");
        (
            info.get("PRETTY_NAME").cloned().unwrap_or_else(|| "Linux".to_string()),
            info.get("VERSION_ID").unwrap_or(&String::new()).clone(),
        )
    } else if Path::new("/etc/os-release").exists() {
        let info = read_os_release("/etc/os-release");
        (
            info.get("PRETTY_NAME").cloned().unwrap_or_else(|| "Linux".to_string()),
            info.get("VERSION_ID").unwrap_or(&String::new()).clone(),
        )
    } else {
        (std::env::consts::OS.to_string(), String::new())
    };
    (name, ver)
}

fn get_cpu_model() -> String {
    if let Ok(content) = fs::read_to_string("/proc/cpuinfo") {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("model name") || trimmed.starts_with("Processor") {
                if let Some((_, val)) = trimmed.split_once(':') {
                    return val.trim().to_string();
                }
            }
        }
    }
    "Unknown CPU".to_string()
}

fn get_cpu_counts() -> (usize, usize) {
    let mut cores: HashSet<(String, String)> = HashSet::new();
    let mut logical = 0;
    let mut phys_id = String::new();
    let mut core_id = String::new();

    if let Ok(content) = fs::read_to_string("/proc/cpuinfo") {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(val) = trimmed.strip_prefix("physical id") {
                if let Some((_, id)) = val.split_once(':') {
                    phys_id = id.trim().to_string();
                }
            } else if let Some(val) = trimmed.strip_prefix("core id") {
                if let Some((_, id)) = val.split_once(':') {
                    core_id = id.trim().to_string();
                }
            } else if trimmed.starts_with("processor") {
                logical += 1;
                if !phys_id.is_empty() && !core_id.is_empty() {
                    cores.insert((phys_id.clone(), core_id.clone()));
                }
                phys_id.clear();
                core_id.clear();
            }
        }
    }

    let physical = if cores.is_empty() {
        logical
    } else {
        cores.len()
    };

    (physical.max(1), logical.max(1))
}

fn get_cpu_freq_max() -> f64 {
    let mut values: Vec<f64> = Vec::new();
    let dir = Path::new("/sys/devices/system/cpu");
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let fname = entry.file_name().to_string_lossy().to_string();
            if fname.starts_with("cpu") && fname.len() > 3
                && fname[3..].chars().all(|c| c.is_ascii_digit())
            {
                let freq_path = entry.path().join("cpufreq").join("cpuinfo_max_freq");
                if let Ok(content) = fs::read_to_string(&freq_path) {
                    if let Ok(khz) = content.trim().parse::<f64>() {
                        if khz > 0.0 {
                            values.push(khz);
                        }
                    }
                }
            }
        }
    }
    if values.is_empty() {
        return 0.0;
    }
    values.into_iter().fold(f64::NEG_INFINITY, f64::max) / 1000.0
}

fn get_total_ram() -> u64 {
    if let Ok(content) = fs::read_to_string("/proc/meminfo") {
        for line in content.lines() {
            if let Some(rest) = line.strip_prefix("MemTotal:") {
                let parts: Vec<&str> = rest.split_whitespace().collect();
                if let Some(kb_str) = parts.first() {
                    if let Ok(kb) = kb_str.parse::<u64>() {
                        return kb * 1024;
                    }
                }
            }
        }
    }
    0
}

fn get_kernel() -> String {
    fs::read_to_string("/proc/sys/kernel/osrelease")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| std::env::consts::OS.to_string())
}

fn pci_id_to_name(vendor_id: &str, device_id: &str) -> Option<String> {
    let addr = format!("{vendor_id}:{device_id}");
    if let Ok(output) = Command::new("lspci").args(["-d", &addr]).output() {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if let Some((_, rest)) = line.split_once(": ") {
                    let name = rest.trim().to_string();
                    let re = regex::Regex::new(r"\s*\(rev\s+[0-9a-fA-F]+\)\s*$").unwrap();
                    let name = re.replace(&name, "").to_string();
                    return Some(name);
                }
            }
        }
    }
    None
}

const KNOWN_GPU_VENDORS: &[(&str, &str)] = &[
    ("0x10de", "NVIDIA"),
    ("0x1002", "AMD"),
    ("0x8086", "Intel"),
];

fn detect_gpus() -> Vec<GpuInfo> {
    let mut gpus: Vec<GpuInfo> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let drm_base = Path::new("/sys/class/drm");

    if drm_base.is_dir() {
        let re = regex::Regex::new(r"^card\d+$").unwrap();
        if let Ok(entries) = fs::read_dir(drm_base) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !re.is_match(&name) {
                    continue;
                }

                let dev_dir = entry.path().join("device");
                let vendor_path = dev_dir.join("vendor");
                let device_path = dev_dir.join("device");

                if !vendor_path.is_file() {
                    continue;
                }

                let vendor_id = fs::read_to_string(&vendor_path).ok()
                    .map(|s| s.trim().to_string());
                let device_id = if device_path.is_file() {
                    fs::read_to_string(&device_path).ok()
                        .map(|s| s.trim().to_string())
                } else {
                    None
                };

                let Some(vendor_id) = vendor_id else { continue };
                let device_id = device_id.unwrap_or_default();
                let key = format!("{vendor_id}:{device_id}");
                if seen.contains(&key) {
                    continue;
                }
                seen.insert(key);

                let vendor_label = KNOWN_GPU_VENDORS
                    .iter()
                    .find(|(id, _)| *id == vendor_id)
                    .map(|(_, label)| *label)
                    .unwrap_or(&vendor_id)
                    .to_string();

                let is_igpu = vendor_id == "0x8086";
                let nice_name = pci_id_to_name(&vendor_id, &device_id);
                let name = nice_name.unwrap_or_else(|| format!("{vendor_label} GPU"));

                gpus.push(GpuInfo {
                    name,
                    vendor: vendor_label,
                    is_integrated: is_igpu,
                });
            }
        }
    }

    if gpus.is_empty() {
        if let Ok(output) = Command::new("lspci").output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    let lower = line.to_lowercase();
                    if lower.contains("vga") || lower.contains("3d") || lower.contains("display") {
                        if let Some((_, name)) = line.split_once(": ") {
                            let name = name.trim().to_string();
                            let mut vendor = String::new();
                            let mut is_igpu = false;
                            for (_vid, vlabel) in KNOWN_GPU_VENDORS {
                                if name.to_lowercase().contains(&vlabel.to_lowercase()) {
                                    vendor = vlabel.to_string();
                                    is_igpu = *vlabel == "Intel";
                                    break;
                                }
                            }
                            gpus.push(GpuInfo {
                                name,
                                vendor,
                                is_integrated: is_igpu,
                            });
                        }
                    }
                }
            }
        }
    }

    gpus
}

pub fn gather_specs() -> SystemSpecs {
    let (os_name, os_version) = get_os_info();
    let (cpu_cores_physical, cpu_cores_logical) = get_cpu_counts();

    SystemSpecs {
        os_name,
        os_version,
        kernel: get_kernel(),
        cpu_model: get_cpu_model(),
        cpu_cores_physical,
        cpu_cores_logical,
        cpu_freq_max_mhz: get_cpu_freq_max(),
        gpus: detect_gpus(),
        ram_total_bytes: get_total_ram(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ram_total_gb() {
        let specs = SystemSpecs {
            os_name: "TestOS".to_string(),
            os_version: "1.0".to_string(),
            kernel: "5.15.0".to_string(),
            cpu_model: "Intel i7".to_string(),
            cpu_cores_physical: 4,
            cpu_cores_logical: 8,
            cpu_freq_max_mhz: 3600.0,
            gpus: vec![],
            ram_total_bytes: 16_000_000_000,
        };
        assert_eq!(specs.ram_total_gb(), 16.0);
    }

    #[test]
    fn test_cpu_freq_max_ghz() {
        let mut specs = SystemSpecs {
            os_name: "TestOS".to_string(),
            os_version: "1.0".to_string(),
            kernel: "5.15.0".to_string(),
            cpu_model: "Intel i7".to_string(),
            cpu_cores_physical: 4,
            cpu_cores_logical: 8,
            cpu_freq_max_mhz: 3600.0,
            gpus: vec![],
            ram_total_bytes: 16_000_000_000,
        };
        assert_eq!(specs.cpu_freq_max_ghz(), 3.6);

        specs.cpu_freq_max_mhz = 0.0;
        assert_eq!(specs.cpu_freq_max_ghz(), 0.0);
    }

    #[test]
    fn test_to_plain_text() {
        let specs_no_gpu = SystemSpecs {
            os_name: "Ubuntu".to_string(),
            os_version: "22.04".to_string(),
            kernel: "5.15.0".to_string(),
            cpu_model: "Intel Core i7-10700K".to_string(),
            cpu_cores_physical: 8,
            cpu_cores_logical: 16,
            cpu_freq_max_mhz: 5100.0,
            gpus: vec![],
            ram_total_bytes: 16_000_000_000,
        };
        let text = specs_no_gpu.to_plain_text();
        assert!(text.contains("OS: Ubuntu 22.04"));
        assert!(text.contains("Kernel: 5.15.0"));
        assert!(text.contains("CPU: Intel Core i7-10700K (8C/16T @ 5.10 GHz)"));
        assert!(text.contains("GPU: Unknown"));
        assert!(text.contains("RAM: 16.0 GB"));

        let specs_with_gpu = SystemSpecs {
            os_name: "Ubuntu 22.04 LTS".to_string(),
            os_version: "22.04".to_string(),
            kernel: "5.15.0".to_string(),
            cpu_model: "Intel Core i7-10700K".to_string(),
            cpu_cores_physical: 8,
            cpu_cores_logical: 16,
            cpu_freq_max_mhz: 5100.0,
            gpus: vec![
                GpuInfo {
                    name: "NVIDIA GeForce RTX 3080".to_string(),
                    vendor: "NVIDIA".to_string(),
                    is_integrated: false,
                },
                GpuInfo {
                    name: "Intel UHD Graphics 630".to_string(),
                    vendor: "Intel".to_string(),
                    is_integrated: true,
                },
            ],
            ram_total_bytes: 16_000_000_000,
        };
        let text_with_gpu = specs_with_gpu.to_plain_text();
        assert!(text_with_gpu.contains("OS: Ubuntu 22.04 LTS"));
        assert!(text_with_gpu.contains("GPU 0: NVIDIA GeForce RTX 3080"));
        assert!(text_with_gpu.contains("GPU 1: Intel UHD Graphics 630 (integrated)"));
    }

    #[test]
    fn test_read_os_release() {
        let mut temp_file = std::env::temp_dir();
        temp_file.push("test-os-release");
        
        let content = "\
# This is a comment
PRETTY_NAME=\"Ubuntu 22.04.2 LTS\"
VERSION_ID=\"22.04\"
INVALID_LINE
";
        fs::write(&temp_file, content).unwrap();

        let info = read_os_release(&temp_file.to_string_lossy());
        assert_eq!(info.get("PRETTY_NAME").unwrap(), "Ubuntu 22.04.2 LTS");
        assert_eq!(info.get("VERSION_ID").unwrap(), "22.04");
        assert_eq!(info.get("INVALID_LINE"), None);

        let _ = fs::remove_file(temp_file);
    }
}

