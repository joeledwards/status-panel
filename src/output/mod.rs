#[cfg(all(target_os = "linux", any(target_arch = "arm", target_arch = "aarch64")))]
mod hd44780;
#[cfg(not(all(target_os = "linux", any(target_arch = "arm", target_arch = "aarch64"))))]
mod unsupported_hardware;
mod log;
mod terminal;

#[cfg(all(target_os = "linux", any(target_arch = "arm", target_arch = "aarch64")))]
pub use hd44780::Hd44780;
#[cfg(not(all(target_os = "linux", any(target_arch = "arm", target_arch = "aarch64"))))]
pub use unsupported_hardware::Hd44780;
pub use log::LogOutput;
pub use terminal::TerminalOutput;

use crate::{Frame, refresh::{RefreshPlanner, UpdateStrategy, WriteRun}};
use anyhow::Result;
use std::time::{Duration, Instant};

pub trait Output {
    fn name(&self) -> &str;
    fn render(&mut self, frame: &Frame, now: Instant) -> Result<()>;
    fn shutdown(&mut self) -> Result<()> { Ok(()) }
}

pub trait CharacterDevice {
    fn initialize(&mut self) -> Result<()>;
    fn write_run(&mut self, run: &WriteRun) -> Result<()>;
    fn shutdown(&mut self) -> Result<()>;
}

pub struct BufferedOutput<D> {
    name: String,
    device: D,
    planner: RefreshPlanner,
    reinitialize_on_full: bool,
    initialized: bool,
}

impl<D: CharacterDevice> BufferedOutput<D> {
    pub fn new(name: impl Into<String>, device: D, strategy: UpdateStrategy,
        forced_refresh: Option<Duration>, reinitialize_on_full: bool) -> Self {
        Self {
            name: name.into(), device,
            planner: RefreshPlanner::new(strategy, forced_refresh),
            reinitialize_on_full, initialized: false,
        }
    }
}

impl<D: CharacterDevice> Output for BufferedOutput<D> {
    fn name(&self) -> &str { &self.name }

    fn render(&mut self, frame: &Frame, now: Instant) -> Result<()> {
        if !self.initialized {
            self.device.initialize()?;
            self.initialized = true;
        }
        let (runs, full, forced) = self.planner.plan(frame, now);
        if forced && self.reinitialize_on_full {
            self.device.initialize()?;
        }
        for run in &runs {
            if let Err(error) = self.device.write_run(run) {
                self.planner.invalidate();
                return Err(error);
            }
        }
        self.planner.commit(frame.clone(), now, full);
        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> { self.device.shutdown() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Dimensions;

    #[derive(Default)] struct Fake { writes: Vec<WriteRun> }
    impl CharacterDevice for Fake {
        fn initialize(&mut self) -> Result<()> { Ok(()) }
        fn write_run(&mut self, run: &WriteRun) -> Result<()> { self.writes.push(run.clone()); Ok(()) }
        fn shutdown(&mut self) -> Result<()> { Ok(()) }
    }

    #[test] fn buffered_device_writes_full_frame_initially() {
        let mut out = BufferedOutput::new("fake", Fake::default(), UpdateStrategy::Changed, None, false);
        out.render(&Frame::from_lines(Dimensions::new(2, 2).unwrap(), ["ab", "cd"]), Instant::now()).unwrap();
        assert_eq!(out.device.writes.len(), 2);
    }
}
