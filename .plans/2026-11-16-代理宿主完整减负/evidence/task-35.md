# 任务35 静态交付证据

代码接线完成；编译/行为未验证。遵守用户禁止任何 Cargo、构建、单元及实机测试的要求，仅阅读源码、静态差异及注册依赖核对。

采用 do-task-plans、rustcodegraph、git-commit 技能。索引 status 已读取，但索引早于本轮共享目录变更，实际映射以当前源文件为准。

## 全文件与全用例映射

| 来源 | 前→后行数 | 职责/目标 |

|---|---:|---|

| `crates/vw-agent/src/tools/mcp_tool.rs` | 42→42 | McpTool<AgentContextHost> 特化和十个真实宿主端口，详细见下文；无新增领域业务分支。 |

| `crates/vw-agent/src/tools/mod.rs` | 312→312 | 真实工具模块注册、公共兼容导出、SkillHost/BatchHost/FileToolHost 端口实现和仍需真实宿主测试挂载。 |

| `crates/vw-agent/src/tools/pdf_read/mod.rs` | 6→2 | 兼容模块导出或具体 Host 泛型特化；实现早已属于导出所指领域库，本轮不计为生产迁移。 |

| `crates/vw-agent/src/tools/pdf_read/render_tests.rs` | 48→0 | crates/libs/tool-files/src/host_migration_30_tests/pdf_render.rs |


- `crates/vw-agent/src/tools/pdf_read/render_tests.rs::file_read_suggests_image_mode_for_pdf_without_text` → `crates/libs/tool-files/src/host_migration_30_tests/pdf_render.rs::file_read_suggests_image_mode_for_pdf_without_text`。断言保留。

| `crates/vw-agent/src/tools/powershell/mod.rs` | 4→4 | 兼容模块导出或具体 Host 泛型特化；实现早已属于导出所指领域库，本轮不计为生产迁移。 |

| `crates/vw-agent/src/tools/powershell/tests.rs` | 64→64 | Windows 具体 AgentContextHost 管理执行与真实进程上下文绑定；身份、只读和中文输出三例保留此适配器整组契约，底层 tool-shell 已持有执行实现。 |


- `crates/vw-agent/src/tools/powershell/tests.rs::powershell_tool_preserves_identity_and_cwd_schema` 保留：Windows 具体 AgentContextHost 管理执行与真实进程上下文绑定；身份、只读和中文输出三例保留此适配器整组契约，底层 tool-shell 已持有执行实现。


- `crates/vw-agent/src/tools/powershell/tests.rs::powershell_tool_keeps_readonly_rejection_before_spawn` 保留：Windows 具体 AgentContextHost 管理执行与真实进程上下文绑定；身份、只读和中文输出三例保留此适配器整组契约，底层 tool-shell 已持有执行实现。


- `crates/vw-agent/src/tools/powershell/tests.rs::powershell_tool_uses_managed_execution_and_decodes_chinese` 保留：Windows 具体 AgentContextHost 管理执行与真实进程上下文绑定；身份、只读和中文输出三例保留此适配器整组契约，底层 tool-shell 已持有执行实现。

| `crates/vw-agent/src/tools/question/mod.rs` | 3→3 | 兼容模块导出或具体 Host 泛型特化；实现早已属于导出所指领域库，本轮不计为生产迁移。 |

| `crates/vw-agent/src/tools/read_state/mod.rs` | 2→2 | 兼容模块导出或具体 Host 泛型特化；实现早已属于导出所指领域库，本轮不计为生产迁移。 |

| `crates/vw-agent/src/tools/registry/mod.rs` | 23→17 | 五个真实工厂目录+进程 flag 输入适配，筛选/DTO/JSON 实现由 tool-assembly 持有；已删除测试挂载和测试专用转发。 |

| `crates/vw-agent/src/tools/registry/tests.rs` | 84→0 | crates/libs/tool-assembly/src/host_migration_35_registry.rs |


- `crates/vw-agent/src/tools/registry/tests.rs::search_is_enabled_for_vibewindow_prefix_or_exa_flag` → `crates/libs/tool-assembly/src/host_migration_35_registry.rs::search_is_enabled_for_vibewindow_prefix_or_exa_flag`。断言保留。


- `crates/vw-agent/src/tools/registry/tests.rs::apply_patch_is_enabled_for_any_model_identifier` → `crates/libs/tool-assembly/src/host_migration_35_registry.rs::apply_patch_is_enabled_for_any_model_identifier`。断言保留。


- `crates/vw-agent/src/tools/registry/tests.rs::question_tool_is_limited_to_exact_interactive_client_names` → `crates/libs/tool-assembly/src/host_migration_35_registry.rs::question_tool_is_limited_to_exact_interactive_client_names`。断言保留。


- `crates/vw-agent/src/tools/registry/tests.rs::blank_models_use_the_same_default_tool_set` → `crates/libs/tool-assembly/src/host_migration_35_registry.rs::blank_models_use_the_same_default_tool_set`。断言保留。


- `crates/vw-agent/src/tools/registry/tests.rs::dto_response_preserves_registry_order_and_fields` → `crates/libs/tool-assembly/src/host_migration_35_registry.rs::dto_response_preserves_registry_order_and_fields`。断言保留。


