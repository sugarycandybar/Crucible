<p align="center">
  <img src="data/io.github.sugarycandybar.Crucible.svg" alt="Crucible icon" width="128" />
</p>

<h1 align="center">Crucible</h1>

<p align="center">
  View specs and stress test hardware
  <br><br>
  <a href="https://flathub.org/en/apps/io.github.sugarycandybar.Crucible"><img src="https://img.shields.io/flathub/downloads/io.github.sugarycandybar.Crucible?label=Flathub%20Downloads&color=brightgreen"></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/sugarycandybar/Crucible?label=License&color=blue"></a>
</p>

## Features

- Easy to use: view your hardware specs and run stress tests without complex tools or configuration.
- Real-time monitoring: watch CPU temperature and usage live during stress tests.
- Safety built-in: automatic temperature cutoff prevents hardware damage during intensive testing.
- Share specs easily: copy a hardware report to share with others.
- All in the app: specs, stress testing, and live monitoring in one place.

## Run Crucible

[![Download on Flathub](https://flathub.org/assets/badges/flathub-badge-en.png)](https://flathub.org/en/apps/io.github.sugarycandybar.Crucible)

<details>
<summary>Build from source (Rust)</summary>

### Prerequisites

Install Rust via [rustup](https://rustup.rs/) or your package manager, and install the build dependencies:

```bash
# Fedora
sudo dnf install gtk4-devel libadwaita-devel stress-ng

# Ubuntu/Debian
sudo apt install libgtk-4-dev libadwaita-1-dev stress-ng
```

### Build & Run

```bash
cargo build --release
./target/release/crucible
```

### Building the Flatpak

```bash
flatpak-builder --user --install --force-clean build-dir packaging/flatpak/io.github.sugarycandybar.Crucible.yml
flatpak run io.github.sugarycandybar.Crucible
```

</details>

## Screenshots

<p align="center">
	<img src="data/screenshots/stress_test.png" alt="Stress testing and monitoring view" width="450" />
</p>

- **Stress Test**: run stress tests and monitor temperature and usage in real time.

<p align="center">
	<img src="data/screenshots/whole_specs.png" alt="Full specs view" width="450" />
</p>

- **Specs**: complete hardware overview with CPU, GPU, memory, and OS details.

## License

GPL-3.0-or-later
