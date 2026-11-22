#!/bin/sh
# Resolve the isolated interpreter relative to the installed ACP entry.
exec "${0%/*}/python" -I -c 'from vibe.acp.entrypoint import main; main()' "$@"
