# Configuration reference

`status-panel` reads a strict TOML configuration file. Unknown sections and
unknown fields are errors rather than being silently ignored.

The smallest useful configuration is:

```toml
[display]
columns = 20
rows = 4
```

This uses the default clock and system modes, enables the terminal output, and
does not access LCD or GPIO hardware.

## Selecting the configuration file

The configuration path is selected in descending precedence:

1. `--config PATH`
2. `STATUS_PANEL_CONFIG`
3. `./status-panel.toml`

For example:

```sh
status-panel --config /etc/status-panel/config.toml
STATUS_PANEL_CONFIG=dev.toml status-panel
```

Relative paths inside the configuration file, such as `content.file`, are
resolved relative to the directory containing the configuration file. They are
not resolved relative to the process's working directory.

## Complete example

```toml
[display]
columns = 20
rows = 4
update = "full"
forced_refresh = "60s"

[content]
modes = ["dashboard", "clock", "system"]
file = "display.txt"
poll_interval = "500ms"

[output.terminal]
enabled = true

[output.log]
enabled = false
only_when_changed = true

[output.lcd]
enabled = false
bus = 1
address = 0x27
backlight = true
reinitialize_on_forced_refresh = false

[button]
enabled = false
bcm_pin = 17
normally_closed = false
```

## `[display]`

The `display` section is required. It defines the logical frame shared by all
enabled outputs.

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `columns` | positive integer | required | Number of character cells in each row. |
| `rows` | positive integer | required | Number of character rows. |
| `update` | `"full"` or `"changed"` | `"full"` | LCD update strategy. |
| `forced_refresh` | duration string | none | Maximum interval between complete LCD overwrites. |

Both dimensions must be greater than zero. The HD44780 backend additionally
supports at most 40 columns and four rows. Terminal-only configurations are not
subject to that hardware limit.

### Update strategies

`full` writes every row whenever the desired frame changes. An unchanged frame
is not repeatedly written. This is the default because it regularly corrects
partial or missed display writes.

`changed` compares the desired frame with that output's shadow frame and writes
only contiguous runs of changed cells. This reduces I2C traffic.

When `forced_refresh` is set, either strategy completely overwrites the LCD at
that cadence even if the frame has not changed. A failed write invalidates the
shadow, so the next attempt is also a complete overwrite.

## `[content]`

The entire section is optional.

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `modes` | array of mode names | `["clock", "system"]` | Generated views, in button-cycle order. |
| `file` | path string | none | Optional file whose existence overrides generated content. |
| `poll_interval` | duration string | `"500ms"` | Interval between content-file checks. |

`modes` must contain at least one generated fallback. The first entry is shown
at startup. When the GPIO button is enabled, each press advances to the next
entry and wraps at the end.

### `dashboard`

The dashboard reproduces the legacy 20x4 clock layout and requires at least 20
columns and four rows:

```text
2026-10-25 22:02 < L
2026-10-26 04:02 < Z
groot
192.168.68.55   | 34
```

The first row is local time and the second is UTC. The fourth row contains the
first detected non-loopback IPv4 address. Its final two digits are UTC seconds,
which act as a heartbeat. The `|` is currently a neutral separator; network
health monitoring is not part of this version.

### `clock`

The clock shows local time followed by UTC time. Displays at least 20 columns
wide include seconds and `L`/`Z` indicators. Narrower displays use a compact
minute-resolution representation. Unused rows are blank.

### `system`

The system view shows the hostname followed by the first detected non-loopback
IPv4 address. System information is refreshed every 60 seconds. Unused rows are
blank.

### File override

When `content.file` exists, its contents replace every generated mode. Removing
the file returns to the currently selected generated mode. Creating, replacing,
editing, and deleting the file are detected without restarting the process.

The file must contain valid UTF-8. Each text line becomes one display row:

- Missing rows are filled with spaces.
- Short lines are padded with spaces.
- Extra rows and columns are truncated.
- An empty file produces a blank frame.
- Display cells currently support ASCII; unsupported bytes are rendered as
  `?`.

If the file exists but cannot be read or decoded, the display shows a compact
`CONTENT ERROR` frame and stderr receives the complete error.

## `[output.terminal]`

This section is optional and defaults to enabled.

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `enabled` | boolean | `true` | Render a bordered, in-place terminal preview. |

