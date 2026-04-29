"""
Hardware discovery — gathers static system specs at launch.
"""
from __future__ import annotations

import glob
import os
import platform
import re
import subprocess
from dataclasses import dataclass, field

import psutil


@dataclass
class GpuInfo:
    """Represents a single GPU."""

    name: str
    vendor: str = ""
    is_integrated: bool = False


@dataclass
class SystemSpecs:
    """All discoverable hardware specs."""

    os_name: str = ""
    os_version: str = ""
    kernel: str = ""
    cpu_model: str = ""
    cpu_cores_physical: int = 0
    cpu_cores_logical: int = 0
    cpu_freq_max_mhz: float = 0.0
    gpus: list[GpuInfo] = field(default_factory=list)
    ram_total_bytes: int = 0

    # --- helpers --------------------------------------------------------

    @property
    def ram_total_gib(self) -> float:
        return self.ram_total_bytes / (1024 ** 3)

    @property
    def ram_total_gb(self) -> float:
        return self.ram_total_bytes / 1_000_000_000

    @property
    def cpu_freq_max_ghz(self) -> float:
        return self.cpu_freq_max_mhz / 1000 if self.cpu_freq_max_mhz else 0.0

    def to_plain_text(self) -> str:
        """Generate a shareable plain-text block."""
        lines = ["System Specs", ""]
        os_display = self.os_name
        if self.os_version and self.os_version not in self.os_name:
            os_display = f"{self.os_name} {self.os_version}"
        lines.append(f"OS: {os_display}")
        lines.append(f"Kernel: {self.kernel}")
        lines.append(
            f"CPU: {self.cpu_model} "
            f"({self.cpu_cores_physical}C/{self.cpu_cores_logical}T"
            f" @ {self.cpu_freq_max_ghz:.2f} GHz)"
        )

        if self.gpus:
            for i, gpu in enumerate(self.gpus):
                tag = " (integrated)" if gpu.is_integrated else ""
                lines.append(f"GPU {i}: {gpu.name}{tag}")
        else:
            lines.append("GPU: Unknown")

        lines.append(f"RAM: {self.ram_total_gb:.1f} GB")
        return "\n".join(lines)

    def to_markdown(self) -> str:
        """Backwards-compatible alias for plain-text export."""
        return self.to_plain_text()


# --- discovery functions ------------------------------------------------


def _get_os_info() -> tuple[str, str]:
    """Return (distro pretty-name, version-id)."""
    try:
        # If running inside Flatpak, read the host's os-release
        if os.path.exists("/run/host/os-release"):
            info = {}
            with open("/run/host/os-release") as f:
                for line in f:
                    line = line.strip()
                    if not line or line.startswith("#"):
                        continue
                    if "=" in line:
                        k, v = line.split("=", 1)
                        info[k] = v.strip('"\'')
            return info.get("PRETTY_NAME", "Linux"), info.get("VERSION_ID", "")
        
        info = platform.freedesktop_os_release()
        return info.get("PRETTY_NAME", "Linux"), info.get("VERSION_ID", "")
    except OSError:
        return platform.system(), platform.version()


def _get_cpu_model() -> str:
    """Read the CPU model string."""
    # Try /proc/cpuinfo first (most reliable on Linux).
    try:
        with open("/proc/cpuinfo") as f:
            for line in f:
                if line.startswith("model name"):
                    return line.split(":", 1)[1].strip()
    except OSError:
        pass

    model = platform.processor()
    return model if model else "Unknown CPU"


def _get_cpu_freq_max() -> float:
    """Return the maximum CPU frequency in MHz."""
    max_khz_values: list[int] = []

    for path in glob.glob("/sys/devices/system/cpu/cpu[0-9]*/cpufreq/cpuinfo_max_freq"):
        try:
            value = int(open(path).read().strip())
            if value > 0:
                max_khz_values.append(value)
        except (OSError, ValueError):
            continue

    if max_khz_values:
        return max(max_khz_values) / 1000.0

    freq = psutil.cpu_freq()
    if freq and freq.max:
        return freq.max
    if freq and freq.current:
        return freq.current
    return 0.0


