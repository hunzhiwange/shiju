# 会话存储

原生桌面会话与调度器共用用户级全局数据库。缺少 `database/config.json` 时使用 SQLite；配置
`{"provider":"postgres","schema":"vibewindow"}` 时使用 PostgreSQL，连接串只从
`VIBEWINDOW_DATABASE_URL` 读取。URL 必须显式选择 `sslmode=require` 或 `sslmode=disable`，前者使用
WebPKI 根证书验证服务端。Debug/Release 配置分别位于
`~/.shiju-agent-dev/database/config.json` 与 `~/.shiju-agent/database/config.json`；首次成功选择会在进程内
固定，修改配置或 URL 后必须重启。

## 数据模型

当前 schema 只包含以下 6 张会话表：

- `session_index`
- `session_messages`
- `session_calls`
- `session_steps`
- `session_todos`
- `session_agent_sessions`

每张表都以 `platform_id/company_id/project_id` 作为项目作用域，并通过复合外键关联
`projects`。会话消息、调用、步骤和 TODO 还通过复合外键关联 `session_index`，删除会话时
只级联删除同一项目内的子记录。

## 边界

原生读写只打开共享数据库并直接初始化最新 schema，不读取、导入或迁移旧会话数据库。
缺少项目作用域、项目不存在、外键不满足或 JSON 损坏时，操作明确失败，不回退到其他路径。

SQLite 与 PostgreSQL 数据完全隔离；运行时只使用所选 provider，不跨端读取、回退、复制、合并、迁移或
双写。切换后所需的数据迁移与校验由用户负责，旧端数据会保留但不再读取。错误不得输出数据库 URL、密码、
令牌或原始敏感载荷。

`storage/session` 仍用于步骤快照、原始 LLM JSON 和项目作用域元数据等文件制品；这些文件
不承担会话索引、会话正文、TODO 或代理会话元数据的数据库职责。WASM 存储保持独立实现。
