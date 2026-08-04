# AI-DATA 业务数据访问测试

## 当前可用性

AI-DATA 网关路由已经接入，外部路径为 `/v1/data/...`，模块内部路由为 `/data/...`。

当前可用能力：

- 设置读取与更新。
- 数据连接增删改查、激活、测试。
- 数据连接目录读取。
- 报表配置增删改查。
- 结构化业务数据查询。
- 自然语言业务数据查询。

数据源支持状态：

| 数据源 | 连接测试 | 目录读取 | 结构化查询 | 备注 |
| --- | --- | --- | --- | --- |
| SQLite | 可用 | 可用 | 可用 | 推荐作为本地冒烟测试数据源。 |
| HTTP | 可用 | 可用 | 可用 | 目录来自 `schema_hint`、`default_path` 和 header key。 |
| Cube | 可用 | 可用 | 可用 | 依赖 Cube `/meta` 和 `/load` 接口。 |
| MySQL | 取决于 `memory-mariadb` feature | 暂未接入 | 取决于 `memory-mariadb` feature | 未启用 feature 时会明确报错。 |
| PostgreSQL | 取决于 `memory-postgres` feature | 暂未接入 | 取决于 `memory-postgres` feature | 未启用 feature 时会明确报错。 |

安全边界：

- SQL 只允许只读 `SELECT` 或 `WITH` 查询。
- SQL 会拒绝潜在写入或危险关键字，如 `insert`、`update`、`delete`、`drop`、`alter`、`truncate`、`attach`、`detach`、`replace`、`pragma`、`create`。
- AI 查询生成的 `connection_id` 必须仍然等于请求上下文解析出的连接，避免模型越权切换数据源。
- 连接中的 `connection_url`、`auth_token` 和 header value 会走密钥存储加密。

## 前置条件

下面示例假设网关地址是：

```bash
BASE_URL=http://127.0.0.1:8080/v1
```

如果本地实际端口不同，替换 `BASE_URL` 即可。所有写入类请求都需要 `Content-Type: application/json`。

如果网关启用了配对、鉴权或租户头，需要按当前部署配置补充对应 header。

## 1. 设置接口

读取设置：

```bash
curl "$BASE_URL/data/settings"
```

期望返回：

```json
{
  "schema_version": 1,
  "default_limit": 100,
  "default_timeout_secs": 30
}
```

更新设置：

```bash
curl -X PUT "$BASE_URL/data/settings" \
  -H 'Content-Type: application/json' \
  -d '{"default_limit":50,"default_timeout_secs":10}'
```

边界行为：

- `default_limit` 会被限制在 `1..=10000`。
- `default_timeout_secs` 会被限制在 `1..=600`。
- `POST /data/settings` 应返回 `405 Method Not Allowed`。

## 2. SQLite 业务数据测试

先准备一个最小业务库：

```bash
sqlite3 /tmp/vw-ai-data-demo.db <<'SQL'
DROP TABLE IF EXISTS orders;
CREATE TABLE orders (
  id INTEGER PRIMARY KEY,
  customer TEXT NOT NULL,
  amount REAL NOT NULL,
  status TEXT NOT NULL,
  created_at TEXT NOT NULL
);
INSERT INTO orders(customer, amount, status, created_at) VALUES
  ('alice', 120.5, 'paid', '2026-07-01'),
  ('bob', 80, 'pending', '2026-07-02'),
  ('alice', 35, 'paid', '2026-07-03');
SQL
```

创建 SQLite 连接：

```bash
curl -X POST "$BASE_URL/data/connections" \
  -H 'Content-Type: application/json' \
  -d '{
    "name": "本地订单 SQLite",
    "kind": "sqlite",
    "sqlite_path": "/tmp/vw-ai-data-demo.db",
    "read_only": true,
    "schema_hint": "orders(id, customer, amount, status, created_at)"
  }'
```

记录响应中的 `id`：

```bash
CONNECTION_ID=<上一步返回的 id>
```

测试连接：

