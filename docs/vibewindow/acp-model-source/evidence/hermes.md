# Hermes 模型来源适配证据

固定版本：官方 PyPI `hermes-agent[acp]==0.19.0` 加本任务的最小 ACP 状态补丁，真实 `hermes --version` 和
ACP agentInfo 均为 0.19.0。仅验证 Darwin arm64，未推断 Linux/Windows。
独立版本与原生配置配方见 [hermes/manifest.json](hermes/manifest.json)。

官方 wheel SHA-256：
`bd0bac012aee38a60894781f4597dc29ee7bedb3448540249921f10d3bef327f`。
临时安装：`target/hermes-0.19.0-venv`（Python 3.12.16），未替换用户全局安装。
没有重写 Agent 或工具行为，也没有调用付费生产模型。

## 实现边界

`hermes::configure` 为 `session_model` 生成 0700 的 HERMES_HOME 与 0600 的
config.yaml、空 .env。主模型、18 类内置辅助任务和 delegation 全部显式指定
custom Provider、会话模型、Chat 中继 endpoint 与短期令牌；主 fallback_providers、
fallback_model 和各辅助 fallback_chain 清空。主模型输出上限来自当前 profile。

CUSTOM_BASE_URL 同样固定到中继，避免其环境优先级覆盖私有配置。令牌只在私有
配置文件中，基础命令保持 `acp`，追加 `--session-model-binding` 检查补丁能力；未打补丁的原版会拒绝该参数。显式冲突的隔离环境或额外 launch/profile/setup
参数在发布文件前拒绝。空 .env 还固定已安装项目 .env 的 fallback 加载语义。
`cli_local` 不调用此 driver，保留原命令、参数、环境和原生模型选择。

当前接入以 temperature 能力为前置条件，避免 Hermes 按模型名称施加采样参数时
绕过客户端能力边界。不支持该能力时明确拒绝，不切换本机账号。Chat 请求的
跨协议转换与能力检查由现有低层中继执行；本任务不声称验证其他原生传输。

Hermes 的机器管理配置和 .env 会在私有配置之后覆盖值。检测到
HERMES_MANAGED_DIR 或 `/etc/hermes` 时直接拒绝客户端模型来源；保留管理员
策略，不禁用、不覆盖它，也不静默使用其中账号。本地来源不受此 driver 限制。

所有内置辅助任务取自固定包 hermes_cli/config.py 的 DEFAULT_CONFIG.auxiliary；
agent/auxiliary_client.py 的显式 custom 路由和与主模型同 Provider 的失败检查
阻止自动发现其他账号。delegation 使用原生 `_resolve_delegation_credentials` /
`_build_child_agent`，没有替换工具或子代理实现。

## 外部模型 fixture 适配

[hermes/verify.py](hermes/verify.py) 复用任务 10 验收器的生产入口、鉴权、模型、
工具结果和生命周期判定，只扩展被模拟的外部模型服务：

- 第二次读取使用同内容的另一份真实临时文件。Hermes 对完全相同读取参数返回
  unchanged 占位，不能拿放宽判定或禁用原生保护替代真实第二次工具结果。
- `/api/v1/models`、`/api/tags`、`/v1/props`、`/props`、`/version` 是 Hermes
  的本地服务指纹 GET；明确返回 404，不计为模型请求。模型目录、生成和证据控制
  端点仍执行原鉴权；未认证模型请求仍增加失败计数。
- 原生配方在自有临时目录固定测试 context_length，避免将目录推断当模型证据。
  `session-model` 配方故意包含错误原生模型和凭据，实际请求必须走客户端中继。

[hermes/fixture_tests.py](hermes/fixture_tests.py) 的两项测试确认独立真实文件和
上述鉴权边界。没有改动并行共享的 fixture、profile、公共版本清单或分派。

## 固定版本状态补丁

