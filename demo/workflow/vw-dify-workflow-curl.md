# Dify Workflow Demo

这个 demo 使用 `demo/vw-dify-workflow-demo.yml`，来源于 `crates/vw-desktop/src/app/dify-workflow.yml`。

## 准备

启动网关：

```bash
cargo run -p vw-cli --bin vibewindow -- gateway --host 127.0.0.1 --port 42617
```

需要定位节点执行、Python `requests` 外部 API 请求、请求参数和响应预览时，用 debug 日志启动：

```bash
RUST_LOG=info,vw_agent::workflow=debug,vw_agent::workflow::http_bridge=debug,vw_agent::workflow::http_request=debug \
cargo run -p vw-cli --bin vibewindow -- gateway --host 127.0.0.1 --port 42617
```

日志会脱敏 `dhb_skey`、token、authorization、cookie 等敏感字段，并截断过长响应。

把请求里的 `dhb_skey` 替换成真实值，并补齐 Chat Messages 运行所需字段：

```bash
jq '
  .inputs.dhb_skey = "f06a4521dcdf0de320b828ab0c20638c"
  | .inputs.max_steps = (.max_steps // 200)
  | del(.max_steps)
  | .response_mode = "blocking"
  | .user = "demo-user"
' \
  demo/vw-dify-workflow-request.json > /tmp/vw-dify-workflow-request.json
```

`current_env` 可选值：

- `release`: 使用 `https://yadmin.example.com`
- `gray`: 使用灰度管理 API
- 其它值: 使用 `https://admin.example.com`

## 直接传 Workflow 内容运行

`/v1/workflow/applications/chat-messages` 支持在请求体顶层传入 `application_workflow`，无需先保存到本地数据库。

```bash
curl -sS -X POST "http://127.0.0.1:42617/v1/workflow/applications/chat-messages" \
  -H "Content-Type: application/json" \
  --data-binary @/tmp/vw-dify-workflow-request.json
```

也可以临时从 YAML 生成请求：

```bash
jq -Rs \
  --arg query "查询昨天的订单" \
  --arg skey "f06a4521dcdf0de320b828ab0c20638c" \
  '{
    application_workflow: .,
    query: $query,
    inputs: {
      dhb_skey: $skey,
      current_env: "release",
      max_steps: 200
    },
    response_mode: "blocking",
    user: "demo-user"
  }' demo/vw-dify-workflow-demo.yml \
| curl -sS -X POST "http://127.0.0.1:42617/v1/workflow/applications/chat-messages" \
  -H "Content-Type: application/json" \
  --data-binary @-
```

```bash
jq -Rs \
  --arg query "查询昨天的订单" \
  --arg skey "f06a4521dcdf0de320b828ab0c20638c" \
  '{
    application_workflow: .,
    query: $query,
    inputs: {
      dhb_skey: $skey,
      current_env: "admin",
      max_steps: 200
    },
    response_mode: "blocking",
    user: "demo-user"
  }' demo/vw-dify-workflow-demo.yml \
| curl -sS -X POST "http://127.0.0.1:42617/v1/workflow/applications/chat-messages" \
  -H "Content-Type: application/json" \
  --data-binary @-
```

## 保存到本地数据库并按 UUID 运行

桌面端保存到本地数据库后，会为当前应用生成 UUID；也可以在“编辑应用信息”里复制当前应用的 UUID。

命令行保存同一个 demo YAML：

```bash
WORKFLOW_UUID=$(
  jq -Rs \
    --arg name "Dify 订单查询 Demo" \
    --arg description "本地 SQLite 保存示例" \
    '{
      name: $name,
      description: $description,
      workflow_yaml: .
    }' demo/vw-dify-workflow-demo.yml \
  | curl -sS -X POST "http://127.0.0.1:42617/v1/workflow/applications" \
      -H "Content-Type: application/json" \
      --data-binary @- \
  | jq -r '.uuid'
)

echo "$WORKFLOW_UUID"
```

