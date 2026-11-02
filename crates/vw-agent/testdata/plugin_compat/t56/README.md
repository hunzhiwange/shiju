# Task 56 fixture

Repository-authored fixtures use the repository license. No external package or credentials are stored.

`plugins/compat/plugin_window_fixture.rs` creates isolated, original Codex-format packages at long Unicode paths: `fixture@catalog`, and `newcomer` versions 2 and 3. Version 3 adds one public HTTPS MCP declaration. Both newcomer packages declare an untrusted command hook whose sentinel must remain absent. The fixture runs the authenticated production gateway router; the Iced window drives production settings messages and gateway client operations.

Expected flow: invalid source → visible failure → retry preview; cancel leaves no installed newcomer; receiving a preview never publishes; explicit confirmation installs version 2, then a second preview shows old/new declarations and explicit confirmation updates to version 3. Light/dark screenshots exercise actual glyph shaping. Original package bytes and sentinel absence are checked after the window exits.

Reference controls: `system_settings_providers/custom_model_modal.rs`, `system_settings_common/`. Test inputs are synthetic and contain no user data. Public network and provider calls are unnecessary for this local installation story; the HTTPS declaration remains unverified and is never contacted.
