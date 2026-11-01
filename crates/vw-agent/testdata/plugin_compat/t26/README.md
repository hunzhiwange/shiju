# OAuth discovery fixtures

Synthetic example.com names and metadata authored for this repository (repository license).
No live identities, credentials or tokens. t26_tests.rs defines the resource/authority pair:
resource=https://mcp.example/mcp; issuer=https://auth.example/tenant; scopes=[read].
Reference: frozen baseline mcp.oauth.discovery, codex-mcp/src/mcp/auth.rs::oauth_login_support.
Safety difference: public HTTPS only, no redirects, no private DNS answers, no credential headers;
ambiguous multiple issuers fail visibly rather than selecting an arbitrary authorization server.
Browser authorization, client registration and refresh remain separate tasks.
