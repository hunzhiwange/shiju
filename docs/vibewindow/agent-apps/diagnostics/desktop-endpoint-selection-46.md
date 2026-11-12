# 任务 46：桌面端点测速与选择验证

2026-11-12，Asia/Shanghai；macOS 27.0 / arm64。直接在原仓库执行，无新工作树。来源 cc-switch HEAD 已核对为 `ca58c0313360eae7c851fbd7553c1614c7af03e7`，上游及只读 plan.md 未修改。

## 生产接线

- 复用既有应用供应商管理 → 编辑表单 → **端点测速与选择**。`message/agent_apps_diagnostics/endpoints.rs::update` 调用任务 41 的 `diagnostic_endpoint_candidates` 和任务 39 的 start/status/cancel SDK；无平行导航。
- `endpoint_state.rs::query` 提取 Claude、Codex、Gemini 当前草稿；候选查询合并草稿、已保存配置、可选预设和自定义地址簿，草稿置于 POST 正文。来源、HTTP 状态、耗时及失败项逐项保留，成功项按耗时排序。
- `State::choose` 仅选中候选；`State::apply` 仅回填已有编辑草稿。最终写配置走原有 Claude/Codex/Gemini 保存函数及原 SDK。地址簿直接打开母任务 4 现有表单，返回后刷新候选并保留原供应商草稿。
- 取消测速保留编辑状态；取消编辑等待已提交任务清理确认。提交尚未返回时关闭，也会在取得 ID 后取消。上下文、访问代次、编辑代次、请求序号、任务身份及结果端点均校验，迟到成功不能覆盖取消终态。
- 原 Speed 执行端仅重收集已保存候选，不能执行草稿/编辑器预设。按突破授权，在现有 `DiagnosticJobs` 内补 `remember_endpoint_candidates` / `resolve_endpoint_candidates`，仅保留验证后的短期引用，仍复用原执行器。最多 128 组、每组 32 项、600 秒过期；按租户、项目、主机、应用、供应商、主体隔离；启动重新校验原权限和供应商存在性，不新增权限配置或端点持久存储。
- Codex 定向改当前选中表的 URL，保留其他供应商、字段和注释。生产所需 `toml_edit 0.22.27` 复用仓库已锁定版本，Cargo.lock 仅新增桌面的直接依赖关系；原 `toml` 仅为 dev-dependency，且文档重序列化不能保留注释。
- 弹窗复用 system_settings_common 控件、固定标题/底栏、4px 滚动条及两侧 7px 留白，反馈使用 toast；中文及动态 URL 使用默认字体。已对照 custom_model_modal，并以 tiny-skia 实际布局、绘制及截图路径检查暗色/亮色主题、中文和长 URL。

## 命令与有效证据

桌面生产模块位于库 crate，旧卡 `--bin shi-ju` 已过时；本次采用真实 `--lib` 注册，不复制业务模块。

| 检查 | 确切命令 | 退出码 / 有效数量 |
| --- | --- | --- |
| 行为红灯 | `cargo test -p vw-desktop --lib diagnostic_desktop_endpoint_selection_requires_save -- --nocapture` | 101；0 passed / 1 failed / 9410 filtered。编译成功后真实 App::update 断言缺少测速选择草稿状态，非编译或夹具失败。 |
| 桌面最终绿灯 | `cargo test -p vw-desktop --lib diagnostic_desktop_endpoint_selection_requires_save -- --nocapture` | 0；2 passed / 0 failed / 9410 filtered，1.45 秒。 |
| 后端最终绿灯 | `cargo test -p vw-agent --lib diagnostic_endpoint_ -- --nocapture` | 0；2 passed / 0 failed / 12419 filtered，0.07 秒。 |
| 格式化 | `cargo fmt --all -- --emit stdout --config skip_children=true` | 0；只用 rustfmt 回写本任务文件，未回写其他会话格式化。 |
| 全仓 Clippy | `cargo clippy --all-targets --all-features -- -D warnings` | 0；4m 14s，包含生产桌面依赖声明及所有目标。后续聚焦测试源码未再改变，复用此证据。 |

- 桌面用例：`diagnostic_desktop_endpoint_selection_requires_save`、`diagnostic_desktop_endpoint_selection_requires_save_other_forms`。受控本地真实 HTTP 验证查询/启动/状态/取消/地址簿/保存的 SDK 路径和正文、选择及回填不发保存请求、最终唯一显式保存携带所选 URL、保留密钥且默认不应用；另检查取消确认、迟到回调、预设与草稿合并、TOML 注释和其他字段保留，以及实际离屏渲染。
- 后端用例：`diagnostic_endpoint_reference_is_scoped_bounded_and_expires`、`diagnostic_endpoint_draft_reference_executes_without_save`。真实隔离 GlobalDatabase 回读前后供应商记录一致；候选查询 200 且未发探测，另一已授权主体消费草稿引用返回 400，撤销权限后启动返回 403，恢复原授权后启动返回 202、读取 200 至完成。实际本地上游计数 **2 次 GET**（热身一次、计时一次），返回 HTTP 401 并保留耗时证据；服务 abort 后 await 确认退出。这是受控 HTTP 证据，不代表外部供应商或模型鉴权通过。
- 中间补测发现并修正了 Codex 文档解析；HTTP 夹具最初短令牌被真实授权层拒绝，改用符合原合同的令牌后重新通过，不把夹具失败计为行为红灯。
- 测试链接器输出与初始红灯相同的 `__eh_frame` 体积提示；测试退出 0，未改变仓库 Cargo profile 或降低 Clippy 要求。

## 资源与交接

所有 Cargo 阶段均原子动态领取 **槽位 2**：

`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`

`CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`

命令环境统一为 `CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_CODEGEN_UNITS=256 CARGO_PROFILE_TEST_LTO=off CARGO_PROFILE_TEST_OPT_LEVEL=0`。各阶段结束释放自己的锁，缓存保留。日志保留在 `/tmp/task46-red.log`、`/tmp/task46-desktop-retry.log`、`/tmp/task46-agent-retry.log`、`/tmp/task46-clippy.log`。

已审查本任务差异与空白检查；共享 API 中其他会话的 read_job 可见性改动不暂存，保留其他会话修改与暂存。原生窗口由用户按母计划人工验收：暗色主题、中文长 URL、取消与明确保存。
