# Goose 模型来源证据

状态：正式 Cargo 回归已完成。下方保留实现阶段的失败/缓存记录，与最新正式通过证据分开。
固定上游 aaif-goose/goose `v1.57.0`，入口保持 `goose acp`（stdio）。仅验证 Darwin arm64。

## 实现与固定补丁

`drivers/goose.rs::configure` 隔离 `GOOSE_PATH_ROOT`，设置原生 OpenAI Provider/完整模型 ID、
loopback HOST/path、短期中继 key，清空可冲突的自定义认证头。冲突配置、非 loopback 或非固定
ACP 参数明确拒绝；令牌不进入 argv。`cli_local` 不调用驱动，保留原生命令、参数、环境。

[v1.57.0-model-lock.patch](goose/v1.57.0-model-lock.patch) 只修改上游五个文件：

- `goose-cli/src/cli.rs`：托管启动显式要求 `--shiju-model-source`，未打补丁的 CLI 拒绝该参数；本地模式不启用模型锁。
- `goose/src/providers/init.rs`：托管进程拒绝通过 recipe/委派切到其他原生 Provider。
- `goose/src/model_config.rs`：主会话、recipe 与辅助模型使用所选模型，完整保留模型 ID。
- `goose-providers/src/openai.rs`：固定普通托管请求的 Chat 路线，在最终 wire payload 固定模型，覆盖辅助/恢复与 reasoning suffix 误识别。
- `goose/src/config/paths.rs`：只让原生 data/state 历史跨请求存活，config/key 仍随私有 profile 撤销。历史目录位于系统临时目录的 `shiju-goose-state-<uid>/<reference-digest>`；摘要来自已有租户/项目/会话/Provider/模型/代次引用。目录必须为当前 UID 所有、真实目录、0700，拒绝符号链接与错误权限。不同引用不混用历史。

没有替换 Agent、工具循环或审批，没有保留过期中继令牌。持久历史与秘密配置分离后，原生恢复
可在新进程、新私有配置目录及新短期 key 下继续。历史清理/安装接线由后续集成统一处理。

