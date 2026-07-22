# 切换 PostgreSQL 数据库

VibeWindow 支持 SQLite 和 PostgreSQL 两种全局数据库后端。SQLite 是默认后端；缺少数据库配置文件时，系统继续使用 SQLite。

项目、系统配置、Gateway、任务调度、会话、工作流和知识库共用同一个全局数据库后端。后端选择对整个进程生效，不能只为其中某个模块单独切换。

## 配置位置

不同构建模式使用不同的配置目录：

| 构建模式 | 数据库配置文件 |
| --- | --- |
| Debug | `~/.vibewindowdev/database/config.json` |
| Release | `~/.vibewindow/database/config.json` |

SQLite 数据文件 `vibe-window.sqlite3` 位于对应的 `database` 目录中。

## 切换到 PostgreSQL

### 1. 准备 PostgreSQL

先创建 VibeWindow 使用的数据库和账号，并确保该账号能够连接数据库以及创建、读取和修改所选 schema 中的对象。

连接串中的用户名、密码、主机、端口和数据库名需要替换为实际值。用户名或密码包含特殊字符时，应进行 URL 编码。

### 2. 创建配置文件

Debug 构建执行：

```bash
mkdir -p ~/.vibewindowdev/database
printf '%s\n' '{"provider":"postgres","schema":"vibewindow"}' \
  > ~/.vibewindowdev/database/config.json
```

Release 构建执行：

```bash
mkdir -p ~/.vibewindow/database
printf '%s\n' '{"provider":"postgres","schema":"vibewindow"}' \
  > ~/.vibewindow/database/config.json
```

`schema` 可以省略，省略后默认使用 `vibewindow`：

```json
{"provider":"postgres"}
```

自定义 schema 必须满足以下要求：

- 只能包含英文字母、数字和下划线。
- 首字符必须是英文字母或下划线。
- 长度不超过 63 字节。

### 3. 设置连接串

PostgreSQL 连接串只能通过 `VIBEWINDOW_DATABASE_URL` 环境变量提供，不应写入 `config.json`：

```bash
export VIBEWINDOW_DATABASE_URL='postgres://用户名:密码@主机:5432/数据库名?sslmode=require'

~/.zshenv
export VIBEWINDOW_DATABASE_URL='postgres://your_db_user:your_db_password@127.0.0.1:5432/vibe-window?sslmode=disable'
```

`sslmode` 必须显式设置，并且只能使用：

- `sslmode=require`：验证服务端证书，适合远程或生产数据库。
- `sslmode=disable`：不使用 TLS，只适合可信的本地环境。

本地开发示例：

```bash
export VIBEWINDOW_DATABASE_URL='postgres://vibewindow:password@127.0.0.1:5432/vibewindow?sslmode=disable'
```

如果从桌面图标、系统服务或其他启动器运行 VibeWindow，需要在相应启动环境中设置该变量；仅在某个终端执行 `export`，不会影响从其他环境启动的进程。

### 4. 重启 VibeWindow

数据库后端和连接池会在进程首次成功读取配置时固定。修改配置文件或连接串后，必须完全退出并重新启动 VibeWindow。

启动失败时，优先检查：

- 使用的配置路径是否与 Debug 或 Release 构建匹配。
- `provider` 是否严格写成小写 `postgres`。
- `VIBEWINDOW_DATABASE_URL` 是否对 VibeWindow 进程可见且非空。
- URL 是否使用 `postgres` scheme，并且只包含一个合法的 `sslmode`。
- 数据库账号是否具有所需的连接和 schema 权限。
- 使用 `sslmode=require` 时，服务端证书是否可由 WebPKI 根证书验证。

## 切回 SQLite

将对应的 `config.json` 改为：

```json
{"provider":"sqlite"}
```

也可以删除该配置文件。随后完全重启 VibeWindow。

SQLite 配置不能包含 `schema` 字段。

## 数据迁移与安全说明

- SQLite 与 PostgreSQL 数据完全隔离。
- 切换后不会自动迁移、复制、合并或回退读取原数据库中的数据。
- 如需保留 SQLite 现有数据，切换前应先备份 `vibe-window.sqlite3`，并另行设计迁移流程。
- 不要把包含密码的 PostgreSQL URL 写入仓库、配置文件、日志或问题报告。
- 配置文件只保存数据库类型和可选 schema，连接凭据始终通过运行环境注入。
