# Codex CLI 验收

```bash
curl -sS -N --max-time 180 -H 'Content-Type: application/json' \
  -H 'X-Platform-ID: 10000' \
  -H 'X-Company-ID: 100000000' \
  -H 'X-Directory: /Users/xiangmin/worker/code/ruejs' \
  --data-binary '{"model":"alibaba-token-plan-cn/deepseek-v4.1-flash-1017","agent":"acp-codex","acp_agent":"codex","acp_model_source":"session_model","options":{"cwd":"/Users/xiangmin/worker/code/ruejs","full_access":true},"messages":[{"role":"user","content":"帮我用 Rue.js 实现一个纯文本编辑器，支持输入、实时显示字数、刷新后恢复内容。只写到 acpdemo/codex/，不修改目录外文件。使用本仓库的 @rue-js/rue；在目标目录内提供可运行页面与运行说明。"}]}' \
  http://127.0.0.1:42617/v1/chat/stream
```
