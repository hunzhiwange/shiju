"""Evaluate synthetic scheduling artifacts against independently frozen answers."""
import json
from pathlib import Path
import sys


def unique(pairs):
    value = {}
    for key, item in pairs:
        if key in value:
            raise ValueError('duplicate key')
        value[key] = item
    return value


def event(text):
    lines = text.replace('\r\n', '\n').strip().split('\n')
    if lines[:2] != ['BEGIN:VCALENDAR', 'VERSION:2.0'] or lines[-1] != 'END:VCALENDAR':
        raise ValueError('invalid calendar')
    if lines.count('BEGIN:VCALENDAR') != 1 or lines.count('END:VCALENDAR') != 1:
        raise ValueError('multiple calendars')
    if lines.count('BEGIN:VEVENT') != 1 or lines.count('END:VEVENT') != 1:
        raise ValueError('one event required')
    start, end = lines.index('BEGIN:VEVENT'), lines.index('END:VEVENT')
    if start >= end or not any(x.startswith('PRODID:') and x[7:] for x in lines[2:start]):
        raise ValueError('missing PRODID or invalid event')
    fields = {}
    attendees = []
    for line in lines[start + 1:end]:
        key, value = line.split(':', 1)
        if key == 'ATTENDEE':
            if not value.lower().startswith('mailto:'):
                raise ValueError('invalid attendee')
            attendees.append(value[7:])
        else:
            if key in fields:
                raise ValueError('duplicate field')
            fields[key] = value
    if not fields.get('UID'):
        raise ValueError('missing UID')
    return {key: fields.get(key) for key in ('DTSTART', 'DTEND', 'SUMMARY')}, sorted(attendees)


def check(candidate, oracle):
    task = json.loads((oracle / 'task.json').read_text())
    output = candidate / task['output']
    if output.is_symlink() or not output.is_file() or output.stat().st_size > 16384:
        return False
    try:
        if json.loads((candidate / 'input.json').read_text()) != task['input']:
            return False
        text = output.read_text()
        if task['kind'] == 'm4':
            return event(text) == event(task['expected'])
        actual = json.loads(text, object_pairs_hook=unique)
        return json.dumps(actual, sort_keys=True) == json.dumps(task['expected'], sort_keys=True)
    except (ValueError, OSError, UnicodeError):
        return False


if __name__ == '__main__':
    sys.exit(0 if check(Path(sys.argv[1]), Path(sys.argv[2])) else 1)