[状态补丁](hermes/hermes-agent-0.19.0-session-state.patch) 只修改 ACP 状态路径、运行时绑定检查、
恢复时的传输选择和启动参数透传，不重写 Agent 或工具行为。原版将 state.db 放在
每轮都会撤销并删除的 HERMES_HOME；实际客户端来源只有首轮 3 次请求，之后恢复
未到达模型服务，取消等待超时。CLI 本地来源同期完成 8 次请求，因此没有归因为认证。

补丁将**仅托管来源**的历史放在系统临时目录下
`shiju-hermes-state-<uid>/<launch-reference digest>/state.db`。
既有 reference 包含 tenant/project/session/provider/model/generation，原生本地来源
没有该 marker，继续使用原 HERMES_HOME/state.db。目录检查所有者、0700 和非符号
链接；数据库与 WAL/SHM 拒绝不安全权限和链接。持久化仅保留原生历史和原有安全
metadata，不将 config.yaml、.env 或当前令牌搬入历史目录。

恢复只读取历史，Provider、模型、endpoint、凭据及协议重新从本轮私有配置解析，
不采用历史中的旧 loopback 地址。实际解析结果与私有模型/Provider/endpoint/key/协议冲突，
或解析失败时，托管来源立即报错，不执行原版的默认 Provider 回退。
[runtime_guard_probe.py](hermes/runtime_guard_probe.py) 用真实 CLI 的冲突 CUSTOM_BASE_URL
先复现接受错误路由，再验证会话在任何模型请求前拒绝；[结果](hermes/runtime-guard.json)
记录 generation_requests=0。CLI 仍在 profile 释放前关闭；令牌撤销与临时
凭据目录删除没有放宽。历史只验证了正常进程重启，系统临时目录被清理后的恢复
不在本次证据范围。

[patch.json](hermes/patch.json) 固定补丁及五个源码文件的原始/修改后 SHA-256。
[apply_patch.py](hermes/apply_patch.py) 对显式目录检查版本源码、路径和补丁摘要，
先 dry-run 再应用并回读，未知源码拒绝写入；重复应用先核对最终摘要。
已验证首次应用、重入及修改源码后的无写入拒绝。原版对新参数报 unrecognized
arguments；已补丁 CLI 的相同参数和 `--check` 通过。

```sh
python3 docs/vibewindow/acp-model-source/evidence/hermes/apply_patch.py --package-dir target/hermes-0.19.0-venv/lib/python3.12/site-packages
```

任务 33 发布 Hermes 能力前须纳入此固定版本补丁；仅安装原版 0.19.0 不够。
没有修改共享 profile、分派、安装器或公共 manifest。

## 验证记录

- 初始有效红灯：槽位 8，`cargo test -p vw-agent --lib --all-features acp_model_source_hermes -- --nocapture`，
  退出 101。命中 3 项具名测试，其中 private_routes_revoke_and_reissue 明确因原占位错误
  `ACP hermes session_model launch has not been verified` panic；随后进程在 panic 展开时
  SIGABRT。更早中断的编译和首轮零测试均不算红灯。
- 最终同一聚焦命令：槽位 2，退出 0，**5 passed / 0 failed / 1 ignored**。
  覆盖配置隔离、撤销重发、冲突环境与 argv、协议/采样能力拒绝、缺失凭据和禁用模型
  在生产请求入口提前拒绝。正向私有目录测试仅在 Unix 编译。
- 刚通过聚焦测试的同一二进制执行 ignored 测试
  `acp_model_source_hermes_real_auxiliary_and_delegation_use_profile`：退出 0，**1 passed**。
  [probe_auxiliary.py](hermes/probe_auxiliary.py) 导入真实固定包，完成 18 类辅助请求、
  原生 delegation 子 Agent 的一轮请求，以及 401、402 的错误传播，共 **21 次**
  `/v1/chat/completions`，全部使用 selected-model 和绑定令牌，没有回退账号。
  这项证据与完整 ACP 生命周期分开，不将 profile 字段检查冒充真实 CLI。
