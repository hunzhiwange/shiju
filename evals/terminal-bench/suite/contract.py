"""Frozen experiment records and append-only, single-attempt history.

Completion describes transport termination; reward independently describes the
checker outcome. Infrastructure failures remain unscored. Hashes are SHA-256
of the exact source/runtime/fixture/checker/profile artifacts used by the caller.
No credentials or raw prompts belong in these records.
"""
from datetime import datetime
import fcntl
import json
import math
from pathlib import Path

from jsonschema import Draft202012Validator, FormatChecker

SCHEMA = json.loads(Path(__file__).with_name('schema.json').read_text())
Draft202012Validator.check_schema(SCHEMA)
_VALIDATOR = Draft202012Validator(SCHEMA, format_checker=FormatChecker())


def validate_record(record):
    """Reject unknown fields, policy drift and internally inconsistent evidence."""
    errors = sorted(_VALIDATOR.iter_errors(record), key=lambda error: str(error.path))
    if errors:
        raise ValueError(errors[0].message)
    for value in (record['reward'], record['validation_seconds']):
        if value is not None and not math.isfinite(value):
            raise ValueError('non-finite score or validation duration')
    start = datetime.fromisoformat(record['started_at'].replace('Z', '+00:00'))
    end = datetime.fromisoformat(record['ended_at'].replace('Z', '+00:00'))
    if end < start or record['validation_seconds'] > (end - start).total_seconds():
        raise ValueError('invalid chronology or cumulative validation time')


def append_record(path, record):
    """Serialize cooperating writers and reject duplicate case or attempt IDs.

    Existing history is validated before append and is never rewritten. A damaged
    or partial history fails closed for manual investigation, without paid retry.
    """
    validate_record(record)
    encoded = json.dumps(record, sort_keys=True, allow_nan=False) + '\n'
    with Path(path).open('a+', encoding='utf-8') as history:
        fcntl.flock(history, fcntl.LOCK_EX)
        history.seek(0)
        for line in history:
            previous = json.loads(line)
            validate_record(previous)
            if previous['attempt_id'] == record['attempt_id']:
                raise ValueError('duplicate attempt identity')
            key = ('experiment_id', 'kind', 'case_id')
            if all(previous[field] == record[field] for field in key):
                raise ValueError('duplicate first attempt; paid retry forbidden')
        history.write(encoded)
        history.flush()
