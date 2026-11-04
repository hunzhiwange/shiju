# iFlow 0.5.19 模型来源验收

## 固定版本与范围

- 平台：Darwin arm64，Node v26.13.0；不声明 Linux/Windows 已验证。
- npm：`@iflow-ai/iflow-cli@0.5.19`，实际隔离安装后的 `--version` 输出 `0.5.19`。
- tarball integrity：`sha512-cOQNLXJAjqcKq6XQpx47EUV8ocY5sdLXiy7MfJm+b1miRlMJmrQvueT75w1h9UqzrlFPio2VluTxbLRBoR/QBA==`，下载包已匹配 manifest。
- 包内含 Apache-2.0 LICENSE；补丁不提交或重新分发 vendor bundle。
- 官方配置文档：<https://github.com/iflow-ai/iflow-cli/blob/4642808afbc6580ac117d930f6c64ac0d84955c7/docs_en/configuration/settings.md>。

## 必需的固定版本补丁

原版 `IFLOW_HOME` 只控制 Storage 路径，用户设置仍使用 `homedir()/.iflow/settings.json`；插件缓存、市场、信任及 hooks 也有独立路径。ACP `authenticate` 会写回 selectedAuthType/apiKey/baseUrl/modelName。仅添加环境变量会污染本地设置。

`patches/acp-model-source/iflow/apply.py` 对两个原始 bundle 的 SHA-256 和每处上下文数量做校验，任何不匹配拒绝修改。补丁统一专用根，修复 hooks/插件/扩展路径，阻止 managed authentication 切回 OAuth，固定所有 Chat 请求（含压缩、子代理）及多模态 helper 的模型。`--shiju-model-source` 只由补丁识别；未补丁版本不得作为托管实现发布。

隔离安装及应用（全部位于 target；跳过 npm 生命周期脚本）：

```sh
npm install --prefix target/iflow-inspect/runtime --ignore-scripts --no-audit --no-fund @iflow-ai/iflow-cli@0.5.19
python3 patches/acp-model-source/iflow/apply.py target/iflow-inspect/runtime/node_modules/@iflow-ai/iflow-cli
```

## 配方与验证入口

本任务没有修改公共 manifest/安装器。任务 33 集成时须发布上述补丁并合并 `patches/acp-model-source/iflow/recipe.json`。

真实 iFlow 声明 `read_file(absolute_path)`。初始私有探针补充该字段后通过；任务 22 后续已将兼容集成到共享 fixture，因此私有 `verify.py` 直接复用当前共享 fixture，未保留兼容副本。它只在临时 manifest 中提供固定 CLI 的绝对路径及专用根配方，然后执行共享脚本和同一 Rust 生产入口，不替换 CLI 或本仓库逻辑、不修改公共 manifest。

在持有共享 Cargo 槽位并设置绝对 CARGO_TARGET_DIR 后：

```sh
python3 patches/acp-model-source/iflow/verify.py --cli target/iflow-inspect/runtime/node_modules/.bin/iflow --mode session-model --output target/acp-model-source-evidence/iflow/session-model
python3 patches/acp-model-source/iflow/verify.py --cli target/iflow-inspect/runtime/node_modules/.bin/iflow --mode cli-local --output target/acp-model-source-evidence/iflow/cli-local
```

原始任务命令也已执行，两种来源均退出 1：公共 manifest 尚无 reviewed offline recipe；不将其当作模型支持证据。

## 独立真实 CLI 探针

在 macOS sandbox-exec 中拒绝真实 `~/.iflow` 全部读写，网络仅允许 localhost；未替换 HOME。原始包因读取真实用户 settings.json 被拒绝而退出。最小路径修补仍出现插件/hooks 越界，完整路径补丁后 ACP 初始化成功，无越界诊断。

真实 managed CLI 已通过 initialize、session/new、session/prompt：fixture 收到 3 次 `/v1/chat/completions`，均为 `acp-fixture-primary` 且认证正确，工具结果数依次为 0/1/2；最终收到带随机 challenge 的回答，fixture errors=0。此探针是供应商 CLI 请求证据，不是 Rust profile 单元测试，也不替代下列完整来源验收。

cli_local 独立探针同样完成 3 次请求、两次读取及 challenge 回复，errors=0；无托管模型环境或启动标记。

历史处理：托管 ACP 历史按 tenant/project/session/source/driver 的 SHA-256 存在独立 0700 目录，文件 0600；路径不含模型、代次或凭据，因此换令牌、切模、销毁临时根后可恢复。配置及 OAuth 仍在临时根；写历史前拒绝包含当前 bearer 的内容，恢复重新读取当前认证。

## Rust 与完整生命周期结果

- 红灯：`cargo test -p vw-agent --lib --all-features acp_model_source_iflow -- --nocapture`，3 个具名测试，2 通过/1 失败，退出 101；失败为原始 driver 的 `launch has not been verified`，不是编译错误或零测试。
- 初次实现绿灯：同命令 3 通过/0 失败，退出 0。
- 历史轮换夹具曾误用带 reserved launch reference 的运行时配置；修正为原始 AcpAgentConfig 后，最终同命令 **4 通过/0 失败**，退出 0。
- `cargo fmt --all`：退出 0；未提交无关格式差异。
- `cargo clippy --all-targets --all-features -- -D warnings`：最终退出 0。
- 未知 bundle 修改后 patch 负向探针：退出 1，两个输入文件均保持原样。
- 正式生产入口曾因公共 `config::get()` 只读全局配置而在 acceptance_tests.rs:205 提前失败，两种来源各命中 1 个真实验收测试、退出 101、请求数 0；这是正式失败证据，不能当成通过。后由共同修复改为当前项目配置读取；本任务未改共享文件。

