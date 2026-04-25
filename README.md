# Anvil

View hardware specs and stress test your system.

![Anvil](packaging/linux/io.github.sugarycandybar.Anvil.svg)

## Features

- **Specs** — See your CPU, GPU, memory, and OS details at a glance
- **Copy Specs** — Copy a formatted Markdown spec sheet to share
- **Stress Testing** — Run CPU stress tests with configurable workers and duration
- **Live Monitoring** — Watch temperature and frequency in real time
- **Safety Cutoff** — Automatically stops tests if temperatures get too high

## Requirements

- Python 3.10+
- GTK 4
- libadwaita 1.x
- psutil
- stress-ng (for stress testing)

## Running from Source

```bash
# Install dependencies (Fedora)
sudo dnf install gtk4-devel libadwaita-devel python3-psutil stress-ng

# Install dependencies (Ubuntu/Debian)
sudo apt install libgtk-4-dev libadwaita-1-dev python3-psutil stress-ng

# Run
python3 anvil.py
```

## Building the Flatpak

```bash
flatpak-builder --user --install --force-clean build-dir packaging/flatpak/io.github.sugarycandybar.Anvil.yml
flatpak run io.github.sugarycandybar.Anvil
```

## License

GPL-3.0-or-later
