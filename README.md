# Crucible

Crucible is a desktop app for viewing system hardware specs and running stress tests with a clean, native-style UI.

It keeps hardware monitoring and testing simple: view detailed system information, benchmark your hardware, and monitor temperatures and performance in real time, all in one place.

## Why Crucible?

- Easy to use: view your hardware specs and run stress tests without complex tools or configuration.
- Real-time monitoring: watch CPU temperature and usage live during stress tests.
- Safety built-in: automatic temperature cutoff prevents hardware damage during intensive testing.
- Share specs easily: copy a hardware report to share with others.
- All in the app: specs, stress testing, and live monitoring in one place.

## Run Crucible

<details>
<summary>Run from source (Python)</summary>

### Linux

1. Install GTK4/libadwaita system packages:

```bash
# Fedora
sudo dnf install gtk4-devel libadwaita-devel python3-psutil stress-ng

# Ubuntu/Debian
sudo apt install libgtk-4-dev libadwaita-1-dev python3-psutil stress-ng
```

2. Run Crucible:

```bash
python3 crucible.py
```

### Building the Flatpak

```bash
flatpak-builder --user --install --force-clean build-dir packaging/flatpak/io.github.sugarycandybar.Crucible.yml
flatpak run io.github.sugarycandybar.Crucible
```

</details>

## Screenshots

<p align="center">
	<img src="packaging/linux/screenshots/stress_test.png" alt="Stress testing and monitoring view" width="450" />
</p>

- **Stress Test**: run stress tests and monitor temperature and usage in real time.

<p align="center">
	<img src="packaging/linux/screenshots/whole_specs.png" alt="Full specs view" width="450" />
</p>

- **Specs**: complete hardware overview with CPU, GPU, memory, and OS details.

## License

GPL-3.0-or-later
