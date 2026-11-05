# 固定版本 Vibe 会话绑定补丁

基线：官方 PyPI `mistral-vibe==3.0.4`。仅在独立 virtualenv 内应用，原始文件 SHA256
由 `upstream-hashes.json` 固定；应用脚本先验证全部基线，再 dry-run 后执行补丁。
补丁为零上下文差异，使用提供的哈希校验应用脚本，不能省略基线检查。
本次真实验收使用 macOS arm64 wheel。Linux/Windows 未验收，不能推断通过。

```sh
target/vibe-tools/bin/uv venv --python 3.12 target/vibe-cli-env
target/vibe-tools/bin/uv pip install --python target/vibe-cli-env/bin/python 'mistral-vibe==3.0.4' --only-binary :all:
target/vibe-cli-env/bin/python docs/vibewindow/acp-model-source/evidence/vibe/apply_patch.py
target/vibe-cli-env/bin/python docs/vibewindow/acp-model-source/evidence/vibe/upstream_tests.py
target/vibe-cli-env/bin/python docs/vibewindow/acp-model-source/evidence/vibe/native_probe.py --mode session-model
target/vibe-cli-env/bin/python docs/vibewindow/acp-model-source/evidence/vibe/native_probe.py --mode cli-local
```

补丁仅新增 `--session-model-config` ACP 参数，以及每次配置合并之后、schema 校验之前
执行的模型绑定投影。原版 CLI 会因不认识该参数拒绝启动；不能静默忽略绑定。
不传参数时保留原生行为。配置必须是当前 VIBE_HOME 下、当前用户所有、无组及其他用户权限的普通文件，
Provider 只能指向带端口的 HTTP loopback，密钥仅取专用短期中继环境变量。

投影仅固定 providers/models/active_model/allowed_models、压缩模型、标题模型与
smart-approve 辅助模型，并清空视觉回退。工具、Agent 类型、权限、MCP 及系统提示保持
上游流程。组织管理配置涉及模型字段时显式拒绝，不能通过此补丁绕过管理策略。
Vibe profile 仍可改变非模型设置。所有重建及子 Agent 使用同一 builder 约束。

当前共享 profile 没有图片能力元数据，driver 保守声明文本模型并禁止视觉回退；
不谎报图片支持。Vibe 原生支持 Chat、Responses、Anthropic；Google 使用现有 Chat
中继转换，未将 Google 原生协议列为 Vibe 证据。Vibe 会发送 temperature，不支持
该参数的所选模型在启动前拒绝。任务 33 发布能力前须将该固定版本补丁纳入实际分发。

两种来源的正式生产验收使用此目录的独立 manifest，不修改公共版本清单。所有 Cargo
及其脚本须先按任务文件领取共享槽位并设置绝对 `CARGO_TARGET_DIR`。

## 每轮凭据撤销与原生历史

生产 profile 每轮都会撤销令牌并删除临时 `VIBE_HOME`。补丁使用已有
`VW_ACP_LAUNCH_REFERENCE` 的严格 64 位十六进制摘要，将原生 session store 放到
`<系统临时目录>/shiju-vibe-state-<uid>/<绑定摘要>/sessions`。所有目录验证当前用户
所有、0700、非符号链接；摘要沿用既有 tenant/project/session/provider/model/generation
身份计算，不含令牌。MCP 描述缓存使用 save_dir 的父目录，因此额外的 `sessions`
层确保其仍位于同一绑定摘要内；Vibe 自身的插件缓存也保持在此作用域。

模型配置和凭据仍只存在于本轮 `VIBE_HOME` 与进程环境，drop 时撤销；保留的是原生
历史及相关缓存，不是账号配置。日志被显式禁用或组织强制日志配置时拒绝该托管恢复路径，
不静默启用或绕过策略。`cli_local` 不传绑定参数，完全不进入此路径。

该目录属于当前主机的私有临时缓存；系统清理它之后，原生历史不再存在，不能承诺跨
系统清理的原生恢复。任务 33 统一生命周期时应使用相同隔离规则处理新建/重放，不得
复用其他来源的锚点。真实生命周期探针验证了轮换私有目录后的 load，并逐文件检查
会话状态中没有本次短期令牌；探针退出时只清理自己的摘要目录。
