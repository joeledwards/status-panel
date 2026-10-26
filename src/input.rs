use crate::config::ButtonConfig;
use anyhow::Result;
#[cfg(all(target_os = "linux", any(target_arch = "arm", target_arch = "aarch64")))]
use rppal::gpio::{Gpio, InputPin, Level};

#[cfg(all(target_os = "linux", any(target_arch = "arm", target_arch = "aarch64")))]
pub struct ModeButton {
    pin: InputPin,
    normally_closed: bool,
    was_pressed: bool,
}

#[cfg(all(target_os = "linux", any(target_arch = "arm", target_arch = "aarch64")))]
impl ModeButton {
    pub fn open(config: &ButtonConfig) -> Result<Option<Self>> {
        if !config.enabled { return Ok(None); }
        let pin = Gpio::new()?.get(config.bcm_pin.expect("validated pin"))?.into_input_pullup();
        let mut button = Self { pin, normally_closed: config.normally_closed, was_pressed: false };
        button.was_pressed = button.is_pressed();
        Ok(Some(button))
    }

    fn is_pressed(&self) -> bool {
        match (self.normally_closed, self.pin.read()) {
            (false, Level::Low) | (true, Level::High) => true,
            _ => false,
        }
    }

    pub fn pressed(&mut self) -> bool {
        let current = self.is_pressed();
        let edge = current && !self.was_pressed;
        self.was_pressed = current;
        edge
    }
}

#[cfg(not(all(target_os = "linux", any(target_arch = "arm", target_arch = "aarch64"))))]
pub struct ModeButton;

#[cfg(not(all(target_os = "linux", any(target_arch = "arm", target_arch = "aarch64"))))]
impl ModeButton {
    pub fn open(config: &ButtonConfig) -> Result<Option<Self>> {
        if config.enabled {
            anyhow::bail!("GPIO is supported only by ARM Linux builds; use --no-hardware on this host");
        }
        Ok(None)
    }
    pub fn pressed(&mut self) -> bool { false }
}
