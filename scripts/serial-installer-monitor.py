#!/usr/bin/env python3
"""Observe installer and reboot markers on the independent UART console."""

from __future__ import annotations

import argparse
import errno
import os
from pathlib import Path
import select
import sys
import termios
import time


INSTALL_COMPLETE = "LYNX_PROGRESS phase=installer_complete progress=100"
INSTALL_FAILED = "LYNX_PROGRESS phase=installer_failed"
LOGIN_PROMPT = "t113s3pro-mainline login:"


class MarkerState:
    def __init__(self) -> None:
        self.installer_complete = False
        self.login_reached = False
        self.tail = ""

    def consume(self, text: str) -> str | None:
        self.tail = (self.tail + text)[-8192:]
        if INSTALL_FAILED in self.tail:
            return "installer_failed"
        if not self.installer_complete and INSTALL_COMPLETE in self.tail:
            marker_end = self.tail.index(INSTALL_COMPLETE) + len(INSTALL_COMPLETE)
            self.tail = self.tail[marker_end:]
            self.installer_complete = True
        if self.installer_complete and LOGIN_PROMPT in self.tail:
            self.login_reached = True
            return "login_reached"
        return None


def configure_serial(fd: int) -> None:
    attributes = termios.tcgetattr(fd)
    attributes[0] = 0
    attributes[1] = 0
    attributes[2] = termios.CS8 | termios.CLOCAL | termios.CREAD
    attributes[3] = 0
    attributes[4] = termios.B115200
    attributes[5] = termios.B115200
    attributes[6][termios.VMIN] = 0
    attributes[6][termios.VTIME] = 1
    termios.tcsetattr(fd, termios.TCSANOW, attributes)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--device", required=True)
    parser.add_argument("--timeout", type=int, default=300)
    parser.add_argument("--log", type=Path)
    args = parser.parse_args()
    if args.timeout < 1:
        parser.error("--timeout must be positive")

    deadline = time.monotonic() + args.timeout
    state = MarkerState()
    log_stream = args.log.open("a", encoding="utf-8") if args.log else None
    try:
        fd = os.open(args.device, os.O_RDONLY | os.O_NOCTTY | os.O_NONBLOCK)
        configure_serial(fd)
        try:
            while time.monotonic() < deadline:
                ready, _, _ = select.select([fd], [], [], min(0.5, deadline - time.monotonic()))
                if not ready:
                    continue
                try:
                    chunk = os.read(fd, 4096)
                except OSError as error:
                    if error.errno in (errno.EAGAIN, errno.EINTR):
                        continue
                    raise
                if not chunk:
                    continue
                text = chunk.decode("utf-8", errors="replace")
                if log_stream:
                    log_stream.write(text)
                    log_stream.flush()
                sys.stdout.write(text)
                sys.stdout.flush()
                result = state.consume(text)
                if result == "installer_failed":
                    print("\n[FAIL] Board-side NAND installer reported failure", file=sys.stderr)
                    return 2
                if result == "login_reached":
                    print("\n[PASS] Installer completed and reboot reached the mainline login prompt")
                    return 0
        finally:
            os.close(fd)
    finally:
        if log_stream:
            log_stream.close()

    if state.installer_complete:
        print("Installer completed, but the login prompt was not observed before timeout", file=sys.stderr)
    else:
        print("Installer completion marker was not observed before timeout", file=sys.stderr)
    return 3


if __name__ == "__main__":
    raise SystemExit(main())
