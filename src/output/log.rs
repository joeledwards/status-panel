use super::Output;
use crate::Frame;
use anyhow::Result;
use std::time::Instant;

pub struct LogOutput { only_when_changed: bool, previous: Option<Frame> }
impl LogOutput { pub fn new(only_when_changed: bool) -> Self { Self { only_when_changed, previous: None } } }

impl Output for LogOutput {
    fn name(&self) -> &str { "log" }
    fn render(&mut self, frame: &Frame, _now: Instant) -> Result<()> {
        if !self.only_when_changed || self.previous.as_ref() != Some(frame) {
            eprintln!("display:");
            for row in frame.lines() { eprintln!("|{}|", String::from_utf8_lossy(row)); }
            self.previous = Some(frame.clone());
        }
        Ok(())
    }
}