- `crates/vw-agent/src/tools/registry/tests.rs::json_views_expose_expected_shapes_and_sorted_openai_names` → `crates/libs/tool-assembly/src/host_migration_35_registry.rs::json_views_expose_expected_shapes_and_sorted_openai_names`。断言保留。

| `crates/vw-agent/src/tools/remote_trigger/mod.rs` | 2→2 | 兼容模块导出或具体 Host 泛型特化；实现早已属于导出所指领域库，本轮不计为生产迁移。 |

| `crates/vw-agent/src/tools/schedule/mod.rs` | 2→2 | 兼容模块导出或具体 Host 泛型特化；实现早已属于导出所指领域库，本轮不计为生产迁移。 |

| `crates/vw-agent/src/tools/scheduler/mod.rs` | 2→2 | 兼容模块导出或具体 Host 泛型特化；实现早已属于导出所指领域库，本轮不计为生产迁移。 |

| `crates/vw-agent/src/tools/screenshot/mod.rs` | 2→2 | 兼容模块导出或具体 Host 泛型特化；实现早已属于导出所指领域库，本轮不计为生产迁移。 |

| `crates/vw-agent/src/tools/session_tool/mod.rs` | 11→11 | ToolAgentHost 泛型特化，原跨目录测试依赖真实会话存储与 task coordinator；保留真实应用宿主接线挂载。 |

| `crates/vw-agent/src/tools/shell/ast/mod.rs` | 3→3 | 兼容模块导出或具体 Host 泛型特化；实现早已属于导出所指领域库，本轮不计为生产迁移。 |

| `crates/vw-agent/src/tools/shell/compound/mod.rs` | 3→3 | 兼容模块导出或具体 Host 泛型特化；实现早已属于导出所指领域库，本轮不计为生产迁移。 |

| `crates/vw-agent/src/tools/shell/path/mod.rs` | 3→3 | 兼容模块导出或具体 Host 泛型特化；实现早已属于导出所指领域库，本轮不计为生产迁移。 |

| `crates/vw-agent/src/tools/shell/permissions/mod.rs` | 3→3 | 兼容模块导出或具体 Host 泛型特化；实现早已属于导出所指领域库，本轮不计为生产迁移。 |

| `crates/vw-agent/src/tools/shell/readonly/mod.rs` | 3→3 | 兼容模块导出或具体 Host 泛型特化；实现早已属于导出所指领域库，本轮不计为生产迁移。 |

| `crates/vw-agent/src/tools/shell/sandbox/executor.rs` | 2→2 | 兼容模块导出或具体 Host 泛型特化；实现早已属于导出所指领域库，本轮不计为生产迁移。 |

| `crates/vw-agent/src/tools/shell/sandbox/mod.rs` | 2→2 | 兼容模块导出或具体 Host 泛型特化；实现早已属于导出所指领域库，本轮不计为生产迁移。 |

| `crates/vw-agent/src/tools/shell/sandbox/policy.rs` | 2→2 | 兼容模块导出或具体 Host 泛型特化；实现早已属于导出所指领域库，本轮不计为生产迁移。 |

| `crates/vw-agent/src/tools/shell/security/mod.rs` | 3→3 | 兼容模块导出或具体 Host 泛型特化；实现早已属于导出所指领域库，本轮不计为生产迁移。 |

| `crates/vw-agent/src/tools/shell/sed/mod.rs` | 3→3 | 兼容模块导出或具体 Host 泛型特化；实现早已属于导出所指领域库，本轮不计为生产迁移。 |

| `crates/vw-agent/src/tools/skill/mod.rs` | 37→37 | SkillTool<AgentContextHost>、accessible_skills 的具体类型绑定及真实技能/权限/项目上下文测试挂载；不复制宿主技能装载。 |

| `crates/vw-agent/src/tools/sleep/mod.rs` | 2→2 | 兼容模块导出或具体 Host 泛型特化；实现早已属于导出所指领域库，本轮不计为生产迁移。 |

| `crates/vw-agent/src/tools/tests/mcp_discovery_tests.rs` | 153→153 | 真实 AgentContextHost 的配置加载和 configured server 连接生命周期，包含 stdio 及坏 HTTP 错误链；McpToolHost 端口不在领域库重实现。 |


- `crates/vw-agent/src/tools/tests/mcp_discovery_tests.rs::discover_and_call_configured_mcp_tool_through_stdio_fixture` 保留：真实 AgentContextHost 的配置加载和 configured server 连接生命周期，包含 stdio 及坏 HTTP 错误链；McpToolHost 端口不在领域库重实现。


- `crates/vw-agent/src/tools/tests/mcp_discovery_tests.rs::mcp_tool_from_spec_default_handler_reports_missing_configured_server` 保留：真实 AgentContextHost 的配置加载和 configured server 连接生命周期，包含 stdio 及坏 HTTP 错误链；McpToolHost 端口不在领域库重实现。


- `crates/vw-agent/src/tools/tests/mcp_discovery_tests.rs::mcp_tool_from_configured_server_uses_server_bound_handler` 保留：真实 AgentContextHost 的配置加载和 configured server 连接生命周期，包含 stdio 及坏 HTTP 错误链；McpToolHost 端口不在领域库重实现。

| `crates/vw-agent/src/tools/tests/mod_tests.rs` | 611→586 | crates/libs/tool-assembly/src/host_migration_35_tests.rs；实际宿主 default_tools/all_tools 工厂构造、运行时注册及工作区 MCP 配置→执行接线；序列化已独立迁出。 |


