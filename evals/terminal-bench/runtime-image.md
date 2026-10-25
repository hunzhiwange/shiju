# Linux 运行时镜像

仅打包任务25验证过的默认 features `vw-cli/shiju` Linux ELF，验证无模型的 Gateway 健康入口；不运行 Harbor、题目或 verifier。基础镜像固定为 manifest 的 fix-git Debian amd64 digest，保留其原有层和通知；该镜像继承上游任务工作区，不是通用发行镜像。Gateway 使用 `/tmp` 的临时 HOME/config，不读取该工作区。

## 重建

先按 [preflight.md](preflight.md) 恢复并校验 Rust 1.99.0、Zig 0.14.1、Linux std 和运行时 sidecars。Dockerfile 不调用 Cargo；跨平台编译使用已验证的宿主工具链。不能复制 macOS 二进制。

所有命令在仓库根目录的同一个 Bash 中执行。领取失败的锁不能删除；十个槽均占用时等待后重试。下面的 trap 仅适用于前台串行执行，必须等待所有子进程和容器退出后才释放锁；强制杀死持有进程后的遗留锁需人工核实。

```bash
set -euo pipefail
REPO="$PWD"
mkdir -p target/rust-slot-locks
SLOT=
for n in {1..10}; do
  if mkdir "target/rust-slot-locks/slot-$n.lock" 2>/dev/null; then
    SLOT=$n; break
  fi
done
test -n "$SLOT" || { echo 'All Cargo slots occupied; retry later' >&2; exit 1; }
LOCK="$REPO/target/rust-slot-locks/slot-$SLOT.lock"
printf 'plan=2026-10-25-底层准确度评测 task=26 session=manual pid=%s\n' "$$" > "$LOCK/owner"
trap 'rm -f "$LOCK/owner"; rmdir "$LOCK"' EXIT
export CARGO_TARGET_DIR="$REPO/target/rust-slot-$SLOT"
TOOLS="$REPO/evals/promptfoo/results/task25/toolchain"
RESULTS="$REPO/evals/promptfoo/results/task26"
mkdir -p "$RESULTS/context"

# Before/after build, record gateway-preflight.py::source_digest and require equality.
# Store HEAD, raw Rust/Cargo diff and binary SHA256 under the ignored RESULTS.
CARGO_BUILD_BUILD_DIR="$CARGO_TARGET_DIR" \
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_DEV_INCREMENTAL=false CARGO_BUILD_JOBS=4 \
RUSTC="$TOOLS/host-rust/bin/rustc" RUSTDOC="$TOOLS/host-rust/bin/rustdoc" \
CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER="$TOOLS/zig-amd64-cc" \
CC_x86_64_unknown_linux_gnu="$TOOLS/zig-amd64-cc" \
CXX_x86_64_unknown_linux_gnu="$TOOLS/zig-amd64-cxx" \
AR_x86_64_unknown_linux_gnu="$TOOLS/zig-ar" \
RANLIB_x86_64_unknown_linux_gnu="$TOOLS/zig-ranlib" \
cargo build -p vw-cli --bin shiju --target x86_64-unknown-linux-gnu --locked

# Verify each input against manifest.runtime.requiredSidecars before copying.
for name in libsqlite3.so libsqlite3.so.0 ca-certificates.crt openssl.cnf; do
  cp -L "$TOOLS/linux-libs-amd64/$name" "$RESULTS/context/$name"
done
# SOURCE_SHA256 must be the just-verified Rust/Cargo snapshot digest.
: "${SOURCE_SHA256:?set the verified snapshot digest}"
docker buildx build --load --platform linux/amd64 --network none \
  --build-context "runtime-slot=$CARGO_TARGET_DIR" \
  --build-arg "CARGO_TARGET_DIR=$CARGO_TARGET_DIR" \
  --build-arg "SOURCE_COMMIT=$(git rev-parse HEAD)" \
  --build-arg "SOURCE_SHA256=$SOURCE_SHA256" \
  --iidfile "$RESULTS/image-id" -t vw-terminal-runtime:task26 \
  -f evals/terminal-bench/Dockerfile "$RESULTS/context"
bash evals/terminal-bench/runtime-smoke.sh "$(cat "$RESULTS/image-id")"
cargo clippy --all-targets --all-features -- -D warnings
```

