use vw_api_types::{
    TenantScope,
    agent_apps_auth::AuthService,
    mcp_management::{
        McpCatalogScope, McpConfigValue, McpConnectionDefinition, McpCredentialRef, McpFailureCode,
    },
};

use super::{
    McpCatalogStore,
    catalog_create::catalog_location,
    catalog_create_tests::{http_draft, with_database},
};
use crate::gateway::settings_store::GatewaySettingsStore;

#[test]
fn cc_mcp_t09_masked_edit_preserves_same_field_reference() -> anyhow::Result<()> {
    with_database(|database| {
        let scope = McpCatalogScope::Tenant { scope: TenantScope::new("platform", "company")? };
        let catalog = McpCatalogStore::new(database.clone()).unwrap();
        let storage = GatewaySettingsStore::new(database)?;
        let (tenant, namespace) = catalog_location(&scope).unwrap();

        let reference = McpConfigValue::Credential {
            reference: McpCredentialRef {
                service: AuthService::ClaudeOfficial,
                account_id: "private-account-ref".into(),
            },
        };
        for (id, field, original, edited) in [
            ("stdio", "env", "ACCESS_TOKEN", "ACCESS_TOKEN"),
            ("http", "headers", "Authorization", "authorization"),
            ("sse", "headers", "Authorization", "authorization"),
        ] {
            let connection = |key: &str, value: McpConfigValue| {
                let fields = [(key.to_owned(), value)].into();
                match id {
                    "stdio" => McpConnectionDefinition::Stdio {
                        command: "/usr/bin/true".into(),
                        args: Vec::new(),
                        env: fields,
                        cwd: None,
                    },
                    "http" => McpConnectionDefinition::StreamableHttp {
                        url: "http://127.0.0.1:41001/mcp".into(),
                        headers: fields,
                    },
                    _ => McpConnectionDefinition::Sse {
                        url: "http://127.0.0.1:41001/sse".into(),
                        headers: fields,
                    },
                }
            };
            let mut draft = http_draft(id, "unused");
            draft.connection = connection(original, reference.clone());
            let created = catalog.create(&scope, draft).unwrap();
            let mask = || McpConfigValue::Literal { value: "••••••••".into() };
            let masks = catalog.credential_masks(&scope, &created.record).unwrap();
            let public = serde_json::to_string(&masks)?;
            assert!(public.contains("••••••••"));
            assert!(!public.contains("private-account-ref"));
            let updated = catalog
                .update_connection(
                    &scope,
                    &created.record,
                    connection(edited, masks[original].clone()),
                )
                .expect("masked edit must retain the saved reference");
            let payload = storage.get(tenant, &namespace, id)?.unwrap();
            let stored: serde_json::Value = serde_json::from_str(&payload)?;
            assert_eq!(stored["connection"][field][edited], serde_json::to_value(&reference)?);
            assert!(!payload.contains("••••••••"));
            assert!(!serde_json::to_string(&updated)?.contains("private-account-ref"));

            assert_eq!(
                catalog.update_connection(&scope, &updated.record, connection("unknown", mask())),
                Err(McpFailureCode::InvalidDefinition)
            );
            assert_eq!(storage.get(tenant, &namespace, id)?.as_deref(), Some(payload.as_str()));
        }
        Ok(())
    })
}

#[test]
fn cc_mcp_t09_raw_auth_fields_are_rejected_before_persisting() -> anyhow::Result<()> {
    with_database(|database| {
        let scope = McpCatalogScope::Tenant { scope: TenantScope::new("platform", "company")? };
        let catalog = McpCatalogStore::new(database.clone()).unwrap();
        let storage = GatewaySettingsStore::new(database)?;
        let (tenant, namespace) = catalog_location(&scope).unwrap();
        let mut draft = http_draft("raw", "https://example.com/mcp");
        let McpConnectionDefinition::StreamableHttp { headers, .. } = &mut draft.connection else {
            unreachable!("HTTP fixture");
        };
        headers.insert(
            "authorization".into(),
            McpConfigValue::Literal { value: "Bearer fixture-private".into() },
        );
        assert_eq!(catalog.create(&scope, draft), Err(McpFailureCode::InvalidDefinition));
        assert!(storage.list(tenant, &namespace)?.is_empty());

        let saved = catalog.create(&scope, http_draft("raw", "https://example.com/mcp")).unwrap();
        for url in [
            "https://user:fixture-private@example.com/mcp",
            "https://example.com/mcp?access%5ftoken=fixture-private",
        ] {
            assert_eq!(
                catalog.update_connection(&scope, &saved.record, http_draft("raw", url).connection),
                Err(McpFailureCode::InvalidDefinition)
            );
        }
        let connection = McpConnectionDefinition::Stdio {
            command: "/bin/sh".into(),
            args: vec![],
            env: [(
                "ACCESS_TOKEN".into(),
                McpConfigValue::Literal { value: "fixture-private".into() },
            )]
            .into(),
            cwd: None,
        };
        assert_eq!(
            catalog.update_connection(&scope, &saved.record, connection),
            Err(McpFailureCode::InvalidDefinition)
        );
        let payload = storage.get(tenant, &namespace, "raw")?.unwrap();
        assert!(!payload.contains("fixture-private"));
        assert_eq!(catalog.list(&scope), Ok(vec![saved]));
        Ok(())
    })
}

