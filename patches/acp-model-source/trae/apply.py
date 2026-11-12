"""Create a separate local-provider composition of the fixed TRAE Go executable.

The supplier executable and its login/quota implementations remain unchanged.
Only the private launcher uses this copy, under an exact loopback network policy.
"""
import hashlib
from pathlib import Path
import struct
import subprocess
import sys

SOURCE_SHA256 = '7648c66ffb535e83e4b1f5bbd820f0384d77234763cfb32700b1be3289358758'


def patch(source, destination):
    data = bytearray(Path(source).read_bytes())
    if hashlib.sha256(data).hexdigest() != SOURCE_SHA256:
        raise ValueError('TRAE fixed native artifact checksum mismatch')
    # query.init constructs four fx options: FlowTrace, MapTraeModelError,
    # CommercializationPreCheck, ReportCustomModelTokenUsage. Register only the
    # first two in the local composition. No auth/quota return value is changed.
    offset = 0x1462458
    assert data[offset:offset + 4].hex() == 'e1037eb2'
    data[offset:offset + 4] = bytes.fromhex('e1037fb2')  # ORR X1, XZR, #2
    # Dedicated native directory accessors, without modifying HOME. Tail-call
    # os.Getenv using Go's arm64 ABI. Method input is unused; return is string.
    for address, key in [(0x100f055b0, 'VW_TRAE_USER_DIR'),
                         (0x100f05670, 'VW_TRAE_CACHE_DIR')]:
        offset = address - 0x100000000
        assert data[offset:offset + 4].hex() == '900b40f9'
        instructions = [0x10000100,  # ADR X0, PC+32 (key in this function)
                        0xd2800001 | (len(key) << 5),  # MOV X1, length
                        0x14000000 | (((0x1001039b0 - (address + 8)) // 4) & 0x3ffffff)]
        data[offset:offset + 12] = struct.pack('<III', *instructions)
        data[offset + 32:offset + 32 + len(key)] = key.encode()
    destination = Path(destination)
    destination.write_bytes(data)
    destination.chmod(0o700)
    subprocess.run(['/usr/bin/codesign', '--force', '--sign', '-', '--timestamp=none',
                    '--identifier', 'shiju.trae.local', str(destination)], check=True,
                   stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)


if __name__ == '__main__':
    patch(sys.argv[1], sys.argv[2])
