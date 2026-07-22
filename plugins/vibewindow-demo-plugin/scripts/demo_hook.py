#!/usr/bin/env python3
"""Tiny plugin hook demo.

The runtime passes a JSON payload on stdin. This script validates that the
payload is JSON and emits a short diagnostic without mutating state.
"""

from __future__ import annotations

import json
import os
import sys


def main() -> int:
    phase = sys.argv[1] if len(sys.argv) > 1 else "unknown"
    raw = sys.stdin.read()
    payload = json.loads(raw) if raw.strip() else {}
    event = payload.get("event") or os.environ.get("VIBEWINDOW_PLUGIN_HOOK_EVENT", "unknown")
    plugin = payload.get("plugin") or os.environ.get("VIBEWINDOW_PLUGIN_NAME", "unknown")
    tool = payload.get("tool", {}).get("name", "n/a")
    print(f"demo hook phase={phase} event={event} plugin={plugin} tool={tool}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:
        print(f"demo hook failed: {exc}", file=sys.stderr)
        raise SystemExit(1)
