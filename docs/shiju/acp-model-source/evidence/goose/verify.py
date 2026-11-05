#!/usr/bin/env python3
"""Use task 10's unchanged production entry with Goose's native shell-read fixture.

Goose 1.57.0 exposes shell rather than read_file. The original Cargo runner gets
enough time for the observed >10-minute build. No success check is relaxed.
"""
from functools import partial
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[5]
sys.path.insert(0, str(ROOT / 'scripts'))
import verify_acp_model_source as verification
from native_probe import Fixture

verification.ModelFixture = Fixture
# Keep task 10's process containment; the all-features binary took 13m to build.
verification.execute_cargo = partial(verification.execute_cargo, timeout=1200)
if __name__ == '__main__':
    raise SystemExit(verification.main())
