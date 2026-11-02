# Task 58 read-only dependency diagnostics

Repository-authored fixtures (repository license) in t58_tests.rs: Agent Plugins 1.0.0 plugin/mcp manifests, a deterministic nonexistent executable, a bundled JavaScript entry containing a sentinel write, and a bundled Python module with a sentinel write. None of those package commands/imports execute. Original files and another package root remain unchanged. Unsafe parent traversal and dependency symlinks are rejected.

Reference: frozen core-plugins/loader.rs, core/src/plugins/render.rs and skills/metadata.rs::resolve_dependency_tool. Existing host MCP declaration parsing and skill root/metadata resolution are reused.

Observations distinguish executable presence, missing host executable, missing package file, unverified dynamic/module/service dependencies, and excluded hosted connectors. Python module presence is inspected as files, never through imports. Legacy unresolved variables, custom/unknown working directories and complex launcher arguments remain unverified. Remote endpoints/authentication are not probed by a static read. Declared tool dependencies remain unverified until an actual host mapping or service is established; tool names never grant permission. Public skills/MCP/Apps are not collectively classified as private services.

The real MCP inspection path checks the resolved executable first and reports missing PATH/runtime without attempting a connection. Gateway catalog/detail/previews carry the observations; preview dependency status distinguishes missing/unverified/presence-only/no-declarations and does not claim complete compatibility.

User explicitly authorized executing this task before 55–57 on 2026-11-02. No session-snapshot implementation from 57 is required to perform these read-only filesystem and executable observations.
