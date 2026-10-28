"""Execute independent synthetic controls; only candidate failures are rejection."""
import csv
import io
import json
import math
from pathlib import Path
import subprocess
import sys
import tempfile


def equivalent(actual, expected):
    if isinstance(expected, dict):
        return (isinstance(actual, dict) and actual.keys() == expected.keys()
                and all(equivalent(actual[k], v) for k, v in expected.items()))
    if isinstance(expected, list):
        return (isinstance(actual, list) and len(actual) == len(expected)
                and all(equivalent(a, b) for a, b in zip(actual, expected)))
    if type(expected) is int:
        return type(actual) is int and actual == expected
    if type(expected) is float:
        return type(actual) in (int, float) and math.isfinite(actual) and abs(actual - expected) < 1e-8
    return type(actual) is type(expected) and actual == expected


def components(scores):
    """Independent set/flood-fill oracle, with four-neighbor component ties."""
    count, height, width = len(scores), len(scores[0]), len(scores[0][0])
    owned = [set() for _ in scores]
    for y in range(height):
        for x in range(width):
            cell = max(range(count), key=lambda c: (scores[c][y][x], -c))
            if scores[cell][y][x] > 0:
                owned[cell].add((y, x))
    outputs = []
    for cells in owned:
        groups = []
        while cells:
            first = min(cells)
            cells.remove(first)
            todo, group = [first], {first}
            while todo:
                y, x = todo.pop()
                for neighbor in [(y-1, x), (y+1, x), (y, x-1), (y, x+1)]:
                    if neighbor in cells:
                        cells.remove(neighbor)
                        todo.append(neighbor)
                        group.add(neighbor)
            groups.append(group)
        keep = min(groups, key=lambda g: (-len(g), min(g))) if groups else set()
        outputs.append([[int((y, x) in keep) for x in range(width)] for y in range(height)])
    return outputs


def check(app, oracle):
    task = json.loads((oracle / 'task.json').read_text())['child']
    program = app / 'solution.py'
    if not program.is_file():
        return False
    with tempfile.TemporaryDirectory(prefix='cell-control-') as temporary:
        work = Path(temporary)
        output = work / 'result.json'

        def run(args, timeout=8):
            output.unlink(missing_ok=True)
            try:
                process = subprocess.run([sys.executable, str(program), *args, '--output_path', str(output)],
                                         cwd=work, capture_output=True, timeout=timeout)
            except subprocess.TimeoutExpired:
                return False
            return process.returncode == 0 and output.is_file()

        def result():
            try:
                return json.loads(output.read_text())
            except (ValueError, UnicodeError):
                return None

        if task == 'm1':
            for fixture in json.loads((oracle / 'controls.json').read_text()):
                path = work / 'mixed.csv'
                path.write_text(fixture['csv'])
                if not run(['--csv_path', str(path)]) or not equivalent(result(), fixture['expected']):
                    return False
        elif task == 'm2':
            # Reference measurements were computed with real predict_torch and
            # the fixed official checkpoint, separately from the gold program.
            input_path = work / 'synthetic.json'
            input_path.write_bytes((oracle / 'input.json').read_bytes())
            expected = json.loads((oracle / 'expected.json').read_text())
            if not run(['--input_path', str(input_path), '--weights_path', '/opt/mobile-sam/mobile_sam.pt'], 112):
                return False
            actual = result()
            if not isinstance(actual, list) or len(actual) != len(expected):
                return False
            for row, reference in zip(actual, expected):
                if not isinstance(row, dict) or row.keys() != reference.keys():
                    return False
                if type(row['pixels']) is not int or row['pixels'] != reference['pixels']:
                    return False
                if not isinstance(row['samples'], list) or len(row['samples']) != len(reference['samples']):
                    return False
                for measured, target in zip([row['score'], *row['samples']], [reference['score'], *reference['samples']]):
                    if type(measured) not in (int, float) or not math.isfinite(measured) or abs(measured-target) > .002:
                        return False
        elif task == 'm3':
            for scores in json.loads((oracle / 'controls.json').read_text()):
                path = work / 'logits.json'
                path.write_text(json.dumps({'scores': scores}))
                if not run(['--input_path', str(path)]) or not equivalent(result(), components(scores)):
                    return False
        elif task == 'm4':
            for fixture in json.loads((oracle / 'controls.json').read_text()):
                path, polygons = work / 'cells.csv', work / 'refined.json'
                path.write_text(fixture['csv'])
                polygons.write_text(json.dumps(fixture['polygons']))
                if not run(['--csv_path', str(path), '--polygons_path', str(polygons)]):
                    return False
                try:
                    with output.open(newline='') as stream:
                        reader = csv.DictReader(stream)
                        fields, actual = reader.fieldnames, list(reader)
                except (UnicodeError, csv.Error):
                    return False
                source = csv.DictReader(io.StringIO(fixture['csv']))
                original = list(source)
                if fields != source.fieldnames or len(actual) != len(original):
                    return False
                for out, old in zip(actual, original):
                    if out.keys() != old.keys():
                        return False
                    for key in old.keys() - {'xmin', 'ymin', 'xmax', 'ymax', 'coords_x', 'coords_y'}:
                        if out[key] != old[key]:
                            return False
                    points = fixture['polygons'][old['id']]
                    xs, ys = [p[0] for p in points], [p[1] for p in points]
                    try:
                        if ([int(out[k]) for k in ['xmin', 'ymin', 'xmax', 'ymax']]
                                != [min(xs), min(ys), max(xs)+1, max(ys)+1]):
                            return False
                        if not equivalent(json.loads(out['coords_x']), xs) or not equivalent(json.loads(out['coords_y']), ys):
                            return False
                    except (ValueError, TypeError):
                        return False
        else:
            raise ValueError('unknown child')
    return True


if __name__ == '__main__':
    sys.exit(0 if check(Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve()) else 1)
