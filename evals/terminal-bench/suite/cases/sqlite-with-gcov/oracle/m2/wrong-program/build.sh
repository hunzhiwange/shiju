#!/bin/sh
set -eu
mkdir -p build
printf "#!/bin/sh\necho 3.50.4\n" > build/sqlite3
chmod +x build/sqlite3
