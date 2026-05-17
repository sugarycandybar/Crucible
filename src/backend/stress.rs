use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StressState {
    Idle,
    Running,
    Stopping,
}

pub struct StressManager {
    process: Option<Child>,
    state: StressState,
    workers: usize,
    start_time: Option<Instant>,
    duration_seconds: u64,
    last_stop_cause: Option<String>,
    last_elapsed_seconds: f64,
}

impl StressManager {
    pub fn new() -> Self {
        StressManager {
            process: None,
            state: StressState::Idle,
            workers: 0,
            start_time: None,
            duration_seconds: 0,
            last_stop_cause: None,
            last_elapsed_seconds: 0.0,
        }
    }

    pub fn state(&mut self) -> StressState {
        if self.state == StressState::Running {
            if let Some(ref mut process) = self.process {
                match process.try_wait() {
                    Ok(Some(_status)) => {
                        let elapsed = self
                            .start_time
                            .map(|t| t.elapsed().as_secs_f64())
                            .unwrap_or(0.0);
                        self.last_elapsed_seconds = elapsed.max(0.0);

                        if self.duration_seconds > 0 && elapsed >= self.duration_seconds.saturating_sub(1) as f64 {
                            self.last_stop_cause = Some("completed".to_string());
                        } else {
                            self.last_stop_cause = Some("exited".to_string());
                        }

                        self.process = None;
                        self.state = StressState::Idle;
                        self.workers = 0;
                        self.duration_seconds = 0;
                    }
                    Ok(None) => {}
                    Err(_) => {
                        self.process = None;
                        self.state = StressState::Idle;
                    }
                }
            }
        }
        self.state
    }

    pub fn is_running(&mut self) -> bool {
        self.state() == StressState::Running
    }

    pub fn elapsed_seconds(&self) -> f64 {
        if self.state != StressState::Running {
            return 0.0;
        }
        self.start_time
            .map(|t| t.elapsed().as_secs_f64())
            .unwrap_or(0.0)
    }

    pub fn last_elapsed_seconds(&self) -> f64 {
        self.last_elapsed_seconds
    }

    pub fn last_stop_cause(&self) -> Option<&str> {
        self.last_stop_cause.as_deref()
    }

    pub fn is_available() -> bool {
        Command::new("stress-ng")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
    }

    pub fn start(&mut self, duration_seconds: u64) -> bool {
        if self.state != StressState::Idle {
            return false;
        }

        let workers = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);

        let mut cmd = Command::new("stress-ng");
        cmd.args(["--cpu", &workers.to_string(), "--metrics-brief"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0)
            .current_dir("/tmp");

        if duration_seconds > 0 {
            cmd.arg("--timeout");
            cmd.arg(format!("{duration_seconds}s"));
        }

        let child = match cmd.spawn() {
            Ok(c) => c,
            Err(_) => return false,
        };

        self.process = Some(child);
        self.workers = workers;
        self.start_time = Some(Instant::now());
        self.duration_seconds = duration_seconds;
        self.last_stop_cause = None;
        self.last_elapsed_seconds = 0.0;
        self.state = StressState::Running;
        true
    }

    pub fn stop(&mut self, cause: &str) {
        let Some(ref mut process) = self.process else {
            self.state = StressState::Idle;
            return;
        };

        self.last_elapsed_seconds = self
            .start_time
            .map(|t| t.elapsed().as_secs_f64().max(0.0))
            .unwrap_or(0.0);
        self.last_stop_cause = Some(cause.to_string());
        self.state = StressState::Stopping;

        let pid = process.id() as i32;
        unsafe {
            libc::killpg(pid, libc::SIGTERM);
        }

        let start = Instant::now();
        while start.elapsed().as_secs() < 3 {
            if let Ok(Some(_)) = process.try_wait() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        if let Ok(None) = process.try_wait() {
            unsafe {
                libc::killpg(pid, libc::SIGKILL);
            }
            let start = Instant::now();
            while start.elapsed().as_secs() < 2 {
                if let Ok(Some(_)) = process.try_wait() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }

        self.process = None;
        self.state = StressState::Idle;
        self.workers = 0;
        self.duration_seconds = 0;
    }

    pub fn kill(&mut self) {
        let Some(ref mut process) = self.process else {
            return;
        };

        self.last_elapsed_seconds = self
            .start_time
            .map(|t| t.elapsed().as_secs_f64().max(0.0))
            .unwrap_or(0.0);
        self.last_stop_cause = Some("killed".to_string());

        let pid = process.id() as i32;
        unsafe {
            libc::killpg(pid, libc::SIGKILL);
        }

        let start = Instant::now();
        while start.elapsed().as_secs() < 2 {
            if let Ok(Some(_)) = process.try_wait() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        self.process = None;
        self.state = StressState::Idle;
        self.workers = 0;
        self.duration_seconds = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stress_manager_new() {
        let mut manager = StressManager::new();
        assert_eq!(manager.state(), StressState::Idle);
        assert!(!manager.is_running());
        assert_eq!(manager.elapsed_seconds(), 0.0);
        assert_eq!(manager.last_elapsed_seconds(), 0.0);
        assert_eq!(manager.last_stop_cause(), None);
    }

    #[test]
    fn test_stress_manager_lifecycle() {
        if !StressManager::is_available() {
            println!("stress-ng is not available, skipping lifecycle test");
            return;
        }

        let mut manager = StressManager::new();
        assert!(manager.start(5)); // start with 5 seconds duration
        assert_eq!(manager.state(), StressState::Running);
        assert!(manager.is_running());

        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(manager.elapsed_seconds() > 0.0);

        manager.stop("manual_stop");
        assert_eq!(manager.state(), StressState::Idle);
        assert!(!manager.is_running());
        assert_eq!(manager.last_stop_cause(), Some("manual_stop"));
        assert!(manager.last_elapsed_seconds() > 0.0);
    }

    #[test]
    fn test_stress_manager_kill() {
        if !StressManager::is_available() {
            println!("stress-ng is not available, skipping kill test");
            return;
        }

        let mut manager = StressManager::new();
        assert!(manager.start(10));
        assert!(manager.is_running());

        std::thread::sleep(std::time::Duration::from_millis(100));

        manager.kill();
        assert_eq!(manager.state(), StressState::Idle);
        assert_eq!(manager.last_stop_cause(), Some("killed"));
    }
}

