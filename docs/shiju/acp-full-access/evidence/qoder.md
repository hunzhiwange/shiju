# Qoder 权限接入

日期：2026-11-06；范围：Qoder ACP 会话访问权限。用户明确取消自动化测试和账号实机验收要求，本次按代码、官方文档及已取得协议字段接入；未运行测试，未请求账号、密码、BYOK 或受控上游。**权限工具执行、审批次数、运行中撤销及跨平台效果未实机验收，不宣称已验证原生免审批。**

## 来源与已取得协议字段

- [官方 ACP 文档](https://docs.qoder.com/cli/acp)说明 Default 与 Bypass Permissions 模式、客户端文件/终端能力和登录方式。
- 固定制品：[@qoder-ai/qodercli 1.1.83](https://registry.npmjs.org/@qoder-ai/qodercli/1.1.83)，macOS arm64、Node v26.13.0；入口 `node bundle/qodercli.js --acp`。tgz SHA256：`2399e81b8b0e04d0a28371cb4f4aa501c09d4eac81a02d6e51b89039ff80e628`；入口 JS SHA256：`66699ea0d17b4080445a8b22e628f150bbfbe137ee4acb57a4041fbc3a50f4d6`。
- 先前真实 initialize 响应：protocolVersion=1，agentInfo=`qoder-cli/1.1.83`，loadSession=true，sessionCapabilities.resume={}。这些字段表示公布能力，不能证明恢复效果。
- 先前 new 响应公布 configOptions：`id=mode`，值 `default/acceptEdits/auto/dontAsk/yolo`；同时提供 model 配置。启动 `--permission-mode bypass_permissions` 时回报当前值 `yolo`。CLI 参数是 snake_case，**ACP 完全访问值是 `yolo`**，不能使用 CLI 值或 Claude 的 `bypassPermissions` 代替。
- 先前探测未发送模型 prompt、未执行工具。空 HOME 下 authenticate 曾成功，其来源未证明与系统账号隔离；没有把该响应当作工具/权限验收，也未进一步使用该账号状态。本次未再次运行外部 CLI。npm 发布包为混淆 JS，无可审查官方权限源码仓库证明；不将文档或字段观察记为 E2/E3 权限验收。

## 生产行为

- `access_policy/qoder.rs` 解析 Qoder 的真实 `mode` 配置；`client/qoder_access.rs::sync_qoder_access` 接到共用同步入口，在 new/load/resume 返回后及 prompt 前应用当前授权。
- 同时存在公布的 `default` 与 `yolo` 时，full_access + ApproveAll 设置 `mode=yolo`；关闭或仅批准读取时设置 `mode=default`。每次调用沿用共用 set_config_option 超时、响应当前值确认和未确认控制屏障。拒绝响应不回退到其他协议猜测。
- 不再自动追加 Qoder 启动绕过参数，普通、本地和托管命令均经同一会话同步。显式启动 bypass 返回清晰错误并要求移除该参数，避免启动权限与会话授权脱节；正常 `--acp` 启动继续可用。
- 未公布原生配置的旧版或自定义入口保留普通客户端审批，不能宣称原生支持。若会话已公布高权限状态而没有可用 default 配置，阻止 prompt，并提示先恢复默认模式，避免关闭授权后沿用高权限历史。
- 计划模式在关闭完全访问时保留，开启完全访问不隐式替换计划。配置响应改变 model 时阻止继续执行。权限同步不修改账号、模型来源、全局配置或会话 ID。
- 显式高权限 mode 控制必须有当前 full_access + ApproveAll 授权；活动 turn 的 mode 控制先请求取消并返回重试错误，不能宣称立刻撤销。DenyAll 在 prompt 前直接拒绝 Qoder 原生工具执行，客户端审批逻辑继续保留拒绝优先级。
- `AcpClient::access_policy` 对 `mode=yolo` 仅报告 Partial / UnverifiedControls；协议设置回读不能代替第三方工具实测。未提供该配置时保留共享客户端分类，不杜撰已验证能力。

## 覆盖与限制

普通与托管入口复用同步逻辑；未运行这些入口的权限工具实测。现有 session_model 账号/权益拒绝保持原状；权限接入不添加认证要求、不修改已有模型来源门控。Linux/Windows 未实机验收；文件、终端、原生沙箱、企业 deny、历史会话恢复、工具/提示词是否发生其他变化均未实机验收。供应商硬拒绝与登录要求仍由原有流程处理。

## 静态检查

本次按用户要求不运行自动化测试或实机验收，有效行为测试数不适用。执行 `cargo fmt --all`，仅保留本任务格式差异；执行仓库要求的 `cargo clippy --all-targets --all-features -- -D warnings`（仅编译与静态检查，不执行测试）。实际 Cargo 共享槽位、退出码及结果见下方记录。

- 最终结果：`cargo fmt --all` 退出 0；`cargo clippy --all-targets --all-features -- -D warnings` 退出 0；`git diff --check` 退出 0。首次静态编译发现原启动测试的空列表类型推断问题，已补充类型后重新检查通过；未执行任何测试。
- 共享槽位：两次静态检查阶段均动态领取槽位 2，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`；每阶段结束释放自己持有的锁，保留缓存。日志位于 `/tmp/qoder9-fmt.log` 与 `/tmp/qoder9-clippy.log`（临时日志不保证长期保留）。
- 按用户更新的完成标准，本任务代码接入已完成；不因缺少实机证据阻塞，不将静态检查表述为工具权限验收。建议下一步直接在现有正常登录环境试用会话完全访问开关。
