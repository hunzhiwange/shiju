# Copilot 1.0.105 模型来源证据

2026-11-04，Darwin arm64；真实 npm `@github/copilot` / `@github/copilot-darwin-arm64` 1.0.105。Linux、Windows 未验收。任务 9 的 profile/隔离边界和任务 10 的生产验收入口已存在；本任务使用这些真实实现，没有模拟仓库逻辑或厂商 CLI。

## 实现

`drivers/copilot.rs::configure`：独立 `COPILOT_HOME`、显式 `COPILOT_PROVIDERS_CONFIG`、`COPILOT_MODEL=vibewindow/<selected>`；私有 providers.json 仅持有可撤销中继令牌。禁止 GitHub token、原生 Provider 环境覆盖及自定义托管 argv；不把令牌放 argv。启用原生离线模式并关闭更新。凭据缺失、禁用模型由已有生产入口在签发 profile 前拒绝。cli_local 绕过 driver，保留原命令、参数与环境。

`preserve_history`：仅将 session-state 链接到以会话身份摘要隔离、所有者和 0700 权限均验证的历史目录。每轮 Provider 文件和令牌随 profile 销毁；历史保持可恢复。来源/模型身份不同不会共享历史目录。独立真实探针扫描历史文件，未发现本次测试 Provider key；日志保留在短期目录中。

Chat 已完成真实验收；Messages 配置分支尚无独立真实验收，不据此发布 Messages 能力。拒绝未经验证的推理模型、Responses 及其他协议；要求工具和 temperature 能力。专用 fixture 通过真实 task 工具启动 general-purpose 子代理并要求两次 view；主/辅助请求均受同一模型绑定约束，不允许回退 GitHub 账号。该证据不代表穷尽所有特殊 agent/模型选项。

## 测试驱动与检查

- 红灯：`cargo test -p vw-agent --lib --all-features acp_model_source_copilot -- --nocapture`，1 个具名测试，退出 101，原因是旧 configure 明确拒绝尚未验证的入口。
- 最终同命令：2 passed / 0 failed，退出 0。用例 `acp_model_source_copilot_private_byok_launch_and_revocation`、`acp_model_source_copilot_conflicting_account_environment_is_rejected` 覆盖配置/argv、撤销、历史跨 profile 重建及账号冲突拒绝。
- `cargo fmt --all`：退出 0。无关变更未纳入本提交。
- `cargo clippy --all-targets --all-features -- -D warnings`：初版及最终历史分离版本均退出 0。

红灯槽位 2；首轮通过/Clippy 槽位 3；最终聚焦与生产验收槽位 2。绝对路径分别为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`、`/Users/xiangmin/worker/code/shiju/target/rust-slot-3`，均动态原子领取。最终阶段同时设置 `CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 到所持槽位，阶段退出清理自己的锁并保留缓存。

## 真实生产入口验收

复用任务 10 `scripts/verify_acp_model_source.py`；独立 runner 只补充外部模型 fixture 的 Copilot `view(path)` 工具回合，以及末尾句点版本输出的精确匹配，不更改生产入口或通过判定。固定原生配方见 [cli-versions.json](copilot/cli-versions.json)。真实固定版本除 env 入口外也支持 providers.json；本地默认模型需写 settings.json，不能沿用旧 config.json。

```bash
python3 docs/vibewindow/acp-model-source/evidence/copilot/verify.py --agent copilot --mode session-model --manifest docs/vibewindow/acp-model-source/evidence/copilot/cli-versions.json --output target/acp-model-source-evidence/copilot/session-model --offline-fixture
python3 docs/vibewindow/acp-model-source/evidence/copilot/verify.py --agent copilot --mode cli-local --manifest docs/vibewindow/acp-model-source/evidence/copilot/cli-versions.json --output target/acp-model-source-evidence/copilot/cli-local --offline-fixture
```

两命令均退出 0，内部各运行 1 个正式 Cargo 生产验收测试，Cargo 退出 0；未使用 cached-production-binary。

- [session-model.json](copilot/session-model.json)：15 个认证成功的 `/v1/chat/completions` 请求，实际模型 acp-fixture-primary、acp-fixture-switched；其中 6 个为辅助请求，覆盖切模前后的两个模型；工具结果达到 2；new、second_turn、cancel、restart、switch、isolated、handshake 全部 true。原生配置故意使用冲突模型/凭据，未被使用。
- [cli-local.json](copilot/cli-local.json)：10 个认证成功的请求（含 3 个辅助请求），实际模型始终 acp-fixture-primary；上述生命周期项全部 true，客户端切模不覆盖原生模型。
- [session-model-before-history.json](copilot/session-model-before-history.json)：修复前首轮成功、第二轮失败、取消无新请求，是分离历史目录的真实失败证据。
- [native-probe.json](copilot/native-probe.json)：独立原生探针，不混同生产入口证据。

共享 registry、能力发布和公共 manifest 未修改，留任务 33 集成；本文件不将单平台 Chat 证据扩大为跨平台/全协议承诺。

配置契约参考：[GitHub 官方 BYOK 文档](https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-llm-models)；以本任务固定版本实测为准。
