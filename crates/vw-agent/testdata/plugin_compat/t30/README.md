# Task 30 bridge fixture

Original test HTML authored for this repository; covered by the repository MIT license.
No third-party package, tokens or real user content is included.

`bridge.html` sends standard JSON-RPC `ui/initialize`, then `ui/notifications/initialized`, then an unauthorized `tools/call`. Expected fields: protocol version `2026-01-26`, tool `forbidden`, empty arguments, request IDs 1 and 2. Theme, long Chinese content and cancellation support manual native WebView verification. `fixture/cancel` is a deliberately unsupported fixture notification, never a production capability.

Reference: MCP Apps official overview, lifecycle and security sections, https://apps.extensions.modelcontextprotocol.io/api/documents/Overview.html (read 2026-11-01). This fixture tests the implemented bridge subset, not full MCP Apps compatibility.

`server.py` extends the repository t29 synthetic stdio server with a temporary `calls.log` side effect. The test retains the same real connection and checks that rejection adds no call. Resource JSON is copied unchanged from the repository t29 fixture into a temporary directory. Real approval decisions use the production ApprovalManager with supervised policy; only the user response is supplied by the test.
