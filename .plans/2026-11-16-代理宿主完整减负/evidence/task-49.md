# 任务49：兼容入口与残留装配

代码完成，编译/行为未验证。按本轮明确约束未运行任何 Cargo、单元测试、实机测试、构建、格式化或测试可执行文件；以下数量来自源码静态核对，不代表运行通过。

## 完整来源映射

| 原宿主文件 | 原行数 | 结果 |
| --- | ---: | --- |
| auth_accounts.rs | 6 | 删除包装和旧 tests 挂载；mod.rs 直接重导出领域模块 |
| auth_accounts_tests.rs | 138 | 全部迁入 app-provider-services/src/host_migration_49_tests.rs |
| data_import.rs | 172 | 删除无调用的 PreparedCategory、category、commit 和 ImportCommit/commit_selected 转发 |
| mod.rs | 18 | 保留7行：mcp真实宿主适配挂载、七个领域模块兼容入口与macOS条件 |
| preferences_import.rs | 2 | 删除，改父模块直接重导出 |
| prompts_store.rs | 2 | 同上 |
| shared_snapshot.rs | 2 | 同上，保留macOS条件 |
| shared_snapshot_import.rs | 2 | 同上，保留macOS条件 |
| store.rs | 2 | 同上 |
| usage_settings.rs | 2 | 同上 |

全部来源346→7行，净减339行（98.0%）。原138行测试、4行测试注册、204行其余源码；后者含文档、空行、兼容转发和172行闲置装配，不能计为真实生产逻辑迁移。现为7行宿主模块装配/兼容导出、零本组测试行。领域新增138行测试和3行条件注册，真实生产算法新增0行。领域全库既有生产/测试行数不作为本轮收益；本任务分项增量分别为0/141行。

## 符号、测试与真实所有者

三个辅助函数 account_scope、private_credentials_root、insert_default_fixture 整体移动，保留真实700权限临时目录、真实凭据文件、真实SQLite/PostgreSQL，不模拟存储。assert_scoped_account_removal 整体移动。

原宿主 agent_apps::auth_accounts::tests 下三个测试，分别映射到 app_provider_services::host_migration_49_tests：

- auth6_remove_store_sqlite
- auth6_remove_store_postgres
- auth6_remove_store_rolls_back_on_sqlite_and_file_errors

断言及函数体原样保留；仅改三类import：AuthAccountStore/Scope/Metadata从agent_app_services读取、GlobalDatabase及PostgresTestDatabase从agent_database读取、AppProviderStore从当前库读取。

测试真实调用 AppProviderStore::remove_auth_account（app-provider-services/src/account_removal.rs），该入口依赖 agent-app-services::AuthAccountStore 完成账户元数据、默认账户和凭据删除。因此实际接收者选择已有直接所有者 app-provider-services，而非使 agent-app-services 反向依赖 app-provider-services。接收库lib.rs自注册，条件与原宿主一致为test+macOS；原宿主文件和path挂载删除。现有dev依赖已含agent-database/test-support、tempfile及所需API依赖，无需更改Cargo.toml/lock。

保留SQLite与PostgreSQL持久化重开、邻域隔离、默认账户选择、重复删除NotFound、物理凭据删除、SQLite触发器失败回滚、符号链接文件失败回滚及恢复后删除全部场景。

PreparedCategory的Providers/Mcp/Rules/Skills/Profiles/Preferences/Usage七分支及category/commit，经当前全crates源码检索，仅在其自身定义中出现，没有构造者或调用者。删除闲置适配而不为无人使用的枚举新增领域依赖；既有领域data_import::PreparedImport、commit_selected、ImportCommit保持原样，既有各领域迁移入口不变。这是死装配清理，不是生产职责迁移收益。

兼容公开路径仍由父模块直接重导出同一个agent_app_services模块。当前mcp/probe_jobs.rs等调用者仍可沿agent_apps::auth_accounts访问原类型；vw-agent已依赖agent-app-services和app-provider-services，依赖方向没有反转。

任务1 evidence/external-mounts.json 中agent_apps唯一跨目录边在mcp/revocation_tests.rs，归属其他文件族；本组没有遗漏的跨目录测试挂载。

## 静态核对与未来验证

已阅读全部十个来源文件、领域测试所用生产入口及依赖声明，核对当前仓库PreparedCategory及data_import兼容路径无外部引用；迁后检索三个原测试名仅出现新领域文件，注册不通过宿主path加载。git diff --check仅检查本任务文件。未运行编译/行为检查，无法宣称构建耗时收益。

未来macOS使用真实测试环境（PostgresTestDatabase所需环境沿用已有夹具）：

`python3 scripts/cargo_slot.py cargo test -p app-provider-services --lib host_migration_49_tests -- --test-threads=1`

预期3个测试，必须确认ran>0、failed=0；无PostgreSQL环境时不得宣称三项通过。

`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib agent_apps::auth_accounts::tests -- --list`

仅用于证明原3项宿主挂载消失，零项不是行为通过证据。未向agent-app-services或agent-mcp增加重复测试模块，不执行其零项过滤。没有本组宿主薄测试需要保留：断言全部面向真实领域存储入口，而非宿主授权或生命周期。