- `crates/vw-agent/src/tools/tests/mod_tests.rs::default_tools_include_apply_patch_once` 保留：实际宿主 default_tools/all_tools 工厂构造、运行时注册及工作区 MCP 配置→执行接线；序列化已独立迁出。


- `crates/vw-agent/src/tools/tests/mod_tests.rs::all_tools_excludes_browser_when_disabled` 保留：实际宿主 default_tools/all_tools 工厂构造、运行时注册及工作区 MCP 配置→执行接线；序列化已独立迁出。


- `crates/vw-agent/src/tools/tests/mod_tests.rs::all_tools_includes_browser_when_enabled` 保留：实际宿主 default_tools/all_tools 工厂构造、运行时注册及工作区 MCP 配置→执行接线；序列化已独立迁出。


- `crates/vw-agent/src/tools/tests/mod_tests.rs::default_tools_names` 保留：实际宿主 default_tools/all_tools 工厂构造、运行时注册及工作区 MCP 配置→执行接线；序列化已独立迁出。


- `crates/vw-agent/src/tools/tests/mod_tests.rs::default_tools_all_have_descriptions` 保留：实际宿主 default_tools/all_tools 工厂构造、运行时注册及工作区 MCP 配置→执行接线；序列化已独立迁出。


- `crates/vw-agent/src/tools/tests/mod_tests.rs::default_tools_all_have_schemas` 保留：实际宿主 default_tools/all_tools 工厂构造、运行时注册及工作区 MCP 配置→执行接线；序列化已独立迁出。


- `crates/vw-agent/src/tools/tests/mod_tests.rs::apply_patch_spec_uses_v2_aliases` 保留：实际宿主 default_tools/all_tools 工厂构造、运行时注册及工作区 MCP 配置→执行接线；序列化已独立迁出。


- `crates/vw-agent/src/tools/tests/mod_tests.rs::registry_specs_include_apply_patch_by_default` 保留：实际宿主 default_tools/all_tools 工厂构造、运行时注册及工作区 MCP 配置→执行接线；序列化已独立迁出。


- `crates/vw-agent/src/tools/tests/mod_tests.rs::registry_specs_include_apply_patch_for_explicit_models` 保留：实际宿主 default_tools/all_tools 工厂构造、运行时注册及工作区 MCP 配置→执行接线；序列化已独立迁出。


- `crates/vw-agent/src/tools/tests/mod_tests.rs::tool_spec_generation` 保留：实际宿主 default_tools/all_tools 工厂构造、运行时注册及工作区 MCP 配置→执行接线；序列化已独立迁出。


- `crates/vw-agent/src/tools/tests/mod_tests.rs::default_tools_specs_expose_bash_alias` 保留：实际宿主 default_tools/all_tools 工厂构造、运行时注册及工作区 MCP 配置→执行接线；序列化已独立迁出。


- `crates/vw-agent/src/tools/tests/mod_tests.rs::registry_specs_expose_bash_and_compact_search_surface` 保留：实际宿主 default_tools/all_tools 工厂构造、运行时注册及工作区 MCP 配置→执行接线；序列化已独立迁出。


- `crates/vw-agent/src/tools/tests/mod_tests.rs::mcp_toolset_tools_are_registered_from_config` 保留：实际宿主 default_tools/all_tools 工厂构造、运行时注册及工作区 MCP 配置→执行接线；序列化已独立迁出。


- `crates/vw-agent/src/tools/tests/mod_tests.rs::mcp_toolset_tools_can_be_executed_from_registry` 保留：实际宿主 default_tools/all_tools 工厂构造、运行时注册及工作区 MCP 配置→执行接线；序列化已独立迁出。


- `crates/vw-agent/src/tools/tests/mod_tests.rs::tool_result_serde` → `crates/libs/tool-assembly/src/host_migration_35_tests.rs::tool_result_serde`。断言保留。


- `crates/vw-agent/src/tools/tests/mod_tests.rs::tool_result_with_error_serde` → `crates/libs/tool-assembly/src/host_migration_35_tests.rs::tool_result_with_error_serde`。断言保留。


- `crates/vw-agent/src/tools/tests/mod_tests.rs::tool_spec_serde` → `crates/libs/tool-assembly/src/host_migration_35_tests.rs::tool_spec_serde`。断言保留。


- `crates/vw-agent/src/tools/tests/mod_tests.rs::all_tools_includes_agent_tool_when_agents_configured` 保留：实际宿主 default_tools/all_tools 工厂构造、运行时注册及工作区 MCP 配置→执行接线；序列化已独立迁出。


- `crates/vw-agent/src/tools/tests/mod_tests.rs::all_tools_includes_default_agent_tool_when_no_agents` 保留：实际宿主 default_tools/all_tools 工厂构造、运行时注册及工作区 MCP 配置→执行接线；序列化已独立迁出。


- `crates/vw-agent/src/tools/tests/mod_tests.rs::all_tools_includes_default_agent_tool_when_only_primary_agents` 保留：实际宿主 default_tools/all_tools 工厂构造、运行时注册及工作区 MCP 配置→执行接线；序列化已独立迁出。


- `crates/vw-agent/src/tools/tests/mod_tests.rs::all_tools_registers_agent_tool_for_delegate_agents` 保留：实际宿主 default_tools/all_tools 工厂构造、运行时注册及工作区 MCP 配置→执行接线；序列化已独立迁出。

