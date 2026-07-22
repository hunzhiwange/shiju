#!/usr/bin/env python3
"""Historical local Matrix worker entrypoint.

The local Matrix lab now uses the Rust runtime:

    dev/matrix/matrix-local.sh worker-run-all

This file remains only to fail clearly for stale local commands.
"""

from __future__ import annotations

import sys


def main() -> int:
    print(
        "matrix-worker-local.py has been retired; "
        "use dev/matrix/matrix-local.sh worker-run-all",
        file=sys.stderr,
    )
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
