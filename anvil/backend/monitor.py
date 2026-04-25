"""
Live system monitor — polls temperatures and CPU frequency.
"""
from __future__ import annotations

from collections import deque
from dataclasses import dataclass, field

import psutil


# How many data points to keep (1 per second → 300 = 5 minutes).
_HISTORY_SIZE = 300


@dataclass
class MonitorSnapshot:
    """A single point-in-time reading."""

    cpu_temp_c: float | None = None
    cpu_freq_mhz: float = 0.0
    cpu_usage_pct: float = 0.0
    ram_used_bytes: int = 0
    ram_total_bytes: int = 0


class SystemMonitor:
    """Polls live sensor data and keeps a rolling history."""

    def __init__(self) -> None:
        self._freq_history: deque[float] = deque(maxlen=_HISTORY_SIZE)
        self._temp_history: deque[float | None] = deque(maxlen=_HISTORY_SIZE)

    # --- public API -----------------------------------------------------

    @property
    def freq_history(self) -> list[float]:
        return list(self._freq_history)

    @property
    def temp_history(self) -> list[float | None]:
        return list(self._temp_history)

    def poll(self) -> MonitorSnapshot:
        """Take a fresh reading and append to history buffers."""
        temp = self._read_cpu_temp()
        freq = self._read_cpu_freq()
        usage = psutil.cpu_percent(interval=None)
        mem = psutil.virtual_memory()

        self._freq_history.append(freq)
        self._temp_history.append(temp)

        return MonitorSnapshot(
            cpu_temp_c=temp,
            cpu_freq_mhz=freq,
            cpu_usage_pct=usage,
            ram_used_bytes=mem.used,
            ram_total_bytes=mem.total,
        )

    # --- internals ------------------------------------------------------

    @staticmethod
    def _read_cpu_temp() -> float | None:
        """Best-effort CPU temperature reading."""
        try:
            temps = psutil.sensors_temperatures()
        except (AttributeError, OSError):
            return None

        if not temps:
            return None

        # Prefer coretemp (Intel) or k10temp (AMD); fall back to first available.
        for key in ("coretemp", "k10temp", "zenpower", "cpu_thermal", "acpitz"):
            if key in temps:
                entries = temps[key]
                if entries:
                    # Average across cores for a single number.
                    readings = [e.current for e in entries if e.current > 0]
                    if readings:
                        return round(sum(readings) / len(readings), 1)

        # Fall back to the first sensor that has data.
        for entries in temps.values():
            readings = [e.current for e in entries if e.current > 0]
            if readings:
                return round(sum(readings) / len(readings), 1)

        return None

    @staticmethod
    def _read_cpu_freq() -> float:
        """Current aggregate CPU frequency in MHz."""
        freq = psutil.cpu_freq()
        if freq:
            return freq.current
        return 0.0