#[cfg(unix)]
#[test]
fn cc_mcp_t09_scoped_material_reaches_only_execution_transports() -> anyhow::Result<()> {
    use crate::agent_apps::auth_accounts::{
        AuthAccountMetadata, AuthAccountScope, AuthAccountStore,
    };
    use reqwest_012::header::AUTHORIZATION;
    use std::{
        net::TcpListener,
        os::unix::fs::PermissionsExt,
        process::{Command, Stdio},
        time::Duration,
    };
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use vw_agent_apps::auth::credential_file::{
        AccountCredential, CredentialFileStore, XaiCredential,
    };
    use vw_api_types::{TimestampMs, mcp_management::McpTransport};

    use super::McpExecutionConnection;

    const ACCESS: &str = "fixture-access-private";
    const REFRESH: &str = "fixture-refresh-private";
    const ID: &str = "scoped-account";

    with_database(|database| {
        let home = tempfile::tempdir()?;
        std::fs::set_permissions(home.path(), std::fs::Permissions::from_mode(0o700))?;
        let tenant = TenantScope::new("platform", "company")?;
        let scope = McpCatalogScope::Tenant { scope: tenant.clone() };
        let owner = AuthAccountScope {
            tenant: &tenant,
            principal: "oidc:owner",
            host: "local-host",
            service: AuthService::XaiOauth,
        };
        let accounts = AuthAccountStore::new(database.clone(), home.path()).unwrap();
        accounts
            .insert_metadata(
                &owner,
                ID,
                &AuthAccountMetadata::new("fixture-identity", TimestampMs(1)).unwrap(),
            )
            .unwrap();
        let credentials = CredentialFileStore::new(home.path()).unwrap();
        credentials
            .lease_xai(owner, ID, true)
            .unwrap()
            .save(&AccountCredential::XaiOauth(XaiCredential {
                access_token: ACCESS.into(),
                refresh_token: REFRESH.into(),
                expires_at: TimestampMs(i64::MAX),
                requires_reauth: false,
            }))
            .unwrap();
        let catalog = McpCatalogStore::new(database.clone()).unwrap();
        let storage = GatewaySettingsStore::new(database)?;
        let (_, namespace) = catalog_location(&scope).unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
        let reference = McpCredentialRef { service: owner.service, account_id: ID.into() };
        let credential = || McpConfigValue::Credential { reference: reference.clone() };

        for transport in [McpTransport::Stdio, McpTransport::StreamableHttp, McpTransport::Sse] {
            let listener = TcpListener::bind(("127.0.0.1", 0))?;
            let url = format!("http://{}/mcp", listener.local_addr()?);
            let headers = || [("Authorization".into(), credential())].into();
            let mut draft = http_draft("entry", &url);
            draft.id = match transport {
                McpTransport::Stdio => "stdio",
                McpTransport::StreamableHttp => "http",
                McpTransport::Sse => "sse",
            }
            .into();
            draft.connection = match transport {
                McpTransport::Stdio => McpConnectionDefinition::Stdio {
                    command: "/bin/sh".into(),
                    args: vec![
                        "-c".into(),
                        "test -n \"$ACCESS_TOKEN\" && test -z \"$REFRESH_TOKEN\"".into(),
                    ],
                    env: [("ACCESS_TOKEN".into(), credential())].into(),
                    cwd: None,
                },
                McpTransport::StreamableHttp => {
                    McpConnectionDefinition::StreamableHttp { url: url.clone(), headers: headers() }
                }
                McpTransport::Sse => {
                    McpConnectionDefinition::Sse { url: url.clone(), headers: headers() }
                }
            };
            let record = catalog.create(&scope, draft).unwrap();

            let resolve = |host, permit| {
                runtime.block_on(catalog.resolve_for_execution(
                    &scope,
                    &record.record,
                    host,
                    &accounts,
                    |_, candidate| if candidate == &reference { permit } else { None },
                ))
            };
            let other_tenant = TenantScope::new("platform", "other-company")?;
            for (host, permit) in [
                (owner.host, None),
                ("other-host", Some(owner)),
                ("other-host", Some(AuthAccountScope { host: "other-host", ..owner })),
                (owner.host, Some(AuthAccountScope { host: "other-host", ..owner })),
                (owner.host, Some(AuthAccountScope { tenant: &other_tenant, ..owner })),
                (owner.host, Some(AuthAccountScope { service: AuthService::CodexOauth, ..owner })),
                (owner.host, Some(AuthAccountScope { principal: "oidc:other", ..owner })),
            ] {
                let error = resolve(host, permit).err();
                assert_eq!(error, Some(McpFailureCode::PermissionDenied));
                let public_error = serde_json::to_string(&error)?;
                assert!(!public_error.contains(ACCESS) && !public_error.contains(REFRESH));
            }
            listener.set_nonblocking(true)?;
            assert_eq!(listener.accept().unwrap_err().kind(), std::io::ErrorKind::WouldBlock);
            match resolve(owner.host, Some(owner)).ok().expect("authorized execution") {
                McpExecutionConnection::Stdio { command, args, env, cwd } => {
                    assert_eq!(transport, McpTransport::Stdio);
                    assert!(cwd.is_none());
                    assert!(env["ACCESS_TOKEN"] == ACCESS);
                    let status = Command::new(command)
                        .args(args)
                        .env_clear()
                        .envs(env)
                        .stdin(Stdio::null())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .status()?;
                    assert!(status.success());
                }
                McpExecutionConnection::Http { transport: resolved, url, headers } => {
                    assert_eq!(resolved, transport);
                    assert!(headers[AUTHORIZATION].is_sensitive());
                    assert!(!format!("{headers:?}").contains(ACCESS));
                    let client = reqwest_012::Client::builder()
                        .no_proxy()
                        .timeout(Duration::from_secs(5))
                        .build()?;
                    let server = async move {
                        let listener = tokio::net::TcpListener::from_std(listener)?;
                        let (mut socket, _) = listener.accept().await?;
                        let mut reader = BufReader::new(&mut socket);
                        let mut authenticated = false;
                        loop {
                            let mut line = String::new();
                            let len = reader.read_line(&mut line).await?;
                            if len == 0 || line == "\r\n" {
                                break;
                            }
                            assert!(!line.contains(REFRESH));
                            authenticated |=
                                line.trim_end() == format!("authorization: Bearer {ACCESS}");
                        }
                        assert!(authenticated);
                        let (kind, body) = if transport == McpTransport::Sse {
                            ("text/event-stream", "event: endpoint\ndata: /message\n\n")
                        } else {
                            ("application/json", r#"{"jsonrpc":"2.0","id":1,"result":{}}"#)
                        };
                        let response = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                            body.len(),
                        );
                        socket.write_all(response.as_bytes()).await?;
                        Ok::<_, anyhow::Error>(())
                    };
                    let response = runtime.block_on(async {
                        let client = async {
                            Ok(client.get(url).headers(headers).send().await?.text().await?)
                        };
                        tokio::time::timeout(Duration::from_secs(5), async {
                            let ((), response) = tokio::try_join!(server, client)?;
                            Ok::<_, anyhow::Error>(response)
                        })
                        .await?
                    })?;
                    assert!(!response.contains(ACCESS) && !response.contains(REFRESH));
                }
            }
            let payload = storage.get(&tenant, &namespace, &record.record.id.0)?.unwrap();
            let projection = serde_json::to_string(&catalog.list(&scope).unwrap())?;
            assert!(payload.contains(ID));
            assert!(!projection.contains(ID));
            for text in [payload, projection] {
                assert!(!text.contains(ACCESS) && !text.contains(REFRESH));
            }
        }
        Ok(())
    })
}
