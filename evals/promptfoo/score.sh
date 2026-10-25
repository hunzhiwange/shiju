#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd -- "$SCRIPT_DIR/../.." && pwd)
CONFIG="$SCRIPT_DIR/promptfooconfig.yaml"
RESULTS_DIR="$SCRIPT_DIR/results"
PROMPTFOO_VERSION=0.122.0

usage() {
  cat <<'EOF'
用法：
  evals/promptfoo/score.sh test
  evals/promptfoo/score.sh run [筛选参数]
  evals/promptfoo/score.sh before [筛选参数]
  evals/promptfoo/score.sh after [筛选参数]
  evals/promptfoo/score.sh compare [before.json [after.json [comparison.md]]]

筛选参数：
  --suite NAME          按 metadata.suite 筛选
  --batch NAME          按 metadata.batch 筛选
  --metadata KEY=VALUE  按任意 metadata 筛选，可重复使用
  --provider REGEX      按 Provider 标签筛选
  --pattern REGEX       按测试 description 筛选
  --failing PATH_OR_ID  只重跑指定结果中的失败项
  --repeat N            覆盖重复次数；run 默认 1，before/after 默认 3
  --output PATH         覆盖 JSON 输出路径
  --model NAME          覆盖本次评测模型

示例：
  evals/promptfoo/score.sh run --batch 4
  evals/promptfoo/score.sh run --suite selection --batch 4
  evals/promptfoo/score.sh run --suite coding --model provider/model
  evals/promptfoo/score.sh run --suite coding
  evals/promptfoo/score.sh before --batch 4
  evals/promptfoo/score.sh after --batch 4
  evals/promptfoo/score.sh compare
EOF
}

require_value() {
  local option=$1
  local value=${2:-}
  if [[ -z "$value" ]]; then
    echo "缺少 $option 的值" >&2
    exit 2
  fi
}

run_eval() {
  local mode=$1
  shift
  local previous="" argument
  for argument in "$@"; do
    if [[ "$previous" == "--suite" && "$argument" == "runtime-accuracy" ]]; then
      cd "$REPO_ROOT"
      node "$SCRIPT_DIR/baseline.mjs" "$mode" "$@"
      return
    fi
    previous=$argument
  done

  local repeat=1
  local output=""
  local model=""
  if [[ "$mode" == "before" || "$mode" == "after" ]]; then
    repeat=3
    output="$RESULTS_DIR/$mode.json"
  fi

  local -a filters=()
  while (($# > 0)); do
    case "$1" in
      --suite)
        require_value "$1" "${2:-}"
        filters+=(--filter-metadata "suite=$2")
        shift 2
        ;;
      --batch)
        require_value "$1" "${2:-}"
        filters+=(--filter-metadata "batch=$2")
        shift 2
        ;;
      --metadata)
        require_value "$1" "${2:-}"
        if [[ "$2" != *=* ]]; then
          echo "--metadata 必须使用 KEY=VALUE 格式" >&2
          exit 2
        fi
        filters+=(--filter-metadata "$2")
        shift 2
        ;;
      --provider)
        require_value "$1" "${2:-}"
        filters+=(--filter-providers "$2")
        shift 2
        ;;
      --pattern)
        require_value "$1" "${2:-}"
        filters+=(--filter-pattern "$2")
        shift 2
        ;;
      --failing)
        require_value "$1" "${2:-}"
        filters+=(--filter-failing "$2")
        shift 2
        ;;
      --repeat)
        require_value "$1" "${2:-}"
        if [[ ! "$2" =~ ^[1-9][0-9]*$ ]]; then
          echo "--repeat 必须是正整数" >&2
          exit 2
        fi
        repeat=$2
        shift 2
        ;;
      --output)
        require_value "$1" "${2:-}"
        output=$2
        shift 2
        ;;
      --model)
        require_value "$1" "${2:-}"
        model=$2
        shift 2
        ;;
      -h|--help)
        usage
        return 0
        ;;
      *)
        echo "未知参数：$1" >&2
        usage >&2
        exit 2
        ;;
    esac
  done

  mkdir -p "$RESULTS_DIR"
  local -a command=(
    npx --yes "promptfoo@$PROMPTFOO_VERSION" eval
    -c "$CONFIG"
    --repeat "$repeat"
    --no-cache
    -j 1
  )
  command+=("${filters[@]}")
  if [[ -n "$output" ]]; then
    command+=(-o "$output")
  fi

  cd "$REPO_ROOT"
  if [[ -n "$model" ]]; then
    VIBEWINDOW_EVAL_MODEL="$model" "${command[@]}"
  else
    "${command[@]}"
  fi
}

compare_results() {
  if (($# > 3)); then
    echo "compare 最多接受 before、after 和报告路径三个参数" >&2
    exit 2
  fi

  local before=${1:-"$RESULTS_DIR/before.json"}
  local after=${2:-"$RESULTS_DIR/after.json"}
  local report=${3:-"$RESULTS_DIR/comparison.md"}
  local report_dir
  report_dir=$(dirname -- "$report")
  mkdir -p "$report_dir"

  local temporary_report
  temporary_report=$(mktemp "$report_dir/.comparison.XXXXXX")
  trap 'rm -f "$temporary_report"' RETURN
  node "$SCRIPT_DIR/compare-results.mjs" "$before" "$after" > "$temporary_report"
  mv "$temporary_report" "$report"
  trap - RETURN
  echo "对比报告：$report"
}

main() {
  local command=${1:-help}
  if (($# > 0)); then
    shift
  fi

  case "$command" in
    test)
      if (($# > 0)); then
        echo "test 不接受额外参数" >&2
        exit 2
      fi
      node --check "$SCRIPT_DIR/assertions.mjs"
      node --check "$SCRIPT_DIR/compare-results.mjs"
      node --check "$SCRIPT_DIR/vibewindow-provider.mjs"
      npx --yes "promptfoo@$PROMPTFOO_VERSION" validate -c "$CONFIG"
      ;;
    run|before|after)
      run_eval "$command" "$@"
      ;;
    compare)
      compare_results "$@"
      ;;
    help|-h|--help)
      usage
      ;;
    *)
      echo "未知命令：$command" >&2
      usage >&2
      exit 2
      ;;
  esac
}

main "$@"