| `crates/vw-agent/src/tools/thread_tool/mod.rs` | 17→17 | ToolAgentHost 泛型特化，原跨目录测试依赖真实会话存储与 task coordinator；保留真实应用宿主接线挂载。 |

| `crates/vw-agent/src/tools/todo/mod.rs` | 12→12 | 保留 read/write 从实际 ToolRuntimeContext.session 到 tool-services 的参数适配和错误映射，Todo 类型及算法由 tool-services 持有。 |

| `crates/vw-agent/src/tools/tool_search.rs` | 97→97 | 真实 AgentContextHost 授权目录绑定、执行器 task-local 与 native receipt 桥接；逐条用例检查此适配器，不用另一份搜索工具冒充真实适配器。领域搜索算法已由 tool-catalog 持有。 |

| `crates/vw-agent/src/tools/tool_search_tests.rs` | 356→356 | 真实 AgentContextHost 授权目录绑定、执行器 task-local 与 native receipt 桥接；逐条用例检查此适配器，不用另一份搜索工具冒充真实适配器。领域搜索算法已由 tool-catalog 持有。 |


- `crates/vw-agent/src/tools/tool_search_tests.rs::codex_discovery_tool_registry_returns_complete_specs_and_exposes_only_hits` 保留：真实 AgentContextHost 授权目录绑定、执行器 task-local 与 native receipt 桥接；逐条用例检查此适配器，不用另一份搜索工具冒充真实适配器。领域搜索算法已由 tool-catalog 持有。


- `crates/vw-agent/src/tools/tool_search_tests.rs::codex_discovery_tool_invalid_arguments_do_not_change_exposure` 保留：真实 AgentContextHost 授权目录绑定、执行器 task-local 与 native receipt 桥接；逐条用例检查此适配器，不用另一份搜索工具冒充真实适配器。领域搜索算法已由 tool-catalog 持有。


- `crates/vw-agent/src/tools/tool_search_tests.rs::codex_discovery_tool_no_match_and_duplicate_hits_are_idempotent` 保留：真实 AgentContextHost 授权目录绑定、执行器 task-local 与 native receipt 桥接；逐条用例检查此适配器，不用另一份搜索工具冒充真实适配器。领域搜索算法已由 tool-catalog 持有。


- `crates/vw-agent/src/tools/tool_search_tests.rs::codex_discovery_tool_denied_search_keeps_authorized_mcp_directly_visible` 保留：真实 AgentContextHost 授权目录绑定、执行器 task-local 与 native receipt 桥接；逐条用例检查此适配器，不用另一份搜索工具冒充真实适配器。领域搜索算法已由 tool-catalog 持有。


- `crates/vw-agent/src/tools/tool_search_tests.rs::codex_discovery_tool_empty_authorization_cannot_restore_tools` 保留：真实 AgentContextHost 授权目录绑定、执行器 task-local 与 native receipt 桥接；逐条用例检查此适配器，不用另一份搜索工具冒充真实适配器。领域搜索算法已由 tool-catalog 持有。


- `crates/vw-agent/src/tools/tool_search_tests.rs::codex_discovery_tool_requires_runtime_catalog_and_isolates_sessions` 保留：真实 AgentContextHost 授权目录绑定、执行器 task-local 与 native receipt 桥接；逐条用例检查此适配器，不用另一份搜索工具冒充真实适配器。领域搜索算法已由 tool-catalog 持有。


- `crates/vw-agent/src/tools/tool_search_tests.rs::codex_discovery_tool_same_name_and_forged_results_cannot_load_tools` 保留：真实 AgentContextHost 授权目录绑定、执行器 task-local 与 native receipt 桥接；逐条用例检查此适配器，不用另一份搜索工具冒充真实适配器。领域搜索算法已由 tool-catalog 持有。


- `crates/vw-agent/src/tools/tool_search_tests.rs::codex_discovery_tool_authorized_summary_omits_denied_servers_and_schemas` 保留：真实 AgentContextHost 授权目录绑定、执行器 task-local 与 native receipt 桥接；逐条用例检查此适配器，不用另一份搜索工具冒充真实适配器。领域搜索算法已由 tool-catalog 持有。


- `crates/vw-agent/src/tools/tool_search_tests.rs::codex_discovery_tool_execution_uses_current_authorization_instead_of_registered_metadata` 保留：真实 AgentContextHost 授权目录绑定、执行器 task-local 与 native receipt 桥接；逐条用例检查此适配器，不用另一份搜索工具冒充真实适配器。领域搜索算法已由 tool-catalog 持有。


- `crates/vw-agent/src/tools/tool_search_tests.rs::dynamic_tools_exposure_non_mcp_deferred_is_discoverable` 保留：真实 AgentContextHost 授权目录绑定、执行器 task-local 与 native receipt 桥接；逐条用例检查此适配器，不用另一份搜索工具冒充真实适配器。领域搜索算法已由 tool-catalog 持有。


- `crates/vw-agent/src/tools/tool_search_tests.rs::dynamic_tools_search_schema_fields_are_searchable` 保留：真实 AgentContextHost 授权目录绑定、执行器 task-local 与 native receipt 桥接；逐条用例检查此适配器，不用另一份搜索工具冒充真实适配器。领域搜索算法已由 tool-catalog 持有。


