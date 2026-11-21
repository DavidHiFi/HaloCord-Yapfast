<div align="center">

<img src="assets/halocord-page.png" alt="Yapfast settings with the HaloCord page open, showing the Stereo loader switch and the microphone bit rate picker" width="100%">

# HaloCord for Yapfast

**HaloCord's voice plugins, built into Yapfast, the light Discord client written in Rust.**

The next HaloCord: stereo microphone and 510 kbit/s calls, with its own settings page.

<a href="#installation">
<img src="https://img.shields.io/badge/%F0%9F%93%96%20INSTALL%20GUIDE-2EA043?style=for-the-badge&labelColor=161B22" alt="Read the install guide" height="42">
</a>

<br><br>

<img src="https://img.shields.io/github/license/DavidHiFi/HaloCord-Yapfast?style=flat-square&label=license&color=8957E5&labelColor=161B22" alt="License">
<img src="https://img.shields.io/badge/platform-Windows-DB6D28?style=flat-square&labelColor=161B22" alt="Platform: Windows">

</div>

---

## Installation

There is no download yet. For now you build it yourself.

1. Install Yapfast from its [releases page](https://github.com/yapfast/yapfast/releases) and link your account once.
2. Install [Rust](https://rustup.rs) and the Visual Studio C++ build tools.
3. Clone this repo. The `halocord` branch is the default.
4. Build it from the project folder (the exact command is under "Building from source" below).
5. Close Yapfast, then copy `target\x86_64-pc-windows-msvc\release\yapfast.exe` over the one in `%LOCALAPPDATA%\Programs\Yapfast`.
6. Open Yapfast and find **HaloCord** at the bottom of Settings.

## Features

### Voice

- **Stereo loader:** sends your microphone in stereo, just as it comes in. Echo cancelling, noise suppression, gain control and leveling are skipped, so only your mic volume and the voice gate or push to talk apply.
- **Bit rate:** pick 64, 128, 256, 384 or 510 kbit/s. It changes live, mid-call.
- **Off switch:** turn the plugin off and calls go back to Yapfast's own sound cleanup at 128 kbit/s.

### Plugins

- **HaloCord page:** its own section in Settings, one switch per plugin, with each plugin's settings under it.
- **Survives updates:** HaloCord's code lives in its own files, with small hooks into Yapfast, so it moves onto each new Yapfast release with little fuss.

## Notes

> [!IMPORTANT]
> **Early days.** This is the first plugin. More of HaloCord's plugins come over one by one.

> [!NOTE]
> **Replacing the old HaloCord.** The Electron-based [HaloCord](https://github.com/DavidHiFi/HaloCord) is retired. This repo is where HaloCord carries on.

> [!WARNING]
> **Yapfast's own updater** puts the stock app back when it updates. Rebuild and copy the file again after each Yapfast update.

<details>
<summary><b>Building from source</b></summary>

<br>

On Windows, from the project folder:

```powershell
$env:YAPFAST_RELEASE = "1"
cargo build --locked --release --target x86_64-pc-windows-msvc --no-default-features --features desktop,discord
```

Rust 1.97.1 installs itself from `rust-toolchain.toml`. The first build is big and takes a while.

Yapfast's own guide covers the Nix route and Linux: see the [main README](../README.md#build-it-yourself). Every hook HaloCord adds to Yapfast's files is listed in [docs/halocord.md](../docs/halocord.md).

</details>

## Contributing

Bug reports with steps to repeat them help most, and so do plugin ideas.

<a href="CONTRIBUTING.md">
<img src="https://img.shields.io/badge/%F0%9F%A4%9D%20HOW%20TO%20CONTRIBUTE-8957E5?style=for-the-badge&labelColor=161B22" alt="How to contribute" height="36">
</a>

## Star history

Thanks to everyone who followed HaloCord over here.

<a href="https://www.star-history.com/?repos=DavidHiFi%2FHaloCord-Yapfast&amp;type=date">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/chart?repos=DavidHiFi/HaloCord-Yapfast&amp;type=date&amp;theme=dark&amp;legend=top-left" />
    <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/chart?repos=DavidHiFi/HaloCord-Yapfast&amp;type=date" />
    <img alt="DavidHiFi/HaloCord-Yapfast GitHub star history" src="https://api.star-history.com/chart?repos=DavidHiFi/HaloCord-Yapfast&amp;type=date" />
  </picture>
</a>

## Credits

- **Author:** Myself, [DavidHiFi](https://github.com/DavidHiFi)

| Who | What |
|---|---|
| **Yapfast** ([yapfast/yapfast](https://github.com/yapfast/yapfast)) | The whole client. HaloCord adds its plugins on top. |
| **TestCord** ([Codeberg](https://codeberg.org/Testcord/testcord)) | The stereo loader idea, from TestCord's plugin of the same name. |
| **Discord Userdoccers** ([docs.discord.food](https://docs.discord.food)) | The Discord documentation Yapfast is built from. |

Built on [Rust](https://www.rust-lang.org), with [iced](https://iced.rs) and [Opus](https://opus-codec.org).

> Missing or wrong credit? [Open an issue](https://github.com/DavidHiFi/HaloCord-Yapfast/issues/new) and I'll fix it.

## Disclaimers

Most of HaloCord's code in this repo was developed with AI coding agents, mainly Claude Code. No single AI agent made the project.

Yapfast is free software under the GNU General Public License, version 3 or later, and so is this copy. This repo is not the official Yapfast. Report HaloCord problems here, not to Yapfast.

<div align="center">
<br>
<sub>Not affiliated with Discord or the Yapfast project. Requires a Discord account.</sub>
</div>
