# 任务30：file_read

代码完成，编译/行为未验证。本轮禁止 Cargo、构建、测试、实机验证；仅做静态差异检查。

## 职责与接线

生产实现此前已在 tool-files，本轮生产迁移收益为0。宿主 mod.rs 保留5行类型特化及真实宿主测试挂载，tools/mod.rs 的 pub mod 和 re-export 仍有工厂真实调用者，不能删除。工具工厂继续通过 FileReadTool<AgentContextHost> 调用 tool_files::file_read::FileReadTool。领域测试从本库 lib.rs 自注册，不反向依赖 vw-agent，不使用 path/include 加载宿主。

25条完整领域用例迁入 host_migration_30_tests；所有函数体经去空白比较与来源一致，断言未删。FileReadHost 仅提供无活动 session 的 task-local 与无技能目录端口；读取、路径授权、配额、真实文件、PDF/图像处理仍调用实际领域实现。未使用宿主端口直接 panic，不能用于证明真实鉴权。所需 permission-types、vw-api-transport、tokio-util 仅 dev-dependencies，Cargo.lock 仅补 tool-files 的三条现有工作区依赖边。

## 全量映射

原前缀均为 vw_agent::app::agent::tools::file_read::tests::tests；目标保持函数名。

| 原函数名 | 目标模块 | 职责/保留理由 |
| --- | --- | --- |
| `file_read_name` | `tool_files::host_migration_30_tests` | 领域读取/安全策略，调用真实工具 |
| `file_read_schema_has_path` | `tool_files::host_migration_30_tests` | 领域读取/安全策略，调用真实工具 |
| `file_read_existing_file` | `tool_files::host_migration_30_tests` | 领域读取/安全策略，调用真实工具 |
| `file_read_call_returns_structured_text_result` | `tool_files::host_migration_30_tests` | 领域读取/安全策略，调用真实工具 |
| `file_read_nonexistent_file` | `tool_files::host_migration_30_tests` | 领域读取/安全策略，调用真实工具 |
| `file_read_blocks_path_traversal` | `tool_files::host_migration_30_tests` | 领域读取/安全策略，调用真实工具 |
| `file_read_blocks_absolute_path` | `tool_files::host_migration_30_tests` | 领域读取/安全策略，调用真实工具 |
| `file_read_blocks_when_rate_limited` | `tool_files::host_migration_30_tests` | 领域读取/安全策略，调用真实工具 |
| `file_read_allows_readonly_mode` | `tool_files::host_migration_30_tests` | 领域读取/安全策略，调用真实工具 |
| `file_read_missing_path_param` | `tool_files::host_migration_30_tests` | 领域读取/安全策略，调用真实工具 |
| `file_read_rejects_unknown_fields` | `tool_files::host_migration_30_tests` | 领域读取/安全策略，调用真实工具 |
| `file_read_empty_file` | `tool_files::host_migration_30_tests` | 领域读取/安全策略，调用真实工具 |
| `file_read_nested_path` | `tool_files::host_migration_30_tests` | 领域读取/安全策略，调用真实工具 |
| `file_read_blocks_symlink_escape` | `tool_files::host_migration_30_tests` | 领域读取/安全策略，调用真实工具 |
| `file_read_outside_workspace_allowed_when_workspace_only_disabled` | `tool_files::host_migration_30_tests` | 领域读取/安全策略，调用真实工具 |
| `file_read_nonexistent_consumes_rate_limit_budget` | `tool_files::host_migration_30_tests` | 领域读取/安全策略，调用真实工具 |
| `file_read_with_offset_and_limit` | `tool_files::host_migration_30_tests` | 领域读取/安全策略，调用真实工具 |
| `file_read_offset_beyond_end` | `tool_files::host_migration_30_tests` | 领域读取/安全策略，调用真实工具 |
| `file_read_returns_file_unchanged_for_duplicate_runtime_read` | `vw_agent::app::agent::tools::file_read::tests::tests` | 真实 AgentContextHost 执行器与 read_state 接线 |
| `file_read_marks_default_truncation_as_partial_in_read_state` | `vw_agent::app::agent::tools::file_read::tests::tests` | 真实 AgentContextHost 执行器与 read_state 接线 |
| `file_read_marks_full_read_even_when_limit_is_provided` | `vw_agent::app::agent::tools::file_read::tests::tests` | 真实 AgentContextHost 执行器与 read_state 接线 |
| `file_read_rejects_oversized_file` | `tool_files::host_migration_30_tests` | 领域读取/安全策略，调用真实工具 |
| `file_read_extracts_pdf_text` | `tool_files::host_migration_30_tests` | 领域读取/安全策略，调用真实工具 |
| `file_read_call_returns_structured_pdf_result` | `tool_files::host_migration_30_tests` | 领域读取/安全策略，调用真实工具 |
| `file_read_lossy_reads_binary_file` | `tool_files::host_migration_30_tests` | 领域读取/安全策略，调用真实工具 |
| `file_read_reads_image_as_structured_result` | `tool_files::host_migration_30_tests` | 领域读取/安全策略，调用真实工具 |
| `file_read_reads_notebook_as_structured_result` | `tool_files::host_migration_30_tests` | 领域读取/安全策略，调用真实工具 |
| `e2e_agent_file_read_pdf_extraction` | `vw_agent::app::agent::tools::file_read::tests::tests` | 真实 Agent + Provider 工厂/流水线 |
| `e2e_agent_file_read_lossy_binary` | `vw_agent::app::agent::tools::file_read::tests::tests` | 真实 Agent + Provider 工厂/流水线 |
| `e2e_live_file_read_pdf` | `vw_agent::app::agent::tools::file_read::tests::tests` | 真实 Agent + Provider 工厂/流水线 |
| `file_read_blocks_null_byte_in_path` | `tool_files::host_migration_30_tests` | 领域读取/安全策略，调用真实工具 |

