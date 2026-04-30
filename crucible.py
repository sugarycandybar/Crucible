#!/usr/bin/env python3
"""
Crucible - Hardware Specs & Stress Testing
A libadwaita application for viewing system hardware
and running stress tests.
"""
import os
import subprocess
import sys
from pathlib import Path

# If running from source, ensure schemas are compiled and GSETTINGS_SCHEMA_DIR is set
base_dir = Path(__file__).resolve().parent
schema_dir = base_dir / "packaging" / "linux"
if (schema_dir / "io.github.sugarycandybar.Crucible.Stability.gschema.xml").exists():
    env_dir = os.environ.get("GSETTINGS_SCHEMA_DIR", "")
    if str(schema_dir) not in env_dir:
        subprocess.run(["glib-compile-schemas", str(schema_dir)], capture_output=True)
        os.environ["GSETTINGS_SCHEMA_DIR"] = f"{schema_dir}:{env_dir}" if env_dir else str(schema_dir)

from crucible.ui.application import CrucibleApplication


def main():
    """Launch the Crucible application."""
    app = CrucibleApplication()
    return app.run(sys.argv)


if __name__ == "__main__":
    sys.exit(main())
