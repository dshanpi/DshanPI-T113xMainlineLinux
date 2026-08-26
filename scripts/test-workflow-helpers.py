#!/usr/bin/env python3
"""Unit tests for host workflow helpers without requiring USB or UART hardware."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parent


def load(name: str, filename: str):
    spec = importlib.util.spec_from_file_location(name, ROOT / filename)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {filename}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


selector = load("select_openix_device", "select-openix-device.py")
monitor = load("serial_installer_monitor", "serial-installer-monitor.py")


class SelectorTests(unittest.TestCase):
    def test_selects_one_allwinner_endpoint(self) -> None:
        output = "\n".join(
            [
                '{"event":"scan_started"}',
                '{"event":"device","bus":3,"port":2,"vid":7994,"pid":61416,"location":"libusb:3:2"}',
                '{"event":"scan_complete","count":1}',
            ]
        )
        self.assertEqual(selector.select_location(selector.parse_devices(output)), "libusb:3:2")

    def test_rejects_ambiguous_endpoints(self) -> None:
        devices = [
            {"vid": 0x1F3A, "location": "libusb:1:1"},
            {"vid": 0x1F3A, "location": "libusb:2:1"},
        ]
        with self.assertRaisesRegex(ValueError, "multiple Allwinner devices"):
            selector.select_location(devices)

    def test_rejects_non_json_output(self) -> None:
        with self.assertRaisesRegex(ValueError, "invalid OpenixCLI JSONL"):
            selector.parse_devices("Scanning USB devices...")


class MarkerTests(unittest.TestCase):
    def test_requires_installer_complete_before_login(self) -> None:
        state = monitor.MarkerState()
        self.assertIsNone(state.consume(monitor.LOGIN_PROMPT))
        self.assertIsNone(state.consume(monitor.INSTALL_COMPLETE))
        self.assertEqual(state.consume(monitor.LOGIN_PROMPT), "login_reached")

    def test_old_login_in_same_buffer_cannot_satisfy_new_install(self) -> None:
        state = monitor.MarkerState()
        self.assertIsNone(state.consume(monitor.LOGIN_PROMPT + monitor.INSTALL_COMPLETE))
        self.assertEqual(state.consume(monitor.LOGIN_PROMPT), "login_reached")

    def test_installer_failure_is_terminal(self) -> None:
        state = monitor.MarkerState()
        self.assertEqual(state.consume(monitor.INSTALL_FAILED), "installer_failed")


if __name__ == "__main__":
    unittest.main()