def _pci_id_to_name(vendor_id: str, device_id: str) -> str | None:
    """Try to resolve a PCI vendor+device to a human name via lspci."""
    # Use lspci (non-machine-readable) for the best device description.
    try:
        out = subprocess.check_output(
            ["lspci", "-d", f"{vendor_id}:{device_id}"],
            text=True,
            timeout=3,
            stderr=subprocess.DEVNULL,
        )
        for line in out.strip().splitlines():
            # Format: "XX:XX.X Class: Vendor Device Name [rev XX]"
            parts = line.split(": ", 1)
            if len(parts) == 2:
                name = parts[1].strip()
                # Strip trailing revision info
                name = re.sub(r"\s*\(rev\s+[0-9a-fA-F]+\)\s*$", "", name)
                return name
    except (FileNotFoundError, subprocess.SubprocessError):
        pass
    return None


_KNOWN_GPU_VENDORS = {
    "0x10de": "NVIDIA",
    "0x1002": "AMD",
    "0x8086": "Intel",
}


def _detect_gpus() -> list[GpuInfo]:
    """Detect GPUs via /sys/class/drm and lspci fallback."""
    gpus: list[GpuInfo] = []
    seen: set[str] = set()
    drm_base = "/sys/class/drm"

    # Walk /sys/class/drm/card* entries.
    if os.path.isdir(drm_base):
        for entry in sorted(os.listdir(drm_base)):
            if not re.match(r"^card\d+$", entry):
                continue

            dev_dir = os.path.join(drm_base, entry, "device")
            vendor_path = os.path.join(dev_dir, "vendor")
            device_path = os.path.join(dev_dir, "device")

            if not os.path.isfile(vendor_path):
                continue

            try:
                vendor_id = open(vendor_path).read().strip()
                device_id = (
                    open(device_path).read().strip()
                    if os.path.isfile(device_path)
                    else ""
                )
            except OSError:
                continue

            key = f"{vendor_id}:{device_id}"
            if key in seen:
                continue
            seen.add(key)

            vendor_label = _KNOWN_GPU_VENDORS.get(vendor_id, vendor_id)
            is_igpu = vendor_id == "0x8086"  # Intel is usually integrated

            # Attempt lspci for a nicer name.
            nice_name = _pci_id_to_name(vendor_id, device_id)
            name = nice_name if nice_name else f"{vendor_label} GPU"

            gpus.append(GpuInfo(name=name, vendor=vendor_label, is_integrated=is_igpu))

    # Fallback: parse lspci directly.
    if not gpus:
        try:
            out = subprocess.check_output(
                ["lspci"], text=True, timeout=3, stderr=subprocess.DEVNULL
            )
            for line in out.splitlines():
                lower = line.lower()
                if "vga" in lower or "3d" in lower or "display" in lower:
                    # Format: "XX:XX.X VGA compatible controller: <name>"
                    parts = line.split(": ", 1)
                    if len(parts) == 2:
                        name = parts[1].strip()
                        vendor = ""
                        is_igpu = False
                        for vid, vlabel in _KNOWN_GPU_VENDORS.items():
                            if vlabel.lower() in name.lower():
                                vendor = vlabel
                                is_igpu = vlabel == "Intel"
                                break
                        gpus.append(
                            GpuInfo(name=name, vendor=vendor, is_integrated=is_igpu)
                        )
        except (FileNotFoundError, subprocess.SubprocessError):
            pass

    return gpus


def gather_specs() -> SystemSpecs:
    """Collect all hardware specs and return a SystemSpecs object."""
    os_name, os_version = _get_os_info()
    return SystemSpecs(
        os_name=os_name,
        os_version=os_version,
        kernel=platform.release(),
        cpu_model=_get_cpu_model(),
        cpu_cores_physical=psutil.cpu_count(logical=False) or 1,
        cpu_cores_logical=psutil.cpu_count(logical=True) or 1,
        cpu_freq_max_mhz=_get_cpu_freq_max(),
        gpus=_detect_gpus(),
        ram_total_bytes=psutil.virtual_memory().total,
    )