```bash
curl -X POST "$BASE_URL/data/connections/$CONNECTION_ID/test"
```

期望 `ok: true`，message 类似 `SQLite OK: /tmp/vw-ai-data-demo.db`。

读取目录：

```bash
curl "$BASE_URL/data/connections/$CONNECTION_ID/catalog"
```

期望能看到表与字段信息。

直接查询业务数据：

```bash
curl -X POST "$BASE_URL/data/query" \
  -H 'Content-Type: application/json' \
  -d "{
    \"connection_id\": \"$CONNECTION_ID\",
    \"query_kind\": \"sql\",
    \"sql\": \"SELECT customer, SUM(amount) AS total FROM orders WHERE status = {{status}} GROUP BY customer ORDER BY total DESC\",
    \"params\": {\"status\":\"paid\"},
    \"page\": 1,
    \"limit\": 10,
    \"count\": \"enabled\"
  }"
```

期望返回分页对象 `page` 和业务行 `items`。

只查总数：

```bash
curl -X POST "$BASE_URL/data/query" \
  -H 'Content-Type: application/json' \
  -d "{
    \"connection_id\": \"$CONNECTION_ID\",
    \"query_kind\": \"sql\",
    \"sql\": \"SELECT * FROM orders\",
    \"count_sql\": \"SELECT COUNT(*) FROM orders\",
    \"count\": \"only\"
  }"
```

期望 `items` 为空或不包含业务列表，`page.total_record` 有总数。

危险 SQL 测试：

```bash
curl -X POST "$BASE_URL/data/query" \
  -H 'Content-Type: application/json' \
  -d "{
    \"connection_id\": \"$CONNECTION_ID\",
    \"query_kind\": \"sql\",
    \"sql\": \"DELETE FROM orders\"
  }"
```

期望返回 `400 Bad Request`，错误包含“仅允许只读 SELECT/CTE 查询”。

## 3. HTTP 业务数据测试

HTTP 数据源适合把已有业务 API 包装成统一查询入口。

创建 HTTP 连接：

```bash
curl -X POST "$BASE_URL/data/connections" \
  -H 'Content-Type: application/json' \
  -d '{
    "name": "业务订单 HTTP",
    "kind": "http",
    "base_url": "https://example.internal",
    "default_path": "/api/orders",
    "auth_token": "Bearer <token>",
    "headers": {
      "X-App": "vibewindow"
    },
    "schema_hint": "GET /api/orders?status=paid 返回 {items,total}"
  }'
```

测试连接：

```bash
curl -X POST "$BASE_URL/data/connections/$CONNECTION_ID/test"
```

行为：

- 默认对 `base_url + default_path` 发起 `GET`。
- 非 2xx 返回会变成 `ok: false`，并截断响应体。

HTTP GET 查询：

```bash
curl -X POST "$BASE_URL/data/query" \
  -H 'Content-Type: application/json' \
  -d "{
    \"connection_id\": \"$CONNECTION_ID\",
    \"query_kind\": \"http\",
    \"http_method\": \"GET\",
    \"http_path\": \"/api/orders\",
    \"params\": {\"status\":\"paid\"},
    \"limit\": 20
  }"
```

HTTP POST 查询：

```bash
curl -X POST "$BASE_URL/data/query" \
  -H 'Content-Type: application/json' \
  -d "{
    \"connection_id\": \"$CONNECTION_ID\",
    \"query_kind\": \"http\",
    \"http_method\": \"POST\",
    \"http_path\": \"/api/orders/search\",
    \"http_body\": {\"status\":\"paid\"},
    \"limit\": 20
  }"
```

响应归一化规则：

- JSON 数组会作为 `items`。
- JSON 对象里的 `data`、`items` 或 `list` 会作为 `items`。
- `total`、`count` 或 `page.total_record` 会作为总数。
- 其他 JSON 或文本会包装为单条结果。

不支持方法测试：