所有 Cargo 阶段从仓库根原子领取锁。实际槽位：

| 阶段 | 槽位 | CARGO_TARGET_DIR | 结果 |
| --- | --- | --- | --- |
| 原始脚本可用性检查 | 2 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-2` | 两命令退出 1，无 Cargo/模型请求 |
| 初次红灯尝试 | 4 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-4` | `.cargo/config.toml` 的 build-dir 导致等锁，编译前终止，不计红灯 |
| 有效红灯及历史隔离复验 | 1 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-1` | 同时设置 `CARGO_BUILD_BUILD_DIR` 为本槽，历史隔离聚焦 4/4、Clippy 0 |
| 所选上下文/输出上限最终复验 | 3 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-3` | 聚焦 4/4，Clippy 0 |

各阶段自己的占用记录均已删除、锁已释放，保留缓存；不删除后来其他任务持有的同名锁。

## 可复现的补充真实生命周期探针

```sh
python3 patches/acp-model-source/iflow/probe.py --cli target/iflow-inspect/runtime/node_modules/.bin/iflow --mode session-model --output target/iflow-inspect/lifecycle-session.json
python3 patches/acp-model-source/iflow/probe.py --cli target/iflow-inspect/runtime/node_modules/.bin/iflow --mode cli-local --output target/iflow-inspect/lifecycle-local.json
```

这是直接真实 CLI 证据，明确标记 `direct-real-cli-supplementary`；不冒充 Cargo 或 Rust 生产入口通过。每轮重建进程、托管配置根用后销毁，复用独立历史；覆盖 new/second_turn/cancel/restart/switch，拒绝真实用户配置读写和非 localhost 网络，检查主/切换模型、工具回合、认证及历史无 bearer。


两种补充生命周期命令均退出 0，各收到 **7 次**已认证 Chat 请求、errors=0，五个生命周期判定全为 true；托管来源最后一次请求使用 `acp-fixture-switched`，本地来源始终 `acp-fixture-primary`。脱敏观测见 `iflow/direct-session.json` 与 `iflow/direct-local.json`。

真实补丁 CLI 负向启动：缺 apiKey 退出 1；有托管锁但缺显式标记退出 1，诊断不含 sentinel 凭据。独立生命周期取消判定要求实际 `stopReason=cancelled`，不会把发送 cancel 或自然结束当成取消成功。

托管 initialize 的独立真实探针已验证仅声明 `openai-compatible`，不受父进程原生 iFlow/OAuth 凭据选取影响；真实用户配置和全部网络在该探针中被禁止。

配置同时覆盖 `IFLOW_tokensLimit` 与 `IFLOW_outputTokensLimit` 为所选模型实际上限；子进程行为测试验证 32768/4096 的传递，最终 4 个具名测试及 Clippy 仍全部通过（槽位 3）。

## 最终正式生产入口结果

共同配置读取修复后，托管来源正式 Cargo 验收通过；本地来源首次仍在 prompt 前失败，后共同修复了持久化中误拒绝 env 映射大写 key 的问题。重新运行后本地来源也通过；未修改 iFlow driver 或削弱验证以绕过该失败。

- `python3 patches/acp-model-source/iflow/verify.py --cli target/iflow-inspect/runtime/node_modules/.bin/iflow --mode session-model --output target/acp-model-source-evidence/iflow/session-model`：退出 **0**，内部原生产 Cargo 入口命中 **1** 测试、退出 **0**；**9** 次认证正确的 Chat 请求，含 primary/switched 两个模型，fixture errors=0。
- `python3 patches/acp-model-source/iflow/verify.py --cli target/iflow-inspect/runtime/node_modules/.bin/iflow --mode cli-local --output target/acp-model-source-evidence/iflow/cli-local`：最终退出 **0**，内部原生产 Cargo 入口命中 **1** 测试、退出 **0**；**7** 次认证正确的 Chat 请求，始终使用本地 primary 模型，fixture errors=0。
- 两者 new/second_turn/cancel/restart/switch/isolated/handshake 均为 true。CLI-local 不支持 session/resume，客户端明确回落到原生 session/load 并成功；没有账号或模型回退。
- 均为正式 Cargo 执行，**未使用 cached-production-binary**。最终真实验收实际槽位 **2**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`；`CARGO_BUILD_BUILD_DIR` 同路径。自己的锁已释放，缓存保留。
- 脱敏正式观测：`iflow/production-session-model.json`、`iflow/production-cli-local.json`。副本不含密钥、请求体或原始供应商输出。

结论：本任务当前范围已完成。公共 manifest、能力发布与补丁安装由任务 33 集成；完整 22 项统一回归由后续任务执行，不声明其他平台已通过。
