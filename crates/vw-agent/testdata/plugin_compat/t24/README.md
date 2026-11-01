# Task 24 local form fixture

Original repository fixture, MIT. Synthetic values only; no credentials or user data.
`server.py` speaks the frozen MCP 2025-06-18 stdio protocol and requests one required integer `age` (18–99), using request ID 91 on each independent connection.
Expected replies: accept with numeric age 24/42, decline without content, cancel without content, and invalid-params for age 17.
Reference symbols: Codex `codex-rs/codex-mcp/src/elicitation.rs` request/responder handling; rmcp 0.8.5 `CreateElicitationRequestParam`, `CreateElicitationResult`, `ElicitationSchema`.
User replies alone are supplied at the existing question boundary; transport, process, initialization, server requests and wire replies are real.
The fixture does not prove actual desktop rendering or public/hosted service compatibility.
