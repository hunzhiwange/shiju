# Qwen Code 0.26.7 模型来源验收

验证平台：Darwin arm64、Node v26.13.0；不声明 Linux/Windows 已通过。

## 固定版本与实现

从官方 npm 获取 `@qwen-code/qwen-code@0.26.7` 到 `target/qwen-source-review/package`，执行真实 `cli-entry.js --version` 得到 `0.26.7`。归档 SHA-512 与公共 manifest 中的 integrity 完全一致；源码基线为 `f45c34d8ccbd651f2f0f4d97c9b013c11af52f5b`，许可证 Apache-2.0。

- 官方配置文档：[Model Providers](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/model-providers/)。固定版 `packages/cli/src/config/settings.ts` 确认 system overrides 最后合并；`packages/core/src/config/storage.ts` 确认 QWEN_HOME 与 QWEN_RUNTIME_DIR 独立。
- `drivers/qwen.rs::configure` 创建随机会话私有模型别名，使用独立 0700 home 与按租户、项目、会话、真实模型隔离的 0700 runtime；通过 OpenAI generation config 的 extra_body.model 还原真实模型。短期中继令牌仅进入环境，不进入 argv。来源为 cli_local 时沿用原命令、参数和原生配置，不调用此 driver。
- 保存系统默认配置路径，保留管理员非路由设置；管理员路由冲突、用户模型环境冲突和非标准 ACP/resume 参数直接拒绝。未改工具授权或调用流程。
- 原版仍会搜索旧 ~/.qwen/.env、~/.env；补丁仅在显式 managed 标记下禁止环境文件覆盖，在内容生成器统一固定主/辅助模型的 OpenAI 路由和凭据，并锁定认证刷新后的配置快照、拒绝 OAuth 认证切换。cli_local 不启用该分支。
- `patches/acp-model-source/qwen/apply.py` 先检查包身份、四个原始文件 SHA-256 和每处上下文数量，再应用补丁；不提交或重新分发 vendor bundle。

隔离安装示例（不写全局安装目录）：

```sh
curl -L --fail https://registry.npmjs.org/@qwen-code/qwen-code/-/qwen-code-0.26.7.tgz -o target/qwen-source-review/qwen.tgz
tar -xzf target/qwen-source-review/qwen.tgz -C target/qwen-source-review
python3 patches/acp-model-source/qwen/apply.py target/qwen-source-review/package
```

## 验证结果

