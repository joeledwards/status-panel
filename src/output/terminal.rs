use super::Output;
use crate::Frame;
use anyhow::Result;
use std::{io::{self, IsTerminal, Write}, time::Instant};

pub struct TerminalOutput { active: bool, first: bool }

impl TerminalOutput {
    pub fn new() -> Self { Self { active: io::stdout().is_terminal(), first: true } }
}

impl Output for TerminalOutput {
    fn name(&self) -> &str { "terminal" }
    fn render(&mut self, frame: &Frame, _now: Instant) -> Result<()> {
        let mut out = io::stdout().lock();
        if self.active {
            if self.first { write!(out, "\x1b[?25l\x1b[2J")?; }
            write!(out, "\x1b[H┌{}┐\n", "─".repeat(frame.dimensions().columns))?;
            for row in frame.lines() { writeln!(out, "│{}│", String::from_utf8_lossy(row))?; }
            write!(out, "└{}┘\x1b[J", "─".repeat(frame.dimensions().columns))?;
        } else {
            for row in frame.lines() { writeln!(out, "{}", String::from_utf8_lossy(row))?; }
        }
        out.flush()?;
        self.first = false;
        Ok(())
    }
    fn shutdown(&mut self) -> Result<()> {
        if self.active {
            print!("\x1b[?25h\x1b[0m\n");
            io::stdout().flush()?;
            self.active = false;
        }
        Ok(())
    }
}

impl Drop for TerminalOutput {
    fn drop(&mut self) { let _ = self.shutdown(); }
}
