#!/bin/bash
set -euo pipefail

# ShiJu CLI 通用构建脚本
# 用法: ./scripts/build-cli.sh --target <triple> [--profile <profile>] [--out-dir <dir>] [--use-cross]
# 示例: ./scripts/build-cli.sh --target x86_64-unknown-linux-gnu --use-cross

usage() {
    cat <<EOF
Usage: $0 [OPTIONS]

Options:
    --target <triple>     目标平台 (必需)
    --profile <profile>   构建配置 (default: release)
    --out-dir <dir>       输出目录 (default: dist/cli/<target>)
    --use-cross           使用 cross 进行交叉编译
    --use-xwin            使用 cargo-xwin 编译 Windows 目标
    -h, --help            显示帮助信息

示例:
    $0 --target x86_64-apple-darwin
    $0 --target x86_64-unknown-linux-gnu --use-cross
    $0 --target aarch64-unknown-linux-musl --use-cross
    $0 --target x86_64-pc-windows-msvc --use-xwin
EOF
    exit 0
}

# 默认值
TARGET=""
PROFILE="release"
OUT_DIR=""
USE_CROSS=false
USE_XWIN=false
SOURCE_BINARY_NAME="shiju"
CANONICAL_BINARY_NAME="shiju"
COMPAT_BINARY_NAME="vibe-agent"
BINARY_NAME="$SOURCE_BINARY_NAME"

# 解析参数
while [[ $# -gt 0 ]]; do
    case $1 in
        --target)
            TARGET="$2"
            shift 2
            ;;
        --profile)
            PROFILE="$2"
            shift 2
            ;;
        --out-dir)
            OUT_DIR="$2"
            shift 2
            ;;
        --use-cross)
            USE_CROSS=true
            shift
            ;;
        --use-xwin)
            USE_XWIN=true
            shift
            ;;
        -h|--help)
            usage
            ;;
        *)
            echo "Unknown option: $1"
            usage
            ;;
    esac
done

# 验证参数
if [[ -z "$TARGET" ]]; then
    echo "Error: --target is required"
    usage
fi

# 设置输出目录
if [[ -z "$OUT_DIR" ]]; then
    OUT_DIR="dist/cli/$TARGET"
fi

# 确定二进制文件名
EXE_SUFFIX=""
if [[ "$TARGET" == *"-windows-"* ]]; then
    EXE_SUFFIX=".exe"
fi

# 确定构建目录
if [[ "$PROFILE" == "release" ]]; then
    BUILD_DIR="target/$TARGET/release"
else
    BUILD_DIR="target/$TARGET/debug"
fi

ensure_windows_msvc_archiver() {
    if command -v llvm-lib &>/dev/null; then
        return 0
    fi

    if command -v brew &>/dev/null; then
        local llvm_prefix=""
        llvm_prefix="$(brew --prefix llvm 2>/dev/null || true)"
        if [[ -n "$llvm_prefix" && -x "$llvm_prefix/bin/llvm-lib" ]]; then
            export PATH="$llvm_prefix/bin:$PATH"
            return 0
        fi
    fi

    echo "Error: llvm-lib not found (required for Windows MSVC cross-compilation)."
    echo "Install LLVM and ensure llvm-lib is on PATH, e.g.: brew install llvm"
    exit 1
}

# 检查工具
check_tools() {
    if [[ "$USE_CROSS" == true ]]; then
        if ! command -v cross &>/dev/null; then
            echo "Error: cross not found. Install with: cargo install cross --locked"
            exit 1
        fi
        if ! docker info &>/dev/null; then
            echo "Error: Docker not running. Start Docker first."
            exit 1
        fi
    fi
    
    if [[ "$USE_XWIN" == true ]]; then
        if ! command -v cargo-xwin &>/dev/null; then
            echo "Error: cargo-xwin not found. Install with: cargo install cargo-xwin"
            exit 1
        fi
        if [[ "$TARGET" == *"-pc-windows-msvc" ]]; then
            ensure_windows_msvc_archiver
        fi
    fi
}

# 安装 target
install_target() {
    if [[ "$USE_CROSS" == true || "$USE_XWIN" == true ]]; then
        return
    fi

    echo "Installing target: $TARGET"
    rustup target add "$TARGET" 2>/dev/null || true
}

# 构建
build() {
    echo "============================================"
    echo "Building $CANONICAL_BINARY_NAME for $TARGET"
    echo "Profile: $PROFILE"
    echo "Output: $OUT_DIR"
    echo "============================================"
    
    install_target
    check_tools
    
    local profile_flag=""
    if [[ "$PROFILE" == "release" ]]; then
        profile_flag="--release"
    fi
    
    if [[ "$USE_CROSS" == true ]]; then
        echo "Using cross for cross-compilation..."
        cross build $profile_flag --target "$TARGET" -p vw-cli --bin "$BINARY_NAME" --all-features
    elif [[ "$USE_XWIN" == true ]]; then
        echo "Using cargo-xwin for Windows cross-compilation..."
        cargo xwin build $profile_flag --target "$TARGET" -p vw-cli --bin "$BINARY_NAME" --all-features
    else
        echo "Using native cargo..."
        cargo build $profile_flag --target "$TARGET" -p vw-cli --bin "$BINARY_NAME" --all-features
    fi
}

# 打包
package() {
    local src_binary="$BUILD_DIR/${BINARY_NAME}${EXE_SUFFIX}"
    local compat_binary="$OUT_DIR/${COMPAT_BINARY_NAME}${EXE_SUFFIX}"
    
    if [[ ! -f "$src_binary" ]]; then
        echo "Error: Binary not found at $src_binary"
        exit 1
    fi
    
    # 创建输出目录
    mkdir -p "$OUT_DIR"
    
    # 复制二进制文件
    cp "$src_binary" "$OUT_DIR/${CANONICAL_BINARY_NAME}${EXE_SUFFIX}"
    cp "$src_binary" "$compat_binary"
    
    # 获取二进制大小
    local size
    size=$(stat -f%z "$src_binary" 2>/dev/null || stat -c%s "$src_binary" 2>/dev/null)
    local size_mb=$((size / 1024 / 1024))
    
    # 创建压缩包
    local archive_name="shiju-$TARGET.tar.gz"
    if [[ -n "$EXE_SUFFIX" ]]; then
        archive_name="shiju-$TARGET.zip"
        (cd "$OUT_DIR" && zip -qry "$archive_name" "${CANONICAL_BINARY_NAME}${EXE_SUFFIX}" "${COMPAT_BINARY_NAME}${EXE_SUFFIX}")
    else
        (cd "$OUT_DIR" && tar -czf "$archive_name" "$CANONICAL_BINARY_NAME" "$COMPAT_BINARY_NAME")
    fi
    
    echo ""
    echo "============================================"
    echo "Build complete!"
    echo "Binary: $OUT_DIR/${CANONICAL_BINARY_NAME}${EXE_SUFFIX}"
    echo "Compat alias: $compat_binary"
    echo "Size: ${size_mb}MB"
    echo "Archive: $OUT_DIR/$archive_name"
    echo "============================================"
}

# 主流程
main() {
    build
    package
}

main
