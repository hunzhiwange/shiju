"""Reproduce the independent synthetic raw floppy; no upstream data required."""
from pathlib import Path

# 16-bit real-mode code at 07c0:0000. Print a NUL-terminated string via
# BIOS VGA teletype (int 10h/0eh) and COM1 (int 14h/01h), then echo keys
# obtained through BIOS int 16h to the VGA console forever.
CODE = bytes.fromhex(
    'b8c007'       # mov ax,07c0h
    '8ed8'         # mov ds,ax
    'be2900'       # mov si,0029h (message)
    'ac84c0740c'   # lodsb; test al,al; jz keyboard
    'b40ecd10'     # VGA teletype
    'b40131d2cd14' # COM1 transmit
    'ebef'         # next message byte
    '31c0cd16'     # keyboard: read key
    'b40ecd10'     # echo to VGA
    'b40131d2cd14' # echo key to COM1
    'ebf0'         # next key
)
assert len(CODE) == 41
sector = CODE + b'SYNTHETIC_BOOT_READY\r\n\0'
sector += bytes(510 - len(sector)) + b'\x55\xaa'
image = sector + bytes(1474560 - 512)
if __name__ == '__main__':
    root = Path(__file__).resolve().parents[1]
    for child in ('m1', 'm2', 'm3', 'm4'):
        (root/'fixtures'/child/'boot.img').write_bytes(image)
    (root/'fixtures/m1/other.img').write_bytes(bytes(1474560))