查看本地已保存的 workflow 列表：

```bash
curl -sS "http://127.0.0.1:42617/v1/workflow/applications" | jq .
```

按 UUID 获取 workflow：

```bash
curl -sS "http://127.0.0.1:42617/v1/workflow/applications/$WORKFLOW_UUID" | jq .
```

推荐把 UUID 放在 path 里运行，body 只放运行参数：

```bash
jq -n \
  --arg query "查询今天的订单" \
  --arg skey "f06a4521dcdf0de320b828ab0c20638c" \
  '{
    query: $query,
    inputs: {
      dhb_skey: $skey,
      current_env: "admin",
      max_steps: 200
    },
    response_mode: "blocking",
    user: "demo-user"
  }' \
| curl -sS -X POST "http://127.0.0.1:42617/v1/workflow/applications/$WORKFLOW_UUID/chat-messages" \
  -H "Content-Type: application/json" \
  --data-binary @-
```

也可以通过集合入口运行，UUID 放在 body 顶层的 `application_uuid` 字段。

```bash
jq -n \
  --arg uuid "$WORKFLOW_UUID" \
  --arg query "查询今天的订单" \
  --arg skey "f06a4521dcdf0de320b828ab0c20638c" \
  '{
    application_uuid: $uuid,
    query: $query,
    inputs: {
      dhb_skey: $skey,
      current_env: "admin",
      max_steps: 200
    },
    response_mode: "blocking",
    user: "demo-user"
  }' \
| curl -sS -X POST "http://127.0.0.1:42617/v1/workflow/applications/chat-messages" \
  -H "Content-Type: application/json" \
  --data-binary @-
```

Chat Messages 返回也推荐把 UUID 放在 path 里：

```bash
jq -n \
  --arg query "查询商品库存" \
  --arg skey "f06a4521dcdf0de320b828ab0c20638c" \
  '{
    query: $query,
    inputs: {
      dhb_skey: $skey,
      current_env: "admin"
    },
    response_mode: "blocking",
    user: "demo-user"
  }' \
| curl -sS -X POST "http://127.0.0.1:42617/v1/workflow/applications/$WORKFLOW_UUID/chat-messages" \
  -H "Content-Type: application/json" \
  --data-binary @-
```

```bash
jq -n \
  --arg query "查询商品库存" \
  --arg skey "f06a4521dcdf0de320b828ab0c20638c" \
  '{
    query: $query,
    inputs: {
      dhb_skey: $skey,
      current_env: "admin"
    },
    response_mode: "streaming",
    user: "demo-user"
  }' \
| curl -N -sS -X POST "http://127.0.0.1:42617/v1/workflow/applications/$WORKFLOW_UUID/chat-messages" \
  -H "Content-Type: application/json" \
  --data-binary @-
```

## 调用 Chat Messages

`/v1/workflow/applications/chat-messages` 是兼容 Dify Chat Messages 的集合入口，内部会跑这个 workflow。请求体用 `response_mode` 控制阻塞或流式返回；不传 `current_env` 时默认走 `https://admin.example.com`。直接调用集合入口时，请求必须在顶层提供 `application_uuid` 或 `application_workflow`；调用 `/v1/workflow/applications/{uuid}/chat-messages` 时不需要在 body 里重复传 UUID。

支持不同数据查询时只需要替换 `query`，常用示例：

- `查询昨天的订单`
- `查询最近7天的退货单`
- `查询商品`
- `查询商品库存`
- `查询客户`

阻塞返回：

```bash
jq -n \
  --arg query "查询商品库存" \
  --arg skey "f06a4521dcdf0de320b828ab0c20638c" \
  '{
    query: $query,
    inputs: {
      dhb_skey: $skey,
      current_env: "admin"
    },
    response_mode: "blocking",
    user: "demo-user"
  }' \
| curl -sS -X POST "http://127.0.0.1:42617/v1/workflow/applications/$WORKFLOW_UUID/chat-messages" \
  -H "Content-Type: application/json" \
  --data-binary @-
```

