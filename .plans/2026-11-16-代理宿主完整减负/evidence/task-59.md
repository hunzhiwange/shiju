# 任务59最新静态交接

宿主51915行，仍差16915行；静态命令已完成，未执行Cargo/构建/测试。恢复顺序及每项真实边界见final.md与task-59-recovery-priorities.json。任务文件保留以遵守硬门槛，不以未来编译验证作为阻塞。

Removed the uncalled crate-private app::config bridge. Its original config_bridge_sets_and_loads_unique_field assertion now lives in session-store::ui_config_tests and uses the same store-result helpers as the public API, with a real temporary SQLite store and explicit default tenant. No production callers were found by rg; app::task and app::agent remain. No Cargo/build/test run.

Moved agent_scheduler_for_home_defers_cutover_until_database_open and its real legacy SQLite fixture to task-scheduler::schema_tests; the existing native lib test module registers it. Preserve the shared ProjectStore/SessionStore/scheduler database and permissions integration in the host. No dependency or production behavior change; static diff check only, no Cargo/tests.
