#!/usr/bin/env bash
# No pulls, credentials, model calls, host ports, or writable host mounts.
set -euo pipefail
image=${1:?usage: runtime-smoke.sh IMAGE}
docker image inspect "$image" >/dev/null
platform=$(docker image inspect --format '{{.Os}}/{{.Architecture}}' "$image")
[[ "$platform" == linux/amd64 ]] || { echo "unexpected platform: $platform" >&2; exit 1; }
container=
cleanup() {
    if [[ -n "$container" ]]; then docker rm -f "$container" >/dev/null; fi
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
container=$(docker create --pull=never --platform linux/amd64 --network none --init \
    --read-only --tmpfs /tmp:rw,size=512m --cpus 1 --memory 2048m \
    "$image")
docker start "$container" >/dev/null
docker exec "$container" sh -c 'test -x /opt/vw/shiju; /opt/vw/shiju --help >/dev/null'
healthy=false
for ((attempt=1; attempt<=60; attempt++)); do
    if response=$(docker exec "$container" bash -c \
        'exec 3<>/dev/tcp/127.0.0.1/42619; printf "GET /v1/health HTTP/1.0\r\nHost: localhost\r\n\r\n" >&3; cat <&3' 2>/dev/null); then
        if [[ "$response" == *"200 OK"* && "$response" == *'"status":"ok"'* ]]; then
            healthy=true
            printf '%s\n' "$response"
            break
        fi
    fi
    [[ $(docker inspect --format '{{.State.Running}}' "$container") == true ]] || break
    sleep 1
done
if [[ "$healthy" != true ]]; then
    docker logs "$container" >&2
    echo 'Gateway health failed' >&2
    exit 1
fi
docker stop --signal SIGTERM -t 10 "$container" >/dev/null
exit_code=$(docker inspect --format '{{.State.ExitCode}}' "$container")
[[ $(docker inspect --format '{{.State.Running}}' "$container") == false && ( "$exit_code" == 0 || "$exit_code" == 143 ) ]] || {
    echo "uncontrolled Gateway shutdown: $exit_code" >&2; exit 1;
}
echo "PASS: executable, Gateway health, controlled shutdown (exit $exit_code), $platform"
