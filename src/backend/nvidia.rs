//! NVIDIA statistics through NVML.

use std::sync::OnceLock;

use nvml_wrapper::enum_wrappers::device::TemperatureSensor;
use nvml_wrapper::Nvml;

#[derive(Debug, Clone, Copy, Default)]
pub struct NvidiaStats {
    pub temperature_c: Option<f64>,
    pub usage_pct: Option<f32>,
    pub vram_used_bytes: u64,
    pub vram_total_bytes: u64,
}

fn nvml() -> Option<&'static Nvml> {
    static NVML: OnceLock<Option<Nvml>> = OnceLock::new();
    NVML.get_or_init(|| Nvml::init().ok()).as_ref()
}

pub fn snapshot() -> Option<NvidiaStats> {
    let nvml = nvml()?;

    let count = nvml.device_count().ok()?;
    if count == 0 {
        return None;
    }

    let mut stats = NvidiaStats::default();
    let mut found = false;

    for index in 0..count {
        let Ok(device) = nvml.device_by_index(index) else {
            continue;
        };
        found = true;

        if let Ok(temp) = device.temperature(TemperatureSensor::Gpu) {
            let temp = f64::from(temp);
            if temp > 0.0 {
                stats.temperature_c = Some(stats.temperature_c.map_or(temp, |max| max.max(temp)));
            }
        }

        if let Ok(utilization) = device.utilization_rates() {
            let usage = utilization.gpu as f32;
            if (0.0..=100.0).contains(&usage) {
                stats.usage_pct = Some(stats.usage_pct.map_or(usage, |max| max.max(usage)));
            }
        }

        if let Ok(memory) = device.memory_info() {
            stats.vram_used_bytes += memory.used;
            stats.vram_total_bytes += memory.total;
        }
    }

    found.then_some(stats)
}

pub fn total_vram_bytes() -> Option<u64> {
    snapshot()
        .map(|stats| stats.vram_total_bytes)
        .filter(|total| *total > 0)
}

pub fn is_present() -> bool {
    let drm_base = std::path::Path::new("/sys/class/drm");
    let Ok(entries) = std::fs::read_dir(drm_base) else {
        return false;
    };
    let card_re = regex::Regex::new(r"^card\d+$").unwrap();

    entries.flatten().any(|entry| {
        if !card_re.is_match(&entry.file_name().to_string_lossy()) {
            return false;
        }
        std::fs::read_to_string(entry.path().join("device").join("vendor"))
            .map(|vendor| vendor.trim() == "0x10de")
            .unwrap_or(false)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nvml_queries_do_not_panic_without_driver() {
        let _ = snapshot();
        let _ = total_vram_bytes();
        let _ = is_present();
    }
}
