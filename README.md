<h1 align="center">Zons VPN</h1>

<h3 align="center">A native Windows client for mihomo</h3>

<p align="center">
  Manage proxy profiles, TUN routing and per-app bypass rules from one desktop app.<br>
  Built with Rust and Slint, with software rendering and a system tray.
</p>

<p align="center">
  <a href="https://github.com/Goldingeng/Zons-VPN/releases/latest"><b>Download for Windows</b></a>
  &nbsp;·&nbsp;
  <b>English</b> | <a href="README.ru.md">Русский</a>
</p>

<p align="center">
  <a href="https://github.com/Goldingeng/Zons-VPN/releases/latest"><img src="https://img.shields.io/github/v/release/Goldingeng/Zons-VPN?label=release" alt="Latest release"></a>
  <a href="https://github.com/Goldingeng/Zons-VPN/releases"><img src="https://img.shields.io/github/downloads/Goldingeng/Zons-VPN/total" alt="Downloads"></a>
  <img src="https://img.shields.io/badge/platform-Windows%20x64-0078D4" alt="Windows x64">
  <img src="https://img.shields.io/badge/made%20with-Rust-orange" alt="Rust">
  <a href="https://slint.dev"><img src="https://img.shields.io/badge/UI-Slint-2379F4" alt="Slint UI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/Goldingeng/Zons-VPN" alt="GPL-3.0 license"></a>
</p>

---

Zons VPN is a native VPN and proxy client for Windows, powered by [mihomo](https://github.com/MetaCubeX/mihomo). It is an alternative to Clash Party (Mihomo Party), with a Slint interface instead of Electron or a browser engine.

Zons VPN manages the client and its routing settings. You need your own compatible subscription or YAML configuration; the app does not provide VPN servers.

## Download and first launch

1. Download **`ZonsVPN.exe`** from the [latest release](https://github.com/Goldingeng/Zons-VPN/releases/latest).
2. Place it in a separate folder and run it. The app downloads the mihomo core from its [official releases](https://github.com/MetaCubeX/mihomo/releases) if it cannot find `mihomo.exe` next to the app.
3. Add a subscription URL under **Profiles**, or place your own YAML configuration in `core\profiles\`, then select the active profile.
4. Enable the system proxy or TUN mode as needed. For **TUN**, place `wintun.dll` from [Wintun](https://www.wintun.net/) next to the executable and run the app as administrator.

Closing the window minimizes it to the system tray. Starting the executable again opens the existing instance.

## Features

| Section | What it does |
|---|---|
| Overview | Core status, system proxy, TUN, rule/global routing mode, traffic speed and current node |
| Proxies | Selector, URLTest and Fallback groups, node selection and latency checks |
| Profiles | Add, update, delete and switch subscription profiles |
| Bypass VPN | Route selected applications directly; enter a process name, pick an `.exe`, or choose from applications currently online |
| Connections | View active connections and close one or all of them |
| Settings | Port, Windows startup, core restart and updates, logs and the app folder |

## Core updates

The app checks GitHub for a new mihomo release once a day. **Update core** in Settings runs the check immediately.

It downloads the `windows-amd64-compatible` build, checks the archive against the release's SHA-256 checksum, extracts it with Windows' built-in `tar.exe`, then checks the executable header and `mihomo -v` output.

The new core is staged as `core\next.exe` and installed the next time the core starts. The previous core is kept as `core\prev.exe` until the new one responds to API requests. If the new core fails to start, the app restores the previous version and does not automatically download the rejected version again.

## Ports and bypass rules

If the configured port is occupied, for example by Clash Party on port 7890, the app selects the next available port and saves it in `settings.ini`.

Before starting the core, the selected profile is copied to `core\run.yaml`. Application bypass rules are inserted at the beginning of the `rules:` section as `PROCESS-NAME,<app>.exe,DIRECT`. Rules are evaluated from top to bottom, so these exceptions take priority over subscription rules. The original profile is kept unchanged.

## Local files

| File | Purpose |
|---|---|
| `subs.txt` | Subscriptions: `name url [user-agent]` |
| `settings.ini` | Active profile, mode, port, proxy, TUN and bypass settings |
| `core\profiles\*.yaml` | Downloaded profiles and your own configurations |
| `core\run.yaml` | The configuration passed to the core |
| `core\mihomo.log` | Core log |
| `core\update.txt` | Last update check and rejected core version |

## Building

You need Rust and the Windows C++ build tools for the MSVC toolchain.

```powershell
cargo build --release --ignore-rust-version
```

The output is `target\release\clashtray.exe`; the release executable is distributed as `ZonsVPN.exe`.

Slint 1.17 declares a minimum Rust version of 1.92. The project's existing build instructions use `--ignore-rust-version` for Rust 1.91. With a toolchain that meets Slint's requirement, use `cargo build --release`.

### Software renderer patch

`vendor/i-slint-renderer-software` contains a modified copy of the Slint 1.17.1 software renderer. Its glyph rasterization uses TrueType hinting, quarter-pixel glyph positioning instead of discarding fractional positions, and gamma 1.4 coverage correction to improve text on dark backgrounds.

When updating Slint, port the changes in `fonts/vectorfont.rs` and `draw_glyph_run` in `lib.rs`, or remove the `[patch.crates-io]` section from `Cargo.toml`.

## License

Zons VPN is licensed under **GPL-3.0-only**; see [LICENSE](LICENSE).

- [mihomo](https://github.com/MetaCubeX/mihomo) is a separate GPL-3.0 program. It is not included in this repository; the app downloads it from the official releases.
- [Slint](https://slint.dev) is used under GPL-3.0. The modified renderer's license files are in [`vendor/i-slint-renderer-software/LICENSES`](vendor/i-slint-renderer-software/LICENSES).