- `crates/vw-agent/src/tools/tool_search_tests.rs::dynamic_tools_search_tenants_and_loaded_state_remain_isolated` 保留：真实 AgentContextHost 授权目录绑定、执行器 task-local 与 native receipt 桥接；逐条用例检查此适配器，不用另一份搜索工具冒充真实适配器。领域搜索算法已由 tool-catalog 持有。

| `crates/vw-agent/src/tools/traits/dynamic_tools_tests.rs` | 31→0 | crates/libs/tool-mcp/src/host_migration_35_tests.rs |


- `crates/vw-agent/src/tools/traits/dynamic_tools_tests.rs::dynamic_tools_contract_mcp_registration_preserves_identity_and_stable_id` → `crates/libs/tool-mcp/src/host_migration_35_tests.rs::dynamic_tools_contract_mcp_registration_preserves_identity_and_stable_id`。断言保留。

| `crates/vw-agent/src/tools/traits/mod.rs` | 6→2 | 兼容模块导出或具体 Host 泛型特化；实现早已属于导出所指领域库，本轮不计为生产迁移。 |

| `crates/vw-agent/src/tools/url_validation/mod.rs` | 6→6 | 兼容模块导出或具体 Host 泛型特化；实现早已属于导出所指领域库，本轮不计为生产迁移。 |

| `crates/vw-agent/src/tools/web_fetch/mod.rs` | 3→3 | 兼容模块导出或具体 Host 泛型特化；实现早已属于导出所指领域库，本轮不计为生产迁移。 |

| `crates/vw-agent/src/tools/web_search_tool/mod.rs` | 3→3 | 兼容模块导出或具体 Host 泛型特化；实现早已属于导出所指领域库，本轮不计为生产迁移。 |

| `crates/vw-agent/src/tools/websearch/mod.rs` | 3→3 | 兼容模块导出或具体 Host 泛型特化；实现早已属于导出所指领域库，本轮不计为生产迁移。 |

| `crates/vw-agent/src/tools/word_control.rs` | 2→2 | 兼容模块导出或具体 Host 泛型特化；实现早已属于导出所指领域库，本轮不计为生产迁移。 |

| `crates/vw-agent/src/tools/word_control_tests.rs` | 424→230 | crates/libs/tool-office/src/host_migration_35_tests.rs；仅保留真实工厂共享网关 authority 和真实 HookRunner 文档审批两例及其 TLS/pairing 夹具；其余协议场景迁到 tool-office，复用真实 OfficeTransport、策略与执行器。 |


- `crates/vw-agent/src/tools/word_control_tests.rs::cc_control_16_no_session_discovers_empty_and_refuses_execution` → `crates/libs/tool-office/src/host_migration_35_tests.rs::cc_control_16_no_session_discovers_empty_and_refuses_execution`。断言保留。


- `crates/vw-agent/src/tools/word_control_tests.rs::cc_control_16_duplicate_titles_require_exact_document_and_generation` → `crates/libs/tool-office/src/host_migration_35_tests.rs::cc_control_16_duplicate_titles_require_exact_document_and_generation`。断言保留。


- `crates/vw-agent/src/tools/word_control_tests.rs::cc_control_16_switch_and_document_grant_both_gate_execution` → `crates/libs/tool-office/src/host_migration_35_tests.rs::cc_control_16_switch_and_document_grant_both_gate_execution`。断言保留。


- `crates/vw-agent/src/tools/word_control_tests.rs::cc_control_16_missing_capability_and_unknown_fields_are_rejected` → `crates/libs/tool-office/src/host_migration_35_tests.rs::cc_control_16_missing_capability_and_unknown_fields_are_rejected`。断言保留。


- `crates/vw-agent/src/tools/word_control_tests.rs::cc_control_16_replace_returns_actual_changed_range` → `crates/libs/tool-office/src/host_migration_35_tests.rs::cc_control_16_replace_returns_actual_changed_range`。断言保留。


- `crates/vw-agent/src/tools/word_control_tests.rs::cc_control_16_disconnect_and_repair_invalidate_old_generation` → `crates/libs/tool-office/src/host_migration_35_tests.rs::cc_control_16_disconnect_and_repair_invalidate_old_generation`。断言保留。


- `crates/vw-agent/src/tools/word_control_tests.rs::cc_control_16_foreign_executor_owner_cannot_discover_or_execute` → `crates/libs/tool-office/src/host_migration_35_tests.rs::cc_control_16_foreign_executor_owner_cannot_discover_or_execute`。断言保留。


- `crates/vw-agent/src/tools/word_control_tests.rs::cc_control_16_revocation_during_command_discards_result` → `crates/libs/tool-office/src/host_migration_35_tests.rs::cc_control_16_revocation_during_command_discards_result`。断言保留。


- `crates/vw-agent/src/tools/word_control_tests.rs::cc_control_16_invalid_text_never_dispatches` → `crates/libs/tool-office/src/host_migration_35_tests.rs::cc_control_16_invalid_text_never_dispatches`。断言保留。


- `crates/vw-agent/src/tools/word_control_tests.rs::cc_control_16_range_conflict_is_failure_and_invalidates_session` → `crates/libs/tool-office/src/host_migration_35_tests.rs::cc_control_16_range_conflict_is_failure_and_invalidates_session`。断言保留。