运行时使用不可变 image ID；tag 仅为本地便利名称。未发布 registry，不能把本地 ID 称为远程 RepoDigest。Docker 的只读 BuildKit bind 挂载使用持有槽位的绝对路径，只有 ELF 被复制到最终层；不在容器内生成或共享 Cargo 缓存。普通构建上下文只包含四个校验过的 sidecars，不包含仓库、results、配置或凭据。不要把仓库根目录作为普通 context。

`runtime-smoke.sh` 要求镜像已存在且为 linux/amd64；禁用拉取与网络，不挂载宿主文件、不发布端口。它检查可执行文件和 CLI help，轮询 `/v1/health` 的 HTTP200 与 `status=ok`，运行时显式使用 Docker `--init` 转发信号与回收子进程，然后 SIGTERM 停止并要求退出0或标准信号退出码143（拒绝超时强杀137），最后回收容器。无需 curl/Python 或模型凭据；健康检查不能证明工具权限、模型推理或题目准确率。

## 本次证据

原始日志、源码差异和完整命令位于忽略目录 `evals/promptfoo/results/task26/`，最终结果见 `evidence.json`、`smoke-missing.log`、`smoke.log`。不将源码/工具载荷写入提交。

停止契约已实测：原生 PID1 的 SIGTERM（10秒）与 SIGINT（5秒）均超时后退出137；Docker `--init` 下 SIGTERM（10秒上限）约0.07秒退出143，SIGINT约0.10秒退出130。运行镜像必须使用 `--init`，烟测固定 SIGTERM 并接受退出0/143，不接受137；这证明信号终止与进程回收，不宣称请求排空或持久化的优雅关闭。无需变更生产 Rust。信号矩阵见 `signals.json`；最初失败日志保留在 `first-attempt/`。

本次源码为提交 `66dc4e0949a44e9606038a3c1c17a59e363fb715` 加 Rust/Cargo 工作树快照；构建前后 SHA256 均为 `5f586a428553e7cf69e5dfd64e120cc49a62b6bc22130154237c56d16921bde5`。ELF SHA256 为 `c7d7295c506c8aa996ff5a243841139fcd9ea640f21d241ec095d2d1c28ecf9d`。源码差异仅存忽略目录。

| 本次检查 | 结果 |
| --- | --- |
| `bash -n evals/terminal-bench/runtime-smoke.sh` | 退出0 |
| 缺失镜像负例 | 退出1，1次，未拉取 |
| 默认features Linux Cargo build | 退出0，源码摘要前后一致 |
| `docker buildx build` | 退出0，命令/参数见 evidence.json |
| `bash evals/terminal-bench/runtime-smoke.sh <image-id>` | 退出0，1次完整烟测；HTTP200/status=ok、build_id匹配、停止143、容器回收 |
| 信号矩阵 | 3次：PID1/SIGINT为137，init/SIGTERM为143，init/SIGINT为130；另保留最初PID1/SIGTERM失败 |
| 镜像审核 | image-audit.json：amd64、基础层不变、镜像内ELF摘要匹配、context只含四个sidecars；未复制宿主凭据 |

本地镜像 manifest list digest：`sha256:e671be5f313bdf8a7cff737081362eae0409a85f8c9cab1081986fdbed8b7788`（包含 BuildKit attestation，未推送）；amd64内容manifest：`sha256:94fc9a2a7f3793e8016a5252970194963151c4ab1949aa417e4c22703be63240`。原始 inspect 位于 evidence.json，不能以可变tag作为身份。

本次实际动态槽位1，`CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`；构建、镜像打包和Clippy共用该绝对目录，前台子进程结束后释放锁并保留缓存。构建挂载为只读；无Docker内Cargo缓存。未修改Rust，无新增Rust测试；不以零测试或编译成功代替容器烟测。模型调用、题目和verifier执行均为0。

`cargo clippy --all-targets --all-features -- -D warnings` 退出0（2m57s）；仅报告既有第三方 `block 0.1.6` future-incompat 提示。有效Rust测试0，行为证据为缺失镜像负例、完整容器烟测和信号矩阵；未运行全量单元测试、Makefile、wasm检查或fmt检查。`evidence.json` 确认 `slotReleased=true`，无遗留烟测容器。