The terminal backend uses ANSI control sequences and has no native curses
dependency. On an interactive terminal it clears the screen, hides the cursor,
and redraws from the upper-left corner. Normal shutdown restores the cursor.

The `--terminal` option enables this output regardless of the configured value.
It does not disable other outputs.

## `[output.log]`

This section is optional and defaults to disabled.

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `enabled` | boolean | `false` | Write complete frames to stderr. |
| `only_when_changed` | boolean | `false` | Suppress frames identical to the previous logged frame. |

The `--log-display` option enables this output regardless of the configured
value. It does not disable other outputs.

## `[output.lcd]`

This section is optional and defaults to disabled.

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `enabled` | boolean | `false` | Open and drive the physical LCD. |
| `bus` | integer | `1` | Linux I2C bus number, as in `/dev/i2c-1`. |
| `address` | integer | `0x27` | Seven-bit I2C backpack address; decimal and TOML hexadecimal forms are accepted. |
| `backlight` | boolean | `true` | Keep the backpack backlight output active. |
| `reinitialize_on_forced_refresh` | boolean | `false` | Reinitialize the controller before cadence-driven full refreshes. |

The address must be between `0x00` and `0x7f`. The initial driver assumes the
common PCF8574-to-HD44780 backpack mapping:

```text
P0=RS P1=RW P2=Enable P3=Backlight P4..P7=Data
```

LCD hardware support is compiled for ARM Linux targets. Enabling it on an
unsupported host is an error. The process must also have permission to access
the selected `/dev/i2c-*` device.

`reinitialize_on_forced_refresh` is more disruptive than a complete content
overwrite and can visibly blink the display. Enable it only if periodic full
overwrites do not recover controller corruption.

## `[button]`

This section is optional and defaults to disabled.

| Field | Type | Default | Description |
| --- | --- | --- | --- |
| `enabled` | boolean | `false` | Initialize GPIO and poll a mode-selection button. |
| `bcm_pin` | integer | none | Broadcom GPIO number; required when enabled. |
| `normally_closed` | boolean | `false` | Invert the expected switch state for a normally-closed button. |

The input uses an internal pull-up. A normally-open button is considered
pressed when the input goes low; a normally-closed button is considered pressed
when it goes high. Each press advances through `content.modes`.

GPIO hardware support is compiled for ARM Linux targets. LCD and GPIO are
independent: disabling `output.lcd.enabled` does not disable the button.

## Durations

Duration fields are strings containing a non-negative integer or decimal value
and one supported suffix:

| Suffix | Unit | Examples |
| --- | --- | --- |
| `ms` | milliseconds | `"100ms"`, `"500ms"` |
| `s` | seconds | `"1s"`, `"2.5s"` |
| `m` | minutes | `"1m"`, `"1.5m"` |
| `h` | hours | `"1h"`, `"24h"` |

A unit is required. Compound values such as `"1m 30s"` are not supported.
`content.poll_interval` must be greater than zero. A zero
`display.forced_refresh` is accepted and causes every refresh evaluation to be
treated as cadence-forced, so it is normally not useful.

## Command-line overrides

| Option | Effect |
| --- | --- |
| `--config PATH` | Select a configuration file. |
| `--no-hardware` | Disable both LCD/I2C and GPIO access regardless of configuration. |
| `--terminal` | Enable terminal output in addition to configured outputs. |
| `--log-display` | Enable stderr frame logging in addition to configured outputs. |
| `--help` | Show command-line help. |
| `--version` | Show the program version. |

At least one output must remain enabled after command-line overrides are
applied. For local development, the safest invocation is:

```sh
status-panel --no-hardware --terminal
```

## Live reload and errors

The selected configuration file is checked every 500 milliseconds. Changes are
parsed and validated as a complete candidate configuration before they are
applied.

- A valid candidate replaces the active configuration and rebuilds outputs.
- An invalid candidate leaves the last-known-good configuration active.
- Configuration failures display a `CONFIG ERROR` frame using working outputs.
- Correcting the file clears the error frame and applies the new configuration.
- Removing and recreating the file is detected.
- Full error details are always written to stderr.

An invalid initial configuration cannot use a last-known-good display, so the
program reports the error and exits. Hardware initialization failures during
startup also prevent startup. Runtime failure of one output is reported without
preventing the remaining outputs from rendering.

Changing configuration resets the selected mode to the first entry in
`content.modes`, refreshes system information, invalidates output shadow state,
and causes the newly constructed outputs to draw a complete frame.