- `crates/vw-agent/src/tools/word_control_tests.rs::cc_control_16_abort_invalidates_connection_and_discards_late_result` → `crates/libs/tool-office/src/host_migration_35_tests.rs::cc_control_16_abort_invalidates_connection_and_discards_late_result`。断言保留。


- `crates/vw-agent/src/tools/word_control_tests.rs::cc_control_16_factory_registers_word_with_shared_gateway_authority` 保留：仅保留真实工厂共享网关 authority 和真实 HookRunner 文档审批两例及其 TLS/pairing 夹具；其余协议场景迁到 tool-office，复用真实 OfficeTransport、策略与执行器。


- `crates/vw-agent/src/tools/word_control_tests.rs::cc_control_16_malformed_readback_is_failure_and_closes_connection` → `crates/libs/tool-office/src/host_migration_35_tests.rs::cc_control_16_malformed_readback_is_failure_and_closes_connection`。断言保留。


- `crates/vw-agent/src/tools/word_control_tests.rs::cc_control_16_document_approval_is_scoped_and_explicit_deny_is_preserved` 保留：仅保留真实工厂共享网关 authority 和真实 HookRunner 文档审批两例及其 TLS/pairing 夹具；其余协议场景迁到 tool-office，复用真实 OfficeTransport、策略与执行器。

## 生产边界与依赖方向

宿主 → tool-assembly/tool-office/tool-files/tool-mcp → tool-context/tool-types；接收库无 vw-agent 反向依赖、无 include/path 加载宿主源码。实际生产调用未改：tools::registry 调 tool_assembly::registry；word_control::WordControlTool 特化 tool_office；pdf_read 特化 tool_files；mcp_tool 特化 tool_mcp。此次新增生产迁移为 0，既有下沉和转发不计为收益。

McpToolHost 逐符号保留：new_connections 创建租户连接 Host；load_config 读取当前应用/工作区配置；connection_required、missing_executable、tool_allowed 查询插件连接要求/依赖/授权；tool_binding 使用当前连接目录；with_client 使用宿主传输与授权；with_connection 复用活动连接；app_resource 和 call_resource 接实际插件资源生命周期。均为真实端口，不在领域测试复制权限实现。

## 自注册与静态核对

tool-assembly::host_migration_35_tests 注册 3 个序列化和 registry 子模块 6 例；registry 使用显式非空目录输入，EXA 真/假均覆盖。完整宿主目录注册断言仍由原 mod_tests 验收。tool-office::host_migration_35_tests 注册 12 个协议场景，保留真实 TLS/OfficeTransport/ComputerControlPolicy；测试 Host 仅提供租户和执行上下文端口，无模拟 Word 策略。tool-mcp 注册 1 个身份契约；tool-files::host_migration_30_tests::pdf_render 注册 1 个 PDF 场景。所有迁出旧挂载已删除；原 Word 挂载仅含两例宿主边界。

当前所列来源 2397→2001 行，净减 396 行（16.5%），未达到 70%。逐文件保留原因见上；不能把原已下沉实现计入收益。继续可迁项：tool_search 原生 receipt 之外的授权目录执行断言可在统一 ContextHost 测试端口复用后迁移；工厂断言需要真实可复用生产构造宿主，不能用伪 ToolConstruction stub 替代。技能/线程跨目录测试仍依赖真实持久化宿主，需收口核验。此证据不宣称整轮减量达标。

按职责分项：来源测试/夹具文件净变化 382 行；其余兼容/注册文件仅去除已迁出测试挂载。生产业务迁移 0；测试夹具增加/重复计入领域物理行数，不宣称编译耗时改善。

## 未来验证（均未运行）

- `python3 scripts/cargo_slot.py cargo test -p tool-assembly --lib host_migration_35_tests -- --test-threads=1`，静态预期 9 例，须确认 ran>0、failed=0。

- `python3 scripts/cargo_slot.py cargo test -p tool-office --lib host_migration_35_tests -- --test-threads=1`，静态预期 12 例，须确认 ran>0、failed=0。

- `python3 scripts/cargo_slot.py cargo test -p tool-mcp --lib host_migration_35_tests -- --test-threads=1`，静态预期 1 例，须确认 ran>0、failed=0。

- `python3 scripts/cargo_slot.py cargo test -p tool-files --lib host_migration_30_tests::pdf_render -- --test-threads=1`，静态预期 1 例，须确认 ran>0、failed=0。

未来宿主按上述每个保留完整模块/用例名过滤；已迁出的测试需在宿主 list 确认消失，不能把零测试当通过。Cargo.lock 未通过 Cargo 更新；新增 dev-dependencies 复用仓库既有版本，未来首次验证须同时核对锁文件。

## 接收库新增行数（全部为测试/夹具，生产0）

- `crates/libs/tool-assembly/src/host_migration_35_tests.rs`：29 行。

- `crates/libs/tool-assembly/src/host_migration_35_registry.rs`：96 行。

- `crates/libs/tool-office/src/host_migration_35_tests.rs`：342 行。

- `crates/libs/tool-mcp/src/host_migration_35_tests.rs`：30 行。

- `crates/libs/tool-files/src/host_migration_30_tests/pdf_render.rs`：49 行。

- `crates/libs/tool-office/src/host_migration_35_tests/host.rs`：50 行。

## 深度恢复（覆盖前文相应保留判断）