- `cargo fmt --all`：槽位 2，退出 0。
- `cargo clippy --all-targets --all-features -- -D warnings`：槽位 2，退出 0（2m 53s）。
- `python3 docs/vibewindow/acp-model-source/evidence/hermes/fixture_tests.py`：退出 0，2 项。
- `python3 docs/vibewindow/acp-model-source/evidence/hermes/state_tests.py --python target/hermes-0.19.0-venv/bin/python`：
  退出 0，4 项；真实补丁函数验证凭据 home 删除后状态保留、非法 reference、符号链接
  和其他用户可读文件的拒绝，全部只使用自有临时目录。

## 真实生产链与待统一正式回归

两种来源调用任务 10 原有生产测试入口及原有全部判定，使用上面刚通过测试的同一
生产二进制与真实 patched Hermes 子进程。只将 Cargo 的启动替换为该二进制执行，
避免其他计划持续修改源码触发重复全库编译。报告明确为
`cached-production-binary`，记录真实 command、binary_exit_code，不冒充正式 Cargo 验收。

| 来源 | 有效测试 | 二进制退出码 | 模型生成请求 | 模型与工具结果 |
| --- | --- | --- | --- | --- |
| [session-model](hermes/session-model-cached.json) | 1 | 0 | 11 | primary → switched，实际工具结果达到 2 |
| [cli-local](hermes/cli-local-cached.json) | 1 | 0 | 8 | 始终 primary，实际工具结果达到 2 |

两份结果均 passed=true、认证全部正确、fixture 错误数 0；新建、第二轮、取消、
重启恢复、切模、隔离及握手均为真。客户端来源的原生配置故意含错误模型和凭据，
真实请求未使用它们。状态补丁之前的客户端生命周期失败没有被覆盖成通过证据。

最终 Cargo 构建与 Clippy 阶段槽位 **2**：
`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`，
CARGO_BUILD_BUILD_DIR 为同一路径。使用 CARGO_BUILD_JOBS=1、RUST_MIN_STACK=33554432。
最终供应商复核在动态领取的槽位 **3** 执行：
`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3`。
从槽位 2 复制同一生产二进制至槽位 3 的独立文件，SHA-256 为
`b2b5a34a9ef24d9117959a4372fcd034e679964d07e93a80c1d0606c8e6e21ce`；
先对复制产物复核 5 项测试与原生辅助探测，再跑两种来源，全部退出 0。
此前恢复阶段使用过槽位 4（格式化）、8（红灯和首次绿灯），诊断重放曾动态领取
1、5（将槽位 8 的同一生产二进制按 SHA-256 校验后复制到当前槽位执行）。
自己的阶段进程均已结束、自有锁已释放，未清理后续领取同一槽位的其他任务锁，缓存保留。

状态为实现完成，待统一正式 Cargo 生命周期回归。按恢复队列要求不重复构建已通过
且未改动的代码；保留编号任务用于统一回归。领取共享槽位并固定两个绝对构建目录、
应用上述补丁后，执行下列原验收器封装；**不传 --test-binary 时内部执行原 Cargo 命令**：

```sh
export PATH="$PWD/target/hermes-0.19.0-venv/bin:$PATH"
cargo test -p vw-agent --lib --all-features acp_model_source_hermes -- --nocapture
python3 docs/vibewindow/acp-model-source/evidence/hermes/verify.py --agent hermes --mode session-model --manifest docs/vibewindow/acp-model-source/evidence/hermes/manifest.json --output target/acp-model-source-evidence/hermes/session-model-formal --offline-fixture
python3 docs/vibewindow/acp-model-source/evidence/hermes/verify.py --agent hermes --mode cli-local --manifest docs/vibewindow/acp-model-source/evidence/hermes/manifest.json --output target/acp-model-source-evidence/hermes/cli-local-formal --offline-fixture
```

两种来源各取得正式 Cargo 的 1 passed 后，再解除待回归并删除编号任务文件。