夹具：test_security 复制到领域及保留宿主（各自真实安全策略）；test_security_with、tiny_png_bytes 整体迁出；minimal_pdf_bytes 在双方各留一份局部PDF夹具，供领域及真实代理测试；read_state_entry 仅保留宿主，验证实际执行器写回。e2e_helpers 全部保留：RecordingProvider::new/chat/chat_with_system、SharedRequests、make_memory、make_observer 用于真实 Agent 集成，无完整 App 状态复制。

## 行数与减量例外

splitlines：宿主生产别名 mod.rs 5→5（其中2行生产、3行测试挂载），测试 tests.rs 1199→561，净减 638 行，占1204行来源 53.0%。领域新增测试 649 行、端口夹具 54 行、lib.rs 新增3行测试注册；领域生产行数不变。原领域 file_read/tests.rs 保持原样，不与不同层次的公共Tool行为测试去重。

宿主保留 tests.rs 561 行：3个执行器上下文用例、2个脚本Provider代理集成、1个需OAuth的ignored真实Provider集成，以及其直接夹具。净减不足70%，提交收口任务验收；未通过删断言凑比例。继续可迁项：未来若 agent-loop 自身具备完整真实生产 Host fixture，可接收两条脚本Provider集成及 e2e_helpers；当前迁入 tool-files 会引入反向上层依赖，不实施。3条读状态用例保留以证明真实 AgentContextHost 端口接线。

## 静态与未来验证

已核对31个测试函数完整归属（25迁出、6保留）、全部原函数体断言一致、领域自注册、旧领域用例不再宿主挂载。已核对任务1 evidence/registrations.json 及 external-mounts.json：本组仅原 tests.rs 直接挂载，未引入外部挂载。git diff --check 完成。没有编译收益或行为通过结论。

未来：
- `python3 scripts/cargo_slot.py cargo test -p tool-files --lib host_migration_30_tests -- --test-threads=1`：Unix预期25条，非Unix24条。
- PDF能力同时补 `python3 scripts/cargo_slot.py cargo test -p tool-files --lib --features rag-pdf host_migration_30_tests -- --test-threads=1`，同数量。
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib tools::file_read::tests::tests -- --test-threads=1`：5运行、1忽略；真实OAuth用例未来取得授权后单独运行完整名称与 --ignored。
- 宿主 `--list` 应只剩这6条，25条迁出名均消失；零测试不代表通过。

以上所有Cargo、编译、行为检查本次均未运行。
