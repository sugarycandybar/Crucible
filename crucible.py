#!/usr/bin/env python3
"""
Crucible - Hardware Specs & Stress Testing
A libadwaita application for viewing system hardware
and running stress tests.
"""
import sys

from crucible.ui.application import CrucibleApplication


def main():
    """Launch the Crucible application."""
    app = CrucibleApplication()
    return app.run(sys.argv)


if __name__ == "__main__":
    sys.exit(main())
