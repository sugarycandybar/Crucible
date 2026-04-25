"""
Stress test manager — wraps stress-ng subprocess.
"""
from __future__ import annotations

import os
import shutil
import signal
import subprocess
import time
from enum import Enum


class StressState(Enum):
    IDLE = "idle"
    RUNNING = "running"
    STOPPING = "stopping"


class StressManager:
    """Manages a single stress-ng process with safety controls."""

    def __init__(self) -> None:
        self._process: subprocess.Popen | None = None
        self._state = StressState.IDLE
        self._workers = 0
        self._start_time = 0.0
        self._duration_seconds = 0
        self._last_stop_cause: str | None = None
        self._last_elapsed_seconds = 0.0

    # --- public API -----------------------------------------------------

    @property
    def state(self) -> StressState:
        # Refresh state if the process exited on its own.
        if self._state == StressState.RUNNING and self._process is not None:
            ret = self._process.poll()
            if ret is not None:
                elapsed = max(0.0, time.monotonic() - self._start_time)
                self._last_elapsed_seconds = elapsed
                if self._duration_seconds > 0 and elapsed >= self._duration_seconds - 1:
                    self._last_stop_cause = "completed"
                else:
                    self._last_stop_cause = "exited"
                self._process = None
                self._state = StressState.IDLE
                self._workers = 0
                self._duration_seconds = 0
        return self._state

    @property
    def is_running(self) -> bool:
        return self.state == StressState.RUNNING

    @property
    def workers(self) -> int:
        return self._workers

    @property
    def elapsed_seconds(self) -> float:
        if self._state != StressState.RUNNING:
            return 0.0
        return time.monotonic() - self._start_time

    @property
    def last_elapsed_seconds(self) -> float:
        return self._last_elapsed_seconds

    @property
    def last_stop_cause(self) -> str | None:
        return self._last_stop_cause

    @staticmethod
    def is_available() -> bool:
        """Check whether stress-ng can be found on PATH."""
        return shutil.which("stress-ng") is not None

    def start(self, duration_seconds: int = 0) -> bool:
        """Start a stress-ng CPU test.

        Args:
            duration_seconds: Test duration. 0 means run until stopped.

        Returns:
            True if the process was started successfully.
        """
        if self._state != StressState.IDLE:
            return False

        workers = os.cpu_count() or 1
        stressng_path = shutil.which("stress-ng") or "stress-ng"
        cmd = [stressng_path, "--cpu", str(workers), "--metrics-brief"]
        if duration_seconds > 0:
            cmd += ["--timeout", f"{duration_seconds}s"]

        try:
            self._process = subprocess.Popen(
                cmd,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                preexec_fn=os.setsid,
                cwd="/tmp",
            )
        except FileNotFoundError:
            return False

        self._workers = workers
        self._start_time = time.monotonic()
        self._duration_seconds = duration_seconds
        self._last_stop_cause = None
        self._last_elapsed_seconds = 0.0
        self._state = StressState.RUNNING
        return True

    def stop(self, cause: str = "manual") -> None:
        """Gracefully stop the running stress test."""
        if self._process is None:
            self._state = StressState.IDLE
            return

        self._last_elapsed_seconds = max(0.0, time.monotonic() - self._start_time)
        self._last_stop_cause = cause
        self._state = StressState.STOPPING

        try:
            # Kill the entire process group so child workers die too.
            os.killpg(os.getpgid(self._process.pid), signal.SIGTERM)
        except (ProcessLookupError, OSError):
            pass

        try:
            self._process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            try:
                os.killpg(os.getpgid(self._process.pid), signal.SIGKILL)
            except (ProcessLookupError, OSError):
                pass
            try:
                self._process.wait(timeout=2)
            except subprocess.TimeoutExpired:
                pass

        self._process = None
        self._state = StressState.IDLE
        self._workers = 0
        self._duration_seconds = 0

    def kill(self) -> None:
        """Hard-kill on application exit — no grace period."""
        if self._process is None:
            return
        self._last_elapsed_seconds = max(0.0, time.monotonic() - self._start_time)
        self._last_stop_cause = "killed"
        try:
            os.killpg(os.getpgid(self._process.pid), signal.SIGKILL)
        except (ProcessLookupError, OSError):
            pass
        try:
            self._process.wait(timeout=2)
        except subprocess.TimeoutExpired:
            pass
        self._process = None
        self._state = StressState.IDLE
        self._workers = 0
        self._duration_seconds = 0
