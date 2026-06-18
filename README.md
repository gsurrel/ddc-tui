# ddc-tui

A fast, keyboard-driven Terminal User Interface (TUI) for controlling external monitors via **DDC/CI** (Display Data Channel Command Interface). 

Built with Rust and [Ratatui](https://ratatui.rs/), `ddc-tui` leverages the community-maintained [`ddccontrol-db`](https://github.com/ddccontrol/ddccontrol-db) to automatically map cryptic VCP (Virtual Control Panel) hex codes to human-readable names and discrete options.

> [!NOTE]
> **Transparency Note:** This project is **vibe-coded**. It should be architecturally sound, relying on proper crates for a more robust experience. You may encounter some unconventional Rust patterns or idiosyncratic UI choices. As for many projects, PRs, refactors, and bug reports are highly encouraged!

## Features

- **Zero-Cost Database Integration**: Parses the `ddccontrol-db` XML files at compile-time via `build.rs`, generating static lookup tables. No runtime XML parsing, no C-dependencies.
- **Smart Hardware Matching**: Automatically reads your monitor's EDID (Manufacturer and Model IDs) and traces the profile inheritance chain (e.g., `DELD06F` -> `DELLCD` -> `VESA`).
- **Fuzzy Profile Override**: Monitor EDIDs can be weird or generic. Press `p` to open a fuzzy-search modal (powered by `strsim`) to manually force a specific monitor profile. Useful if your monitor is not known.
- **Optimized I2C Polling**: Caches display handles to avoid re-scanning the I2C/DP-Aux bus to speed-up the responsiveness.
- **Native TUI Widgets**: 
  - Continuous values (Brightness, Contrast) use native Ratatui `Gauge` widgets.
  - Discrete values (Input Source, Color Presets) use natively wrapping "Pills".
  - Write-only commands (Factory Reset, Degauss) are grouped at the bottom as executable Actions.
- **Power User Shortcuts**: Hold `Shift` while adjusting a continuous gauge to jump by 10% of the total range.

## Installation & Building

This project relies on `ddccontrol-db` as a git submodule to ensure the database is compiled directly into the binary.

```bash
# Clone the repository with submodules
git clone --recursive https://github.com/gsurrel/ddc-tui.git
cd ddc-tui

# If you already cloned it, initialize the submodule:
# git submodule update --init

# Build and run
cargo run
```

*Note: On Linux, you may need to run the application with elevated privileges (e.g., `sudo`) or ensure your user is in the `i2c` group to access the hardware bus. Please do not use `sudo` for such a thing, though.*

## Usage

The interface is split into three main areas: the Monitor List (left), the VCP Controls (right), and the Status Bar (bottom).

### Keybindings

| Key | Action |
| :--- | :--- |
| `Tab` | Switch focus between the Monitor List and the Controls panel. |
| `↑` / `↓` | Navigate through monitors or VCP features. |
| `←` / `→` | Adjust the selected continuous gauge or cycle through discrete pills. |
| `Shift` + `←` / `→` | **Fast Adjust**: Increment/decrement continuous gauges by 10% of their max range. |
| `Enter` | Execute an **Action** (e.g., Restore Factory Defaults). |
| `p` | Open the **Profile Override** fuzzy search modal. |
| `r` | Force a refresh/re-probe of the currently selected monitor. |
| `q` | Quit the application. |

### Monitor Profile Override

Sometimes a monitor reports a generic or incorrect Plug and Play (PNP) ID via EDID, causing the app to fall back to the generic `VESA` profile and miss specific features (like exact HDMI input mappings). 

1. Press `p` to open the Profile Search modal.
2. Type the brand or model name of your monitor (e.g., "LG 27UK850"). The list will filter and rank results (using Jaro-Winkler string similarity).
3. Use `↑` / `↓` to select the correct profile and press `Enter`.
4. The app will immediately re-probe the monitor using the overridden profile chain (displayed in the title bar).

## Adding a Monitor

**`ddc-tui` does not maintain its own hardware database.** It relies entirely on the upstream [`ddccontrol-db`](https://github.com/ddccontrol/ddccontrol-db) project. 

If your monitor is missing specific controls, or if discrete values (like Input Sources) are mapped incorrectly, you should contribute directly to the upstream database. Once your PR is merged upstream, simply update the git submodule in this project and recompile to inherit the new mappings.

Please refer to the official upstream guide for instructions on how to sniff I2C capabilities and add a monitor:
**[How to add a monitor to ddccontrol-db](https://github.com/ddccontrol/ddccontrol-db/blob/master/doc/how-to-add-a-monitor.md)**

## Technical Architecture

For those interested in the rust-jargon and architecture:

- **`build.rs` & `roxmltree`**: At compile time, the build script reads `options.xml.in` and all `monitor/*.xml` files. It resolves `<include>` tags to trace inheritance and generates a `db_generated.rs` file containing static arrays of `BaseControl` and `MonitorProfile` structs.
- **`ddc-hi`**: Used for cross-platform DDC/CI communication. It abstracts away the differences between Linux `i2c-dev` and Windows/DP-Aux APIs.
- **Strictly Typed Features**: The `FeatureType` enum strictly types VCP codes into `Continuous`, `Discrete`, and `Action` variants at probe-time. This allows the UI layer (`ui.rs`) to remain completely "dumb" and data-driven, simply rendering whatever the App layer provides.
- **Smart Scrolling**: The TUI implements a bounded, centered scrolling algorithm for the control list to ensure the selected item remains in the middle of the viewport without overscrolling.

## License

This project is licensed under the GPL-3.0 License. 

The `ddccontrol-db` submodule is licensed under the GPL-2.0 License.