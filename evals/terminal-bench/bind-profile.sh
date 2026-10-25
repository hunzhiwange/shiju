#!/usr/bin/env bash
# Loopback-only bootstrap. Payloads never appear in command arguments or logs.
set -euo pipefail
set +x
request() {
  local method=$1 path=$2 file=$3 body status
  body=$(cat "$file")
  exec 3<>/dev/tcp/127.0.0.1/42619
  LC_ALL=C printf '%s %s HTTP/1.1\r\nHost: localhost\r\nX-Platform-ID: local\r\nX-Company-ID: local\r\nContent-Type: application/json\r\nContent-Length: %s\r\nConnection: close\r\n\r\n%s' \
    "$method" "$path" "${#body}" "$body" >&3
  IFS= read -r status <&3
  cat <&3 >/dev/null
  exec 3>&-
  [[ $status == *' 200 '* ]] || { printf 'Profile HTTP status: %s %s\n' "$method" "$status" >&2; return 1; }
}
trap 'rm -f "$1/provider.json" "$1/auth.json"' EXIT
request PATCH /v1/config "$1/provider.json"
request PUT /v1/auth/token-plan "$1/auth.json"
