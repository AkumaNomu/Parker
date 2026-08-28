#!/usr/bin/env python3
"""Capture an interactive region through the GNOME screenshot portal."""

from __future__ import annotations

import os
import shutil
import sys
from pathlib import Path
from urllib.parse import unquote, urlparse

import gi

gi.require_version("Gio", "2.0")
from gi.repository import Gio, GLib


def fail(message: str, code: int = 1) -> int:
    print(message, file=sys.stderr)
    return code


def main() -> int:
    if len(sys.argv) != 2:
        return fail("Usage: portal_capture.py OUTPUT")
    destination = Path(sys.argv[1]).expanduser()
    token = f"parker_{os.getpid()}"
    bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
    parameters = GLib.Variant(
        "(sa{sv})",
        (
            "",
            {
                "interactive": GLib.Variant("b", True),
                "modal": GLib.Variant("b", False),
                "handle_token": GLib.Variant("s", token),
            },
        ),
    )
    reply = bus.call_sync(
        "org.freedesktop.portal.Desktop",
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.Screenshot",
        "Screenshot",
        parameters,
        GLib.VariantType("(o)"),
        Gio.DBusCallFlags.NONE,
        10_000,
        None,
    )
    request = reply.unpack()[0]
    loop = GLib.MainLoop()
    response: dict[str, object] = {}

    def on_response(_connection, _sender, path, _interface, _member, parameters):
        if path != request:
            return
        response["body"] = parameters.unpack()
        loop.quit()

    def on_timeout():
        if "body" not in response:
            response["timeout"] = True
            loop.quit()
        return False

    subscription = bus.signal_subscribe(
        None,
        "org.freedesktop.portal.Request",
        "Response",
        request,
        None,
        Gio.DBusSignalFlags.NONE,
        on_response,
    )
    GLib.timeout_add_seconds(120, on_timeout)
    try:
        loop.run()
    finally:
        bus.signal_unsubscribe(subscription)

    if response.get("timeout"):
        return fail("The screenshot portal timed out. Try again.")
    code, results = response.get("body", (1, {}))
    if code != 0:
        return fail("Selection cancelled.", 10 if code == 1 else 1)
    uri = results.get("uri") if isinstance(results, dict) else None
    if not isinstance(uri, str) or not uri.startswith("file:"):
        return fail("The screenshot portal returned no image.")
    source = Path(unquote(urlparse(uri).path))
    if not source.is_file():
        return fail("The screenshot portal returned a missing image.")
    destination.parent.mkdir(parents=True, exist_ok=True)
    try:
        shutil.copyfile(source, destination)
    finally:
        try:
            source.unlink()
        except OSError:
            pass
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        raise SystemExit(f"GNOME screenshot portal failed: {error}")
