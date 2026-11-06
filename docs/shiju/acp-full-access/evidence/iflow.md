# iFlow 完全访问接入

日期：2026-11-06；依赖批次 13 提交 `fce74e3b8` 已存在。
按用户最新要求不新增或运行测试，不申请账号、密码、BYOK 或受控上游。
有效测试数 0；实机未测不作为禁用正常入口的理由。

## 固定发行源码（E2）

从 [官方 npm 0.5.19 元数据](https://registry.npmjs.org/@iflow-ai/iflow-cli/0.5.19)
读取并下载 [官方发行包](https://registry.npmjs.org/@iflow-ai/iflow-cli/-/iflow-cli-0.5.19.tgz)，
包 SHA256：`4ac4ec1d6e52362c5794bb07213886bafb554d4036752b8af8c8ca08462ab365`。
只读取 package.json 与 bundle/iflow.js，未安装或执行发行包。
普通/托管入口继续使用 `iflow --experimental-acp`，既有 session_model 私有配置及补丁不改。
本次源码核对在 macOS arm64 完成，不构成其他平台执行证据。

[官方入口](https://cli.iflow.cn/)描述默认 YOLO；
[官方配置文档](https://docs.iflow.cn/cli/configuration/settings/)列出 approvalMode。
这些 CLI 描述不替代 ACP 行为核对。固定发行 bundle 的 ACP 实现确认：

- `getAvailableModes` 公布 smart、yolo、default、plan；分别为智能审批、免确认、普通确认与只读计划。
- `setMode` 校验 sessionId 与上述四个精确值，通过会话 config.setApprovalMode 修改内存权限；不保存用户设置、不切换模型、工具或系统提示词。遥测记录模式名，未发现此 setter 写入全局配置。
- set_mode 返回 success 与 currentModeId 扩展字段；没有标准 CurrentModeUpdate。共享 Rust 协议只采纳成功应答，不能据此伪造标准 modes 回读或宣称工具已验收。
- new 返回 modes；load 重新构造会话配置和历史，仅返回 sessionId。没有标准 configOptions/set_config_option 或 resume 路由；现有 resume 的同 ID load 回退继续沿用。
- ACP 默认配置可落入 YOLO，因此关闭完全访问时也必须主动发送 default，不能只停止客户端自动批准。

## 生产接入（E1）

`access_policy::iflow` 保存精确值；`AcpClient::sync_iflow_access` 接入共享
new/load/resume 同步点及每次 prompt 前屏障。
当前 `full_access && ApproveAll` 请求 yolo；关闭或 ApproveReads 请求 default。
DenyAll 在 prompt 入口拒绝原生执行；手动 smart/yolo 等非受限 mode 要求当前完全访问授权。
显式 plan 保留，避免把计划任务隐式改成工具执行。客户端单独保存已应答的 mode 意图，
不冒充标准模式回读；new/load/resume 清除旧意图并重新按当前授权同步。

复用 `confirmed_mode_control` 的超时、错误及 pending_controls 屏障。
有 modes 时必须公布目标值；唯独 iFlow load 无 modes 时允许依据固定源码发送
精确 default/yolo，不将这个兼容规则扩散到其他后端或任意 mode。
控制失败保持屏障并明确报错，不能在权限未知时继续 prompt。
每轮重新同步，避免仅依赖初始模式。运行中手动模式切换先取消并等待当前轮完成。
应用授权切换沿用包含 fullAccess/permissionMode 的客户端缓存身份与旧进程停止机制，
在加载原 sessionId 后使用当前授权；不承诺瞬时撤销已经启动的工具。

分类为部分支持 / UnverifiedControls，不标记 VerifiedNativeControl。
普通/托管、cli_local/session_model 共用该 ACP 客户端路径；既有模型来源、平台与认证门控保留。
标准 request_permission、客户端文件和终端仍沿共享当前授权处理；供应商硬拒绝不被覆盖。
本接入不写全局设置、不注入启动 YOLO 参数、不解除原生沙箱，不取消计划退出或用户问答。
发行源码还存在独立子代理配置，不将主会话 mode 外推为所有子代理、企业 deny 或沙箱的完整保证。

## 验证边界

未运行第三方 CLI，没有实际 initialize/new/load/resume/config/mode 捕获报文、
审批次数、文件/终端结果、false→true→false 执行验收或跨平台证据。
上述协议摘要来自固定发行源码，不是实际报文。未运行测试（用户明确要求），有效测试数 0。

静态检查（编译通过不等同权限执行已实机验证）：

- 动态领取槽位 1 完成初次检查，后续最终检查使用动态领取槽位 2。
- 最终 `CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`；
  初次为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。
- `cargo fmt --all`：退出码 0；仅保留本任务差异，恢复无关格式化内容。
- `cargo clippy --all-targets --all-features -- -D warnings`：最终退出码 0，7.95s。
- `git diff --check -- crates/vw-acp docs/shiju/acp-full-access`：退出码 0。
- 未执行测试，features 不适用，有效测试数 0（用户明确跳过）。
- 本任务 Cargo 子进程全部结束后删除自己的 owner 及槽位锁，保留编译缓存。

审查范围仅为 iFlow 模式同步、当前授权保护、load 兼容、能力分类与证据矩阵。
既有其他工作区变更未纳入本任务提交。
