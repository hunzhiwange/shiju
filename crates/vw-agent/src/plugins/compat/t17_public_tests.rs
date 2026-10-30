//! Public authentication declaration parsing and safe connection policy.
use super::t17_tests::{captured, endpoint, list, options, plugin, write};
use crate::plugins::mcp::merge_enabled_plugin_mcp_servers;
use agent_client_protocol::{HttpHeader, McpServer, McpServerHttp};
use serde_json::json;
#[tokio::test]
async fn codex_plugin_compat_t17_accepts_public_auth_declarations_without_claiming_login() {
    for auth in [
        json!({"type":"none"}),
        json!({"type":"mixed","defaultScopes":[]}),
        json!({"type":"oauth","client":{"mode":"provided","clientId":"public-client","tokenEndpointAuthMethod":"none"},"authorizationUrl":"https://example.test/authorize","tokenUrl":"https://example.test/token","registrationUrl":"https://example.test/register","authorizationServerBase":"https://example.test","resource":"https://example.test/resource","baseScopes":["read"],"defaultScopes":[],"oidcEnabled":true,"oidcConfigurationUrl":"https://example.test/.well-known/openid-configuration","oidcUserinfoEndpoint":"https://example.test/userinfo","oidcScopesSupported":["openid"]}),
        json!({"type":"oauth","client":{"mode":"dcr"}}),
        json!({"type":"oauth","client":{"mode":"cimd"}}),
        json!({"type":"api_key","headerScheme":"bearer"}),
        json!({"type":"api_key","headerScheme":"basic"}),
        json!({"type":"api_key","headerScheme":"custom_header","headerName":"X-Fixture"}),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let options = options(temp.path());
        let (url, mut rx, task) = endpoint().await;
        let root = options.workspace_dir.join(".vibewindow/plugins/public");
        write(
            &root.join("plugin.json"),
            json!({"$schema":"https://agent-plugins.org/schemas/1.0.0/plugin.schema.json","name":"public"}),
        );
        write(
            &root.join("mcp.json"),
            json!({"$schema":super::mcp_portable::SCHEMA,"mcpServers":{"shared":{"type":"streamable-http","url":url,"extensions":{"com.openai":{"auth":auth}}}}}),
        );
        let mut servers = Vec::new();
        let result = merge_enabled_plugin_mcp_servers(&mut servers, &options);
        assert!(
            result.is_ok(),
            "public auth declaration must be parsed without granting login: {result:?}"
        );
        match auth["type"].as_str().unwrap() {
            "none" | "mixed" => {
                list(&servers[0]).await.unwrap();
                assert!(!captured(&mut rx).await.contains_key("authorization"));
            }
            "oauth" | "api_key" => {
                let error = list(&servers[0]).await.unwrap_err();
                assert!(format!("{error:#}").contains("separate"));
                assert!(
                    rx.try_recv().is_err(),
                    "declaration must not silently start authorization or send incomplete authentication"
                );
                if auth["type"] == "api_key" {
                    let mut configured = vec![McpServer::Http(
                        McpServerHttp::new("shared", &url)
                            .headers(vec![HttpHeader::new("X-Fixture", "host-connection")]),
                    )];
                    merge_enabled_plugin_mcp_servers(&mut configured, &options).unwrap();
                    list(&configured[0]).await.unwrap();
                    assert_eq!(
                        captured(&mut rx).await.get("x-fixture").map(String::as_str),
                        Some("host-connection"),
                        "explicit host connection remains authoritative"
                    );
                }
            }
            _ => unreachable!(),
        }
        let manifest = crate::plugins::load_plugin_manifest(&root).unwrap();
        let (outcome, _) = crate::plugins::mcp::load_plugin_mcp_declarations(&manifest).unwrap();
        let super::mcp_legacy::PluginMcpDeclaration::Portable { config, .. } = &outcome.servers[0]
        else {
            panic!("portable expected")
        };
        assert_eq!(
            config["extensions"]["com.openai"]["auth"], auth,
            "public fields and explicit empty scopes must remain intact"
        );
        task.abort();
    }
}

#[test]
fn codex_plugin_compat_t17_rejects_invalid_public_auth_without_values() {
    for (index, auth) in [
        json!({"type":"none","clientSecret":"fixture-only-secret"}),
        json!({"type":"hosted"}),
        json!({"type":"oauth","client":{"mode":"provided"}}),
        json!({"type":"oauth","client":{"mode":"provided","clientId":" ","tokenEndpointAuthMethod":"none"}}),
        json!({"type":"oauth","client":{"mode":"provided","clientId":"public","tokenEndpointAuthMethod":"unknown"}}),
        json!({"type":"oauth","client":{"mode":"dcr","tokenEndpointAuthMethod":"none"}}),
        json!({"type":"oauth","defaultScopes":[" "]}),
        json!({"type":"oauth","baseScopes":false}),
        json!({"type":"oauth","tokenUrl":"https://fixture-only-secret@example.test/token"}),
        json!({"type":"oauth","authorizationUrl":"https://example.test#fragment"}),
        json!({"type":"oauth","registrationUrl":"relative"}),
        json!({"type":"oauth","oidcEnabled":"fixture-only-secret"}),
        json!({"type":"oauth","defaultScopes":null}),
        json!({"type":"api_key","headerScheme":"custom_header"}),
        json!({"type":"api_key","headerScheme":"custom_header","headerName":"Host"}),
        json!({"type":"api_key","headerScheme":"custom_header","headerName":"invalid name"}),
        json!({"type":"api_key","headerScheme":"bearer","headerName":"X-Fixture"}),
        json!({"type":"api_key","headerScheme":"unknown"}),
        json!({"type":"api_key","headerScheme":"basic","token":"fixture-only-secret"}),
    ].into_iter().enumerate() {
        let temp = tempfile::tempdir().unwrap();
        let options = options(temp.path());
        plugin(
            &options,
            "local",
            json!({"url":"https://example.test/mcp","extensions":{"com.openai":{"auth":auth}}}),
        );
        let mut servers = Vec::new();
        let result = merge_enabled_plugin_mcp_servers(&mut servers, &options);
        assert!(result.is_err(), "public auth rejection case {index} must fail");
        let error = result.unwrap_err();
        assert!(!format!("{error:#}").contains("fixture-only-secret"));
        assert!(servers.is_empty());
    }
}
