#!/usr/bin/env python3
"""
Anvil - Hardware Specs & Stress Testing
A libadwaita application for viewing system hardware
and running stress tests.
"""
import sys

from anvil.ui.application import AnvilApplication


def main():
    """Launch the Anvil application."""
    app = AnvilApplication()
    return app.run(sys.argv)


if __name__ == "__main__":
    sys.exit(main())
