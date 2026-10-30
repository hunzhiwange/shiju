"""Black-box synthetic deletion, byte recovery, filtering and file persistence."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile


def main():
    app, oracle = map(Path, sys.argv[1:])
    child = json.loads((oracle/'task.json').read_text())['child']
    program = app/'solution.py'
    if not program.is_file() or program.is_symlink():
        return 1
    with tempfile.TemporaryDirectory(prefix='.recovery-', dir=app) as directory:
        work = Path(directory)
        for i in range(4):
            protected = {}
            if child == 'm1':
                records = [dict(inode=i*10+j, path=path, allocated=allocated, links=links)
                           for j, (path, allocated, links) in enumerate([
                               ('/deleted/launchcode.txt', False, 0),
                               ('/live/launchcode.txt', True, 0),
                               ('/linked/launchcode.txt', False, 1),
                               ('/deleted/launchcode.txt.bak', False, 0),
                               ('/deleted/notes.txt', False, 0)])]
                if i == 3:
                    records = records[1:]
                request = {'records': list(reversed(records))}
                expected = [i*10] if i != 3 else []
            elif child == 'm2':
                data = bytearray(b'noise-only!'*128)
                parts = [b'PASSWORD=TEST', b'\x00fragment'+bytes([i]), b'END\xff']
                extents = []
                for part, offset in enumerate([500+i, 50+i, 900+i]):
                    data[offset:offset+len(parts[part])] = parts[part]
                    extents.append(dict(part=part, offset=offset, length=len(parts[part])))
                image = work/'disk.bin'
                image.write_bytes(data)
                protected[image] = bytes(data)
                request = {'image': str(image), 'extents': list(reversed(extents))}
                expected = {'hex': b''.join(parts).hex()}
            elif child == 'm3':
                valid = '8XD'+str(i)*17+'W54'
                request = {'candidates': [valid, valid, valid.lower(), '8xd'+'a'*17+'w54', valid+'X',
                                         valid[:-4]+'W54', ' '+valid, 'PASSWORD='+valid,
                                         '8XD'+'É'*17+'W54', '8XD'+'_'*17+'W54']}
                expected = [valid]
                if i == 3:
                    request = {'candidates': []}
                    expected = []
            elif child == 'm4':
                source, output = work/'records.json', work/'recovered.txt'
                rows = [dict(id='target', password=f'FICTIONAL{i}B'),
                        dict(id='decoy', password='DECOY'),
                        dict(id='target', password=f'FICTIONAL{i}A'),
                        dict(id='target', password=f'FICTIONAL{i}B')]
                source.write_text(json.dumps(rows))
                output.write_text('stale previous result\n')
                protected[source] = source.read_bytes()
                request = dict(source=str(source), output=str(output),
                               target_id='missing' if i == 3 else 'target')
                expected = b'' if i == 3 else f'FICTIONAL{i}A\nFICTIONAL{i}B\n'.encode()
            else:
                raise ValueError('unknown child')
            result = subprocess.run([sys.executable, str(program)], input=json.dumps(request),
                                    text=True, capture_output=True, timeout=10, cwd=work)
            if result.returncode or any(path.read_bytes() != value for path, value in protected.items()):
                return 1
            if child == 'm4':
                if output.is_symlink() or not output.is_file() or output.read_bytes() != expected:
                    return 1
            else:
                try:
                    actual = json.loads(result.stdout)
                except (ValueError, UnicodeError):
                    return 1
                if json.dumps(actual, sort_keys=True) != json.dumps(expected, sort_keys=True):
                    return 1
    return 0


if __name__ == '__main__':
    sys.exit(main())
