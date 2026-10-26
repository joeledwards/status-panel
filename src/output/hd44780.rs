use super::CharacterDevice;
use crate::{Dimensions, refresh::WriteRun};
use anyhow::{Result, bail};
use rppal::i2c::I2c;
use std::{thread, time::Duration};

const CLEAR: u8 = 0x01;
const RETURN_HOME: u8 = 0x02;
const ENTRY_MODE: u8 = 0x04;
const DISPLAY_CONTROL: u8 = 0x08;
const FUNCTION_SET: u8 = 0x20;
const SET_DDRAM: u8 = 0x80;
const ENTRY_LEFT: u8 = 0x02;
const DISPLAY_ON: u8 = 0x04;
const TWO_LINE: u8 = 0x08;
const BACKLIGHT: u8 = 0x08;
const ENABLE: u8 = 0x04;
const REGISTER_SELECT: u8 = 0x01;

pub struct Hd44780 {
    i2c: I2c,
    dimensions: Dimensions,
    backlight: u8,
}

impl Hd44780 {
    pub fn open(bus: u8, address: u16, dimensions: Dimensions, backlight: bool) -> Result<Self> {
        if dimensions.rows > 4 || dimensions.columns > 40 {
            bail!("HD44780 geometry cannot exceed 40 columns by four rows");
        }
        let mut i2c = I2c::with_bus(bus)?;
        i2c.set_slave_address(address)?;
        Ok(Self { i2c, dimensions, backlight: if backlight { BACKLIGHT } else { 0 } })
    }

    fn write(&mut self, value: u8) -> Result<()> { self.i2c.write(&[value | self.backlight])?; Ok(()) }
    fn pulse(&mut self, value: u8) -> Result<()> {
        self.write(value | ENABLE)?;
        thread::sleep(Duration::from_micros(1));
        self.write(value & !ENABLE)?;
        thread::sleep(Duration::from_micros(50));
        Ok(())
    }
    fn nibble(&mut self, value: u8) -> Result<()> { self.write(value)?; self.pulse(value) }
    fn send(&mut self, value: u8, mode: u8) -> Result<()> {
        self.nibble((value & 0xf0) | mode)?;
        self.nibble(((value << 4) & 0xf0) | mode)
    }
    fn command(&mut self, value: u8) -> Result<()> { self.send(value, 0) }
    fn data(&mut self, value: u8) -> Result<()> { self.send(value, REGISTER_SELECT) }
    fn set_cursor(&mut self, row: usize, column: usize) -> Result<()> {
        const OFFSETS: [u8; 4] = [0x00, 0x40, 0x14, 0x54];
        self.command(SET_DDRAM | (OFFSETS[row] + column as u8))
    }
}

impl CharacterDevice for Hd44780 {
    fn initialize(&mut self) -> Result<()> {
        thread::sleep(Duration::from_millis(50));
        self.write(self.backlight)?;
        thread::sleep(Duration::from_millis(1));
        self.nibble(0x30)?; thread::sleep(Duration::from_micros(4_200));
        self.nibble(0x30)?; thread::sleep(Duration::from_micros(4_200));
        self.nibble(0x30)?; thread::sleep(Duration::from_micros(160));
        self.nibble(0x20)?;
        self.command(FUNCTION_SET | TWO_LINE)?;
        self.command(DISPLAY_CONTROL | DISPLAY_ON)?;
        self.command(CLEAR)?; thread::sleep(Duration::from_millis(2));
        self.command(ENTRY_MODE | ENTRY_LEFT)?;
        self.command(RETURN_HOME)?; thread::sleep(Duration::from_millis(2));
        Ok(())
    }

    fn write_run(&mut self, run: &WriteRun) -> Result<()> {
        if run.row >= self.dimensions.rows || run.column + run.cells.len() > self.dimensions.columns {
            bail!("write run is outside display bounds");
        }
        self.set_cursor(run.row, run.column)?;
        for &cell in &run.cells { self.data(cell)?; }
        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> {
        self.command(CLEAR)?; thread::sleep(Duration::from_millis(2));
        self.backlight = 0;
        self.command(DISPLAY_CONTROL)?;
        self.write(0)
    }
}
