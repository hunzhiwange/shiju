#!/bin/sh
set -eu
root=$1
mkdir -p "$root/main" "$root/dev"
git init --bare --initial-branch=main "$root/server.git"
cat > "$root/server.git/hooks/post-receive" <<'HOOK'
#!/bin/sh
set -eu
while read old new ref; do
 case "$ref" in
 refs/heads/main) dest=main ;;
 refs/heads/dev) dest=dev ;;
 *) dest=main ;;
 esac
 git archive "$new" | tar -x -C "$PWD/../$dest"
done
HOOK
chmod 755 "$root/server.git/hooks/post-receive"