按 final.md 与 evidence/remaining-host.json 的任务35清单继续拆分。前文声称搜索整组、Word Hook审批和 PowerShell 必须留宿主的判断已撤回：这些断言已经改用领域真实实现及窄上下文端口。未运行任何 Cargo、构建或测试。

生产接线：tool-assembly::tool_search::ContextToolSearchTool<H> 统一持有 metadata/input validation、当前上下文缺失拒绝、当前授权目录解析、真实 search 与成功回执顺序。SearchHost 仅提供当前授权 session 和记录成功回执两个真实调用所需端口。宿主 ToolSearchTool 已改为此泛型类型特化；原 trait 实现删除。AgentContextHost::search_session 克隆同一个 Arc，不复制授权/已加载状态；record_search 保留 call_id、互斥锁和成功参数写入。native_output/prepare_native_search/take_native_output 继续由宿主完成 AISDK 协议桥接。ToolDiscoverySession.inner 改用 Arc 只为共享原身份，rebase 仍创建独立状态。

测试搜索 Host 使用真实领域 session 与 tool-context 执行器；仅 receipt 端口为空，因为本组无 native 协议断言。真实 native receipt 测试仍由会话协议所有者覆盖，不能将这12例冒充原生协议验证。Word 审批使用真实 agent-hooks::HookRunner 和真实 OfficePolicy，没有仿写审批分支；仅省略原 Host lifecycle cleanup（本例从未调用清理）。PowerShell 复用 tool-shell 已有 Host 和执行进程/上下文实现，Windows cfg 保持，所有原断言原名保留。

新增依赖只有 tool-office 的 dev-dependency agent-hooks；方向 tool-office(test)→agent-hooks，不反向依赖 vw-agent。新增泛型 SearchHost 的唯一生产实现是 AgentContextHost；测试 Host 自注册在 tool-assembly 的 host_migration_35_tests::search 下。

### 本轮逐用例映射

- `codex_discovery_tool_registry_returns_complete_specs_and_exposes_only_hits` → `crates/libs/tool-assembly/src/host_migration_35_search.rs::codex_discovery_tool_registry_returns_complete_specs_and_exposes_only_hits`；原挂载已删除。
- `codex_discovery_tool_invalid_arguments_do_not_change_exposure` → `crates/libs/tool-assembly/src/host_migration_35_search.rs::codex_discovery_tool_invalid_arguments_do_not_change_exposure`；原挂载已删除。
- `codex_discovery_tool_no_match_and_duplicate_hits_are_idempotent` → `crates/libs/tool-assembly/src/host_migration_35_search.rs::codex_discovery_tool_no_match_and_duplicate_hits_are_idempotent`；原挂载已删除。
- `codex_discovery_tool_denied_search_keeps_authorized_mcp_directly_visible` → `crates/libs/tool-assembly/src/host_migration_35_search.rs::codex_discovery_tool_denied_search_keeps_authorized_mcp_directly_visible`；原挂载已删除。
- `codex_discovery_tool_empty_authorization_cannot_restore_tools` → `crates/libs/tool-assembly/src/host_migration_35_search.rs::codex_discovery_tool_empty_authorization_cannot_restore_tools`；原挂载已删除。
- `codex_discovery_tool_requires_runtime_catalog_and_isolates_sessions` → `crates/libs/tool-assembly/src/host_migration_35_search.rs::codex_discovery_tool_requires_runtime_catalog_and_isolates_sessions`；原挂载已删除。
- `codex_discovery_tool_same_name_and_forged_results_cannot_load_tools` → `crates/libs/tool-assembly/src/host_migration_35_search.rs::codex_discovery_tool_same_name_and_forged_results_cannot_load_tools`；原挂载已删除。
- `codex_discovery_tool_authorized_summary_omits_denied_servers_and_schemas` → `crates/libs/tool-assembly/src/host_migration_35_search.rs::codex_discovery_tool_authorized_summary_omits_denied_servers_and_schemas`；原挂载已删除。
- `codex_discovery_tool_execution_uses_current_authorization_instead_of_registered_metadata` → `crates/libs/tool-assembly/src/host_migration_35_search.rs::codex_discovery_tool_execution_uses_current_authorization_instead_of_registered_metadata`；原挂载已删除。
- `dynamic_tools_exposure_non_mcp_deferred_is_discoverable` → `crates/libs/tool-assembly/src/host_migration_35_search.rs::dynamic_tools_exposure_non_mcp_deferred_is_discoverable`；原挂载已删除。
- `dynamic_tools_search_schema_fields_are_searchable` → `crates/libs/tool-assembly/src/host_migration_35_search.rs::dynamic_tools_search_schema_fields_are_searchable`；原挂载已删除。
- `dynamic_tools_search_tenants_and_loaded_state_remain_isolated` → `crates/libs/tool-assembly/src/host_migration_35_search.rs::dynamic_tools_search_tenants_and_loaded_state_remain_isolated`；原挂载已删除。
- `powershell_tool_preserves_identity_and_cwd_schema` → `crates/libs/tool-shell/src/host_migration_31_tests/powershell_35_tests.rs::powershell_tool_preserves_identity_and_cwd_schema`；原挂载已删除。
- `powershell_tool_keeps_readonly_rejection_before_spawn` → `crates/libs/tool-shell/src/host_migration_31_tests/powershell_35_tests.rs::powershell_tool_keeps_readonly_rejection_before_spawn`；原挂载已删除。
- `powershell_tool_uses_managed_execution_and_decodes_chinese` → `crates/libs/tool-shell/src/host_migration_31_tests/powershell_35_tests.rs::powershell_tool_uses_managed_execution_and_decodes_chinese`；原挂载已删除。
- `cc_control_16_document_approval_is_scoped_and_explicit_deny_is_preserved` → `tool-office::host_migration_35_tests`；实际Hook/文档审批分支保留。