```bash
curl -X POST "$BASE_URL/data/query" \
  -H 'Content-Type: application/json' \
  -d "{
    \"connection_id\": \"$CONNECTION_ID\",
    \"query_kind\": \"http\",
    \"http_method\": \"PUT\",
    \"http_path\": \"/api/orders/1\"
  }"
```

期望返回 `400 Bad Request`，错误包含“仅支持 GET/POST/HEAD”。

## 4. Cube 业务数据测试

创建 Cube 连接：

```bash
curl -X POST "$BASE_URL/data/connections" \
  -H 'Content-Type: application/json' \
  -d '{
    "name": "Cube 指标服务",
    "kind": "cube",
    "base_url": "https://cube.example.internal",
    "default_path": "/cubejs-api/v1",
    "auth_token": "<cube token>",
    "schema_hint": "Orders.count, Orders.amount, Orders.status"
  }'
```

测试连接：

```bash
curl -X POST "$BASE_URL/data/connections/$CONNECTION_ID/test"
```

行为：

- 测试连接会访问 `GET /cubejs-api/v1/meta`。
- 目录读取也会访问 `GET /cubejs-api/v1/meta`。

Cube 查询：

```bash
curl -X POST "$BASE_URL/data/query" \
  -H 'Content-Type: application/json' \
  -d "{
    \"connection_id\": \"$CONNECTION_ID\",
    \"query_kind\": \"cube\",
    \"cube_query\": {
      \"measures\": [\"Orders.count\"],
      \"dimensions\": [\"Orders.status\"]
    },
    \"limit\": 100
  }"
```

期望返回 Cube load API 的数据结果，并归一化为 `items`。

## 5. MySQL 和 PostgreSQL 测试

创建 MySQL 连接：

```bash
curl -X POST "$BASE_URL/data/connections" \
  -H 'Content-Type: application/json' \
  -d '{
    "name": "业务 MySQL",
    "kind": "mysql",
    "connection_url": "mysql://readonly:password@127.0.0.1:3306/app"
  }'
```

创建 PostgreSQL 连接：

```bash
curl -X POST "$BASE_URL/data/connections" \
  -H 'Content-Type: application/json' \
  -d '{
    "name": "业务 PostgreSQL",
    "kind": "postgres",
    "connection_url": "postgres://readonly:password@127.0.0.1:5432/app"
  }'
```

当前判断方式：

- 未启用 `memory-mariadb` 时，MySQL 测试/查询会返回“当前构建未启用 MySQL 执行器”。
- 未启用 `memory-postgres` 时，PostgreSQL 测试/查询会返回“当前构建未启用 PostgreSQL 执行器”。
- 即使启用 feature，MySQL/PostgreSQL 的 catalog 仍返回“目录读取暂未接入 AI-DATA catalog”。

只读查询示例：

```bash
curl -X POST "$BASE_URL/data/query" \
  -H 'Content-Type: application/json' \
  -d "{
    \"connection_id\": \"$CONNECTION_ID\",
    \"query_kind\": \"sql\",
    \"sql\": \"SELECT id, name FROM customers ORDER BY id DESC\",
    \"limit\": 20
  }"
```

## 6. 报表配置测试

报表把业务数据源配置固化下来，后续查询只需要传 `report_id` 或 `slug`。

创建报表：

```bash
curl -X POST "$BASE_URL/data/reports" \
  -H 'Content-Type: application/json' \
  -d "{
    \"name\": \"订单概览\",
    \"slug\": \"orders-overview\",
    \"data_source\": \"sql\",
    \"default_source_key\": \"orders\",
    \"report_config\": {
      \"title\": \"订单概览\",
      \"default_order_by\": \"created_at DESC\"
    },
    \"sources\": [{
      \"source_key\": \"orders\",
      \"connection_id\": \"$CONNECTION_ID\",
      \"query_kind\": \"sql\",
      \"sql\": \"SELECT id, customer, amount, status, created_at FROM orders WHERE status = {{status}}\",
      \"count_sql\": \"SELECT COUNT(*) FROM orders WHERE status = {{status}}\"
    }]
  }"
```

