# Source from the repository root (bash/zsh): source scripts/relax-env.sh
# Applies only to this terminal. Other Cargo commands still use the real Cargo.
if [ ! -f scripts/relax_dev.py ]; then
    printf '%s\n' '请先 cd 到 ShiJu 仓库根目录。' >&2
    return 1
fi
python3 scripts/relax_dev.py setup || return
SHIJU_RELAX_REPO="$PWD"
cargo() {
    case "$*" in
        'run -p vw-desktop --bin shi-ju')
            python3 "$SHIJU_RELAX_REPO/scripts/relax_dev.py" desktop ;;
        'run -p vw-cli --bin shiju -- gateway')
            python3 "$SHIJU_RELAX_REPO/scripts/relax_dev.py" gateway ;;
        *) command cargo "$@" ;;
    esac
}
printf '%s\n' '已启用当前终端两条指定 cargo run 的 Relax 加速；关闭终端恢复。'
