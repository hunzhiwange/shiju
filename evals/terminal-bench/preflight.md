# 公开终端基准预检

本次固定五个真实 legacy 任务，验证环境和接入契约；未实现适配器，未调用模型、执行题目或 verifier。模型保持 `token-plan/deepseek-v4-flash-0731`。这不是正式榜单题集发布版，也不是五题准确率结果。

## 冻结来源与许可

| 来源 | 不可变身份 | 取证 |
| --- | --- | --- |
| Harbor | `6099b8cf2bfbbb3e7ff1eb2163bfc4524dbd9ba5`，0.24.0 | 安装后版本、CLI help、实际 API 签名及 Task 解析 |
| Terminal-Bench | `e67a2cb7db4d9795c212909b3395d8c22ad60aec`，legacy 子集 | 每题 Git tree 与四文件 SHA256 见 manifest |
| VibeWindow | manifest 中的 HEAD + Rust/Cargo 工作树摘要 | 包含既有未提交改动；构建前后摘要一致，原始 diff 仅存忽略目录 |

五个 taskId：`fix-git`、`git-multibranch`、`log-summary-date-ranges`、`regex-log`、`sqlite-db-truncate`。全部位于冻结仓库的 `legacy/<taskId>`。manifest 保留上游 tag 供追溯，实际运行只用 digest；不以 latest 标识实验。

两仓库根 LICENSE 均为 Apache-2.0；Harbor pyproject 也声明此许可。重分发须保留许可证、作者/第三方 notices；task 资源仍需保留其原有通知。上游任务含禁止进入训练语料的 canary，源码、答案和工具载荷仅保存在 `evals/promptfoo/results/task25/`，不复制进提交或发送给被测 Agent。

可追溯官方来源：

