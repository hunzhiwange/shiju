//! Public authentication declaration parsing and safe connection policy.
use super::t17_tests::{captured, endpoint, list, options, write};
use super::merge_enabled_plugin_mcp_servers;
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
            json!({"$schema":plugin_mcp::mcp_portable::SCHEMA,"mcpServers":{"shared":{"type":"streamable-http","url":url,"extensions":{"com.openai":{"auth":auth}}}}}),
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
        let manifest = crate::load_plugin_manifest(&root).unwrap();
        let (outcome, _) = crate::mcp::load_plugin_mcp_declarations(&manifest).unwrap();
        let plugin_mcp::mcp_legacy::PluginMcpDeclaration::Portable { config, .. } = &outcome.servers[0]
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
