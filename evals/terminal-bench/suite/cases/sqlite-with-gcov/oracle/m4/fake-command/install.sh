#!/bin/sh
set -eu
mkdir -p bin
printf "#!/bin/sh\necho 3.50.4\n" > bin/sqlite3
chmod +x bin/sqlite3