官方二进制包 SHA-256：`80d43c49b77302777e9d2fe32faf3c40c1028dafbea1e0a39b0378404a8a074b`。
源码归档：[官方 v1.57.0](https://codeload.github.com/aaif-goose/goose/tar.gz/refs/tags/v1.57.0)，
SHA-256：`36fcab524e25ca563830757bf2da8a68716ea2a5d7ca2f055dec4647b78548d9`。
最终补丁二进制 SHA-256：`758bb14d83ac54928b52ed2740870b5708d141d46bc9e7cbe2c2a08c146deccc`。
编译器 rustc 1.99.1。初次模型锁构建在槽位 4 退出 0（4m15s）；最终历史修复构建在槽位 2
退出 0（1m46s）。使用已有 `rustls-tls,bundled-mcp`，其他可选本地推理/AWS/语音特性未验收。
补丁为零上下文统一 diff（使用下述 patch 命令或 git apply --unidiff-zero），已在干净固定归档上成功应用。
已去除原生 workspace 格式化噪声；五个最小补丁结果经同一 rustfmt 归一化后与实际编译源码完全一致。

领取共享槽位并导出两个绝对构建目录后，复现原生构建：

```sh
patch -p1 -d target/acp-cli/goose-source/goose-1.57.0 < docs/vibewindow/acp-model-source/evidence/goose/v1.57.0-model-lock.patch
cd target/acp-cli/goose-source/goose-1.57.0
cargo build -p goose-cli --bin goose --no-default-features --features rustls-tls,bundled-mcp --locked
```

## Rust 与真实原生 CLI

- 红灯：`cargo test -p vw-agent --lib --all-features acp_model_source_goose -- --nocapture`，槽位 3，退出 101；命中 3 项，正向子进程用例命中原占位错误，另两项通过；随后系统 panic unwinder abort。不是编译失败/零测试。
- 最终同一命令：槽位 3，退出 0，**4 passed / 0 failed**（10m40s 构建，2.15s 执行）。
- `cargo fmt --all`：退出 0。
- `cargo clippy --all-targets --all-features -- -D warnings`：槽位 3，退出 0（55.72s）。

四项具名测试：

- `acp_model_source_goose_child_environment_is_private_and_revocable`
- `acp_model_source_goose_rejects_conflicting_overrides_without_secrets`
- `acp_model_source_goose_rejects_non_acp_command_and_non_loopback_relay`
- `acp_model_source_goose_missing_credentials_and_disabled_model_never_use_native_account`

包含真实子进程环境、0700 私有目录、argv 秘密检查、删除/撤销，以及真实 HTTP 中继的缺失
上游凭据 401、旧模型快照不能复活已禁用模型；不写真实用户配置目录。

独立原生探测：[native_probe.py](goose/native_probe.py)，仅 HTTP 模型服务被模拟，CLI、ACP、
实际 shell 读取、辅助 Agent 与会话数据库均为真实实现。

```sh
python3 docs/vibewindow/acp-model-source/evidence/goose/native_probe.py --binary target/acp-cli/goose-v1.57.0-shiju/goose --output target/goose-native-profile-renewal.json
```

[最终原生结果](goose/native-green.json)：退出 0，客户端来源 11 次、本地来源 9 次鉴权请求，
路径均为 `/v1/chat/completions`，错误数 0。两次读取、第二轮、recipe 冲突、辅助模型、取消、
恢复及切模均通过；客户端来源删除旧私有配置目录并更换 fixture key 后仍恢复成功。
客户端辅助请求固定 primary，本地辅助请求按显式参数使用 `forbidden-auxiliary-model`，证明未注入客户端模型锁。

模型锁代码在初版构建已取得[负向实测](goose/native-negative.json)：跨 Provider recipe 被拒，
模型请求 0；托管 key 为空但磁盘有有效本地 key 时，8 次请求被拒、鉴权成功请求 0，没有回退。
[完整 ID 检查](goose/literal-model.json) 收到 2 次 `selected-model-high` 请求，未截去后缀。
这些代码后续未变，未重复执行已通过检查。

## 原生产入口与正式 Cargo 回归的区别

任务 10 的 Rust 入口与成功条件保持不变。[verify.py](goose/verify.py) 只适配外部模型 fixture
的 Goose 原生 shell 工具：只发出 `cat --` 加安全引用的自有 challenge 文件，并支持原生模型目录
发现。原 Cargo runner 的进程清理保持不变，构建等待扩到 1200s，以容纳已观察到的 >10 分钟编译。
公共 fixture/profile/manifest/分派未改，独立清单为 [manifest.json](goose/manifest.json)。

实际标准结果原样保留：

- [session-model](goose/session-model-standard.json)：Cargo 101，实际执行 1 个测试，4 次正确鉴权请求及两次工具结果；历史目录修复前的第二轮/取消不到达模型服务，失败位置 `acceptance_tests.rs:90`。
- [cli-local](goose/cli-local-standard.json)：无关任务修改触发再次整库编译，依调度指示停止；没有记为通过或有效模型请求。

上述生命周期失败已通过原生五文件补丁修复。按后续明确指示，[cached_verify.py](goose/cached_verify.py)
仅将 Cargo 启动替换为同一已编译生产测试二进制，保留原入口、成功判据、真实 Goose 子进程和 fixture。
二进制编译于槽位 3，复制到本轮领取的槽位 2 执行，[摘要与来源](goose/cached-binary-identity.json)
明确记录；没有重新编译或声称该复制是 Cargo 构建。

| 来源 | 有效测试 | binary_exit_code | 实际模型请求 | 生命周期 |
| --- | ---: | ---: | ---: | --- |
| [session-model](goose/session-model-cached.json) | 1 | 0 | 11 | primary → switched，工具结果达到 2，全部通过 |
| [cli-local](goose/cli-local-cached.json) | 1 | 0 | 8 | 始终 primary，工具结果达到 2，全部通过 |

两份结果均 `verification_kind=cached-production-binary`、`passed=true`，认证全部正确、错误数 0；
新建、第二轮、取消、恢复、切换、隔离及握手全部为真。没有 `cargo_exit_code=0` 的伪称。
最终原生/缓存生产验证都在共享槽位 2，标准聚焦与 Clippy 在槽位 3，初次构建在槽位 4；
绝对 CARGO_TARGET_DIR 分别为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`、
`/Users/xiangmin/worker/code/shiju/target/rust-slot-3`、`/Users/xiangmin/worker/code/shiju/target/rust-slot-4`。
初轮仅预检用过槽位 5：`/Users/xiangmin/worker/code/shiju/target/rust-slot-5`。
Cargo 阶段的 CARGO_BUILD_BUILD_DIR 与 CARGO_TARGET_DIR 相同。所有本任务锁均已释放，保留缓存。

## 正式 Cargo 回归（已通过）

2026-11-04 基于最新共享工作区动态领取槽位 **5**，
`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-5`，
`CARGO_BUILD_BUILD_DIR` 为同一绝对路径。执行以下正式入口；没有使用 cached_verify.py：

```sh
export PATH="$PWD/target/acp-cli/goose-v1.57.0-shiju:$PATH"
cargo test -p vw-agent --lib --all-features acp_model_source_goose -- --nocapture
python3 docs/vibewindow/acp-model-source/evidence/goose/verify.py --agent goose --mode session-model --manifest docs/vibewindow/acp-model-source/evidence/goose/manifest.json --output target/acp-model-source-evidence/goose/session-model-final --offline-fixture
python3 docs/vibewindow/acp-model-source/evidence/goose/verify.py --agent goose --mode cli-local --manifest docs/vibewindow/acp-model-source/evidence/goose/manifest.json --output target/acp-model-source-evidence/goose/cli-local-final --offline-fixture
cargo clippy --all-targets --all-features -- -D warnings
```

本轮聚焦命令退出 0，4 passed / 0 failed（构建 4m00s、执行 1.72s）。共享 macOS 链接器
提示 __eh_frame 超过 compact unwind 索引体积上限；测试正常通过，没有修改该共享构建配置。
本任务 Rust 实现未变，已通过且不受影响的 Clippy 不重复执行，先前退出 0 的证据保留在上文。

| 正式来源 | 有效 Rust 验收测试 | cargo_exit_code | 实际模型请求 | 结果 |
| --- | ---: | ---: | ---: | --- |
| [session-model](goose/session-model-final.json) | 1 | 0 | 11 | primary → switched，全部通过 |
| [cli-local](goose/cli-local-final.json) | 1 | 0 | 8 | 始终 primary，全部通过 |

两份报告均由真正的 Cargo 命令运行原 Rust 生产验收入口，`passed=true`；认证全部正确，
fixture 错误为 0，工具结果达到 2；新建、第二轮、取消、恢复、切换、隔离、握手全部为真。
报告没有 binary_exit_code 或 cached-production-binary 标记。真实补丁 CLI 的 SHA-256 与上文一致。
全部 Cargo、验收器及 CLI 子进程结束后已释放本任务槽位锁，缓存保留。

已解除待回归并按契约删除编号任务文件。下一步由任务 33 集成固定补丁安装与共享能力发布。