通过报表 slug 查询：

```bash
curl -X POST "$BASE_URL/data/query" \
  -H 'Content-Type: application/json' \
  -d '{
    "report_id": "orders-overview",
    "params": {"status":"paid"},
    "page": 1,
    "limit": 10,
    "count": "enabled"
  }'
```

更新报表：

```bash
curl -X PUT "$BASE_URL/data/reports/$REPORT_ID" \
  -H 'Content-Type: application/json' \
  -d "{
    \"name\": \"订单概览新版\",
    \"slug\": \"orders-overview\",
    \"data_source\": \"sql\",
    \"default_source_key\": \"orders\",
    \"report_config\": {},
    \"sources\": [{
      \"source_key\": \"orders\",
      \"connection_id\": \"$CONNECTION_ID\",
      \"query_kind\": \"sql\",
      \"sql\": \"SELECT * FROM orders\"
    }]
  }"
```

删除报表：

```bash
curl -X DELETE "$BASE_URL/data/reports/$REPORT_ID"
```

报表失败场景：

- `name` 为空：`400 Bad Request`。
- `slug` 为空：`400 Bad Request`。
- `sources` 为空：`400 Bad Request`。
- `slug` 重复：`400 Bad Request`。
- 查询不存在的 `report_id`：`400 Bad Request`。
- 删除不存在的报表：`404 Not Found`。

## 7. 自然语言查询测试

自然语言查询会先让模型根据连接、报表、目录和用户问题生成执行计划，再执行查询。

基于连接查询：

```bash
curl -X POST "$BASE_URL/data/ai/query" \
  -H 'Content-Type: application/json' \
  -d "{
    \"connection_id\": \"$CONNECTION_ID\",
    \"prompt\": \"统计每个客户已支付订单总金额，按金额倒序\",
    \"limit\": 10
  }"
```

基于报表查询：

```bash
curl -X POST "$BASE_URL/data/ai/query" \
  -H 'Content-Type: application/json' \
  -d '{
    "report_id": "orders-overview",
    "prompt": "列出已支付订单，并总结总金额",
    "params": {"status":"paid"},
    "limit": 10
  }'
```

期望响应：

```json
{
  "plan": {},
  "result": {},
  "answer": "可选中文总结",
  "raw_model_response": "模型原始计划"
}
```

失败场景：

- `prompt` 缺失或类型不匹配：`422 Unprocessable Entity`。
- 未提供 `report_id` 或 `connection_id`：`400 Bad Request`。
- 模型返回非法 JSON：`400 Bad Request`。
- 模型返回其他连接的 `connection_id`：`400 Bad Request`。
- 底层查询失败：`400 Bad Request`。

## 8. 通用路由失败场景

路由层已覆盖以下行为：

| 场景 | 期望 |
| --- | --- |
| `GET /data/settings` | `200 OK` |
| `GET /data/connections` | `200 OK` |
| `GET /data/reports` | `200 OK` |
| 未知路径 | `404 Not Found` |
| 路径尾部多 `/` | `404 Not Found` |
| JSON 接口缺少 `Content-Type: application/json` | `415 Unsupported Media Type` |
| JSON 格式错误 | `400 Bad Request` |
| 不支持的 HTTP method | `405 Method Not Allowed` |
| 查询缺少 `connection_id` 且缺少 `report_id` | `400 Bad Request` |
| 不存在的连接详情、激活、测试、目录、删除 | `404 Not Found` |

## 9. 建议的最小验收顺序

1. `GET /v1/data/settings` 确认路由可达。
2. 创建 SQLite 测试库。
3. `POST /v1/data/connections` 创建 SQLite 连接。
4. `POST /v1/data/connections/{connection_id}/test` 确认连接可用。
5. `GET /v1/data/connections/{connection_id}/catalog` 确认目录可读。
6. `POST /v1/data/query` 执行只读 SQL。
7. 创建报表并用 `report_id` 或 `slug` 查询。
8. 配好模型 provider 后再测 `POST /v1/data/ai/query`。