Cargo 使用动态领取的共享槽位 4，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-4`。由于仓库另设 build-dir，所有验证同时设置 `CARGO_BUILD_BUILD_DIR` 为同一绝对路径。锁在阶段结束后释放，保留编译缓存。补充共享路由负向验证重新动态领取槽位 2，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`，同样已释放。最终 CLI 配置快照补丁的两来源复验动态领取槽位 3，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3`，复验完成后已释放。

| 检查 | 命令/入口 | 结果 |
| --- | --- | --- |
| 格式化 | `cargo fmt --all` | 退出 0 |
| 有效红灯 | `cargo test -p vw-agent --lib --all-features acp_model_source_qwen -- --nocapture` | 退出 101；命中 1 项，原 driver 的未验证拒绝使 private_launch_revokes_credentials 失败 |
| 聚焦绿灯 | 同上 | 退出 0；3 passed，0 failed |
| 托管来源 | `qwen/verify.py --mode session-model` | 退出 0；1 项真实生产验收测试，10 次 Chat 请求 |
| 本地来源 | `qwen/verify.py --mode cli-local` | 退出 0；1 项真实生产验收测试，15 次 Chat 请求 |
| 原生 helper | `qwen/probe.py --mode session-model --auxiliary` | 退出 0；8 次请求，包含 auxiliary=true 与两次真实文件读取 |
| 缺失中继凭据 | 真实 CLI managed 启动，省略 OPENAI_API_KEY | 退出 1；在认证前明确拒绝 |
| 共享来源负向检查 | `cargo test -p vw-agent --lib --all-features acp_model_source_routing -- --nocapture` | 退出 0；10 passed，含缺失凭据、禁用模型与 cli_local 无客户端凭据路径 |
| 补丁完整性 | 原始文件应用成功；修改后来源再次应用 | 退出分别 0/1，拒绝时没有部分写入 |
| 全仓 Clippy | `cargo clippy --all-targets --all-features -- -D warnings` | 退出 0 |

三项 driver 测试为 `acp_model_source_qwen_private_launch_revokes_credentials`、`acp_model_source_qwen_conflicts_fail_before_config_publication`、`acp_model_source_qwen_native_resume_arguments_are_rejected`，覆盖子进程环境、argv 无令牌、home 清理、令牌撤销、路由/环境冲突与原生恢复参数拒绝。

首次尝试被并行 Pi/OpenCode 测试的 TenantScope 类型错误阻断；不计为红灯，没有修改它们。对应任务修复后，上述红绿灯与 Clippy 均实际运行通过。配置快照补丁复验期间，还遇到并行新增 agent_profile_acceptance_tests.rs:368 的编译错误（有效测试 0，不计行为失败）；本地来源随后通过，托管来源单独重试后通过。测试链接器提示现有巨大 __eh_frame；没有 Rust/Clippy 警告被豁免。

## 真实请求证据与复现

已保存 [session-model.json](qwen/session-model.json)、[cli-local.json](qwen/cli-local.json)、[auxiliary.json](qwen/auxiliary.json)、[missing-key.json](qwen/missing-key.json)、[patch-integrity.json](qwen/patch-integrity.json)。前两项均使用共享脚本和生产 do_stream_request_acp 入口；新建、续轮、取消、进程恢复、切模与隔离均为 true。所有请求均到 `/v1/chat/completions`，认证正确、fixture errors=0；托管来源出现 `acp-fixture-primary` 和 `acp-fixture-switched`，本地来源始终为其原生配置的 primary。

辅助探针在 macOS sandbox-exec 中禁止真实 ~/.qwen 和 ~/.env 读写，网络仅允许 localhost；原生 helper 确实向同一所选模型发请求，而不是用假 CLI 替代。探针还尝试 ACP authenticate(qwen-oauth)，确认托管认证锁拒绝切换。辅助探针只为验收显式显示原生工具目录，不改变 driver 的工具发现或权限设置。

Qwen 会去重对未变化文件的重复读取，故独立验收包装器准备两个含相同随机 challenge 的文件。`verify.py` 仅补充私有 manifest 配方、真实可执行路径、冲突项目设置及第二个 challenge 文件；不模拟本仓库逻辑，不修改共享脚本。冲突项目设置包含 OAuth、同真实模型名的 modelProviders 和本地凭据，未造成借用或回退。

持有共享槽位后执行：

```sh
python3 patches/acp-model-source/qwen/verify.py --cli target/qwen-source-review/bin/qwen --mode session-model --output target/acp-model-source-evidence/qwen/session-model-reviewed.json
python3 patches/acp-model-source/qwen/verify.py --cli target/qwen-source-review/bin/qwen --mode cli-local --output target/acp-model-source-evidence/qwen/cli-local-reviewed.json
python3 patches/acp-model-source/qwen/probe.py --cli target/qwen-source-review/bin/qwen --mode session-model --auxiliary --output target/qwen-source-review/auxiliary.json
```

`bin/qwen` 是指向 `../package/cli-entry.js` 的符号链接；它不包装或替代 CLI。原任务两条公共 manifest 命令也已分别运行，退出 1，原因为公共 manifest 尚无 reviewed offline recipe；这些 unavailable 报告不是成功证据。上面的私有包装器提供本任务禁止并行修改的公共配方，调用同一共享验证脚本。

## 下一步

任务 33 串行集成该固定版本补丁、recipe.json、双文件 challenge 兼容与能力发布；本任务未修改 registry、安装器、共享 manifest 或只读 plan.md。其他平台须独立执行真实 CLI 验收。
