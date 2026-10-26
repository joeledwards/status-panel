use crate::{Dimensions, refresh::UpdateStrategy};
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::{fs, path::{Path, PathBuf}, time::Duration};

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub display: DisplayConfig,
    #[serde(default)]
    pub content: ContentConfig,
    #[serde(default)]
    pub output: OutputConfig,
    #[serde(default)]
    pub button: ButtonConfig,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DisplayConfig {
    pub columns: usize,
    pub rows: usize,
    #[serde(default)]
    pub update: UpdateStrategy,
    #[serde(default, deserialize_with = "optional_duration")]
    pub forced_refresh: Option<Duration>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ContentConfig {
    pub modes: Vec<ContentMode>,
    pub file: Option<PathBuf>,
    #[serde(deserialize_with = "duration")]
    pub poll_interval: Duration,
}

impl Default for ContentConfig {
    fn default() -> Self {
        Self {
            modes: vec![ContentMode::Clock, ContentMode::System],
            file: None,
            poll_interval: Duration::from_millis(500),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContentMode { Dashboard, Clock, System }

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct OutputConfig {
    pub terminal: TerminalConfig,
    pub log: LogConfig,
    pub lcd: LcdConfig,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TerminalConfig { pub enabled: bool }
impl Default for TerminalConfig { fn default() -> Self { Self { enabled: true } } }

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LogConfig { pub enabled: bool, pub only_when_changed: bool }

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LcdConfig {
    pub enabled: bool,
    pub bus: u8,
    pub address: u16,
    pub backlight: bool,
    pub reinitialize_on_forced_refresh: bool,
}
impl Default for LcdConfig {
    fn default() -> Self {
        Self { enabled: false, bus: 1, address: 0x27, backlight: true, reinitialize_on_forced_refresh: false }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ButtonConfig {
    pub enabled: bool,
    pub bcm_pin: Option<u8>,
    pub normally_closed: bool,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        let text = std::str::from_utf8(&bytes).context("configuration is not UTF-8")?;
        let mut config: Self = toml::from_str(text).context("parsing configuration")?;
        config.resolve_paths(path);
        config.validate()?;
        Ok(config)
    }

    pub fn dimensions(&self) -> Dimensions {
        Dimensions::new(self.display.columns, self.display.rows).expect("validated dimensions")
    }

    fn resolve_paths(&mut self, config_path: &Path) {
        let Some(parent) = config_path.parent() else { return; };
        if let Some(path) = &self.content.file {
            if path.is_relative() { self.content.file = Some(parent.join(path)); }
        }
    }

    fn validate(&self) -> Result<()> {
        Dimensions::new(self.display.columns, self.display.rows)?;
        if self.content.modes.is_empty() { bail!("content.modes must contain at least one mode"); }
        if self.content.poll_interval.is_zero() { bail!("content.poll_interval must be greater than zero"); }
        if self.content.modes.contains(&ContentMode::Dashboard)
            && (self.display.columns < 20 || self.display.rows < 4)
        {
            bail!("the dashboard content mode requires geometry of at least 20 columns by 4 rows");
        }
        if self.output.lcd.enabled && self.output.lcd.address > 0x7f {
            bail!("output.lcd.address must be a 7-bit I2C address");
        }
        if self.button.enabled && self.button.bcm_pin.is_none() {
            bail!("button.bcm_pin is required when the button is enabled");
        }
        Ok(())
    }
}

fn duration<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where D: serde::Deserializer<'de> {
    let value = String::deserialize(deserializer)?;
    parse_duration(&value).map_err(serde::de::Error::custom)
}

fn optional_duration<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
where D: serde::Deserializer<'de> {
    Option::<String>::deserialize(deserializer)?.map(|v| parse_duration(&v)).transpose()
        .map_err(serde::de::Error::custom)
}

pub fn parse_duration(value: &str) -> Result<Duration> {
    let value = value.trim();
    let split = value.find(|c: char| !c.is_ascii_digit() && c != '.')
        .ok_or_else(|| anyhow::anyhow!("duration requires a unit: {value}"))?;
    let number: f64 = value[..split].parse().with_context(|| format!("invalid duration: {value}"))?;
    if !number.is_finite() || number < 0.0 { bail!("invalid duration: {value}"); }
    let seconds = match &value[split..] {
        "ms" => number / 1_000.0,
        "s" => number,
        "m" => number * 60.0,
        "h" => number * 3_600.0,
        unit => bail!("unsupported duration unit {unit:?}"),
    };
    Ok(Duration::from_secs_f64(seconds))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn parses_durations() {
        assert_eq!(parse_duration("500ms").unwrap(), Duration::from_millis(500));
        assert_eq!(parse_duration("1.5m").unwrap(), Duration::from_secs(90));
        assert!(parse_duration("1").is_err());
    }
}
