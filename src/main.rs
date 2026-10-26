use anyhow::{Context, Result};
use clap::Parser;
use status_panel::{
    Frame,
    config::{Config, ContentMode},
    content,
    input::ModeButton,
    output::{BufferedOutput, Hd44780, LogOutput, Output, TerminalOutput},
};
use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::{Arc, atomic::{AtomicBool, Ordering}},
    thread,
    time::{Duration, Instant},
};

#[derive(Debug, Parser)]
#[command(version, about)]
struct Args {
    #[arg(long, env = "STATUS_PANEL_CONFIG", default_value = "status-panel.toml")]
    config: PathBuf,
    #[arg(long, help = "Disable LCD and GPIO access")]
    no_hardware: bool,
    #[arg(long, help = "Enable the in-place terminal display")]
    terminal: bool,
    #[arg(long, help = "Log display frames to stderr")]
    log_display: bool,
}

struct Runtime {
    config: Config,
    outputs: Vec<Box<dyn Output>>,
    button: Option<ModeButton>,
    mode_index: usize,
    file_cache: Option<Frame>,
    next_file_poll: Instant,
    system_info: content::SystemInfo,
    next_system_poll: Instant,
}

impl Runtime {
    fn build(config: Config, args: &Args) -> Result<Self> {
        let mut outputs: Vec<Box<dyn Output>> = Vec::new();
        if config.output.terminal.enabled || args.terminal {
            outputs.push(Box::new(TerminalOutput::new()));
        }
        if config.output.log.enabled || args.log_display {
            outputs.push(Box::new(LogOutput::new(config.output.log.only_when_changed)));
        }
        if config.output.lcd.enabled && !args.no_hardware {
            let lcd = Hd44780::open(config.output.lcd.bus, config.output.lcd.address,
                config.dimensions(), config.output.lcd.backlight).context("opening LCD")?;
            outputs.push(Box::new(BufferedOutput::new(
                "lcd", lcd, config.display.update, config.display.forced_refresh,
                config.output.lcd.reinitialize_on_forced_refresh,
            )));
        }
        if outputs.is_empty() { anyhow::bail!("at least one output must be enabled"); }
        let button = if args.no_hardware { None } else { ModeButton::open(&config.button)? };
        Ok(Self {
            config, outputs, button, mode_index: 0, file_cache: None,
            next_file_poll: Instant::now(),
            system_info: content::SystemInfo::discover(),
            next_system_poll: Instant::now() + Duration::from_secs(60),
        })
    }

    fn frame(&mut self, now: Instant) -> Result<Frame> {
        if now >= self.next_system_poll {
            self.system_info = content::SystemInfo::discover();
            self.next_system_poll = now + Duration::from_secs(60);
        }
        if now >= self.next_file_poll {
            self.next_file_poll = now + self.config.content.poll_interval;
            self.file_cache = match self.config.content.file.as_deref() {
                Some(path) if path.exists() => Some(content::file_content(self.config.dimensions(), path)
                    .with_context(|| format!("reading content file {}", path.display()))?),
                _ => None,
            };
        }
        if let Some(frame) = &self.file_cache { return Ok(frame.clone()); }
        let modes: Vec<_> = self.config.content.modes.iter().copied()
            .filter(|mode| *mode != ContentMode::File).collect();
        let mode = modes[self.mode_index % modes.len()];
        content::render(
            mode,
            self.config.dimensions(),
            self.config.content.file.as_deref(),
            &self.system_info,
        )
    }

    fn poll_button(&mut self) {
        if self.button.as_mut().is_some_and(ModeButton::pressed) {
            self.mode_index = self.mode_index.wrapping_add(1);
            self.file_cache = None;
        }
    }

    fn render(&mut self, frame: &Frame, now: Instant) {
        for output in &mut self.outputs {
            if let Err(error) = output.render(frame, now) {
                eprintln!("{} output failed: {error:#}", output.name());
            }
        }
    }

    fn shutdown(&mut self) {
        for output in &mut self.outputs {
            if let Err(error) = output.shutdown() { eprintln!("{} shutdown failed: {error:#}", output.name()); }
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    let config_path = absolute(&args.config)?;
    let initial = Config::load(&config_path)?;
    let mut observed_config = Some(fs::read(&config_path)?);
    let mut runtime = Runtime::build(initial, &args)?;
    let running = Arc::new(AtomicBool::new(true));
    let signal = Arc::clone(&running);
    ctrlc::set_handler(move || signal.store(false, Ordering::SeqCst))?;
    let mut next_config_poll = Instant::now();
    let mut overlay: Option<Frame> = None;

    while running.load(Ordering::SeqCst) {
        let now = Instant::now();
        if now >= next_config_poll {
            next_config_poll = now + Duration::from_millis(500);
            match fs::read(&config_path) {
                Ok(bytes) if observed_config.as_ref() != Some(&bytes) => {
                    observed_config = Some(bytes);
                    match Config::load(&config_path).and_then(|cfg| Runtime::build(cfg, &args)) {
                        Ok(replacement) => {
                            runtime.shutdown();
                            runtime = replacement;
                            overlay = None;
                            eprintln!("configuration reloaded from {}", config_path.display());
                        }
                        Err(error) => {
                            eprintln!("configuration reload failed: {error:#}");
                            overlay = Some(content::error_frame(runtime.config.dimensions(), "CONFIG ERROR", &error));
                        }
                    }
                }
                Ok(_) => {}
                Err(error) => {
                    if observed_config.take().is_some() {
                        let error = anyhow::anyhow!("reading {}: {error}", config_path.display());
                        overlay = Some(content::error_frame(runtime.config.dimensions(), "CONFIG ERROR", &error));
                    }
                }
            }
        }
        runtime.poll_button();
        let frame = if let Some(frame) = &overlay { frame.clone() } else {
            match runtime.frame(now) {
                Ok(frame) => frame,
                Err(error) => {
                    eprintln!("content error: {error:#}");
                    content::error_frame(runtime.config.dimensions(), "CONTENT ERROR", &error)
                }
            }
        };
        runtime.render(&frame, now);
        thread::sleep(Duration::from_millis(100));
    }
    runtime.shutdown();
    Ok(())
}

fn absolute(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() { return Ok(path.to_path_buf()); }
    Ok(env::current_dir()?.join(path))
}
