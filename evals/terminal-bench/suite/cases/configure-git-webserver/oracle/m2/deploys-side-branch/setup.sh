#!/bin/sh
set -eu
root=$1
mkdir -p "$root"
git init --bare --initial-branch=master "$root/server.git"
mkdir -p "$root/public"
cat > "$root/server.git/hooks/post-receive" <<'HOOK'
#!/bin/sh
set -eu
while read old new ref; do
    if [ -n "$ref" ]; then
        git --work-tree="$PWD/../public" checkout -f "$new"
    fi
done
HOOK
chmod 755 "$root/server.git/hooks/post-receive"
