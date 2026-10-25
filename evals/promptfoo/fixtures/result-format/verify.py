"""Validate the captured export projections without invoking Promptfoo."""
import json
from collections import Counter
from pathlib import Path

fixture = Path(__file__).parent
for name, count, groups in [
    ("one-case", 3, {"case-pass": 3}),
    ("two-cases", 6, {"case-pass": 3, "case-fail": 3}),
]:
    document = json.loads((fixture / (name + ".json")).read_text())
    rows = document["results"]["results"]
    assert len(rows) == count
    assert Counter(row["testCase"]["description"] for row in rows) == groups
    assert len({row["id"] for row in rows}) == count
    assert all(not any(key in row for key in ("attempt", "attemptId", "repeatIndex")) for row in rows)
    assert sum(row["success"] for row in rows) == 3
    assert all(row["success"] == row["gradingResult"]["pass"] for row in rows)
    if count == 6:
        stats = document["results"]["stats"]
        assert (stats["successes"], stats["failures"], stats["errors"]) == (3, 3, 0)
    print(f"{name}: {count} rows, {groups}; verified")
