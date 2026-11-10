use super::validation::{ValidationError, parse_native, validate_new_name};
use serde_json::json;
use vw_api_types::mcp_management::{McpConnectionDefinition, McpRecordId};

#[test]
fn cc_mcp_t04_native_default_preserves_stdio_arguments() {
    let connection = parse_native(json!({
        "command": "node",
        "args": ["path with spaces", ""],
        "env": {"MODE": "local"}
    }))
    .unwrap_or_else(|error| panic!("{error:?}"));
    let McpConnectionDefinition::Stdio { command, args, .. } = connection else {
        panic!("missing type must default to stdio");
    };
    assert_eq!(command, "node");
    assert_eq!(args, ["path with spaces", ""]);
}

#[test]
fn cc_mcp_t04_invalid_stdio_shapes_are_rejected() {
    for input in [
        json!({"type": 7, "command": "node"}),
        json!({"command": 7}),
        json!({"command": "node", "args": [7]}),
        json!({"command": "node", "env": {"MODE": 7}}),
        json!({"command": "node", "env": []}),
        json!({"type": "stdio", "url": "http://127.0.0.1/mcp"}),
    ] {
        assert!(parse_native(input).is_err());
    }
    assert_eq!(
        parse_native(json!({"command": "  "})).err(),
        Some(ValidationError::InvalidField("command"))
    );
}

#[test]
fn cc_mcp_t04_remote_aliases_and_url_only_input_are_supported() {
    let url = "http://127.0.0.1:9876/mcp";
    for input in [
        json!({"url": url}),
        json!({"type": "http", "url": url}),
        json!({"type": "streamable-http", "url": url}),
        json!({"type": "streamableHttp", "url": url, "http_headers": {"X-Mode": "local"}}),
    ] {
        assert!(matches!(parse_native(input), Ok(McpConnectionDefinition::StreamableHttp { .. })));
    }
    assert!(matches!(
        parse_native(json!({"type": "sse", "url": url})),
        Ok(McpConnectionDefinition::Sse { .. })
    ));
}

#[test]
fn cc_mcp_t04_remote_rejects_non_http_url_and_malformed_headers() {
    for transport in ["http", "sse"] {
        for url in [
            "",
            "file:///tmp/socket",
            "https://",
            "https:example.org",
            "https:///example.org",
            "https:/example.org/?next=http://example.org",
            "https://a\nb.test",
        ] {
            assert_eq!(
                parse_native(json!({"type": transport, "url": url})).err(),
                Some(ValidationError::InvalidField("url"))
            );
        }
        for headers in [
            json!({"bad key": "value"}),
            json!({"X-Mode": "one\r\nX-Other: two"}),
            json!({"X-Mode": "one", "x-mode": "two"}),
            json!({"X-Mode": true}),
        ] {
            assert_eq!(
                parse_native(json!({
                    "type": transport,
                    "url": "http://127.0.0.1:9876/mcp",
                    "headers": headers
                }))
                .err(),
                Some(ValidationError::InvalidField("headers"))
            );
        }
    }
}

#[test]
fn cc_mcp_t04_native_display_masks_cannot_become_literals() {
    for (input, field) in [
        (json!({"command": "node", "env": {"TOKEN": "••••••••"}}), "env"),
        (
            json!({"type": "sse", "url": "https://example.org/sse", "headers": {"Authorization": "••••••••"}}),
            "headers",
        ),
    ] {
        assert_eq!(parse_native(input).err(), Some(ValidationError::InvalidField(field)));
    }
}

#[test]
fn cc_mcp_t04_catalog_name_rejects_whitespace_and_exact_duplicates() {
    let occupied = [McpRecordId("notes".into())];
    for name in [" ", "two words", "two\u{2003}words", "name\0"] {
        assert_eq!(validate_new_name(name, &occupied), Err(ValidationError::InvalidField("name")));
    }
    assert_eq!(validate_new_name(" notes ", &occupied), Err(ValidationError::NameConflict));
    assert_eq!(validate_new_name(" Notes ", &occupied), Ok(McpRecordId("Notes".into())));
}

#[test]
fn cc_mcp_t04_managed_values_reject_masks_and_accept_account_references() {
    use super::validation::validate_connection;

    for (connection, field) in [
        (
            json!({
                "transport": "stdio",
                "command": "node",
                "env": {"TOKEN": {"kind": "literal", "value": "••••••••"}}
            }),
            "env",
        ),
        (
            json!({
                "transport": "streamable_http",
                "url": "http://127.0.0.1:9876/mcp",
                "headers": {"Authorization": {"kind": "credential", "reference": {
                    "service": "codex_oauth", "accountId": " "
                }}}
            }),
            "headers",
        ),
    ] {
        let connection = serde_json::from_value(connection).unwrap();
        assert_eq!(validate_connection(&connection), Err(ValidationError::InvalidField(field)));
    }
    let connection = serde_json::from_value(json!({
        "transport": "sse",
        "url": "https://example.org/mcp?source=fixture",
        "headers": {"Authorization": {"kind": "credential", "reference": {
            "service": "codex_oauth", "accountId": "fixture"
        }}}
    }))
    .unwrap();
    assert_eq!(validate_connection(&connection), Ok(()));
}

#[cfg(unix)]
#[test]
fn cc_mcp_t04_host_executable_check_reads_metadata_without_launching() {
    use super::validation::validate_resolved_command;
    use std::{fs, os::unix::fs::PermissionsExt, path::Path};

    let root = tempfile::tempdir().unwrap();
    let program = root.path().join("fixture with spaces");
    let marker = root.path().join("fixture with spaces.ran");
    fs::write(&program, "#!/bin/sh\nprintf ran > \"$0.ran\"\n").unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o600)).unwrap();
    for path in [None, Some(Path::new("fixture")), Some(root.path()), Some(program.as_path())] {
        assert_eq!(validate_resolved_command(path), Err(ValidationError::CommandUnavailable));
    }
    fs::set_permissions(&program, fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(validate_resolved_command(Some(&program)), Ok(program.as_path()));
    assert!(!marker.exists());
}

#[test]
fn cc_mcp_t04_invalid_remote_definition_never_connects() {
    use std::{io::ErrorKind, net::TcpListener};

    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}/mcp", listener.local_addr().unwrap());
    for transport in ["http", "sse"] {
        let result = parse_native(json!({
            "type": transport,
            "url": url,
            "headers": {"X-Mode": "invalid\r\n"}
        }));
        assert_eq!(result.err(), Some(ValidationError::InvalidField("headers")));
    }
    assert_eq!(listener.accept().unwrap_err().kind(), ErrorKind::WouldBlock);
}