### 来源逐文件最新行数（同初始口径）

- `crates/vw-agent/src/tools/mcp_tool.rs`：42→42。
- `crates/vw-agent/src/tools/mod.rs`：312→312。
- `crates/vw-agent/src/tools/pdf_read/mod.rs`：6→2。
- `crates/vw-agent/src/tools/pdf_read/render_tests.rs`：48→0。
- `crates/vw-agent/src/tools/powershell/mod.rs`：4→2。
- `crates/vw-agent/src/tools/powershell/tests.rs`：64→0。
- `crates/vw-agent/src/tools/question/mod.rs`：3→3。
- `crates/vw-agent/src/tools/read_state/mod.rs`：2→2。
- `crates/vw-agent/src/tools/registry/mod.rs`：23→17。
- `crates/vw-agent/src/tools/registry/tests.rs`：84→0。
- `crates/vw-agent/src/tools/remote_trigger/mod.rs`：2→2。
- `crates/vw-agent/src/tools/schedule/mod.rs`：2→2。
- `crates/vw-agent/src/tools/scheduler/mod.rs`：2→2。
- `crates/vw-agent/src/tools/screenshot/mod.rs`：2→2。
- `crates/vw-agent/src/tools/session_tool/mod.rs`：11→11。
- `crates/vw-agent/src/tools/shell/ast/mod.rs`：3→3。
- `crates/vw-agent/src/tools/shell/compound/mod.rs`：3→3。
- `crates/vw-agent/src/tools/shell/path/mod.rs`：3→3。
- `crates/vw-agent/src/tools/shell/permissions/mod.rs`：3→3。
- `crates/vw-agent/src/tools/shell/readonly/mod.rs`：3→3。
- `crates/vw-agent/src/tools/shell/sandbox/executor.rs`：2→2。
- `crates/vw-agent/src/tools/shell/sandbox/mod.rs`：2→2。
- `crates/vw-agent/src/tools/shell/sandbox/policy.rs`：2→2。
- `crates/vw-agent/src/tools/shell/security/mod.rs`：3→3。
- `crates/vw-agent/src/tools/shell/sed/mod.rs`：3→3。
- `crates/vw-agent/src/tools/skill/mod.rs`：37→37。
- `crates/vw-agent/src/tools/sleep/mod.rs`：2→2。
- `crates/vw-agent/src/tools/tests/mcp_discovery_tests.rs`：153→153。
- `crates/vw-agent/src/tools/tests/mod_tests.rs`：611→586。
- `crates/vw-agent/src/tools/thread_tool/mod.rs`：17→17。
- `crates/vw-agent/src/tools/todo/mod.rs`：12→12。
- `crates/vw-agent/src/tools/tool_search.rs`：97→76。
- `crates/vw-agent/src/tools/tool_search_tests.rs`：356→0。
- `crates/vw-agent/src/tools/traits/dynamic_tools_tests.rs`：31→0。
- `crates/vw-agent/src/tools/traits/mod.rs`：6→2。
- `crates/vw-agent/src/tools/url_validation/mod.rs`：6→6。
- `crates/vw-agent/src/tools/web_fetch/mod.rs`：3→3。
- `crates/vw-agent/src/tools/web_search_tool/mod.rs`：3→3。
- `crates/vw-agent/src/tools/websearch/mod.rs`：3→3。
- `crates/vw-agent/src/tools/word_control.rs`：2→2。
- `crates/vw-agent/src/tools/word_control_tests.rs`：424→37。

总计 2397→1365，净减 1032（43.1%）。生产新增迁移为通用搜索执行器；其余新增减少来自完整断言及夹具迁出。兼容入口与真实工厂仍在宿主，不把原有重导出算作生产成果。仍未达到70%，主要残留为 tools/tests/mod_tests.rs 的真实默认/完整工厂与 MCP 配置端到端注册，以及 tools/mod.rs 具体宿主类型/端口装配；逐用例保留原因沿用前表，已迁出项以上节覆盖。收口须继续审查这些边界的最小性，不宣称整体门槛已达标。

### 未来精确验证（未执行）

- `python3 scripts/cargo_slot.py cargo test -p tool-assembly --lib host_migration_35_tests -- --test-threads=1`：预期21例（前9+搜索12）。
- `python3 scripts/cargo_slot.py cargo test -p tool-office --lib host_migration_35_tests -- --test-threads=1`：预期13例（前12+审批1）。
- Windows：`python3 scripts/cargo_slot.py cargo test -p tool-shell --lib powershell_35_tests -- --test-threads=1`：预期3例；其它平台不以零例宣称通过。
- vw-agent保留 `tools::word_control_tests::cc_control_16_factory_registers_word_with_shared_gateway_authority` 1例。原搜索/PowerShell挂载消失须以后续真实宿主list证明，未运行。

本轮静态核对：测试名称/断言一对一，原源码挂载删除；接收模块在本库注册；源码依赖方向与差异检查。编译、行为、系统授权、真实进程及 Cargo.lock 重生成全部未执行。