流式返回：

```bash
jq -n \
  --arg query "查询商品" \
  --arg skey "f06a4521dcdf0de320b828ab0c20638c" \
  '{
    query: $query,
    inputs: {
      dhb_skey: $skey,
      current_env: "admin"
    },
    response_mode: "streaming",
    user: "demo-user"
  }' \
| curl -N -sS -X POST "http://127.0.0.1:42617/v1/workflow/applications/$WORKFLOW_UUID/chat-messages" \
  -H "Content-Type: application/json" \
  --data-binary @-
```

也可以调用集合入口，UUID 放在和 `query` 平级的 `application_uuid`：

```bash
jq -n \
  --arg uuid "$WORKFLOW_UUID" \
  --arg query "查询商品库存" \
  --arg skey "f06a4521dcdf0de320b828ab0c20638c" \
  '{
    query: $query,
    application_uuid: $uuid,
    inputs: {
      dhb_skey: $skey,
      current_env: "admin"
    },
    response_mode: "blocking",
    user: "demo-user"
  }' \
| curl -sS -X POST "http://127.0.0.1:42617/v1/workflow/applications/chat-messages" \
  -H "Content-Type: application/json" \
  --data-binary @-
```

如果要临时指定 workflow YAML，可以把内容放在和 `query` 平级的 `application_workflow`：

```bash
jq -Rs \
  --arg query "查询客户" \
  --arg skey "f06a4521dcdf0de320b828ab0c20638c" \
  '{
    query: $query,
    application_workflow: .,
    inputs: {
      dhb_skey: $skey,
      current_env: "admin"
    },
    response_mode: "blocking",
    user: "demo-user"
  }' demo/vw-dify-workflow-demo.yml \
| curl -sS -X POST "http://127.0.0.1:42617/v1/workflow/applications/chat-messages" \
  -H "Content-Type: application/json" \
  --data-binary @-
```

## Postman

阻塞调用：

- Method: `POST`
- URL: `http://127.0.0.1:42617/v1/workflow/applications/替换为应用UUID/chat-messages`
- Headers: `Content-Type: application/json`
- Body: `raw` / `JSON`

```json
{
  "query": "查询商品库存",
  "inputs": {
    "dhb_skey": "f06a4521dcdf0de320b828ab0c20638c",
    "current_env": "admin"
  },
  "response_mode": "blocking",
  "user": "demo-user"
}
```

流式调用：

- Method: `POST`
- URL: `http://127.0.0.1:42617/v1/workflow/applications/替换为应用UUID/chat-messages`
- Headers: `Content-Type: application/json`
- Body: `raw` / `JSON`

```json
{
  "query": "查询最近7天的退货单",
  "inputs": {
    "dhb_skey": "f06a4521dcdf0de320b828ab0c20638c",
    "current_env": "admin"
  },
  "response_mode": "streaming",
  "user": "demo-user"
}
```

Postman 收流式响应时会看到 `text/event-stream`，每段数据是 `data: {...}`；阻塞响应则是普通 JSON。

## 说明

这个工作流会调用 LLM 节点识别意图并提取参数，还会根据分支访问订单、退单、商品、客户或库存 API。demo 内置 LLM 节点使用 `deepseek-v4-flash`，运行前需要保证本地 provider 配置里有可用 DeepSeek 模型，且 `dhb_skey` 有权限访问对应管理 API。

## 本地试跑结果

已用占位 `dhb_skey` 调过一次，workflow 能进入 runner：

- `start` 节点成功，`dhb_skey` 在响应里会被脱敏为 `[REDACTED]`
- LLM 节点模型已切换为当前支持的 `deepseek-v4-flash`

替换真实 `dhb_skey` 后即可继续跑到后续业务 API 分支。
