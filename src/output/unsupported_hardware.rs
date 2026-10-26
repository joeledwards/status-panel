use super::CharacterDevice;
use crate::{Dimensions, refresh::WriteRun};
use anyhow::{Result, bail};

pub struct Hd44780;

impl Hd44780 {
    pub fn open(_bus: u8, _address: u16, _dimensions: Dimensions, _backlight: bool) -> Result<Self> {
        bail!("LCD hardware is supported only by ARM Linux builds; use --no-hardware on this host")
    }
}

impl CharacterDevice for Hd44780 {
    fn initialize(&mut self) -> Result<()> { bail!("LCD hardware is unavailable") }
    fn write_run(&mut self, _run: &WriteRun) -> Result<()> { bail!("LCD hardware is unavailable") }
    fn shutdown(&mut self) -> Result<()> { Ok(()) }
}
