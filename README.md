# status-panel

A configurable character-cell status display for clocks, system information,
and file-driven content. It can drive an HD44780 display through a common
PCF8574 I2C backpack, render an in-place terminal preview, log frames, or use
multiple outputs simultaneously.

The program is safe to run without Raspberry Pi hardware. Hardware is opened
only when its output or input is enabled, and `--no-hardware` overrides both
LCD and GPIO configuration.

## Current status

This is the initial Rust implementation. Its pure frame, refresh, content, and
configuration layers are isolated from direct hardware access. The Rust
toolchain policy is intentionally pinned only to the stable channel until the
project's supported Rust version is selected.

## Configuration

Copy `status-panel.example.toml` to `status-panel.toml` and edit it. Display
geometry is required; there is no implicit physical-display size.

The configuration path is selected in this order:

1. `--config PATH`
2. `STATUS_PANEL_CONFIG`
3. `./status-panel.toml`

Relative content-file paths are resolved relative to the configuration file.
When that file exists, its lines override generated content. Removing it
returns to the selected generated mode. Extra rows and columns are
truncated and short rows are padded.

Available generated modes are `dashboard`, `clock`, and `system`. The
`dashboard` mode reproduces the legacy 20x4 layout: local time, UTC time,
hostname, and IP address with seconds as a heartbeat. It requires geometry of
at least 20 columns by four rows. Enable it with:

```toml
[content]
modes = ["dashboard", "clock", "system"]
```

Configuration and content are polled and applied without restarting. Invalid
replacement configuration leaves the last-known-good configuration active and
places a compact error on every working output. Complete details are also
written to stderr.

## Running

All Rust commands must use the workspace wrapper from this repository:

```sh
../../scripts/with-rust cargo run -- --config status-panel.toml
../../scripts/with-rust cargo run -- --no-hardware --terminal
```

Useful development overrides are:

```text
--no-hardware   Do not access I2C or GPIO
--terminal      Enable the in-place terminal output
--log-display   Log rendered frames to stderr
```

## Refresh behavior

`full` is the default update strategy. It writes complete rows whenever the
desired frame changes. `changed` writes only contiguous changed runs. Both
strategies can perform a forced full overwrite on a fixed cadence. A failed
hardware write invalidates the shadow frame so the next attempt is complete.

Controller reinitialization is distinct from a full content overwrite because
it can visibly blink the display. Set `reinitialize_on_forced_refresh = true`
only when periodic content overwrites do not recover the physical display.

## Hardware assumptions

The initial driver uses the common PCF8574 backpack mapping also used by the
legacy Scala clock:

```text
P0=RS P1=RW P2=Enable P3=Backlight P4..P7=Data
```

GPIO pin numbers use Broadcom (BCM) numbering. Hardware access has not been
verified in this workspace and must be tested on the target Raspberry Pi.

## Verification

Once the workspace-local Rust toolchain and dependencies are available:

```sh
../../scripts/with-rust cargo fmt --check
../../scripts/with-rust cargo clippy --all-targets -- -D warnings
../../scripts/with-rust cargo test --lib
```

These are isolated build and unit-test commands; they do not access display or
GPIO hardware.