- [custom-agents 文档](https://docs.harborframework.com/agents/custom-agents)、[verifier 文档](https://docs.harborframework.com/tasks/verifier)。实时文档只作导航，契约以冻结源码为准。
- [Harbor installed/base.py](https://github.com/harbor-framework/harbor/blob/6099b8cf2bfbbb3e7ff1eb2163bfc4524dbd9ba5/src/harbor/agents/installed/base.py)、[agents/base.py](https://github.com/harbor-framework/harbor/blob/6099b8cf2bfbbb3e7ff1eb2163bfc4524dbd9ba5/src/harbor/agents/base.py)、[environments/base.py](https://github.com/harbor-framework/harbor/blob/6099b8cf2bfbbb3e7ff1eb2163bfc4524dbd9ba5/src/harbor/environments/base.py)。
- [冻结 TaskConfig](https://github.com/harbor-framework/harbor/blob/6099b8cf2bfbbb3e7ff1eb2163bfc4524dbd9ba5/src/harbor/models/task/config.py)、[CLI jobs.py](https://github.com/harbor-framework/harbor/blob/6099b8cf2bfbbb3e7ff1eb2163bfc4524dbd9ba5/src/harbor/cli/jobs.py)、[题目目录](https://github.com/harbor-framework/terminal-bench/tree/e67a2cb7db4d9795c212909b3395d8c22ad60aec/legacy)。

## 环境与架构

宿主 darwin/arm64，Docker 28.1.1、服务端 linux/arm64，4 CPU、6213808128 字节内存。五个上游镜像只有 linux/amd64，因此必须显式使用 `--platform linux/amd64`，运行时也构建为 x86_64 GNU/Linux。官方镜像内 `uname -m` 验证仿真；不把此前 ARM64 ELF 的 help/health 作为本路线的证明。

每题声明 1 CPU、2048 MiB、10240 MiB 存储、GPU=0，agent/verifier 均 900s、build 600s。串行运行；不能降低资源或超时来适应机器，也不能与其它占用假定无资源争用。未选入要求 8/16 GiB 或 H100 的当前主线任务。

Gateway 运行需要 Linux SQLite 动态库和可信 CA/OpenSSL 配置。`regex-log` 是最小 Ubuntu 镜像，无 Python、证书目录；不能假定所有题都有 curl/Python。已从固定 fix-git 官方 Debian 镜像复制 `libsqlite3.so.0`、`ca-certificates.crt`、`openssl.cnf`，作为只读运行时附属文件，摘要见 manifest。库目录同时提供 `libsqlite3.so` 和指向它的 `libsqlite3.so.0`。设置 `LD_LIBRARY_PATH`、`SSL_CERT_FILE`、`OPENSSL_CONF`，未关闭 TLS 验证。

模型端点从官方 amd64 fix-git 镜像无凭据 GET 返回 401，退出0；证明 DNS/TLS/HTTP 可达，未证明鉴权或模型推理成功。未发送令牌、请求 chat/completions 或绑定宿主凭据。模型 profile 的凭据与权限绑定仍由后续任务处理，不写入 manifest。

## 已验证源码构建路线（供任务26使用）

使用宿主 Rust 1.99.0 的独立副本、Zig 0.14.1 和 glibc 2.36 目标；不在 Docker 中再建 Rust target。任务25的真实命令、槽位、环境及退出码见忽略目录 `build-evidence.json`；原始编译日志为 `cargo-build.log`。构建使用 vw-cli 默认 features，未修改 Rust/Cargo 源码。

后续重建先按任务26完整槽位契约动态领取 `target/rust-slot-1..10` 中的一个槽，export 其绝对 `CARGO_TARGET_DIR`，记录 owner；子进程结束后才释放。不要照抄本次实际槽位1作为固定槽，也不要调用默认 target。下列路径均相对于仓库根，`TOOLS` 为任务25已恢复并校验的工具目录：

```bash
REPO="$PWD"
TOOLS="$REPO/evals/promptfoo/results/task25/toolchain"
CARGO_BUILD_BUILD_DIR="$CARGO_TARGET_DIR" \
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_DEV_INCREMENTAL=false CARGO_BUILD_JOBS=4 \
RUSTC="$TOOLS/host-rust/bin/rustc" RUSTDOC="$TOOLS/host-rust/bin/rustdoc" \
CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER="$TOOLS/zig-amd64-cc" \
CC_x86_64_unknown_linux_gnu="$TOOLS/zig-amd64-cc" \
CXX_x86_64_unknown_linux_gnu="$TOOLS/zig-amd64-cxx" \
AR_x86_64_unknown_linux_gnu="$TOOLS/zig-ar" \
RANLIB_x86_64_unknown_linux_gnu="$TOOLS/zig-ranlib" \
cargo build -p vw-cli --bin shiju --target x86_64-unknown-linux-gnu --locked
```

工具缓存重建方法（已实际验证，不运行镜像大构建）：

1. 从宿主 `stable-aarch64-apple-darwin` 复制完整工具链到 `$TOOLS/host-rust`，先确认 `rustc --version` 为 manifest 固定值；不能仅依据名为 1.99.0 的目录存在就认为其中有编译器。本次首个构建探测因此失败，复制完整 stable 工具链后通过。
2. 下载 [Rust std](https://static.rust-lang.org/dist/2026-10-01/rust-std-1.99.0-x86_64-unknown-linux-gnu.tar.xz)，按 manifest 的 SHA256 校验，解包后执行 `sh install.sh --prefix="$TOOLS/host-rust" --disable-ldconfig`。不修改全局 rustup。
3. 下载 [Zig wheel](https://files.pythonhosted.org/packages/16/a8/6989b2d0f61f6b33ec7f711b48089fc9d1a53787ce657e81b0b941d22f59/ziglang-0.14.1-py3-none-macosx_12_0_arm64.whl)，按 manifest 校验，解包并 chmod `$TOOLS/ziglang/zig`。
4. `zig-amd64-cc/cxx` 是可执行 Python 包装：过滤输入的 `--target=...`，再 `os.execv(zig, ["zig", "cc"或"c++", "-target", "x86_64-linux-gnu.2.36"] + args)`；没有 `-c` 时在 args 末尾加 `-L "$TOOLS/linux-libs-amd64"`。`zig-ar/ranlib` 分别 exec `zig ar/ranlib "$@"`。实际包装源文本保留在忽略目录，未引入生产配置。
5. `docker create --platform linux/amd64` 固定 fix-git 镜像；用 `docker cp -L` 取出 `/usr/lib/x86_64-linux-gnu/libsqlite3.so.0` 为 `$TOOLS/linux-libs-amd64/libsqlite3.so`，建立 `.so.0` 相对软链接；复制上述可信证书/配置，然后删除临时容器。输入文件摘要须匹配 manifest。

本次实际 `CARGO_TARGET_DIR` 为仓库绝对路径下 `target/rust-slot-1`，编译退出0、3m38s，构建前后 source_digest 相同；生成 ELF 的 SHA256 见 manifest。二进制复制到 results/runtime 后再释放槽位，探测不继续占用 target 文件。任务26可以据此打包，但本任务没有新增 Dockerfile/runtime-smoke.sh，也没有完成任务26。

## 唯一候选入口与启动命令

选容器内 Gateway + SSE。非交互 `agent -m` 虽存在，但 `cli/run.rs` 调用旧 `run_single_message`/`run_tool_call_loop`，不用于公开基准。CLI 的 Gateway 分派为 `cli.rs::Commands::Gateway` → `lib.rs` → `gateway::run_gateway`；`--config-dir` 是全局参数。

以下命令按各题 manifest 的固定 `IMAGE` 和官方 cwd 运行；不把 `/app/personal-site` 一律覆盖成 `/app`。无 Python/curl 的题也能通过 Bash 内置 `/dev/tcp` 健康检查：

```bash
RESULTS="$PWD/evals/promptfoo/results/task25"
docker run -d --pull=never --platform linux/amd64 --network none \
  --name vw-task25-probe --read-only --tmpfs /tmp:rw,size=512m \
  -e HOME=/tmp/vw-home -e RUST_LOG=warn -e LD_LIBRARY_PATH=/opt/vw-libs \
  -e SSL_CERT_FILE=/opt/vw-libs/ca-certificates.crt \
  -e OPENSSL_CONF=/opt/vw-libs/openssl.cnf \
  -v "$RESULTS/runtime/shiju:/opt/vw/shiju:ro" \
  -v "$RESULTS/toolchain/linux-libs-amd64:/opt/vw-libs:ro" \
  --entrypoint /bin/sh "$IMAGE" -c \
  'mkdir -p /tmp/vw-home /tmp/vw-config; exec /opt/vw/shiju --config-dir /tmp/vw-config gateway --host 127.0.0.1 --port 42619'
docker exec vw-task25-probe bash -c \
  'exec 3<>/dev/tcp/127.0.0.1/42619; printf "GET /v1/health HTTP/1.0\r\nHost: localhost\r\n\r\n" >&3; cat <&3'
docker stop -t 2 vw-task25-probe
docker rm -f vw-task25-probe
```

健康探测使用只读题目文件系统、临时 HOME/config、无网络和无宿主端口发布；这些是无模型入口探测条件，不作为后续 Agent 的完整权限配置。题目执行时必须沿既有权限契约配置实际任务工作区，不把只读启动成功当成工具执行能力通过。

## Harbor 与 SSE 接口（供任务27）

安装使用冻结 Harbor 源码（包括 `src/`、`packages/`、pyproject、uv.lock）和独立 Python 3.13 环境。上游要求 uv >=0.13.5；本次恢复固定 uv 0.13.5，再执行 `uv sync --project <frozen-harbor> --frozen --no-dev --no-editable`，按上游锁文件安装。uv 本身应放在独立工具环境中，避免 sync 清理项目环境内的 uv 包。`HARBOR_TELEMETRY=0 harbor --version`、`harbor run --help` 均退出0。

动态 import/inspect 已确认：

```python
BaseInstalledAgent.install(self, environment: BaseEnvironment) -> None
BaseAgent.run(self, instruction: str, environment: BaseEnvironment,
              context: AgentContext) -> None
BaseInstalledAgent.exec_as_agent(self, environment, command: str,
    env: dict[str, str] | None = None, cwd: str | None = None,
    timeout_sec: int | None = None) -> Any
BaseEnvironment.exec(self, command: str, cwd=None, env=None,
    timeout_sec=None, user: str | int | None = None) -> ExecResult
BaseEnvironment.upload_file(self, source_path: Path | str, target_path: str)
```

后续适配器继承 BaseInstalledAgent；上传受信运行时并在官方 task 环境启动 Gateway，run 仅传 instruction，不传 tests/solution。Harbor 本地目录命令以冻结 CLI 为准，使用 `--path`，不是把文档中的 `-t` 直接当文件路径：

```bash
HARBOR_TELEMETRY=0 harbor run --path <frozen-terminal-bench>/legacy/<taskId> \
  --agent <adapter.module>:<ClassName> --model token-plan/deepseek-v4-flash-0731
```

上述命令属于任务27之后的模型运行，本次未执行；适配器模块尚不存在，不是假装已完成的调用命令。

SSE 请求：`POST /v1/chat/stream?directory=<URL编码后的官方任务cwd>`；`Content-Type: application/json`，每次尝试唯一 `X-Request-ID`，需要时保持相同租户请求头。body：

```json
{"session_id":"<unique-attempt>","model":"token-plan/deepseek-v4-flash-0731","messages":[{"role":"user","content":"<instruction.md>"}]}
```

本体 `vw-api-types/src/chat.rs::GatewayChatStreamRequest` 另支持可选 system、reasoning_effort、agent、allowed_tools、options 等，不新增猜测参数。Gateway `handlers/session.rs` 注册路由，`session/stream.rs::chat_stream` 绑定 directory/请求身份并调用 producer → `session::runtime::start_prompt` → `session::processor::run`。CLI processor bridge 也调用同一个 processor；并未将旧 `agent -m` 宣称为相同 session 入口。

无 curl/Python 时可使用同一二进制的 `shiju realtime --host 127.0.0.1 --port 42619 --directory <cwd> --output jsonl --yes chat-stream --request-id <UUID> --input /tmp/request.json` 客户端（已核对实际 `--help` 与 input 读取源码；也可 `--input -` 从 stdin 读取 JSON）。它仍请求 Gateway SSE，不在宿主另起 Agent loop。`--yes` 只确认这条 CLI 请求操作，不修改 Gateway 工具审批或运行时权限。终止接口为 `POST /v1/chat/stream/{request_id}/stop`；Last-Event-ID 用于同请求重连，不用重连隐式重跑任务。适配器必须保存原始事件、终态、usage/缺失标记和失败退出状态。

## 评分边界（供任务28）

五题 schema_version=1.1，`Task(...)` 解析和输入/测试路径校验均成功。冻结 Harbor 的 `VerifierConfig` 中 environment_mode 未设置、且没有 verifier.environment 时采用 shared；保留原题语义，不照实时文档的新任务默认值改成 separate。入口为 `tests/test.sh`，输出 `/logs/verifier/reward.txt` 或 reward.json；二者都存在时 Harbor 优先 JSON。必须保留脚本实际失败，缺失/无效 reward 为基础设施错误；Agent 自述、HTTP200、Gateway done 都不是得分。

本次未运行官方 verifier、oracle 正负例、真实 SSE 模型轮次或适配器测试；这些属于任务27/28，不能由健康检查代替。未改变内部15题、holdout、oracle、权限、预算或原始基线。

## 证据索引与检查

原始文件仅在忽略目录 `evals/promptfoo/results/task25/`：`build-evidence.json`、`cargo-build.log`、`harbor-api-validation.json`、`harbor-run-help.txt`、`runtime-amd64-probes.json`、各镜像 manifest/pull/OCI 下载日志、`official-image-model-network.log`、`runtime-source.diff`。

网络超时与最初缺工具、缺 SQLite/证书失败均保留为失败证据；仅把后续新验证确实通过的项记为通过。最终 manifest 与证据对应性检查覆盖五个真实 taskId、Git tree、输入四文件摘要、镜像 digest/platform、实际 Harbor API、构建源码/二进制摘要及容器回收。纯文档交付，不新增 Rust 行为测试、不运行全量测试；本任务构建验证不等于准确率评测。

### 最终新鲜证据

| 检查 | 结果 |
| --- | --- |
| 默认 features amd64 Cargo 构建 | 退出0；槽位1；源码摘要前后一致，锁已释放 |
| 冻结 Harbor 版本/CLI帮助/实际 API | 退出0，0.24.0，签名已动态核对 |
| 五题 Task 解析与有效 verifier 模式 | 5/5，shared |
| 冻结 Git tree / 文件 SHA256 | 5/5 tree、20/20 文件匹配 |
| 官方镜像 digest / amd64 平台 | 5/5 匹配 |
| 官方镜像内 Gateway health / build_id | 5/5 HTTP200，均匹配本次 ELF 摘要 |
| CLI帮助 / 停止 / 容器回收 | 5/5，退出0，无遗留任务容器 |
| 官方镜像无凭据模型端点 GET | 退出0，HTTP401；无模型调用 |
| `python3 evals/promptfoo/results/task25/verify-deliverables.py` | 退出0；上述内容与 manifest 对应校验通过 |

Docker 普通拉取遇到 TLS 超时/低速的三个镜像，改为读取官方 registry 的固定 manifest、逐 blob 分段下载并核对 SHA256，再原样 OCI 导入（3/3 `docker load` 退出0）；没有重建或替换上游层。OCI 导入后的身份用 Docker manifest-backed image ID 或 RepoDigest 对照固定 digest，不能仅依赖可变本地 tag。已终止被替代的慢速 pull 子进程，原始失败与成功日志保留。

有效环境验收为五题格式、五镜像启动，不是五次基准作答：模型调用、题目执行、verifier 执行均为0。未修改生产代码，未运行 Cargo clippy/全量测试（本次交付仅文档/清单，受授权的 Cargo build 用于工具链与入口取证）。
