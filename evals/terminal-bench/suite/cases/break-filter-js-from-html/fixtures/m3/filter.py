import re, sys
from pathlib import Path

def clean(s):
    s = re.sub(r"<script\b[^>]*>.*?</script\s*>", "", s, flags=re.I | re.S)
    return re.sub(r' on[a-z]+="[^"]*"', '', s)

if __name__ == '__main__':
    p=Path(sys.argv[1]);p.write_text(clean(p.read_text()))
