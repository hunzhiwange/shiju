#!/bin/bash
set -euo pipefail

usage() {
    cat <<EOF
Usage: $0 --output-dir <dir> [--package <name>] [cargo-llvm-cov args...]

Options:
    --output-dir <dir>   覆盖率报告输出目录
    --package <name>     仅生成指定 package 的单元测试覆盖率
    -h, --help           显示帮助信息

示例:
    $0 --output-dir coverage/workspace
    $0 --package vw-acp --output-dir coverage/vw-acp
EOF
    exit 0
}

OUTPUT_DIR=""
PACKAGE=""
EXTRA_ARGS=()
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
ORIGINAL_HOME="${HOME:-}"
VW_ACP_PROCESS_IPC_IGNORE_REGEX='crates/vw-acp/src/(client/process(_signals)?|queue_ipc(_health|_transport)?|queue_lease_store|session_runtime/(queue_owner_process|prompt_runner|runtime/(prompt|queue_owner)))\.rs$'

while [[ $# -gt 0 ]]; do
    case "$1" in
        --output-dir)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        --package)
            PACKAGE="$2"
            shift 2
            ;;
        -h|--help)
            usage
            ;;
        *)
            EXTRA_ARGS+=("$1")
            shift
            ;;
    esac
done

if [[ -z "$OUTPUT_DIR" ]]; then
    echo "Error: --output-dir is required"
    usage
fi

if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
    echo "Error: cargo-llvm-cov is required. Install with: cargo install cargo-llvm-cov"
    exit 1
fi

mkdir -p "$OUTPUT_DIR"
mkdir -p "$REPO_ROOT/target/test-home"

export CARGO_HOME="${CARGO_HOME:-$ORIGINAL_HOME/.cargo}"
export RUSTUP_HOME="${RUSTUP_HOME:-$ORIGINAL_HOME/.rustup}"
export HOME="${TEST_HOME:-$REPO_ROOT/target/test-home}"
export RUST_TEST_THREADS="${RUST_TEST_THREADS:-1}"

if [[ -n "$PACKAGE" ]]; then
    TARGET_ARGS=(--bins)
    PACKAGE_MANIFEST="$(
        cargo metadata --no-deps --format-version=1 \
            | PACKAGE="$PACKAGE" perl -0ne '
                my $package = quotemeta($ENV{"PACKAGE"});
                if (/"name":"$package".*?"manifest_path":"([^"]+)"/s) {
                    my $path = $1;
                    $path =~ s#\\\\/#/#g;
                    print $path;
                }
            '
    )"
    if [[ -n "$PACKAGE_MANIFEST" ]]; then
        PACKAGE_DIR="$(cd "$(dirname "$PACKAGE_MANIFEST")" && pwd)"
        if [[ -f "$PACKAGE_DIR/src/lib.rs" ]] || grep -Eq '^[[:space:]]*\[lib\][[:space:]]*$' "$PACKAGE_MANIFEST"; then
            TARGET_ARGS=(--lib --bins)
        fi
    fi
    CMD=(cargo llvm-cov -p "$PACKAGE" --all-features "${TARGET_ARGS[@]}" --html --output-dir "$OUTPUT_DIR")
else
    CMD=(cargo llvm-cov --workspace --all-features --lib --bins --html --output-dir "$OUTPUT_DIR")
fi

if [[ ${#EXTRA_ARGS[@]} -gt 0 ]]; then
    CMD+=("${EXTRA_ARGS[@]}")
fi

if [[ -z "$PACKAGE" || "$PACKAGE" == "vw-acp" ]]; then
    CMD+=(--ignore-filename-regex "$VW_ACP_PROCESS_IPC_IGNORE_REGEX")
fi

perl -e 'alarm shift; exec @ARGV' "${TEST_TIMEOUT:-600}" "${CMD[@]}"
echo "Coverage report: $OUTPUT_DIR/html/index.html"
