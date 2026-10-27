from pathlib import Path
import sys
Path(sys.argv[2]).write_text('secret[incorrect]\n')
