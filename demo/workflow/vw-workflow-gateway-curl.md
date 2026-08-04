# VibeWindow Workflow Gateway Demo

启动网关：

```bash
cargo run -p vw-cli --bin vibewindow -- gateway --host 127.0.0.1 --port 42617
```

直接传 Workflow 内容运行：

```bash
curl -sS -X POST "http://127.0.0.1:42617/v1/workflow/applications/chat-messages" \
  -H "Content-Type: application/json" \
  --data-binary @demo/vw-workflow-gateway-request.json
```

保存为本地应用并获取 UUID：

```bash
APPLICATION_UUID=$(
  jq '{
    name: "Gateway workflow demo",
    description: "Run workflow by application UUID",
    workflow_yaml: .application_workflow
  }' demo/vw-workflow-gateway-request.json \
  | curl -sS -X POST "http://127.0.0.1:42617/v1/workflow/applications" \
      -H "Content-Type: application/json" \
      --data-binary @- \
  | jq -r '.uuid'
)

printf '%s\n' "$APPLICATION_UUID"
```

通过路径 UUID 运行：

```bash
jq 'del(.application_uuid, .application_workflow)' demo/vw-workflow-gateway-request.json \
| curl -sS -X POST "http://127.0.0.1:42617/v1/workflow/applications/${APPLICATION_UUID}/chat-messages" \
    -H "Content-Type: application/json" \
    --data-binary @-
```

通过集合接口指定应用 UUID 运行：

```bash
jq --arg uuid "$APPLICATION_UUID" 'del(.application_workflow) + {application_uuid: $uuid}' demo/vw-workflow-gateway-request.json \
| curl -sS -X POST "http://127.0.0.1:42617/v1/workflow/applications/chat-messages" \
    -H "Content-Type: application/json" \
    --data-binary @-
```

Postman 不会执行 `$(jq ...)` 这类 shell 表达式。导入 Postman 时使用：

```text
demo/vw-workflow-gateway.postman_collection.json
```

如果手动新建 Postman 请求：

- Method: `POST`
- URL: `http://127.0.0.1:42617/v1/workflow/applications/chat-messages`
- Headers: `Content-Type: application/json`
- Body: `raw` / `JSON`
- Body 内容使用 `demo/vw-workflow-gateway-request.json`

如果启用了配对认证，先配对拿 token：

```bash
curl -sS -X POST "http://127.0.0.1:42617/v1/pair" \
  -H "X-Pairing-Code: <PAIRING_CODE>"
```

然后在调用里补 header：

```bash
-H "Authorization: Bearer <TOKEN>"
```
